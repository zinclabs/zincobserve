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

pub mod destinations;
pub mod templates;

use actix_web::{delete, get, post, web, HttpResponse, Responder};
use std::io::Error;

use crate::{meta::alert::Alert, service::alerts};

/** Create new alert for specified stream of an organization */
#[utoipa::path(
    context_path = "/api",
    tag = "Alerts",
    operation_id = "SaveAlert",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        ("stream_name" = String, Path, description = "Stream name"),
        ("alert_name" = String, Path, description = "Alert name"),
      ),
    responses(
        (status = 200, description="Success", content_type = "application/json", body = HttpResponse),
    )
)]
#[post("/{org_id}/{stream_name}/alerts/{alert_name}")]
pub async fn save_alert(
    path: web::Path<(String, String, String)>,
    alert: web::Json<Alert>,
) -> Result<HttpResponse, Error> {
    let (org_id, stream_name, name) = path.into_inner();
    alerts::save_alert(org_id, stream_name, name, alert.into_inner()).await
}
/** List all alerts for a specified stream of an organization */
#[utoipa::path(
    context_path = "/api",
    tag = "Alerts",
    operation_id = "ListStreamAlerts",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        ("stream_name" = String, Path, description = "Stream name"),
      ),
    responses(
        (status = 200, description="Success", content_type = "application/json", body = AlertList),
    )
)]
#[get("/{org_id}/{stream_name}/alerts")]
async fn list_stream_alerts(path: web::Path<(String, String)>) -> impl Responder {
    let (org_id, stream_name) = path.into_inner();
    alerts::list_alert(org_id, Some(stream_name.as_str())).await
}

/** List all alerts for an organization */
#[utoipa::path(
    context_path = "/api",
    tag = "Alerts",
    operation_id = "ListAlerts",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
      ),
    responses(
        (status = 200, description="Success", content_type = "application/json", body = AlertList),
    )
)]
#[get("/{org_id}/alerts")]
async fn list_alerts(path: web::Path<String>) -> impl Responder {
    let org_id = path.into_inner();
    alerts::list_alert(org_id, None).await
}

/** Get alert by alert name */
#[utoipa::path(
    context_path = "/api",
    tag = "Alerts",
    operation_id = "GetAlert",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        ("stream_name" = String, Path, description = "Stream name"),
        ("alert_name" = String, Path, description = "Alert name"),
      ),
    responses(
        (status = 200, description="Success", content_type = "application/json", body = Alert),
        (status = 404, description="NotFound", content_type = "application/json", body = HttpResponse),
    )
)]
#[get("/{org_id}/{stream_name}/alerts/{alert_name}")]
async fn get_alert(path: web::Path<(String, String, String)>) -> impl Responder {
    let (org_id, stream_name, name) = path.into_inner();
    alerts::get_alert(org_id, stream_name, name).await
}

/** Delete alert by alert name */
#[utoipa::path(
    context_path = "/api",
    tag = "Alerts",
    operation_id = "DeleteAlert",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        ("stream_name" = String, Path, description = "Stream name"),
        ("alert_name" = String, Path, description = "Alert name"),
    ),
    responses(
        (status = 200, description="Success", content_type = "application/json", body = HttpResponse),
        (status = 404, description="NotFound", content_type = "application/json", body = HttpResponse),
    )
)]
#[delete("/{org_id}/{stream_name}/alerts/{alert_name}")]
async fn delete_alert(path: web::Path<(String, String, String)>) -> impl Responder {
    let (org_id, stream_name, name) = path.into_inner();
    alerts::delete_alert(org_id, stream_name, name).await
}
