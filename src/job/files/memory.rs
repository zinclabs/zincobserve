// Copyright 2022 Zinc Labs Inc. and Contributors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use bytes::Bytes;
use datafusion::arrow;
use datafusion::arrow::json::reader::infer_json_schema;
use parquet::{arrow::ArrowWriter, file::properties::WriterProperties};
use std::io::BufReader;
use std::sync::Arc;
use tokio::{sync::Semaphore, task, time};

use crate::common::utils::populate_file_meta;
use crate::infra::cluster;
use crate::infra::config::{get_parquet_compression, CONFIG};
use crate::infra::file_lock;
use crate::infra::metrics;
use crate::infra::storage;
use crate::infra::storage::generate_partioned_file_key;
use crate::meta::common::FileMeta;
use crate::meta::StreamType;
use crate::service::db;
use crate::service::schema::schema_evolution;

/// TaskResult (local file path, remote file key, file meta, stream type)
type TaskResult = (String, String, FileMeta, StreamType);

pub async fn run() -> Result<(), anyhow::Error> {
    if !cluster::is_ingester(&cluster::LOCAL_NODE_ROLE) {
        return Ok(()); // not an ingester, no need to init job
    }

    let mut interval = time::interval(time::Duration::from_secs(CONFIG.limit.file_push_interval));
    interval.tick().await; // trigger the first run
    loop {
        interval.tick().await;
        if let Err(e) = move_files_to_storage().await {
            log::error!("Error moving memory files to remote: {}", e);
        }
    }
}

/*
 * upload compressed files to storage & delete moved files from local
 */
async fn move_files_to_storage() -> Result<(), anyhow::Error> {
    let files = file_lock::FILES.read().unwrap().clone();

    // use multiple threads to upload files
    let mut tasks: Vec<task::JoinHandle<Result<TaskResult, anyhow::Error>>> = Vec::new();

    let semaphore = std::sync::Arc::new(Semaphore::new(CONFIG.limit.file_move_thread_num));
    for (file, data) in files {
        let local_file = file.to_owned();
        let columns = local_file.split('/').collect::<Vec<&str>>();
        if columns.len() != 5 {
            continue;
        }
        let _ = columns[0].to_string();
        let org_id = columns[1].to_string();
        let stream_type: StreamType = StreamType::from(columns[2]);
        let stream_name = columns[3].to_string();
        let file_name = columns[4].to_string();

        log::info!("[JOB] convert memory file: {}", file);

        // check if we are allowed to ingest or just delete the file
        if db::compact::delete::is_deleting_stream(&org_id, &stream_name, stream_type, None) {
            log::info!(
                "[JOB] the stream [{}/{}/{}] is deleting, just delete file: {}",
                &org_id,
                stream_type,
                &stream_name,
                file
            );
            if file_lock::FILES.write().unwrap().remove(&file).is_none() {
                log::error!("[JOB] Failed to remove memory file: {}", file)
            }
            continue;
        }

        let mut partitions = file_name.split('_').collect::<Vec<&str>>();
        partitions.retain(|&x| x.contains('='));
        let mut partition_key = String::from("");
        for key in partitions {
            partition_key.push_str(key);
            partition_key.push('/');
        }

        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let task: task::JoinHandle<Result<(String, String, FileMeta, StreamType), anyhow::Error>> =
            task::spawn(async move {
                let ret = upload_file(
                    &org_id,
                    &stream_name,
                    stream_type,
                    &local_file,
                    data,
                    partition_key,
                )
                .await;
                drop(permit);
                match ret {
                    Ok((key, meta, stream_type)) => Ok((local_file, key, meta, stream_type)),
                    Err(e) => Err(e),
                }
            });
        tasks.push(task);
        task::yield_now().await;
    }

    for task in tasks {
        match task.await {
            Ok(ret) => match ret {
                Ok((path, key, meta, _stream_type)) => {
                    match db::file_list::local::set(&key, meta, false).await {
                        Ok(_) => {
                            if file_lock::FILES.write().unwrap().remove(&path).is_none() {
                                log::error!("[JOB] Failed to remove memory file: {}", path)
                            } else {
                                // println!("removed file: {}", key);
                                let columns = key.split('/').collect::<Vec<&str>>();
                                let _ = columns[0].to_string();
                                let org_id = columns[1].to_string();
                                let stream_type = columns[2].to_string();
                                let stream_name = columns[3].to_string();
                                metrics::INGEST_WAL_USED_BYTES
                                    .with_label_values(&[&org_id, &stream_name, &stream_type])
                                    .sub(meta.original_size as i64);
                            }
                        }
                        Err(e) => log::error!(
                            "[JOB] Failed write memory file meta:{}, error: {}",
                            path,
                            e.to_string()
                        ),
                    }
                }
                Err(e) => log::error!("[JOB] Error while uploading memory file to storage {}", e),
            },
            Err(e) => log::error!("[JOB] Error while uploading memory file to storage {}", e),
        };
    }

    Ok(())
}

