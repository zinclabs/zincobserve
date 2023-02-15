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

use std::{fs, path::Path};
use tokio::time;

use crate::common::file::scan_files;
use crate::infra::cluster;
use crate::infra::config::CONFIG;
use crate::infra::file_lock;
use crate::infra::storage;
use crate::meta::StreamType;

pub async fn run() -> Result<(), anyhow::Error> {
    if !cluster::is_ingester(&cluster::LOCAL_NODE_ROLE) {
        return Ok(()); // not an ingester, no need to init job
    }

    // create wal dir
    fs::create_dir_all(&CONFIG.common.data_wal_dir)?;

    let mut interval = time::interval(time::Duration::from_secs(CONFIG.limit.file_push_interval));
    interval.tick().await; // trigger the first run
    loop {
        interval.tick().await;
        if let Err(e) = move_file_list_to_storage().await {
            log::error!("Error moving file_list to remote: {}", e);
        }
    }
}

/*
 * upload compressed file_list to storage & delete moved files from local
 */
async fn move_file_list_to_storage() -> Result<(), anyhow::Error> {
    let data_dir = Path::new(&CONFIG.common.data_wal_dir)
        .canonicalize()
        .unwrap();

    let pattern = format!("{}/file_list/*.json", &CONFIG.common.data_wal_dir);
    let files = scan_files(&pattern);

    for file in files {
        let local_file = file.to_owned();
        let local_path = Path::new(&file).canonicalize().unwrap();
        let file_path = local_path
            .strip_prefix(&data_dir)
            .unwrap()
            .to_str()
            .unwrap()
            .replace('\\', "/");
        let columns = file_path.split('/').collect::<Vec<&str>>();
        // check file is in use
        if columns.len() != 2 {
            continue;
        }
        let file_name = columns[1].to_string();

        // check the file is using for write
        if file_lock::check_in_use("", "", StreamType::Filelist, &file_name) {
            continue;
        }
        log::info!("[JOB] convert file_list: {}", file);

        match upload_file(&local_file, &file_name).await {
            Ok(_) => match fs::remove_file(&local_file) {
                Ok(_) => {}
                Err(e) => {
                    log::error!(
                        "[JOB] Failed to remove file_list {}: {}",
                        local_file,
                        e.to_string()
                    )
                }
            },
            Err(e) => log::error!("[JOB] Error while uploading file_list to storage {}", e),
        }
    }
    Ok(())
}

async fn upload_file(path_str: &str, file_key: &str) -> Result<(), anyhow::Error> {
    let mut file = fs::File::open(path_str).unwrap();
    log::info!("[JOB] File_list upload begin: local: {}", path_str);

    let mut encoder = zstd::Encoder::new(Vec::new(), 3)?;
    std::io::copy(&mut file, &mut encoder)?;
    let compressed_bytes = encoder.finish().unwrap();

    let file_columns = file_key.split('_').collect::<Vec<&str>>();
    let new_file_key = format!(
        "file_list/{}/{}/{}/{}/{}.zst",
        file_columns[1], file_columns[2], file_columns[3], file_columns[4], file_columns[5]
    );

    let storage = &storage::DEFAULT;
    let result = storage
        .put(&new_file_key, bytes::Bytes::from(compressed_bytes))
        .await;
    match result {
        Ok(_output) => {
            log::info!("[JOB] File_list upload success: {}", new_file_key);
            Ok(())
        }
        Err(err) => {
            log::error!("[JOB] File_list upload error: {:?}", err);
            Err(anyhow::anyhow!(err))
        }
    }
}
