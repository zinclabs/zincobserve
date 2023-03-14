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
use arrow_schema::{DataType, Field};
use chrono::{TimeZone, Utc};
use datafusion::arrow::datatypes::Schema;
#[cfg(feature = "zo_functions")]
use mlua::{Function, Lua, LuaSerdeExt, Value as LuaValue};
use serde_json::{Map, Value};

use super::triggers;
use crate::common;
use crate::common::notification::send_notification;
#[cfg(feature = "zo_functions")]
use crate::infra::config::STREAM_FUNCTIONS;
use crate::infra::config::{CONFIG, STREAM_ALERTS};
use crate::meta::alert::{Alert, Evaluate, Trigger};
#[cfg(feature = "zo_functions")]
use crate::meta::functions::Transform;
use crate::meta::ingestion::RecordStatus;
use crate::meta::StreamType;
use crate::service::schema::check_for_schema;

pub mod bulk;
pub mod json;
pub mod multi;

pub(crate) fn get_upto_discard_error() -> String {
    format!(
        "too old data, by default only last {} hours data can be ingested. Data dscarded.",
        CONFIG.limit.ingest_allowed_upto
    )
}

fn get_stream_name(v: &Value) -> String {
    let local_val = v.as_object().unwrap();
    if local_val.contains_key("index") {
        String::from(
            local_val
                .get("index")
                .unwrap()
                .as_object()
                .unwrap()
                .get("_index")
                .unwrap()
                .as_str()
                .unwrap(),
        )
    } else {
        String::from(
            local_val
                .get("create")
                .unwrap()
                .as_object()
                .unwrap()
                .get("_index")
                .unwrap()
                .as_str()
                .unwrap(),
        )
    }
}

#[cfg(feature = "zo_functions")]
fn load_lua_transform(lua: &Lua, js_func: String) -> Function {
    lua.load(&js_func).eval().unwrap()
}
#[cfg(feature = "zo_functions")]
fn lua_transform(lua: &Lua, row: &Value, func: &Function) -> Value {
    let input = lua.to_value(&row).unwrap();
    let _res = func.call::<_, LuaValue>(input);
    match _res {
        Ok(res) => lua.from_value(res).unwrap(),
        Err(err) => {
            log::error!("Err from lua {:?}", err.to_string());
            row.clone()
        }
    }
}
#[cfg(feature = "zo_functions")]
async fn get_stream_transforms<'a>(
    key: String,
    stream_name: String,
    stream_tansform_map: &mut AHashMap<String, Vec<Transform>>,
    stream_lua_map: &mut AHashMap<String, Function<'a>>,
    lua: &'a Lua,
) {
    if stream_tansform_map.contains_key(&key) {
        return;
    }
    let transforms = STREAM_FUNCTIONS.get(&key);
    if transforms.is_none() {
        return;
    }

    let mut func: Function;
    let mut local_tans: Vec<Transform> = (*transforms.unwrap().list).to_vec();
    local_tans.sort_by(|a, b| a.order.cmp(&b.order));
    for trans in &local_tans {
        let func_key = format!("{}/{}", &stream_name, trans.name);
        func = load_lua_transform(lua, trans.function.clone());
        stream_lua_map.insert(func_key, func.to_owned());
    }
    stream_tansform_map.insert(key, local_tans.clone());
}

async fn get_stream_alerts<'a>(key: String, stream_alerts_map: &mut AHashMap<String, Vec<Alert>>) {
    if stream_alerts_map.contains_key(&key) {
        return;
    }
    let alerts_list = STREAM_ALERTS.get(&key);
    if alerts_list.is_none() {
        return;
    }
    let mut alerts = alerts_list.unwrap().list.clone();
    alerts.retain(|alert| alert.is_real_time);
    stream_alerts_map.insert(key, alerts);
}

async fn get_stream_partition_keys(
    stream_name: String,
    stream_schema_map: AHashMap<String, Schema>,
) -> Vec<String> {
    let mut keys: Vec<String> = vec![];
    if stream_schema_map.contains_key(&stream_name) {
        let schema = stream_schema_map.get(&stream_name).unwrap();
        let mut meta = schema.metadata().clone();
        meta.remove("created_at");
        let mut v: Vec<_> = meta.into_iter().collect();
        v.sort();
        for (_, value) in v {
            keys.push(value);
        }
    }
    keys
}

