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

use actix_web::{get, put, web, HttpResponse, Result};
use actix_web_httpauth::extractors::basic::BasicAuth;
use serde::Serialize;
use std::io::Error;

use crate::handler::http::auth::is_admin_user;
use crate::infra::config::{CONFIG, USERS};
use crate::meta::organization::PasscodeResponse;
use crate::service::organization::get_passcode;
use crate::service::organization::{self, update_passcode};

const DEFAULT: &str = "default";
const CUSTOM: &str = "custom";
const THRESHOLD: i64 = 9383939382;

#[derive(Serialize, Clone)]
struct Organization {
    identifier: String,
    label: String,
}

#[derive(Serialize, Clone)]
struct User {
    first_name: String,
    last_name: String,
    email: String,
}

#[derive(Serialize, Clone)]
struct OrganizationDetails {
    id: i64,
    identifier: String,
    name: String,
    user_email: String,
    ingest_threshold: i64,
    search_threshold: i64,
    #[serde(rename = "type")]
    org_type: String,
    #[serde(rename = "UserObj")]
    user_obj: User,
}

#[derive(Serialize)]
struct OrganizationResponse {
    data: Vec<OrganizationDetails>,
}

#[get("/organizarions_by_username/{user_name}")]
pub async fn organizarions_by_username(
    user_name: web::Path<String>,
) -> Result<HttpResponse, Error> {
    let mut orgs = Vec::new();
    let user_name = user_name.to_string();
    if is_admin_user(&user_name).await {
        let obj = Organization {
            identifier: DEFAULT.to_string(),
            label: DEFAULT.to_string(),
        };

        for user in USERS.iter() {
            if !user.key().ends_with(&CONFIG.auth.username) {
                orgs.push(Organization {
                    identifier: user.key().split('/').collect::<Vec<&str>>()[0].to_string(),
                    label: user.key().split('/').collect::<Vec<&str>>()[0].to_string(),
                });
            }
        }

        orgs.push(obj);
    } else {
        for user in USERS.iter() {
            if user.key().ends_with(format!("/{}", user_name).as_str()) {
                orgs.push(Organization {
                    identifier: user.key().split('/').collect::<Vec<&str>>()[0].to_string(),
                    label: user.key().split('/').collect::<Vec<&str>>()[0].to_string(),
                });
            }
        }
    }

    Ok(HttpResponse::Ok().json(orgs))
}

#[get("/{org_id}/organizations")]
pub async fn organizations(credentials: BasicAuth) -> Result<HttpResponse, Error> {
    //let org = org_id.into_inner();
    let user_id = credentials.user_id();
    let mut id = 0;

    let mut orgs: Vec<OrganizationDetails> = vec![];
    let user_detail = User {
        first_name: user_id.to_string(),
        last_name: user_id.to_string(),
        email: user_id.to_string(),
    };

    if is_admin_user(user_id).await {
        id += 1;
        orgs.push(OrganizationDetails {
            id,
            identifier: DEFAULT.to_string(),
            name: DEFAULT.to_string(),
            user_email: user_id.to_string(),
            ingest_threshold: THRESHOLD,
            search_threshold: THRESHOLD,
            org_type: DEFAULT.to_string(),
            user_obj: user_detail.clone(),
        });

        for user in USERS.iter() {
            if !user.key().ends_with(&CONFIG.auth.username) {
                id += 1;
                orgs.push(OrganizationDetails {
                    id,
                    identifier: user.key().split('/').collect::<Vec<&str>>()[0].to_string(),
                    name: user.key().split('/').collect::<Vec<&str>>()[0].to_string(),
                    user_email: user_id.to_string(),
                    ingest_threshold: THRESHOLD,
                    search_threshold: THRESHOLD,
                    org_type: CUSTOM.to_string(),
                    user_obj: user_detail.clone(),
                });
            }
        }
    } else {
        for user in USERS.iter() {
            if user.key().ends_with(format!("/{}", user_id).as_str()) {
                id += 1;

                orgs.push(OrganizationDetails {
                    id,
                    identifier: user.key().split('/').collect::<Vec<&str>>()[0].to_string(),
                    name: user.key().split('/').collect::<Vec<&str>>()[0].to_string(),
                    user_email: user_id.to_string(),
                    ingest_threshold: THRESHOLD,
                    search_threshold: THRESHOLD,
                    org_type: DEFAULT.to_string(),
                    user_obj: user_detail.clone(),
                });
            }
        }
    }

    let org_response = OrganizationResponse { data: orgs };

    Ok(HttpResponse::Ok().json(org_response))
}
#[get("/{org_id}/summary")]
async fn org_summary(org_id: web::Path<String>) -> Result<HttpResponse, Error> {
    let org = org_id.into_inner();
    let org_summary = organization::get_summary(&org).await;
    Ok(HttpResponse::Ok().json(org_summary))
}

#[get("/{org_id}/organizations/passcode")]
async fn get_user_passcode(
    credentials: BasicAuth,
    org_id: web::Path<String>,
) -> Result<HttpResponse, Error> {
    let org = org_id.into_inner();
    let user_id = credentials.user_id();
    let mut org_id = Some(org.as_str());
    if is_admin_user(user_id).await {
        org_id = None;
    }
    let passcode = get_passcode(org_id, user_id).await;
    Ok(HttpResponse::Ok().json(PasscodeResponse { data: passcode }))
}

#[put("/{org_id}/organizations/passcode")]
async fn update_user_passcode(
    credentials: BasicAuth,
    org_id: web::Path<String>,
) -> Result<HttpResponse, Error> {
    let org = org_id.into_inner();
    let user_id = credentials.user_id();
    let mut org_id = Some(org.as_str());
    if is_admin_user(user_id).await {
        org_id = None;
    }
    let passcode = update_passcode(org_id, user_id).await;
    Ok(HttpResponse::Ok().json(PasscodeResponse { data: passcode }))
}
