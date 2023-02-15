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

use actix_web::{http, HttpResponse};
use ahash::AHashMap;
use bytes::{BufMut, BytesMut};
use chrono::{Duration, TimeZone, Utc};
use datafusion::arrow::datatypes::Schema;
use prost::Message;
use std::{collections::HashMap, fs::OpenOptions, io::Error};
use tracing::info_span;

use crate::infra::file_lock;
use crate::{
    common::{json, time::parse_i64_to_timestamp_micros},
    infra::{
        cluster,
        config::{CONFIG, METRIC_CLUSTER_LEADER, METRIC_CLUSTER_MAP},
    },
    meta::{
        self,
        prom::{ClusterLeader, Metric},
        StreamType,
    },
    service::{
        db,
        metrics::prometheus::WriteRequest,
        schema::{add_stream_schema, stream_schema_exists},
    },
};

pub mod prometheus {
    include!(concat!(env!("OUT_DIR"), "/prometheus.rs"));
}

const COUNTER: &str = "Counter";
const GAUGE: &str = "Gauge";
const HISTOGRAM: &str = "Histogram";
const TOTAL_SUFFIX: &str = "_total";
const SUM_SUFFIX: &str = "_sum";
const COUNT_SUFFIX: &str = "_count";
const BUCKET_SUFFIX: &str = "_bucket";
const MAIN_LABLE: &str = "__name__";
const CLUSTER_LABEL: &str = "cluster";
const REPLICA_LABEL: &str = "__replica__";

pub async fn prometheus_write_proto(
    org_id: &str,
    thread_id: actix_web::web::Data<usize>,
    body: actix_web::web::Bytes,
) -> Result<HttpResponse, Error> {
    let loc_span = info_span!("service:prom:prometheus_write_proto");
    let _guard = loc_span.enter();
    if !cluster::is_ingester(&cluster::LOCAL_NODE_ROLE) {
        return Ok(
            HttpResponse::InternalServerError().json(meta::http::HttpResponse::error(
                http::StatusCode::INTERNAL_SERVER_ERROR.into(),
                Some("not an ingester".to_string()),
            )),
        );
    }

    let mut min_ts = (Utc::now() + Duration::hours(CONFIG.limit.allowed_upto)).timestamp_micros();
    let dedup_enabled = CONFIG.common.metrics_dedup_enabled;
    let election_interval = CONFIG.limit.metrics_leader_election_interval * 1000000;
    let mut last_received: i64 = 0;
    let mut has_entry = false;
    let mut accept_record = false;
    let mut cluster_name: String = String::new();
    let mut metric_data_map: AHashMap<String, HashMap<String, Vec<String>>> = AHashMap::new();
    let mut metric_file_map: AHashMap<String, String> = AHashMap::new();
    let mut metric_schema_map: AHashMap<String, Schema> = AHashMap::new();

    let decoded = snap::raw::Decoder::new()
        .decompress_vec(&body)
        .expect("Invalid snappy compressed data");
    let request = WriteRequest::decode(bytes::Bytes::from(decoded)).expect("Invalid protobuf");
    let mut i = 0;
    for event in request.timeseries {
        let mut replica_label = String::new();
        let mut main_lable = event.labels.clone();
        main_lable.retain(|x| x.name.eq(MAIN_LABLE));
        let buf = metric_data_map
            .entry(main_lable[0].value.clone())
            .or_default();

        metric_file_map
            .entry(main_lable[0].value.clone())
            .or_insert_with(|| String::from(""));

        let mut optional_lables = event.labels;
        optional_lables.retain(|x| !x.name.eq("MAIN_LABLE"));

        let mut map = AHashMap::new();
        let mut contains_le = false;
        if optional_lables.len() > 1 {
            for label in optional_lables.clone() {
                if label.name.eq("le") {
                    contains_le = true;
                } else if !has_entry && label.name.eq(REPLICA_LABEL) {
                    replica_label = label.value.clone();
                } else if !has_entry && label.name.eq(CLUSTER_LABEL) && cluster_name.is_empty() {
                    cluster_name = format!("{}/{}", org_id, label.value);
                }
                map.insert(label.name, label.value);
            }
        }
        let metric_type = get_metric_type(main_lable[0].value.clone(), contains_le);

        for sample in event.samples {
            let mut abnormal_val = false;
            let mut sample_val = sample.value;
            //revisit in future
            if sample_val.is_infinite() {
                if sample_val == f64::INFINITY || sample_val > f64::MAX {
                    sample_val = f64::MAX;
                } else if sample_val == f64::NEG_INFINITY || sample_val < f64::MIN {
                    sample_val = f64::MIN;
                }
            } else if sample_val.is_nan() {
                // skip the entry from adding to store
                abnormal_val = true;
                sample_val = 0.0;
            }
            let mut timestamp = parse_i64_to_timestamp_micros(sample.timestamp).unwrap();
            if timestamp == 0 {
                timestamp = Utc::now().timestamp_micros();
            }
            if timestamp < min_ts {
                min_ts = timestamp;
            }
            let metric = Metric {
                name: main_lable[0].value.clone(),
                value: sample_val,
                collection: map.clone(),
                _timestamp: timestamp,
                metric_type: metric_type.clone(),
            };
            let value_str = json::to_string(&metric).unwrap();
            // get hour file name
            let hour_key = Utc
                .timestamp_nanos(timestamp * 1000)
                .format("%Y_%m_%d_%H")
                .to_string();
            let hour_buf = buf.entry(hour_key).or_default();
            if i == 0 && dedup_enabled {
                let leader = METRIC_CLUSTER_LEADER.get(&cluster_name);
                match leader {
                    Some(leader) => {
                        last_received = leader.last_received;
                        has_entry = true;
                    }
                    None => {
                        has_entry = false;
                    }
                }
                accept_record = prom_ha_handler(
                    has_entry,
                    cluster_name.clone(),
                    replica_label.clone(),
                    last_received,
                    election_interval,
                )
                .await;
                has_entry = true;
            }
            if !accept_record {
                //do not accept any entried for request
                return Ok(HttpResponse::Ok().into());
            } else if !abnormal_val {
                hour_buf.push(value_str);
            }
            i += 1;
        }
    }

    /*  for item in METRIC_CLUSTER_MAP.iter() {
        log::info!(
            "Cluster members for {:?} --> {:?}",
            item.key(),
            item.value()
        );
    }
      for item in METRIC_CLUSTER_LEADER.iter() {
        log::info!("Cluster leader for {:?} --> {:?}", item.key(), item.value());
    } */

    for (metric_name, metric_data) in metric_data_map {
        // write to file
        let mut metric_file_name = "".to_string();
        let mut write_buf = BytesMut::new();
        for (key, entry) in metric_data {
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
                &metric_name,
                StreamType::Metrics,
                &key,
                false,
            );
            if metric_file_name.is_empty() {
                metric_file_name = file.full_name();
            }
            file.write(write_buf.as_ref());
        }

        let schema_exists = stream_schema_exists(
            org_id,
            &metric_name,
            StreamType::Metrics,
            &mut metric_schema_map,
        )
        .await;
        if !schema_exists.has_fields && !metric_file_name.is_empty() {
            let file = OpenOptions::new()
                .read(true)
                .open(&metric_file_name)
                .unwrap();
            add_stream_schema(
                org_id,
                &metric_name,
                StreamType::Metrics,
                &file,
                &mut metric_schema_map,
                min_ts,
            )
            .await;
        }
    }
    Ok(HttpResponse::Ok().into())
}

