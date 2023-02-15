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
use opentelemetry::trace::{SpanId, TraceId};
use opentelemetry_proto::tonic::common::v1::AnyValue;
use opentelemetry_proto::tonic::{
    collector::trace::v1::ExportTraceServiceRequest,
    collector::trace::v1::ExportTraceServiceResponse,
};
use prost::Message;
use serde_json::{json, Map, Value};
use std::fs::OpenOptions;
use std::io::Error;
use tracing::info_span;

use crate::infra::config::CONFIG;
use crate::infra::file_lock;
use crate::infra::storage::generate_partioned_file_key;
use crate::meta::traces::Event;
use crate::service::schema::{add_stream_schema, stream_schema_exists};
use crate::{
    common::json,
    infra::cluster,
    meta::{
        self,
        traces::{Span, SpanRefType},
        StreamType,
    },
};

pub mod metadata;
pub mod otlp_http;

const PARENT_SPAN_ID: &str = "reference.parent_span_id";
const PARENT_TRACE_ID: &str = "reference.parent_trace_id";
const REF_TYPE: &str = "reference.ref_type";
const SERVICE_NAME: &str = "service.name";
const SERVICE: &str = "service";

pub async fn handle_trace_request(
    org_id: &str,
    thread_id: std::sync::Arc<usize>,
    request: ExportTraceServiceRequest,
) -> Result<HttpResponse, Error> {
    let loc_span = info_span!("service:mod:handle_trace_request");
    let _guard = loc_span.enter();
    if !cluster::is_ingester(&cluster::LOCAL_NODE_ROLE) {
        return Ok(
            HttpResponse::InternalServerError().json(meta::http::HttpResponse::error(
                http::StatusCode::INTERNAL_SERVER_ERROR.into(),
                Some("not an ingester".to_string()),
            )),
        );
    }
    let traces_stream_name = "default";

    let mut trace_meta_coll: AHashMap<String, Vec<serde_json::Map<String, Value>>> =
        AHashMap::new();

    let mut data_buf: AHashMap<String, Vec<String>> = AHashMap::new();
    let mut traces_schema_map: AHashMap<String, Schema> = AHashMap::new();
    let mut min_ts = (Utc::now() + Duration::hours(CONFIG.limit.allowed_upto)).timestamp_micros();
    let mut service_name: String = traces_stream_name.to_string();
    let res_spans = request.resource_spans;
    for res_span in res_spans {
        let mut service_att_map: AHashMap<String, Value> = AHashMap::new();
        let resource = res_span.resource.unwrap();

        for res_attr in resource.attributes {
            if res_attr.key.eq(SERVICE_NAME) {
                let loc_service_name = get_val(res_attr.value);
                if !loc_service_name.eq(&Value::Null) {
                    service_name = loc_service_name.as_str().unwrap().to_string();
                    service_att_map.insert(res_attr.key, loc_service_name);
                }
            } else {
                service_att_map.insert(
                    format!("{}.{}", SERVICE, res_attr.key),
                    get_val(res_attr.value),
                );
            }
        }
        let inst_resources = res_span.instrumentation_library_spans;
        for inst_span in inst_resources {
            let spans = inst_span.spans;
            for span in spans {
                let span_id: String = SpanId::from_bytes(
                    span.span_id
                        .as_slice()
                        .try_into()
                        .expect("slice with incorrect length"),
                )
                .to_string();
                let trace_id: String = TraceId::from_bytes(
                    span.trace_id
                        .as_slice()
                        .try_into()
                        .expect("slice with incorrect length"),
                )
                .to_string();
                let mut span_ref = AHashMap::new();
                if !span.parent_span_id.is_empty() {
                    span_ref.insert(PARENT_TRACE_ID.to_string(), trace_id.clone());
                    span_ref.insert(
                        PARENT_SPAN_ID.to_string(),
                        SpanId::from_bytes(
                            span.parent_span_id
                                .as_slice()
                                .try_into()
                                .expect("slice with incorrect length"),
                        )
                        .to_string(),
                    );
                    span_ref.insert(REF_TYPE.to_string(), format!("{:?}", SpanRefType::ChildOf));
                }
                let start_time: u64 = span.start_time_unix_nano;
                let end_time: u64 = span.end_time_unix_nano;
                let mut span_att_map: AHashMap<String, Value> = AHashMap::new();
                for span_att in span.attributes {
                    span_att_map.insert(span_att.key, get_val(span_att.value));
                }

                let mut events = vec![];
                let mut event_att_map: AHashMap<String, Value> = AHashMap::new();
                for event in span.events {
                    for event_att in event.attributes {
                        event_att_map.insert(event_att.key, get_val(event_att.value));
                    }
                    events.push(Event {
                        name: event.name,
                        _timestamp: event.time_unix_nano,
                        attributes: event_att_map.clone(),
                    })
                }

                // get hour file name
                let mut hour_key = Utc
                    .timestamp_nanos(start_time.try_into().unwrap())
                    .format("%Y_%m_%d_%H")
                    .to_string();

                hour_key.push_str(&format!("_service={}", service_name.clone()));

                let hour_buf = data_buf.entry(hour_key.clone()).or_default();
                let timestamp = start_time / 1000;
                let local_val = Span {
                    trace_id: trace_id.clone(),
                    span_id,
                    span_kind: span.kind.to_string(),
                    operation_name: span.name.clone(),
                    start_time,
                    end_time,
                    duration: end_time - start_time,
                    reference: span_ref,
                    service_name: service_name.clone(),
                    attributes: span_att_map,
                    service: service_att_map.clone(),
                    flags: 1, //TODO add appropriate value
                    _timestamp: timestamp,
                    events: serde_json::to_string(&events).unwrap(),
                };
                let value_str = json::to_string(&local_val).unwrap();
                hour_buf.push(value_str);

                if timestamp < min_ts.try_into().unwrap() {
                    min_ts = timestamp as i64;
                }

                //Trace Metadata
                let mut trace_meta = Map::new();
                trace_meta.insert(
                    "trace_id".to_owned(),
                    serde_json::Value::String(trace_id.clone()),
                );
                trace_meta.insert("_timestamp".to_owned(), start_time.into());

                let hour_meta_buf = trace_meta_coll.entry(hour_key.clone()).or_default();
                hour_meta_buf.push(trace_meta);
            }
        }
    }

    let mut write_buf = BytesMut::new();
    for (key, entry) in data_buf {
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
            traces_stream_name,
            StreamType::Traces,
            &key,
            false,
        );
        let traces_file_name = file.full_name();

        file.write(write_buf.as_ref());

        let schema_exists = stream_schema_exists(
            org_id,
            traces_stream_name,
            StreamType::Traces,
            &mut traces_schema_map,
        )
        .await;
        if !schema_exists.has_fields && !traces_file_name.is_empty() {
            let file = OpenOptions::new()
                .read(true)
                .open(&traces_file_name)
                .unwrap();
            add_stream_schema(
                org_id,
                traces_stream_name,
                StreamType::Traces,
                &file,
                &mut traces_schema_map,
                min_ts,
            )
            .await;
        }
        let mut hour_meta_buf: Vec<serde_json::Map<String, Value>> =
            trace_meta_coll.get(&key).unwrap().to_vec();

        let dest_file = get_storage_file_name(&traces_file_name);
        for item in hour_meta_buf.iter_mut() {
            item.insert(
                "file_name".to_owned(),
                serde_json::Value::String(dest_file.clone()),
            );
        }

        //metadata::ingest(org_id, traces_stream_name, 0, hour_meta_buf.clone()).await;
    }

    let res = ExportTraceServiceResponse {};
    let mut out = BytesMut::with_capacity(res.encoded_len());
    res.encode(&mut out).expect("Out of memory");

    return Ok(HttpResponse::Ok()
        .status(http::StatusCode::OK)
        .content_type("application/x-protobuf")
        .body(out));
}

