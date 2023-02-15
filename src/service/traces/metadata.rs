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

use ahash::AHashMap;
use bytes::{BufMut, BytesMut};
use chrono::{TimeZone, Utc};
use serde_json::Value;
use tracing::info_span;

use crate::infra::file_lock;
use crate::{
    common::{
        json::{self, flatten_json},
        time::parse_timestamp_micro_from_value,
    },
    infra::{cluster, config::CONFIG},
    meta::StreamType,
};

pub async fn ingest(
    org_id: &str,
    stream_name: &str,
    thread_id: usize,
    records: Vec<serde_json::Map<String, Value>>,
) {
    let mut buf: AHashMap<String, Vec<String>> = AHashMap::new();
    let loc_span = info_span!("service:metadata:ingest");
    let _guard = loc_span.enter();
    if !cluster::is_ingester(&cluster::LOCAL_NODE_ROLE) {
        return;
    }
    for item in records.iter() {
        let value = serde_json::to_value(item).unwrap();
        //JSON Flattening
        let mut value = flatten_json(&value);
        // get json object
        let local_val = value.as_object_mut().unwrap();
        // handle timestamp
        let timestamp = match local_val.get(&CONFIG.common.time_stamp_col) {
            Some(v) => match parse_timestamp_micro_from_value(v) {
                Ok(t) => t,
                Err(_) => Utc::now().timestamp_micros(),
            },
            None => Utc::now().timestamp_micros(),
        };
        local_val.insert(
            CONFIG.common.time_stamp_col.clone(),
            Value::Number(timestamp.into()),
        );
        let trace_id = local_val.get("trace_id").unwrap().as_str().unwrap();
        let value_str = json::to_string(&local_val).unwrap();

        // get hour file name
        let hour_key = Utc
            .timestamp_nanos(timestamp * 1000)
            .format("%Y")
            .to_string();

        let key = format!("{}_00_00_00_tpre={}", hour_key, &trace_id[0..2]);

        let hour_buf = buf.entry(key).or_default();
        hour_buf.push(value_str.to_string());
    }

    // write to file
    let mut meta_file_name = "".to_string();
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
            thread_id,
            org_id,
            stream_name,
            StreamType::Metadata,
            &key,
            false,
        );
        if meta_file_name.is_empty() {
            meta_file_name = file.full_name();
        }
        file.write(write_buf.as_ref());
    }
}