// generate partition key for the record
pub fn get_partition_key_str(s: &str) -> String {
    let s = s.replace(['/', '_'], ".");
    if s.len() > 100 {
        s[0..100].to_string()
    } else {
        s
    }
}

pub fn cast_to_type(mut value: Value, delta: Vec<Field>) -> Option<String> {
    let local_map = value.as_object_mut().unwrap();
    let mut parse_error = false;
    for field in delta {
        let field_map = local_map.get(field.name());
        if let Some(val) = field_map {
            if val.is_null() {
                local_map.insert(field.name().clone(), val.clone());
                continue;
            }
            let local_val = get_value(val);
            match field.data_type() {
                DataType::Boolean => {
                    match local_val.parse::<bool>() {
                        Ok(val) => {
                            local_map.insert(field.name().clone(), val.into());
                        }
                        Err(_) => parse_error = true,
                    };
                }
                DataType::Int8 => {
                    match local_val.parse::<i8>() {
                        Ok(val) => {
                            local_map.insert(field.name().clone(), val.into());
                        }
                        Err(_) => parse_error = true,
                    };
                }
                DataType::Int16 => {
                    match local_val.parse::<i16>() {
                        Ok(val) => {
                            local_map.insert(field.name().clone(), val.into());
                        }
                        Err(_) => parse_error = true,
                    };
                }
                DataType::Int32 => {
                    match local_val.parse::<i32>() {
                        Ok(val) => {
                            local_map.insert(field.name().clone(), val.into());
                        }
                        Err(_) => parse_error = true,
                    };
                }
                DataType::Int64 => {
                    match local_val.parse::<i64>() {
                        Ok(val) => {
                            local_map.insert(field.name().clone(), val.into());
                        }
                        Err(_) => parse_error = true,
                    };
                }
                DataType::UInt8 => {
                    match local_val.parse::<u8>() {
                        Ok(val) => {
                            local_map.insert(field.name().clone(), val.into());
                        }
                        Err(_) => parse_error = true,
                    };
                }
                DataType::UInt16 => {
                    match local_val.parse::<u16>() {
                        Ok(val) => {
                            local_map.insert(field.name().clone(), val.into());
                        }
                        Err(_) => parse_error = true,
                    };
                }
                DataType::UInt32 => {
                    match local_val.parse::<u32>() {
                        Ok(val) => {
                            local_map.insert(field.name().clone(), val.into());
                        }
                        Err(_) => parse_error = true,
                    };
                }
                DataType::UInt64 => {
                    match local_val.parse::<u64>() {
                        Ok(val) => {
                            local_map.insert(field.name().clone(), val.into());
                        }
                        Err(_) => parse_error = true,
                    };
                }
                DataType::Float16 => {
                    match local_val.parse::<f32>() {
                        Ok(val) => {
                            local_map.insert(field.name().clone(), val.into());
                        }
                        Err(_) => parse_error = true,
                    };
                }
                DataType::Float32 => {
                    match local_val.parse::<f32>() {
                        Ok(val) => {
                            local_map.insert(field.name().clone(), val.into());
                        }
                        Err(_) => parse_error = true,
                    };
                }
                DataType::Float64 => {
                    match local_val.parse::<f64>() {
                        Ok(val) => {
                            local_map.insert(field.name().clone(), val.into());
                        }
                        Err(_) => parse_error = true,
                    };
                }
                DataType::Utf8 => {
                    match local_val.parse::<String>() {
                        Ok(val) => {
                            local_map.insert(field.name().clone(), val.into());
                        }
                        Err(_) => parse_error = true,
                    };
                }
                _ => println!("{:?}", local_val),
            };
        }
    }
    if !parse_error {
        Some(common::json::to_string(&local_map).unwrap())
    } else {
        None
    }
}