fn get_val(attr_val: Option<AnyValue>) -> Value {
    match attr_val {
        Some(local_val) => match local_val.value {
            Some(val) => match val {
                opentelemetry_proto::tonic::common::v1::any_value::Value::StringValue(
                    inner_val,
                ) => json!(inner_val.as_str()),
                opentelemetry_proto::tonic::common::v1::any_value::Value::BoolValue(inner_val) => {
                    json!(inner_val)
                }
                opentelemetry_proto::tonic::common::v1::any_value::Value::IntValue(inner_val) => {
                    json!(inner_val)
                }
                opentelemetry_proto::tonic::common::v1::any_value::Value::DoubleValue(
                    inner_val,
                ) => json!(inner_val),
                opentelemetry_proto::tonic::common::v1::any_value::Value::ArrayValue(inner_val) => {
                    let mut vals = vec![];
                    for item in inner_val.values.iter().cloned() {
                        vals.push(get_val(Some(item)))
                    }
                    json!(vals)
                }
                opentelemetry_proto::tonic::common::v1::any_value::Value::KvlistValue(
                    inner_val,
                ) => {
                    let mut vals = Map::new();
                    for item in inner_val.values.iter().cloned() {
                        vals.insert(item.key, get_val(item.value));
                    }
                    json!(vals)
                }
                opentelemetry_proto::tonic::common::v1::any_value::Value::BytesValue(inner_val) => {
                    json!(inner_val)
                }
            },
            None => Value::Null,
        },
        None => Value::Null,
    }
}

fn get_storage_file_name(local_path: &str) -> String {
    let file_path = local_path
        .strip_prefix(&CONFIG.common.data_wal_dir)
        .unwrap();
    let columns = file_path.split('/').collect::<Vec<&str>>();
    let _ = columns[0].to_string();
    let org_id = columns[1].to_string();
    let stream_type: StreamType = StreamType::from(columns[2]);
    let stream_name = columns[3].to_string();
    let file_name = columns[4].to_string();

    let new_file = generate_partioned_file_key(
        &org_id,
        &stream_name,
        stream_type,
        Utc::now().timestamp_micros(),
        &CONFIG.common.file_ext_parquet,
    );

    let mut partitions = file_name.split('_').collect::<Vec<&str>>();
    partitions.retain(|&x| x.contains('='));
    let mut partition_key = String::from("");
    for key in partitions {
        partition_key.push_str(key);
        partition_key.push('/');
    }

    if partition_key.eq("") {
        format!("{}{}", new_file.0, new_file.1)
    } else {
        format!("{}{}{}", new_file.0, partition_key, new_file.1)
    }
}
