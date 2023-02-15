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

use actix_web::{http, web, HttpResponse};
use ahash::AHashMap;
use bytes::{BufMut, BytesMut};
use chrono::{Duration, Utc};
use datafusion::arrow::datatypes::Schema;
#[cfg(feature = "zo_functions")]
use mlua::{Function, Lua};
use prometheus::GaugeVec;
use serde_json::Value;
use std::io::{BufRead, BufReader, Error};

use super::StreamMeta;
use crate::common::json;
use crate::infra::cluster;
use crate::infra::config::CONFIG;
use crate::infra::file_lock;
use crate::meta::alert::Alert;
#[cfg(feature = "zo_functions")]
use crate::meta::functions::Transform;
use crate::meta::http::HttpResponse as MetaHttpResponse;
use crate::meta::ingestion::{
    IngestionResponse, RecordStatus, StreamData, StreamSchemaChk, StreamStatus,
};
use crate::meta::StreamType;
use crate::service::schema::stream_schema_exists;
use crate::{common::time::parse_timestamp_micro_from_value, meta::alert::Trigger};

pub async fn ingest(
    org_id: &str,
    body: actix_web::web::Bytes,
    thread_id: web::Data<usize>,
    ingest_stats: web::Data<GaugeVec>,
) -> Result<HttpResponse, Error> {
    // let loc_span = info_span!("service:logs:bulk:ingest");
    // let _guard = loc_span.enter();
    if !cluster::is_ingester(&cluster::LOCAL_NODE_ROLE) {
        return Ok(
            HttpResponse::InternalServerError().json(MetaHttpResponse::error(
                http::StatusCode::INTERNAL_SERVER_ERROR.into(),
                Some("not an ingester".to_string()),
            )),
        );
    }
    let mut min_ts = (Utc::now() + Duration::hours(CONFIG.limit.allowed_upto)).timestamp_micros();
    #[cfg(feature = "zo_functions")]
    let lua = Lua::new();
    #[cfg(feature = "zo_functions")]
    let mut stream_lua_map: AHashMap<String, Function> = AHashMap::new();

    let mut stream_file_map: AHashMap<String, String> = AHashMap::new();
    let mut stream_schema_map: AHashMap<String, Schema> = AHashMap::new();
    let mut stream_data_map = AHashMap::new();
    #[cfg(feature = "zo_functions")]
    let mut stream_tansform_map: AHashMap<String, Vec<Transform>> = AHashMap::new();
    let mut stream_partition_keys_map: AHashMap<String, (StreamSchemaChk, Vec<String>)> =
        AHashMap::new();
    let mut stream_alerts_map: AHashMap<String, Vec<Alert>> = AHashMap::new();

    let mut stream_name = String::from("");
    let mut stream_trigger_map: AHashMap<String, Trigger> = AHashMap::new();

    let mut next_line_is_data = false;
    let reader = BufReader::new(body.as_ref());
    for line in reader.lines() {
        let line = line?;
        if line.is_empty() {
            continue;
        }
        #[cfg(feature = "zo_functions")]
        let mut value: Value = json::from_slice(line.as_bytes())?;
        #[cfg(not(feature = "zo_functions"))]
        let value: Value = json::from_slice(line.as_bytes())?;
        if !next_line_is_data {
            // check bulk operate
            if value.get("delete").is_some() {
                continue; // skip, we don't support delete
            }
            next_line_is_data = true;
            stream_name = super::get_stream_name(&value);

            // Start Register Transfoms for stream
            #[cfg(feature = "zo_functions")]
            let key = format!("{}/{}/{}", &org_id, StreamType::Logs, &stream_name);
            #[cfg(feature = "zo_functions")]
            super::get_stream_transforms(
                key,
                stream_name.clone(),
                &mut stream_tansform_map,
                &mut stream_lua_map,
                &lua,
                //scope,
            )
            .await;
            // End Register Transfoms for index

            // Start get stream alerts
            let key = format!("{}/{}", &org_id, &stream_name);
            super::get_stream_alerts(key, &mut stream_alerts_map).await;
            // End get stream alert

            if !stream_partition_keys_map.contains_key(&stream_name.clone()) {
                let stream_schema = stream_schema_exists(
                    org_id,
                    &stream_name,
                    StreamType::Logs,
                    &mut stream_schema_map,
                )
                .await;
                let mut partition_keys: Vec<String> = vec![];
                if stream_schema.has_partition_keys {
                    partition_keys = super::get_stream_partition_keys(
                        stream_name.clone(),
                        stream_schema_map.clone(),
                    )
                    .await;
                }
                stream_partition_keys_map
                    .insert(stream_name.clone(), (stream_schema, partition_keys.clone()));
            }

            stream_data_map
                .entry(stream_name.clone())
                .or_insert(StreamData {
                    data: AHashMap::new(),
                    status: RecordStatus {
                        successful: 0,
                        failed: 0,
                        error: "".to_string(),
                    },
                });

            stream_file_map
                .entry(stream_name.clone())
                .or_insert_with(|| String::from(""));
        } else {
            next_line_is_data = false;

            let stream_data = stream_data_map.get_mut(&stream_name).unwrap();
            let buf = &mut stream_data.data;
            let status = &mut stream_data.status;

            //Start row based transform
            #[cfg(feature = "zo_functions")]
            let key = format!("{}/{}/{}", &org_id, StreamType::Logs, &stream_name);
            #[cfg(feature = "zo_functions")]
            if let Some(transforms) = stream_tansform_map.get(&key) {
                for trans in transforms {
                    let func_key = format!("{}/{}", &stream_name, trans.name);
                    value =
                        super::lua_transform(&lua, &value, stream_lua_map.get(&func_key).unwrap());
                }
            }
            if value.is_null() || !value.is_object() {
                status.failed += 1; // transform failed or dropped
                continue;
            }
            //End row based transform

            //JSON Flattening
            let mut value = json::flatten_json(&value);
            // get json object
            let local_val = value.as_object_mut().unwrap();

            // Rename columns starting with @ Start
            for (key, entry) in local_val.clone() {
                if key.starts_with('@') {
                    local_val.remove(&key);
                    let new_key = key.replace('@', "_");
                    local_val.insert(new_key, entry);
                }
            }

            // handle timestamp
            let timestamp = match local_val.get(&CONFIG.common.time_stamp_col) {
                Some(v) => match parse_timestamp_micro_from_value(v) {
                    Ok(t) => t,
                    Err(e) => {
                        status.failed += 1;
                        status.error = e.to_string();
                        continue;
                    }
                },
                None => Utc::now().timestamp_micros(),
            };
            // check ingestion time
            let earlest_time = Utc::now() + Duration::hours(0 - CONFIG.limit.allowed_upto);
            if timestamp < earlest_time.timestamp_micros() {
                status.failed += 1; // to old data, just discard
                status.error = super::get_upto_discard_error();
                continue;
            }
            if timestamp < min_ts {
                min_ts = timestamp;
            }
            local_val.insert(
                CONFIG.common.time_stamp_col.clone(),
                Value::Number(timestamp.into()),
            );
            let partition_keys: Vec<String> = match stream_partition_keys_map.get(&stream_name) {
                Some((_, partition_keys)) => partition_keys.to_vec(),
                None => vec![],
            };

            let local_trigger = super::add_valid_record(
                StreamMeta {
                    org_id: org_id.to_string(),
                    stream_name: stream_name.clone(),
                    partition_keys,
                    stream_alerts_map: stream_alerts_map.clone(),
                },
                &mut stream_schema_map,
                status,
                buf,
                local_val,
            )
            .await;
            if local_trigger.is_some() {
                stream_trigger_map.insert(stream_name.clone(), local_trigger.unwrap());
            }
        }
    }

    let mut response_vec: Vec<StreamStatus> = Vec::new();
    for (stream_name, stream_data) in stream_data_map {
        // write to file
        let mut stream_file_name = "".to_string();
        let mut write_buf = BytesMut::new();
        for (key, entry) in stream_data.data {
            if entry.is_empty() {
                continue;
            }
            write_buf.clear();
            for row in &entry {
                write_buf.put(row.as_bytes());
                write_buf.put("\n".as_bytes());
            }
            let file = file_lock::get_or_create(
                *thread_id.as_ref(),
                org_id,
                &stream_name,
                StreamType::Logs,
                &key,
                CONFIG.common.wal_memory_mode_enabled,
            );
            if stream_file_name.is_empty() {
                stream_file_name = file.full_name();
            }
            file.write(write_buf.as_ref());

            // metrics
            ingest_stats
                .with_label_values(&[org_id, &stream_name, "records"])
                .add(entry.len() as f64);
            ingest_stats
                .with_label_values(&[org_id, &stream_name, "original_size"])
                .add(write_buf.len() as f64);
        }

        // metrics
        ingest_stats
            .with_label_values(&[org_id, &stream_name, "req_num"])
            .inc();

        response_vec.push(StreamStatus {
            name: stream_name.to_string(),
            status: stream_data.status,
        });
    }

    // only one trigger per request, as it updates etcd
    for (_, entry) in &stream_trigger_map {
        let mut alerts = stream_alerts_map
            .get(&format!("{}/{}", entry.org, entry.stream))
            .unwrap()
            .clone();

        alerts.retain(|alert| alert.name.eq(&entry.alert_name));
        if !alerts.is_empty() {
            super::send_ingest_notification(entry.clone(), alerts.first().unwrap().clone()).await;
        }
    }

    //dispose_v8();
    Ok(HttpResponse::Ok().json(IngestionResponse::new(
        http::StatusCode::OK.into(),
        response_vec,
    )))
}