fn get_metric_type(main_lable: String, contains_le: bool) -> String {
    if main_lable.ends_with(TOTAL_SUFFIX)
        || main_lable.ends_with(SUM_SUFFIX)
        || main_lable.ends_with(COUNT_SUFFIX)
    {
        COUNTER.to_string()
    } else if main_lable.ends_with(BUCKET_SUFFIX) && contains_le {
        HISTOGRAM.to_string()
    } else {
        GAUGE.to_string()
    }
}

async fn prom_ha_handler(
    has_entry: bool,
    cluster_name: String,
    replica_label: String,
    last_received: i64,
    election_interval: i64,
) -> bool {
    let mut _accept_record = false;
    let curr_ts = Utc::now().timestamp_micros();
    if !has_entry {
        METRIC_CLUSTER_MAP.insert(cluster_name.clone(), vec![]);
        log::info!(
            "Making {} leader for {} ",
            replica_label.clone(),
            cluster_name.clone(),
        );
        METRIC_CLUSTER_LEADER.insert(
            cluster_name.clone(),
            ClusterLeader {
                name: replica_label.clone(),
                last_received: curr_ts,
            },
        );
        _accept_record = true;
    } else {
        let mut leader = METRIC_CLUSTER_LEADER
            .get_mut(&cluster_name.clone())
            .unwrap();
        if replica_label.eq(&leader.name) {
            _accept_record = true;
            leader.last_received = curr_ts;
            log::info!(
                "Updating last received data for {} to {} ",
                &leader.name,
                Utc.timestamp_nanos(last_received * 1000)
            );
        } else if curr_ts - last_received > election_interval {
            //elect new leader as didnt receive data for last 30 secs
            log::info!(
                "Electing {} new leader for {} as last received data from {} at {} ",
                replica_label.clone(),
                cluster_name,
                &leader.name,
                Utc.timestamp_nanos(last_received * 1000)
            );
            leader.name = replica_label.clone();
            leader.last_received = curr_ts;
            _accept_record = true;
        } else {
            log::info!(
                "Rejecting entry from {}  as leader is {}",
                replica_label,
                &leader.name,
            );
            _accept_record = false;
        }
    }
    let mut replica_list = vec![];
    if METRIC_CLUSTER_MAP.contains_key(&cluster_name) {
        replica_list = METRIC_CLUSTER_MAP.get_mut(&cluster_name).unwrap().to_vec();
        if !replica_list.contains(&replica_label.clone()) {
            replica_list.push(replica_label.clone());
            METRIC_CLUSTER_MAP.insert(cluster_name.clone(), replica_list.clone());
            let _ = db::set_prom_cluster_info(&cluster_name, replica_list.to_vec()).await;
        }
    } else {
        replica_list.push(replica_label.clone());
        METRIC_CLUSTER_MAP.insert(cluster_name.clone(), replica_list.clone());
        let _ = db::set_prom_cluster_info(&cluster_name, replica_list.to_vec()).await;
    }

    _accept_record
}

#[cfg(test)]
mod test_utils {
    use super::*;
    #[test]
    fn test_get_metric_type() {
        let htype = get_metric_type(
            "apiserver_admission_controller_admission_duration_seconds_bucket".to_string(),
            true,
        );

        assert_eq!(htype.as_str(), HISTOGRAM);
    }
}
