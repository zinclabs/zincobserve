use crate::{
    handler::http::auth::is_admin_user,
    infra::config::{STREAM_SCHEMAS, USERS},
    service::organization::{self},
};
use actix_web::{get, web, HttpResponse, Result};
use serde::Serialize;
use std::io::Error;

#[derive(Serialize)]
struct Organization {
    identifier: String,
    label: String,
}

#[derive(Serialize)]
struct User {
    first_name: String,
    last_name: String,
    email: String,
}

#[derive(Serialize)]
struct OrganizationDetails {
    id: i64,
    identifier: String,
    name: String,
    user_email: String,
    ingest_threshold: i64,
    search_threshold: i64,
    org_type: String,
    user_obj: User,
}

#[derive(Serialize)]
struct OrganizationResponse {
    data: [OrganizationDetails; 1],
}

#[get("/organizarions_by_username/{user_name}")]
pub async fn organizarions_by_username(
    user_name: web::Path<String>,
) -> Result<HttpResponse, Error> {
    let mut orgs = Vec::new();
    let user_name = user_name.to_string();
    if is_admin_user(&user_name).await {
        let obj = Organization {
            identifier: "default".to_string(),
            label: "Default".to_string(),
        };
        for schema in STREAM_SCHEMAS.iter() {
            orgs.push(Organization {
                identifier: schema.key().split('/').collect::<Vec<&str>>()[0].to_string(),
                label: schema.key().split('/').collect::<Vec<&str>>()[0].to_string(),
            });
        }
        orgs.push(obj);
    } else {
        for user in USERS.iter() {
            if user.key().contains(format!("/{}", user_name).as_str()) {
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
pub async fn organizations() -> Result<HttpResponse, Error> {
    println!("Inside organizations");
    let user_detail = User {
        first_name: "admin".to_string(),
        last_name: "admin".to_string(),
        email: "admin".to_string(),
    };
    let obj = OrganizationDetails {
        id: 1,
        identifier: "default".to_string(),
        name: "Default".to_string(),
        user_email: "admin".to_string(),
        ingest_threshold: 9383939382,
        search_threshold: 9383939382,
        org_type: "default".to_string(),
        user_obj: user_detail,
    };

    let org_response = OrganizationResponse { data: [obj] };

    Ok(HttpResponse::Ok().json(org_response))
}
#[get("/{org_id}/summary")]
async fn org_summary(org_id: web::Path<String>) -> Result<HttpResponse, Error> {
    let org = org_id.into_inner();
    let org_summary = organization::get_summary(&org).await;
    Ok(HttpResponse::Ok().json(org_summary))
}