pub fn get_value(value: &Value) -> String {
    if value.is_boolean() {
        value.as_bool().unwrap().to_string()
    } else if value.is_f64() {
        value.as_f64().unwrap().to_string()
    } else if value.is_i64() {
        value.as_i64().unwrap().to_string()
    } else if value.is_u64() {
        value.as_u64().unwrap().to_string()
    } else if value.is_string() {
        value.as_str().unwrap().to_string()
    } else {
        "".to_string()
    }
}

async fn add_valid_record(
    stream_meta: StreamMeta,
    stream_schema_map: &mut AHashMap<String, Schema>,
    status: &mut RecordStatus,
    buf: &mut AHashMap<String, Vec<String>>,
    local_val: &mut Map<String, Value>,
) -> Option<Trigger> {
    let mut trigger: Option<Trigger> = None;
    let timestamp: i64 = local_val
        .get(&CONFIG.common.time_stamp_col.clone())
        .unwrap()
        .as_i64()
        .unwrap();
    // get hour key
    let hour_key = get_hour_key(timestamp, stream_meta.partition_keys, local_val);
    let hour_buf = buf.entry(hour_key.clone()).or_default();

    let mut value_str = common::json::to_string(&local_val).unwrap();
    // check schema
    let (schema_conformance, delta_fields) = check_for_schema(
        &stream_meta.org_id,
        &stream_meta.stream_name,
        StreamType::Logs,
        &value_str,
        stream_schema_map,
        timestamp,
    )
    .await;

    if schema_conformance {
        let valid_record = if delta_fields.is_some() {
            let delta = delta_fields.unwrap();
            let loc_value: Value = common::json::from_slice(value_str.as_bytes()).unwrap();
            let ret_val = cast_to_type(loc_value, delta);
            if ret_val.is_some() {
                value_str = ret_val.unwrap();
                true
            } else {
                status.failed += 1;
                false
            }
        } else {
            true
        };

        if valid_record {
            if !stream_meta.stream_alerts_map.is_empty() {
                // Start check for alert trigger
                let key = format!("{}/{}", &stream_meta.org_id, &stream_meta.stream_name);
                if let Some(alerts) = stream_meta.stream_alerts_map.get(&key) {
                    for alert in alerts {
                        if alert.is_real_time {
                            let set_trigger = alert.condition.evaluate(local_val.clone());
                            if set_trigger {
                                // let _ = triggers::save_trigger(alert.name.clone(), trigger).await;
                                trigger = Some(Trigger {
                                    timestamp,
                                    is_valid: true,
                                    alert_name: alert.name.clone(),
                                    stream: stream_meta.stream_name.to_string(),
                                    org: stream_meta.org_id.to_string(),
                                    last_sent_at: 0,
                                    count: 0,
                                    is_ingest_time: true,
                                });
                            }
                        }
                    }
                }
                // End check for alert trigger
            }
            hour_buf.push(value_str);
            status.successful += 1;
        };
    } else {
        status.failed += 1;
    }
    trigger
}

fn get_hour_key(
    timestamp: i64,
    partition_keys: Vec<String>,
    local_val: &mut Map<String, Value>,
) -> String {
    // get hour file name
    let mut hour_key = Utc
        .timestamp_nanos(timestamp * 1000)
        .format("%Y_%m_%d_%H")
        .to_string();

    for key in &partition_keys {
        match local_val.get(key) {
            Some(v) => {
                let val = if v.is_string() {
                    format!("{}={}", key, v.as_str().unwrap())
                } else {
                    format!("{}={}", key, v)
                };
                hour_key.push_str(&format!("_{}", get_partition_key_str(&val)));
            }
            None => continue,
        };
    }
    hour_key
}

struct StreamMeta {
    org_id: String,
    stream_name: String,
    partition_keys: Vec<String>,
    stream_alerts_map: AHashMap<String, Vec<Alert>>,
}

pub async fn send_ingest_notification(mut trigger: Trigger, alert: Alert) {
    let _ = send_notification(&alert, &trigger.clone()).await;
    trigger.last_sent_at = Utc::now().timestamp_micros();
    trigger.count += 1;
    let _ = triggers::save_trigger(trigger.alert_name.clone(), trigger.clone()).await;
}
