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
use datafusion::arrow::datatypes::{DataType, Field, Schema};
use datafusion::arrow::error::ArrowError;
use datafusion::arrow::json::reader::infer_json_schema;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufReader, Seek, SeekFrom};
use tracing::info_span;

use crate::infra::config::CONFIG;
use crate::meta::{ingestion::StreamSchemaChk, StreamType};
use crate::service::db;

pub async fn schema_evolution(
    org_id: &str,
    stream_name: &str,
    stream_type: StreamType,
    inferred_schema: Schema,
    min_ts: i64,
) {
    let loc_span = info_span!("service:schema:schema_evolution");
    let _guard = loc_span.enter();

    let schema = db::schema::get(org_id, stream_name, Some(stream_type))
        .await
        .unwrap();

    if schema == Schema::empty() {
        let mut metadata = HashMap::new();
        metadata.insert("created_at".to_string(), min_ts.to_string());
        log::info!("schema_evolution: setting schema for {:?}", stream_name);
        db::schema::set(
            org_id,
            stream_name,
            stream_type,
            &inferred_schema.clone().with_metadata(metadata),
            Some(min_ts),
        )
        .await
        .unwrap();
    } else if !inferred_schema.fields().eq(schema.fields()) {
        let schema_fields: HashSet<_> = schema.fields().iter().collect();
        let field_datatype_delta: Vec<_> = inferred_schema
            .clone()
            .fields
            .into_iter()
            .filter(|item| !schema_fields.contains(item))
            .map(|v| format!("{}:[{}]", v.name(), v.data_type()))
            .collect();
        if field_datatype_delta.is_empty() {
            return;
        }
        log::info!(
            "schema_evolution: updating schema for {:?} delta is {:?}",
            stream_name,
            field_datatype_delta
        );
        match try_merge(vec![schema.clone(), inferred_schema.clone()]) {
            Err(e) => {
                log::error!(
                    "schema_evolution: schema merge failed for {:?} err: {:?}",
                    stream_name,
                    e
                );
            }
            Ok(merged) => {
                db::schema::set(org_id, stream_name, stream_type, &merged, Some(min_ts))
                    .await
                    .unwrap();
            }
        };
    }
}

// Hack to allow widening conversion , method overrides Schema::try_merge
fn try_merge(schemas: impl IntoIterator<Item = Schema>) -> Result<Schema, ArrowError> {
    let mut merged_metadata: HashMap<String, String> = HashMap::new();
    let mut merged_fields: Vec<Field> = Vec::new();
    // TODO : this dummy initilization is to avoid compilar complaining for unintilized value
    let mut temp_field = Field::new("dummy", DataType::Utf8, false);

    for schema in schemas {
        for (key, value) in schema.metadata() {
            // merge metadata
            if let Some(old_val) = merged_metadata.get(key) {
                if old_val != value {
                    return Err(ArrowError::SchemaError(
                        "Fail to merge schema due to conflicting metadata.".to_string(),
                    ));
                }
            }
            merged_metadata.insert(key.to_string(), value.to_string());
        }
        // merge fields
        let mut found_at = 0;
        for field in schema.fields().iter() {
            let mut new_field = true;
            let mut allowed = false;
            for (stream, mut merged_field) in merged_fields.iter_mut().enumerate() {
                if field.name() != merged_field.name() {
                    continue;
                }
                new_field = false;
                if merged_field.data_type() != field.data_type() {
                    if !CONFIG.common.widening_schema_evoluation {
                        return Err(ArrowError::SchemaError(format!(
                            "Fail to merge schema due to conflicting data type[{}:{}].",
                            merged_field.data_type(),
                            field.data_type()
                        )));
                    }
                    allowed = is_widening_conversion(merged_field.data_type(), field.data_type());
                    if allowed {
                        temp_field = Field::new(
                            merged_field.name(),
                            field.data_type().to_owned(),
                            merged_field.is_nullable(),
                        );
                        merged_field = &mut temp_field;
                    }
                }
                found_at = stream;
                merged_field.try_merge(field)?;
            }
            // found a new field, add to field list
            if new_field {
                merged_fields.push(field.clone());
            }
            if allowed {
                let _ = std::mem::replace(&mut merged_fields[found_at], temp_field.to_owned());
            }
        }
    }
    let merged = Schema::new_with_metadata(merged_fields, merged_metadata);
    Ok(merged)
}

