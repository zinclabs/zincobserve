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

use actix_web::{delete, get, post, web, HttpResponse, Responder};
use std::io::Error;

use crate::{meta::alert::DestinationTemplate, service::alerts::templates};

/** Create new alert templates for an organization */
#[utoipa::path(
    context_path = "/api",
    tag = "Alerts",
    operation_id = "CreateTemplate",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        ("template_name" = String, Path, description = "Template name"),
      ),
    responses(
        (status = 200, description="Success", content_type = "application/json", body = HttpResponse),
    )
)]
#[post("/{org_id}/alerts/templates/{template_name}")]
pub async fn save_template(
    path: web::Path<(String, String)>,
    alert: web::Json<DestinationTemplate>,
) -> Result<HttpResponse, Error> {
    let (org_id, name) = path.into_inner();
    templates::save_template(org_id, name, alert.into_inner()).await
}

/** List all templates for an organization */
#[utoipa::path(
    context_path = "/api",
    tag = "Alerts",
    operation_id = "ListTemplates",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
      ),
    responses(
        (status = 200, description="Success", content_type = "application/json", body = Vec<DestinationTemplate>),
    )
)]
#[get("/{org_id}/alerts/templates")]
async fn list_templates(path: web::Path<String>) -> impl Responder {
    let org_id = path.into_inner();
    templates::list_templates(org_id).await
}

/** Get alert template by template name */
#[utoipa::path(
    context_path = "/api",
    tag = "Alerts",
    operation_id = "GetTemplate",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        ("template_name" = String, Path, description = "Template name"),
      ),
    responses(
        (status = 200, description="Success", content_type = "application/json", body = DestinationTemplate),
        (status = 404, description="NotFound", content_type = "application/json", body = HttpResponse),
    )
)]
#[get("/{org_id}/alerts/templates/{template_name}")]
async fn get_template(path: web::Path<(String, String)>) -> impl Responder {
    let (org_id, name) = path.into_inner();
    templates::get_template(org_id, name).await
}

/** Delete alert template by alert template name */
#[utoipa::path(
    context_path = "/api",
    tag = "Alerts",
    operation_id = "DeleteAlertTemplate",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        ("template_name" = String, Path, description = "Template name"),
    ),
    responses(
        (status = 200, description="Success", content_type = "application/json", body = HttpResponse),
        (status = 404, description="NotFound", content_type = "application/json", body = HttpResponse),
    )
)]
#[delete("/{org_id}/alerts/templates/{template_name}")]
async fn delete_template(path: web::Path<(String, String)>) -> impl Responder {
    let (org_id, name) = path.into_inner();
    templates::delete_template(org_id, name).await
}