async fn upload_file(
    org_id: &str,
    stream_name: &str,
    stream_type: StreamType,
    path_str: &str,
    buf: Bytes,
    partition_key: String,
) -> Result<(String, FileMeta, StreamType), anyhow::Error> {
    let file_size = buf.len() as u64;
    log::info!("[JOB] File upload begin: memory: {}", path_str);

    metrics::INGEST_WAL_READ_BYTES
        .with_label_values(&[org_id, stream_name, stream_type.to_string().as_str()])
        .inc_by(file_size);

    let mut schema_reader = BufReader::new(buf.as_ref());
    let inferred_schema = infer_json_schema(&mut schema_reader, None).unwrap();
    let arrow_schema = Arc::new(inferred_schema.clone());
    drop(schema_reader);

    let mut json_reader = BufReader::new(buf.as_ref());
    let json = arrow::json::Reader::new(
        &mut json_reader,
        arrow_schema.clone(),
        arrow::json::reader::DecoderOptions::new(),
    );

    let mut buf_parquet = Vec::new();
    let props = WriterProperties::builder()
        .set_compression(get_parquet_compression())
        .set_write_batch_size(8192)
        .set_data_pagesize_limit(1024 * 512)
        .set_max_row_group_size(1024 * 1024 * 256);
    let writer_props = props.build();
    let mut writer =
        ArrowWriter::try_new(&mut buf_parquet, arrow_schema.clone(), Some(writer_props)).unwrap();
    let mut meta_batch = vec![];
    for batch in json {
        let batch_write = batch.unwrap();
        writer.write(&batch_write).expect("Writing batch");
        meta_batch.push(batch_write);
    }
    writer.close().unwrap();

    //let file_name = path.file_name();
    let mut file_meta = FileMeta {
        min_ts: 0,
        max_ts: 0,
        records: 0,
        original_size: file_size,
        compressed_size: buf_parquet.len() as u64,
    };

    populate_file_meta(arrow_schema.clone(), vec![meta_batch], &mut file_meta).await;

    schema_evolution(
        org_id,
        stream_name,
        stream_type,
        inferred_schema,
        file_meta.min_ts,
    )
    .await;

    let new_file = generate_partioned_file_key(
        org_id,
        stream_name,
        stream_type,
        file_meta.min_ts,
        &CONFIG.common.file_ext_parquet,
    );

    let new_file_key = if partition_key.eq("") {
        format!("files/{}{}", new_file.0, new_file.1)
    } else {
        format!("files/{}{}{}", new_file.0, partition_key, new_file.1)
    };

    let storage = &storage::DEFAULT;
    let result = storage
        .put(&new_file_key, bytes::Bytes::from(buf_parquet))
        .await;
    match result {
        Ok(_output) => {
            log::info!("[JOB] memory file upload success: {}", new_file_key);
            Ok((new_file_key, file_meta, stream_type))
        }
        Err(err) => {
            log::error!("[JOB] memory file upload error: {:?}", err);
            Err(anyhow::anyhow!(err))
        }
    }
}
