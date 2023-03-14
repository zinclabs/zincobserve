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
use arrow_schema::DataType;
use chrono::Utc;
use std::collections::HashMap;
use std::io::Error;
use tracing::info_span;

use super::search::sql::Sql;
use super::{db, triggers};
use crate::handler::grpc::cluster_rpc;
use crate::meta;
use crate::meta::alert::{Alert, AlertList, Trigger};
use crate::meta::http::HttpResponse as MetaHttpResponse;
use crate::meta::search::Query;

pub mod destinations;
pub mod templates;

pub async fn save_alert(
    org_id: String,
    stream_name: String,
    name: String,
    mut alert: Alert,
) -> Result<HttpResponse, Error> {
    let loc_span = info_span!("service:alerts:save");
    let _guard = loc_span.enter();

    alert.stream = stream_name.to_string();
    alert.name = name.to_string();

    let in_dest = alert.clone().destination;

    // before saving alert check alert destination
    let dest = db::alerts::destinations::get(&org_id, &in_dest)
        .await
        .unwrap();

    if dest.is_none() {
        return Ok(HttpResponse::NotFound().json(MetaHttpResponse::error(
            http::StatusCode::NOT_FOUND.into(),
            Some(format!("Destination with name {} not found", in_dest)),
        )));
    }

    // before saving alert check column type to decide numeric condition
    let schema = db::schema::get(&org_id, &stream_name, Some(crate::meta::StreamType::Logs))
        .await
        .unwrap();
    let fields = schema.fields;
    if fields.is_empty() {
        return Ok(HttpResponse::NotFound().json(MetaHttpResponse::error(
            http::StatusCode::NOT_FOUND.into(),
            Some(format!("Stream with name {} not found", stream_name)),
        )));
    }

    if alert.query.is_none() {
        alert.query = Some(Query {
            sql: format!("select * from {}", stream_name),
            start_time: 0,
            end_time: 0,
            sql_mode: "full".to_owned(),
            track_total_hits: false,
            from: 0,
            size: 0,
        });
        alert.is_real_time = true;

        //for condtion in alert.trigger.iter_mut() {
        let mut local_fields = fields.clone();

        local_fields.retain(|field| field.name().eq(&alert.condition.column));

        if !local_fields.is_empty() {
            if local_fields
                .first()
                .unwrap()
                .data_type()
                .eq(&DataType::Utf8)
                || local_fields
                    .first()
                    .unwrap()
                    .data_type()
                    .eq(&DataType::Boolean)
            {
                alert.condition.is_numeric = Some(false);
            } else {
                alert.condition.is_numeric = Some(true);
            }
        } else {
            return Ok(HttpResponse::BadRequest().json(MetaHttpResponse::error(
                http::StatusCode::BAD_REQUEST.into(),
                Some(format!(
                    "Column named {} not found on stream {}",
                    &alert.condition.column, stream_name
                )),
            )));
        }
        //}
    } else {
        let meta_req = meta::search::Request {
            query: alert.clone().query.unwrap(),
            aggs: HashMap::new(),
            encoding: meta::search::RequestEncoding::Empty,
        };
        let req: cluster_rpc::SearchRequest = meta_req.into();
        let sql = Sql::new(&req).await;

        if sql.is_err() {
            return Ok(HttpResponse::BadRequest().json(MetaHttpResponse::error(
                http::StatusCode::BAD_REQUEST.into(),
                Some(format!("Invalid query : {:?} ", sql.err())),
            )));
        }
    }

    db::alerts::set(
        org_id.as_str(),
        stream_name.as_str(),
        name.as_str(),
        alert.clone(),
    )
    .await
    .unwrap();
    // For non-ingest alert set trigger immediately
    if !alert.is_real_time {
        let trigger = Trigger {
            timestamp: Utc::now().timestamp_micros(),
            is_valid: true,
            alert_name: alert.name.clone(),
            stream: stream_name,
            org: org_id,
            last_sent_at: 0,
            count: 0,
            is_ingest_time: false,
        };
        let _ = triggers::save_trigger(trigger.alert_name.clone(), trigger.clone()).await;
    }

    Ok(HttpResponse::Ok().json(MetaHttpResponse::message(
        http::StatusCode::OK.into(),
        "Alert saved".to_string(),
    )))
}

pub async fn list_alert(org_id: String, stream_name: Option<&str>) -> Result<HttpResponse, Error> {
    let loc_span = info_span!("service:alerts:list");
    let _guard = loc_span.enter();
    let alerts_list = db::alerts::list(org_id.as_str(), stream_name)
        .await
        .unwrap();
    Ok(HttpResponse::Ok().json(AlertList { list: alerts_list }))
}

pub async fn delete_alert(
    org_id: String,
    stream_name: String,
    name: String,
) -> Result<HttpResponse, Error> {
    let loc_span = info_span!("service:alerts:delete");
    let _guard = loc_span.enter();
    let result = db::alerts::delete(org_id.as_str(), stream_name.as_str(), name.as_str()).await;
    match result {
        Ok(_) => Ok(HttpResponse::Ok().json(MetaHttpResponse::message(
            http::StatusCode::OK.into(),
            "Alert deleted ".to_string(),
        ))),
        Err(err) => Ok(HttpResponse::NotFound().json(MetaHttpResponse::error(
            http::StatusCode::NOT_FOUND.into(),
            Some(err.to_string()),
        ))),
    }
}

pub async fn get_alert(
    org_id: String,
    stream_name: String,
    name: String,
) -> Result<HttpResponse, Error> {
    let loc_span = info_span!("service:alerts:get");
    let _guard = loc_span.enter();
    let result = db::alerts::get(org_id.as_str(), stream_name.as_str(), name.as_str()).await;
    match result {
        Ok(alert) => Ok(HttpResponse::Ok().json(alert)),
        Err(_) => Ok(HttpResponse::NotFound().json(MetaHttpResponse::error(
            http::StatusCode::NOT_FOUND.into(),
            Some("alert not found".to_string()),
        ))),
    }
}

#[cfg(test)]
mod test {
    use crate::meta::alert::{AllOperator, Condition};

    use super::*;

    #[actix_web::test]
    async fn test_alerts() {
        let alert = Alert {
            name: "500error".to_string(),
            stream: "olympics".to_string(),
            query: Some(Query {
                sql: format!("select count(*) as occurance from olympics"),
                start_time: 0,
                end_time: 0,
                sql_mode: "full".to_owned(),
                track_total_hits: false,
                from: 0,
                size: 0,
            }),
            condition: Condition {
                column: "occurance".to_owned(),
                operator: AllOperator::GreaterThanEquals,
                ignore_case: None,
                value: serde_json::json!("5"),
                is_numeric: None,
            },
            duration: 1,
            frequency: 1,
            time_between_alerts: 10,
            destination: "test".to_string(),
            is_real_time: false,
            context_attributes: None,
        };
        let res = save_alert(
            "nexus".to_string(),
            "olympics".to_string(),
            "500error".to_string(),
            alert,
        )
        .await;
        assert!(res.is_ok());

        let list_res = list_alert("nexus".to_string(), Some("olympics")).await;
        assert!(list_res.is_ok());

        let list_res = list_alert("nexus".to_string(), None).await;
        assert!(list_res.is_ok());

        let get_res = get_alert(
            "nexus".to_string(),
            "olympics".to_string(),
            "500error".to_string(),
        )
        .await;
        assert!(get_res.is_ok());

        let del_res = delete_alert(
            "nexus".to_string(),
            "olympics".to_string(),
            "500error".to_string(),
        )
        .await;
        assert!(del_res.is_ok());
    }
}
