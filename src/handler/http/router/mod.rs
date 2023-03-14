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

use actix_cors::Cors;
use actix_web::{http::header, web};
use actix_web_httpauth::middleware::HttpAuthentication;
use std::sync::Arc;
use utoipa::OpenApi;
use utoipa_swagger_ui::{SwaggerUi, Url};

use super::auth::validator;
use super::request::alerts::*;
use super::request::dashboards::*;
use super::request::functions;
use super::request::ingest;
use super::request::organization::*;
use super::request::prom::*;
use super::request::search;
use super::request::status;
use super::request::stream;
use super::request::traces::*;
use super::request::users;
use crate::infra::config::CONFIG;

pub mod openapi;
pub mod ui;

pub fn get_basic_routes(cfg: &mut web::ServiceConfig) {
    let cors = Cors::default()
        .send_wildcard()
        .allowed_methods(vec!["HEAD", "GET", "POST", "PUT", "OPTIONS", "DELETE"])
        .allowed_headers(vec![
            header::AUTHORIZATION,
            header::ACCEPT,
            header::CONTENT_TYPE,
        ])
        .allow_any_origin()
        .max_age(3600);
    let cors = Arc::new(cors);

    cfg.service(status::healthz);
    cfg.service(
        web::scope("/auth")
            .wrap(cors.clone())
            .service(organizations_by_username)
            .service(users::authentication),
    );
    cfg.service(web::scope("/config").wrap(cors).service(status::zo_config));

    cfg.service(SwaggerUi::new("/swagger/{_:.*}").urls(vec![(
        Url::new("api", "/api-doc/openapi.json"),
        openapi::ApiDoc::openapi(),
    )]));

    if CONFIG.common.ui_enabled {
        cfg.service(web::redirect("/", "./web/"));
        cfg.service(web::redirect("/web/ingestion/curl", "../ingestion"));
        cfg.service(web::redirect("/web/ingestion/fluentbit", "../ingestion"));
        cfg.service(web::redirect("/web/ingestion/fluentd", "../ingestion"));
        cfg.service(web::redirect("/web/ingestion/vector", "../ingestion"));
        cfg.service(ui::serve);
    }
}

pub fn get_service_routes(cfg: &mut web::ServiceConfig) {
    let auth = HttpAuthentication::basic(validator);
    let cors = Cors::default()
        .send_wildcard()
        .allowed_methods(vec!["HEAD", "GET", "POST", "PUT", "OPTIONS", "DELETE"])
        .allowed_headers(vec![
            header::AUTHORIZATION,
            header::ACCEPT,
            header::CONTENT_TYPE,
        ])
        .allow_any_origin()
        .max_age(3600);
    let cors = Arc::new(cors);

    cfg.service(
        web::scope("/api")
            .wrap(auth)
            .wrap(cors)
            .service(status::cache_status)
            .service(ingest::bulk)
            .service(ingest::multi)
            .service(ingest::json)
            .service(search::search)
            .service(search::around)
            .service(stream::schema)
            .service(stream::settings)
            .service(stream::delete)
            .service(stream::list)
            .service(stream::org_index)
            .service(functions::save_function)
            .service(functions::list_functions)
            .service(functions::delete_function)
            .service(functions::save_stream_function)
            .service(functions::list_stream_function)
            .service(functions::delete_stream_function)
            .service(users::list)
            .service(users::save)
            .service(users::delete)
            .service(users::add_user_to_org)
            .service(prometheus_write)
            .service(save_dashboard)
            .service(get_dashboard)
            .service(list_dashboards)
            .service(delete_dashboard)
            .service(traces_write)
            .service(organizations)
            .service(save_alert)
            .service(get_alert)
            .service(list_alerts)
            .service(list_stream_alerts)
            .service(delete_alert)
            .service(org_summary)
            .service(get_user_passcode)
            .service(update_user_passcode)
            .service(users::update)
            .service(templates::save_template)
            .service(templates::get_template)
            .service(templates::delete_template)
            .service(templates::list_templates)
            .service(destinations::save_destination)
            .service(destinations::get_destination)
            .service(destinations::list_destinations)
            .service(destinations::delete_destination),
    );
}
