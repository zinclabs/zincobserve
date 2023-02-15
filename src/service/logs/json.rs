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
use std::io::Error;

use super::StreamMeta;
use crate::common::json;
use crate::common::time::parse_timestamp_micro_from_value;
use crate::infra::cluster;
use crate::infra::config::CONFIG;
#[cfg(feature = "zo_functions")]
use crate::infra::config::STREAM_FUNCTIONS;
use crate::infra::file_lock;
use crate::meta::alert::{Alert, Trigger};
#[cfg(feature = "zo_functions")]
use crate::meta::functions::Transform;
use crate::meta::http::HttpResponse as MetaHttpResponse;
use crate::meta::ingestion::{IngestionResponse, RecordStatus, StreamStatus};
use crate::meta::StreamType;
use crate::service::schema::stream_schema_exists;

pub async fn ingest(
    org_id: &str,
    stream_name: &str,
    body: actix_web::web::Bytes,
    thread_id: web::Data<usize>,
    ingest_stats: web::Data<GaugeVec>,
) -> Result<HttpResponse, Error> {
    // let loc_span = info_span!("service:logs:json:ingest");
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

    let mut stream_schema_map: AHashMap<String, Schema> = AHashMap::new();
    let mut stream_alerts_map: AHashMap<String, Vec<Alert>> = AHashMap::new();
    let mut stream_status = StreamStatus {
        name: stream_name.to_owned(),
        status: RecordStatus {
            successful: 0,
            failed: 0,
            error: "".to_string(),
        },
    };

    let mut trigger: Option<Trigger> = None;
    // Start Register Transfoms for stream
    #[cfg(feature = "zo_functions")]
    let mut local_tans: Vec<Transform> = vec![];
    #[cfg(feature = "zo_functions")]
    let key = format!("{}/{}/{}", &org_id, StreamType::Logs, &stream_name);
    #[cfg(feature = "zo_functions")]
    if let Some(transforms) = STREAM_FUNCTIONS.get(&key) {
        local_tans = (*transforms.list).to_vec();
        local_tans.sort_by(|a, b| a.order.cmp(&b.order));
        let mut func: Function;
        for trans in &local_tans {
            let func_key = format!("{}/{}", &stream_name, trans.name);
            func = super::load_lua_transform(&lua, trans.function.clone());
            stream_lua_map.insert(func_key, func.to_owned());
        }
    }
    // End Register Transfoms for stream

    let stream_schema = stream_schema_exists(
        org_id,
        stream_name,
        StreamType::Logs,
        &mut stream_schema_map,
    )
    .await;
    let mut partition_keys: Vec<String> = vec![];
    if stream_schema.has_partition_keys {
        partition_keys =
            super::get_stream_partition_keys(stream_name.to_string(), stream_schema_map.clone())
                .await;
    }

    // Start get stream alerts
    let key = format!("{}/{}", &org_id, &stream_name);
    super::get_stream_alerts(key, &mut stream_alerts_map).await;
    // End get stream alert

    let mut buf: AHashMap<String, Vec<String>> = AHashMap::new();
    let body_vec = body.to_vec();
    let reader: Vec<Value> = json::from_slice(&body_vec)?;
    for item in reader.iter() {
        #[cfg(feature = "zo_functions")]
        let mut value = item.to_owned();
        #[cfg(not(feature = "zo_functions"))]
        let value = item.to_owned();
        #[cfg(feature = "zo_functions")]
        //Start row based transform
        for trans in &local_tans {
            let func_key = format!("{}/{}", &stream_name, trans.name);
            value = super::lua_transform(&lua, &value, stream_lua_map.get(&func_key).unwrap());
        }
        if value.is_null() || !value.is_object() {
            stream_status.status.failed += 1; // transform failed or dropped
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
                    stream_status.status.failed += 1;
                    stream_status.status.error = e.to_string();
                    continue;
                }
            },
            None => Utc::now().timestamp_micros(),
        };
        // check ingestion time
        let earlest_time = Utc::now() + Duration::hours(0 - CONFIG.limit.allowed_upto);
        if timestamp < earlest_time.timestamp_micros() {
            stream_status.status.failed += 1; // to old data, just discard
            stream_status.status.error = super::get_upto_discard_error();
            continue;
        }
        if timestamp < min_ts {
            min_ts = timestamp;
        }
        local_val.insert(
            CONFIG.common.time_stamp_col.clone(),
            Value::Number(timestamp.into()),
        );

        let local_trigger = super::add_valid_record(
            StreamMeta {
                org_id: org_id.to_string(),
                stream_name: stream_name.to_string(),
                partition_keys: partition_keys.clone(),
                stream_alerts_map: stream_alerts_map.clone(),
            },
            &mut stream_schema_map,
            &mut stream_status.status,
            &mut buf,
            local_val,
        )
        .await;

        if local_trigger.is_some() {
            trigger = Some(local_trigger.unwrap());
        }
    }

    // write to file
    let mut stream_file_name = "".to_string();
    let mut write_buf = BytesMut::new();
    for (key, entry) in buf {
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
            stream_name,
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
            .with_label_values(&[org_id, stream_name, "records"])
            .add(entry.len() as f64);
        ingest_stats
            .with_label_values(&[org_id, stream_name, "original_size"])
            .add(write_buf.len() as f64);
    }

    if stream_file_name.is_empty() {
        return Ok(HttpResponse::Ok().json(IngestionResponse::new(
            http::StatusCode::OK.into(),
            vec![stream_status],
        )));
    }

    // only one trigger per request, as it updates etcd
    if trigger.is_some() {
        let val = trigger.unwrap();
        let mut alerts = stream_alerts_map
            .get(&format!("{}/{}", val.org, val.stream))
            .unwrap()
            .clone();

        alerts.retain(|alert| alert.name.eq(&val.alert_name));
        if !alerts.is_empty() {
            super::send_ingest_notification(val.clone(), alerts.first().unwrap().clone()).await;
        }
    }

    ingest_stats
        .with_label_values(&[org_id, stream_name, "req_num"])
        .inc();

    //Ok(HttpResponse::Ok().json(stream_status))
    Ok(HttpResponse::Ok().json(IngestionResponse::new(
        http::StatusCode::OK.into(),
        vec![stream_status],
    )))
}