fn is_widening_conversion(from: &DataType, to: &DataType) -> bool {
    let allowed_type = match from {
        DataType::Boolean => vec![DataType::Utf8],
        DataType::Int8 => vec![
            DataType::Utf8,
            DataType::Int16,
            DataType::Int32,
            DataType::Int64,
        ],
        DataType::Int16 => vec![DataType::Utf8, DataType::Int32, DataType::Int64],
        DataType::Int32 => vec![DataType::Utf8, DataType::Int64],
        DataType::Int64 => vec![DataType::Utf8],
        DataType::UInt8 => vec![
            DataType::Utf8,
            DataType::UInt16,
            DataType::UInt32,
            DataType::UInt64,
        ],
        DataType::UInt16 => vec![DataType::Utf8, DataType::UInt32, DataType::UInt64],
        DataType::UInt32 => vec![DataType::Utf8, DataType::UInt64],
        DataType::UInt64 => vec![DataType::Utf8],
        DataType::Float16 => vec![DataType::Utf8, DataType::Float32, DataType::Float64],
        DataType::Float32 => vec![DataType::Utf8, DataType::Float64],
        DataType::Float64 => vec![DataType::Utf8],
        _ => vec![DataType::Utf8],
    };
    allowed_type.contains(to)
}

pub async fn check_for_schema(
    org_id: &str,
    stream_name: &str,
    stream_type: StreamType,
    val_str: &str,
    stream_schema_map: &mut AHashMap<String, Schema>,
    record_ts: i64,
) -> (bool, Option<Vec<Field>>) {
    let schema = if stream_schema_map.contains_key(stream_name) {
        stream_schema_map.get(stream_name).unwrap().clone()
    } else {
        let schema = db::schema::get(org_id, stream_name, Some(stream_type))
            .await
            .unwrap();
        stream_schema_map.insert(stream_name.to_string(), schema.clone());
        schema
    };

    let mut schema_reader = BufReader::new(val_str.as_bytes());
    let inferred_schema = infer_json_schema(&mut schema_reader, None).unwrap();
    if schema.fields.eq(&inferred_schema.fields) {
        return (true, None);
    }

    if schema == Schema::empty() {
        stream_schema_map.insert(stream_name.to_string(), inferred_schema.clone());
        db::schema::set(
            org_id,
            stream_name,
            stream_type,
            &inferred_schema,
            Some(record_ts),
        )
        .await
        .unwrap();
        return (true, None);
    }

    let inferred_fields: HashSet<_> = inferred_schema.fields().iter().collect();
    let field_datatype_delta: Vec<_> = schema
        .clone()
        .fields
        .into_iter()
        .filter(|item| !inferred_fields.contains(item))
        .collect();
    if !CONFIG.common.widening_schema_evoluation {
        return (true, Some(field_datatype_delta));
    }

    match try_merge(vec![schema.clone(), inferred_schema.clone()]) {
        Err(ref _e) => (true, Some(field_datatype_delta)), //return (false, None),
        Ok(merged) => {
            log::info!("Schema widening for stream {:?}", stream_name);
            db::schema::set(org_id, stream_name, stream_type, &merged, Some(record_ts))
                .await
                .unwrap();
            let item_set: HashSet<_> = schema.fields.iter().collect();
            let field_datatype_delta: Vec<_> = inferred_schema
                .clone()
                .fields
                .into_iter()
                .filter(|item| !item_set.contains(item))
                .collect();
            stream_schema_map.insert(stream_name.to_string(), merged);
            (true, Some(field_datatype_delta))
        }
    }
}

