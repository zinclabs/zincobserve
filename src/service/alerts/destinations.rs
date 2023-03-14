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
use std::io::Error;
use tracing::info_span;

use crate::meta::alert::AlertDestination;
use crate::meta::http::HttpResponse as MetaHttpResponse;
use crate::service::db;

pub async fn save_destination(
    org_id: String,
    name: String,
    destination: AlertDestination,
) -> Result<HttpResponse, Error> {
    let loc_span = info_span!("service:alerts:destinations:save");
    let _guard = loc_span.enter();

    db::alerts::destinations::set(org_id.as_str(), name.as_str(), destination.clone())
        .await
        .unwrap();

    Ok(HttpResponse::Ok().json(MetaHttpResponse::message(
        http::StatusCode::OK.into(),
        "Alert destination saved".to_string(),
    )))
}

pub async fn list_destinations(org_id: String) -> Result<HttpResponse, Error> {
    let loc_span = info_span!("service:alerts:destinations:list");
    let _guard = loc_span.enter();
    let list = db::alerts::destinations::list(org_id.as_str())
        .await
        .unwrap();
    Ok(HttpResponse::Ok().json(list))
}

pub async fn delete_destination(org_id: String, name: String) -> Result<HttpResponse, Error> {
    let loc_span = info_span!("service:alerts:destinations:delete");
    let _guard = loc_span.enter();
    let result = db::alerts::destinations::delete(org_id.as_str(), name.as_str()).await;
    match result {
        Ok(_) => Ok(HttpResponse::Ok().json(MetaHttpResponse::message(
            http::StatusCode::OK.into(),
            "Alert destination deleted ".to_string(),
        ))),
        Err(err) => Ok(HttpResponse::NotFound().json(MetaHttpResponse::error(
            http::StatusCode::NOT_FOUND.into(),
            Some(err.to_string()),
        ))),
    }
}

pub async fn get_destination(org_id: String, name: String) -> Result<HttpResponse, Error> {
    let loc_span = info_span!("service:alerts:destinations:get");
    let _guard = loc_span.enter();
    let result = db::alerts::destinations::get(org_id.as_str(), name.as_str()).await;
    match result {
        Ok(alert) => Ok(HttpResponse::Ok().json(alert)),
        Err(_) => Ok(HttpResponse::NotFound().json(MetaHttpResponse::error(
            http::StatusCode::NOT_FOUND.into(),
            Some("Alert destination not found".to_string()),
        ))),
    }
}
