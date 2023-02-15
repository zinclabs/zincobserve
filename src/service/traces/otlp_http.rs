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
use opentelemetry_proto::tonic::collector::trace::v1::ExportTraceServiceRequest;
use prost::Message;
use serde_json::{Map, Value};
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Error};
use tracing::info_span;

use super::get_storage_file_name;
use crate::infra::config::CONFIG;
use crate::infra::file_lock;
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

const PARENT_SPAN_ID: &str = "reference.parent_span_id";
const PARENT_TRACE_ID: &str = "reference.parent_trace_id";
const REF_TYPE: &str = "reference.ref_type";
const SERVICE_NAME: &str = "service.name";
const SERVICE: &str = "service";

pub async fn traces_proto(
    org_id: &str,
    thread_id: std::sync::Arc<usize>,
    body: actix_web::web::Bytes,
) -> Result<HttpResponse, Error> {
    let loc_span = info_span!("service:otlp_http:traces_proto");
    let _guard = loc_span.enter();
    //let thread = thread_id.into_inner();

    let request = ExportTraceServiceRequest::decode(body).expect("Invalid protobuf");
    super::handle_trace_request(org_id, thread_id, request).await
}

pub async fn traces_json(
    org_id: &str,
    thread_id: std::sync::Arc<usize>,
    body: actix_web::web::Bytes,
) -> Result<HttpResponse, Error> {
    let loc_span = info_span!("service:otlp_http:traces_json");
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
    let mut min_ts = (Utc::now() + Duration::hours(CONFIG.limit.allowed_upto)).timestamp_micros();
    let mut data_buf: AHashMap<String, Vec<String>> = AHashMap::new();
    let mut traces_schema_map: AHashMap<String, Schema> = AHashMap::new();
    let mut service_name: String = traces_stream_name.to_string();
    let reader = BufReader::new(body.as_ref());
    for line in reader.lines() {
        let line = line?;
        if line.is_empty() {
            continue;
        }

        let traces: Value = serde_json::from_slice(line.as_bytes()).unwrap();
        if traces.get("resourceSpans").is_some() {
            let res_spans = traces.get("resourceSpans").unwrap().as_array().unwrap();
            for res_span in res_spans {
                let mut service_att_map: AHashMap<String, Value> = AHashMap::new();
                if res_span.get("resource").is_some() {
                    let resource = res_span.get("resource").unwrap().as_object().unwrap();
                    if resource.get("attributes").is_some() {
                        let attributes = resource.get("attributes").unwrap().as_array().unwrap();
                        for res_attr in attributes {
                            let local_attr = res_attr.as_object().unwrap();
                            if local_attr
                                .get("key")
                                .unwrap()
                                .as_str()
                                .unwrap()
                                .eq(SERVICE_NAME)
                            {
                                let loc_service_name =
                                    local_attr.get("value").unwrap().as_object().unwrap();
                                for item in loc_service_name {
                                    service_name = item.1.as_str().unwrap().to_string();
                                    service_att_map
                                        .insert(SERVICE_NAME.to_string(), item.1.clone());
                                }
                            } else {
                                service_att_map.insert(
                                    format!(
                                        "{}.{}",
                                        SERVICE,
                                        local_attr.get("key").unwrap().as_str().unwrap()
                                    ),
                                    get_val_for_attr(local_attr.get("value").unwrap().clone()),
                                );
                            }
                        }
                    }
                }
                let scope_resources = res_span.get("scopeSpans");
                let inst_resources = if scope_resources.is_some() {
                    scope_resources.unwrap().as_array().unwrap()
                } else {
                    res_span
                        .get("instrumentationLibrarySpans")
                        .unwrap()
                        .as_array()
                        .unwrap()
                };
                for inst_span in inst_resources {
                    if inst_span.get("spans").is_some() {
                        let spans = inst_span.get("spans").unwrap().as_array().unwrap();
                        for span in spans {
                            let span_id: String =
                                span.get("spanId").unwrap().as_str().unwrap().to_string();
                            let trace_id: String =
                                span.get("traceId").unwrap().as_str().unwrap().to_string();

                            let mut span_ref = AHashMap::new();
                            if span.get("parentSpanId").is_some() {
                                span_ref.insert(
                                    PARENT_SPAN_ID.to_string(),
                                    span.get("parentSpanId")
                                        .unwrap()
                                        .as_str()
                                        .unwrap()
                                        .to_string(),
                                );
                                span_ref.insert(PARENT_TRACE_ID.to_string(), trace_id.clone());
                                span_ref.insert(
                                    REF_TYPE.to_string(),
                                    format!("{:?}", SpanRefType::ChildOf),
                                );
                            }

                            let start_time: u64 =
                                span.get("startTimeUnixNano").unwrap().as_u64().unwrap();
                            let end_time: u64 =
                                span.get("endTimeUnixNano").unwrap().as_u64().unwrap();
                            let mut span_att_map: AHashMap<String, Value> = AHashMap::new();
                            let attributes = span.get("attributes").unwrap().as_array().unwrap();
                            for span_att in attributes {
                                span_att_map.insert(
                                    span_att.get("key").unwrap().as_str().unwrap().to_string(),
                                    get_val_for_attr(span_att.get("value").unwrap().clone()),
                                );
                            }

                            let mut events = vec![];
                            let mut event_att_map: AHashMap<String, Value> = AHashMap::new();

                            let span_events = span.get("events").unwrap().as_array().unwrap();
                            for event in span_events {
                                let attributes =
                                    event.get("attributes").unwrap().as_array().unwrap();
                                for event_att in attributes {
                                    event_att_map.insert(
                                        event_att.get("key").unwrap().as_str().unwrap().to_string(),
                                        get_val_for_attr(event_att.get("value").unwrap().clone()),
                                    );
                                }
                                events.push(Event {
                                    name: event.get("name").unwrap().as_str().unwrap().to_string(),
                                    _timestamp: event
                                        .get("timeUnixNano")
                                        .unwrap()
                                        .as_u64()
                                        .unwrap(),
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
                                span_kind: span.get("kind").unwrap().to_string(),
                                operation_name: span
                                    .get("name")
                                    .unwrap()
                                    .as_str()
                                    .unwrap()
                                    .to_string(),
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
                            if timestamp < min_ts.try_into().unwrap() {
                                min_ts = timestamp as i64;
                            }
                            let value_str = json::to_string(&local_val).unwrap();
                            hour_buf.push(value_str);
                            //Trace Metadata
                            let mut trace_meta = Map::new();
                            trace_meta.insert(
                                "trace_id".to_owned(),
                                serde_json::Value::String(trace_id.clone()),
                            );
                            trace_meta.insert("_timestamp".to_owned(), start_time.into());

                            let hour_meta_buf =
                                trace_meta_coll.entry(hour_key.clone()).or_default();
                            hour_meta_buf.push(trace_meta);
                        }
                    }
                }
            }
        } else {
            return Ok(
                HttpResponse::BadRequest().json(meta::http::HttpResponse::error(
                    http::StatusCode::BAD_REQUEST.into(),
                    Some("Bad Request".to_string()),
                )),
            );
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

    Ok(HttpResponse::Ok().json(meta::http::HttpResponse::message(
        http::StatusCode::OK.into(),
        "request processed".to_string(),
    )))

    //Ok(HttpResponse::Ok().into())
}

/* fn _rename_keys(json: &mut serde_json::Value) {
    match json {
        serde_json::Value::Array(a) => a.iter_mut().for_each(_rename_keys),
        serde_json::Value::Object(o) => {
            let mut replace = serde_json::Map::with_capacity(o.len());
            o.retain(|k, v| {
                _rename_keys(v);
                replace.insert(
                    k.as_str().to_case(Case::Snake),
                    std::mem::replace(v, serde_json::Value::Null),
                );
                true
            });
            *o = replace;
        }
        _ => (),
    }
} */

fn get_val_for_attr(attr_val: Value) -> Value {
    let local_val = attr_val.as_object().unwrap();
    if let Some((_key, value)) = local_val.into_iter().next() {
        return value.clone();
    };
    ().into()
}