pub async fn stream_schema_exists(
    org_id: &str,
    stream_name: &str,
    stream_type: StreamType,
    stream_schema_map: &mut AHashMap<String, Schema>,
) -> StreamSchemaChk {
    let mut schema_chk = StreamSchemaChk {
        conforms: true,
        has_fields: false,
        has_partition_keys: false,
    };
    let schema;
    if stream_schema_map.contains_key(stream_name) {
        schema = stream_schema_map.get(stream_name).unwrap().clone();
    } else {
        schema = db::schema::get(org_id, stream_name, Some(stream_type))
            .await
            .unwrap();
        stream_schema_map.insert(stream_name.to_string(), schema.clone());
    }
    let fields = schema.fields();
    let mut meta = schema.metadata().clone();

    if !fields.is_empty() {
        schema_chk.has_fields = true;
    }
    if !meta.is_empty() {
        meta.remove("created_at");
        if !meta.is_empty() {
            schema_chk.has_partition_keys = true;
        }
    }
    schema_chk
}

pub async fn add_stream_schema(
    org_id: &str,
    stream_name: &str,
    stream_type: StreamType,
    file: &File,
    stream_schema_map: &mut AHashMap<String, Schema>,
    min_ts: i64,
) {
    let loc_span = info_span!("service:schema:add_stream_schema");
    let _guard = loc_span.enter();

    let mut local_file = file;
    local_file.seek(SeekFrom::Start(0)).unwrap();
    let mut schema_reader = BufReader::new(local_file);
    let inferred_schema = infer_json_schema(&mut schema_reader, None).unwrap();

    let existing_schema = stream_schema_map.get(&stream_name.to_string());
    let mut metadata = match existing_schema {
        Some(schema) => schema.metadata().clone(),
        None => HashMap::new(),
    };
    metadata.insert("created_at".to_string(), min_ts.to_string());
    db::schema::set(
        org_id,
        stream_name,
        stream_type,
        &inferred_schema.clone().with_metadata(metadata),
        Some(min_ts),
    )
    .await
    .unwrap();
    stream_schema_map.insert(stream_name.to_string(), inferred_schema.clone());
}

#[cfg(test)]
mod test {
    use super::*;
    use ahash::AHashMap;
    use datafusion::arrow::datatypes::{DataType, Field, Schema};
    #[test]
    fn test_is_widening_conversion() {
        let res = is_widening_conversion(&DataType::Int8, &DataType::Int32);
        assert_eq!(res, true);
    }
    #[test]
    fn test_try_merge() {
        let merged = try_merge(vec![
            Schema::new(vec![
                Field::new("c1", DataType::Int64, false),
                Field::new("c2", DataType::Utf8, false),
            ]),
            Schema::new(vec![
                Field::new("c1", DataType::Int64, true),
                Field::new("c2", DataType::Utf8, false),
                Field::new("c3", DataType::Utf8, false),
            ]),
        ])
        .unwrap();

        assert_eq!(
            merged,
            Schema::new(vec![
                Field::new("c1", DataType::Int64, true),
                Field::new("c2", DataType::Utf8, false),
                Field::new("c3", DataType::Utf8, false),
            ]),
        );
    }

    #[actix_web::test]
    async fn test_check_for_schema() {
        let stream_name = "Sample";
        let org_name = "nexus";
        let record = r#"{"Year": 1896, "City": "Athens", "_timestamp": 1234234234234}"#;

        let schema = Schema::new(vec![
            Field::new("Year", DataType::Int64, false),
            Field::new("City", DataType::Utf8, false),
            Field::new("_timestamp", DataType::Int64, false),
        ]);
        let mut map: AHashMap<String, Schema> = AHashMap::new();
        map.insert(stream_name.to_string(), schema);
        let result = check_for_schema(
            org_name,
            stream_name,
            StreamType::Logs,
            record,
            &mut map,
            1234234234234,
        )
        .await;
        assert_eq!(result.0, true);
    }
}
