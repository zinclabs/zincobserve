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

use actix_web::{
    http::{self, StatusCode},
    HttpResponse,
};
use rand::distributions::{Alphanumeric, DistString};
use std::io::Error;
use uuid::Uuid;

use super::db;
use crate::{
    common::auth::get_hash,
    infra::config::ROOT_USER,
    meta::{
        http::HttpResponse as MetaHttpResponse,
        organization::DEFAULT_ORG,
        user::{UserOrg, UserRequest},
    },
};
use crate::{
    common::auth::is_root_user,
    meta::user::{User, UserList, UserResponse, UserRole},
};
use crate::{infra::config::USERS, meta::user::UpdateUser};

pub async fn post_user(org_id: &str, usr_req: UserRequest) -> Result<HttpResponse, Error> {
    let salt = Uuid::new_v4().to_string();
    let password = get_hash(&usr_req.password, &salt);
    let token = Alphanumeric.sample_string(&mut rand::thread_rng(), 16);
    let user = usr_req.to_new_dbuser(password, salt, org_id.replace(' ', "_"), token);
    db::user::set(user).await.unwrap();
    Ok(HttpResponse::Ok().json(MetaHttpResponse::message(
        http::StatusCode::OK.into(),
        "User saved successfully".to_string(),
    )))
}

pub async fn update_user(
    org_id: &str,
    email: &str,
    self_update: bool,
    initiator_id: &str,
    user: UpdateUser,
) -> Result<HttpResponse, Error> {
    let mut allow_password_update = false;

    let existing_user = if is_root_user(email).await {
        db::user::get(None, email).await
    } else {
        db::user::get(Some(org_id), email).await
    };

    if existing_user.is_ok() {
        let mut new_user;
        let mut is_updated = false;
        let mut is_org_updated = false;
        let mut message = "";
        match existing_user.unwrap() {
            Some(local_user) => {
                if !self_update {
                    if is_root_user(initiator_id).await {
                        allow_password_update = true
                    } else {
                        let initiating_user = db::user::get(Some(org_id), initiator_id)
                            .await
                            .unwrap()
                            .unwrap();
                        if (local_user.role.eq(&UserRole::Root)
                            && initiating_user.role.eq(&UserRole::Root))
                            || (!local_user.role.eq(&UserRole::Root)
                                && (initiating_user.role.eq(&UserRole::Admin)
                                    || initiating_user.role.eq(&UserRole::Root)))
                        {
                            allow_password_update = true
                        }
                    }
                }

                new_user = local_user.clone();
                if self_update && user.old_password.is_some() && user.new_password.is_some() {
                    if local_user.password.eq(&get_hash(
                        &user.clone().old_password.unwrap(),
                        &local_user.salt,
                    )) {
                        new_user.password = get_hash(&user.new_password.unwrap(), &local_user.salt);
                        is_updated = true;
                    } else {
                        message =
                            "Existing/old password mismatch, please provide valid existing password"
                    }
                } else if self_update && user.old_password.is_none() {
                    message = "Please provide existing password"
                } else if !self_update && allow_password_update && user.new_password.is_some() {
                    new_user.password = get_hash(&user.new_password.unwrap(), &local_user.salt);
                    is_updated = true;
                } else {
                    message = "You are not authorised to change the password"
                }
                if user.first_name.is_some() {
                    new_user.first_name = user.first_name.unwrap();
                    is_updated = true;
                }
                if user.last_name.is_some() {
                    new_user.last_name = user.last_name.unwrap();
                    is_updated = true;
                }
                if user.role.is_some()
                    && (!self_update
                        || (local_user.role.eq(&UserRole::Admin)
                            || local_user.role.eq(&UserRole::Root)))
                {
                    new_user.role = user.role.unwrap();
                    is_org_updated = true;
                }
                if user.token.is_some() {
                    new_user.token = user.token.unwrap();
                    is_org_updated = true;
                }
                if is_updated || is_org_updated {
                    let user = db::user::get_db_user(email).await;
                    let token = Alphanumeric.sample_string(&mut rand::thread_rng(), 16);
                    match user {
                        Ok(mut db_user) => {
                            db_user.password = new_user.password;
                            db_user.first_name = new_user.first_name;
                            db_user.last_name = new_user.last_name;
                            if is_org_updated {
                                let mut orgs = db_user.clone().organizations;
                                let new_orgs = if orgs.is_empty() {
                                    vec![UserOrg {
                                        name: org_id.to_string(),
                                        token,
                                        role: new_user.role,
                                    }]
                                } else {
                                    orgs.retain(|org| !org.name.eq(org_id));
                                    orgs.push(UserOrg {
                                        name: org_id.to_string(),
                                        token,
                                        role: new_user.role,
                                    });
                                    orgs
                                };
                                db_user.organizations = new_orgs;
                            }

                            db::user::set(db_user).await.unwrap();
                            Ok(HttpResponse::Ok().json(MetaHttpResponse::message(
                                http::StatusCode::OK.into(),
                                "User updated successfully".to_string(),
                            )))
                        }
                        Err(_) => Ok(HttpResponse::NotFound().json(MetaHttpResponse::error(
                            StatusCode::NOT_FOUND.into(),
                            Some("User not found".to_string()),
                        ))),
                    }
                } else {
                    if message.is_empty() {
                        message = "Not allowed to update";
                    }
                    Ok(HttpResponse::BadRequest().json(MetaHttpResponse::message(
                        http::StatusCode::BAD_REQUEST.into(),
                        message.to_string(),
                    )))
                }
            }
            None => Ok(HttpResponse::NotFound().json(MetaHttpResponse::error(
                StatusCode::NOT_FOUND.into(),
                Some("User not found".to_string()),
            ))),
        }
    } else {
        Ok(HttpResponse::NotFound().json(MetaHttpResponse::error(
            StatusCode::NOT_FOUND.into(),
            Some("User not found".to_string()),
        )))
    }
}

pub async fn add_user_to_org(
    org_id: &str,
    email: &str,
    role: UserRole,
    initiator_id: &str,
) -> Result<HttpResponse, Error> {
    let existing_user = db::user::get_db_user(email).await;
    if existing_user.is_ok() {
        let mut db_user = existing_user.unwrap();
        let local_org;
        let initiating_user = if is_root_user(initiator_id).await {
            local_org = org_id.replace(' ', "_");
            ROOT_USER.get("root").unwrap().value().clone()
        } else {
            local_org = org_id.to_owned();
            db::user::get(Some(org_id), initiator_id)
                .await
                .unwrap()
                .unwrap()
        };
        if initiating_user.role.eq(&UserRole::Root) || initiating_user.role.eq(&UserRole::Admin) {
            let token = Alphanumeric.sample_string(&mut rand::thread_rng(), 16);
            let mut orgs = db_user.clone().organizations;
            let new_orgs = if orgs.is_empty() {
                vec![UserOrg {
                    name: local_org.to_string(),
                    token,
                    role,
                }]
            } else {
                orgs.retain(|org| !org.name.eq(&local_org));
                orgs.push(UserOrg {
                    name: local_org.to_string(),
                    token,
                    role,
                });
                orgs
            };
            db_user.organizations = new_orgs;
            db::user::set(db_user).await.unwrap();
            Ok(HttpResponse::Ok().json(MetaHttpResponse::message(
                http::StatusCode::OK.into(),
                "User added to org successfully".to_string(),
            )))
        } else {
            Ok(HttpResponse::Unauthorized().json(MetaHttpResponse::error(
                StatusCode::UNAUTHORIZED.into(),
                Some("Not Allowed".to_string()),
            )))
        }
    } else {
        Ok(HttpResponse::NotFound().json(MetaHttpResponse::error(
            StatusCode::NOT_FOUND.into(),
            Some("User not found".to_string()),
        )))
    }
}

pub async fn get_user(org_id: Option<&str>, name: &str) -> Option<User> {
    let key = match org_id {
        Some(local_org) => format!("{}/{}", local_org, name),
        None => format!("{}/{}", DEFAULT_ORG, name),
    };
    let user = USERS.get(&key);
    match user {
        Some(loc_user) => Some(loc_user.value().clone()),
        None => {
            let res = db::user::get(org_id, name).await;
            if res.is_err() {
                None
            } else {
                res.unwrap()
                //local_user.map(|user| user)
                /*  match local_user {
                    Some(user) => Some(user),
                    None => None,
                } */
            }
        }
    }
}

pub async fn list_users(org_id: &str) -> Result<HttpResponse, Error> {
    let mut user_list: Vec<UserResponse> = vec![];
    for user in USERS.iter() {
        if user.key().starts_with(org_id) {
            user_list.push(UserResponse {
                email: user.value().email.clone(),
                role: user.value().role.clone(),
                first_name: user.value().first_name.clone(),
                last_name: user.value().last_name.clone(),
            })
        }
    }

    Ok(HttpResponse::Ok().json(UserList { data: user_list }))
}

pub async fn remove_user_from_org(org_id: &str, email_id: &str) -> Result<HttpResponse, Error> {
    let ret_user = db::user::get_db_user(email_id).await;
    match ret_user {
        Ok(mut user) => {
            if !user.organizations.is_empty() {
                let mut orgs = user.clone().organizations;
                if orgs.len() == 1 {
                    let _ = db::user::delete(email_id).await;
                } else {
                    orgs.retain(|x| !x.name.eq(&org_id.to_string()));
                    user.organizations = orgs;
                    let resp = db::user::set(user).await;
                    //special case as we cache flattened user struct
                    if resp.is_ok() {
                        USERS.remove(&format!("{}/{}", org_id, email_id));
                    }
                }
                Ok(HttpResponse::Ok().json(MetaHttpResponse::message(
                    http::StatusCode::OK.into(),
                    "User removed from organization".to_string(),
                )))
            } else {
                Ok(HttpResponse::NotFound().json(MetaHttpResponse::error(
                    StatusCode::NOT_FOUND.into(),
                    Some("User for the organization not found".to_owned()),
                )))
            }
        }
        Err(_) => Ok(HttpResponse::NotFound().json(MetaHttpResponse::error(
            StatusCode::NOT_FOUND.into(),
            Some("User for the organization not found".to_owned()),
        ))),
    }
}

pub async fn delete_user(email_id: &str) -> Result<HttpResponse, Error> {
    let result = db::user::delete(email_id).await;
    match result {
        Ok(_) => Ok(HttpResponse::Ok().json(MetaHttpResponse::message(
            http::StatusCode::OK.into(),
            "User deleted".to_string(),
        ))),
        Err(err) => Ok(HttpResponse::NotFound().json(MetaHttpResponse::error(
            StatusCode::NOT_FOUND.into(),
            Some(err.to_string()),
        ))),
    }
}

pub async fn root_user_exists() -> bool {
    let local_users = ROOT_USER.clone();
    if !local_users.is_empty() {
        local_users.is_empty()
    } else {
        db::user::root_user_exists().await
    }
}

pub fn is_user_from_org(orgs: Vec<UserOrg>, org_id: &str) -> (bool, UserOrg) {
    if orgs.is_empty() {
        (false, UserOrg::default())
    } else {
        let mut local_orgs = orgs;
        local_orgs.retain(|org| !org.name.eq(&org_id.to_string()));
        if local_orgs.is_empty() {
            (false, UserOrg::default())
        } else {
            (true, local_orgs.first().unwrap().clone())
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    async fn set_up() {
        USERS.insert(
            "dummy/admin@zo.dev".to_string(),
            User {
                email: "admin@zo.dev".to_string(),
                password: "pass#123".to_string(),
                role: crate::meta::user::UserRole::Admin,
                salt: String::new(),
                token: "token".to_string(),
                first_name: "admin".to_owned(),
                last_name: "".to_owned(),
                org: "dummy".to_string(),
            },
        );
    }

    #[actix_web::test]
    async fn test_list_users() {
        set_up().await;
        assert!(list_users("dummy").await.is_ok())
    }

    #[actix_web::test]
    async fn test_root_user_exists() {
        set_up().await;
        assert!(!root_user_exists().await);
    }

    #[actix_web::test]
    async fn test_get_user() {
        set_up().await;
        assert!(get_user(Some("dummy"), "admin@zo.dev").await.is_some())
    }

    #[actix_web::test]
    async fn test_post_user() {
        let resp = post_user(
            "dummy",
            UserRequest {
                email: "admin@zo.dev".to_string(),
                password: "pass#123".to_string(),
                role: crate::meta::user::UserRole::Admin,
                first_name: "admin".to_owned(),
                last_name: "".to_owned(),
            },
        )
        .await;
        assert!(resp.is_ok());
    }

    #[actix_web::test]
    async fn test_user() {
        let _ = post_user(
            "test_org",
            UserRequest {
                email: "user@example.com".to_string(),
                password: "pass#123".to_string(),
                role: crate::meta::user::UserRole::Admin,
                first_name: "admin".to_owned(),
                last_name: "".to_owned(),
            },
        )
        .await;

        USERS.insert(
            "dummy/admin@zo.dev".to_string(),
            User {
                email: "admin@zo.dev".to_string(),
                password: "pass#123".to_string(),
                role: crate::meta::user::UserRole::Admin,
                salt: String::new(),
                token: "token".to_string(),
                first_name: "admin".to_owned(),
                last_name: "".to_owned(),
                org: "dummy".to_string(),
            },
        );

        let resp = update_user(
            "dummy",
            "user@example.com",
            true,
            "user@example.com",
            UpdateUser {
                token: Some("new_token".to_string()),
                first_name: Some("first_name".to_string()),
                last_name: Some("last_name".to_string()),
                old_password: Some("pass".to_string()),
                new_password: Some("new_pass".to_string()),
                role: Some(crate::meta::user::UserRole::Member),
            },
        )
        .await;

        assert!(resp.is_ok());

        let resp = update_user(
            "dummy",
            "user@example.com",
            false,
            "admin@zo.dev",
            UpdateUser {
                token: Some("new_token".to_string()),
                first_name: Some("first_name".to_string()),
                last_name: Some("last_name".to_string()),
                old_password: None,
                new_password: None,
                role: Some(crate::meta::user::UserRole::Admin),
            },
        )
        .await;

        assert!(resp.is_ok());

        let resp = add_user_to_org(
            "dummy",
            "user@example.com",
            UserRole::Member,
            "admin@zo.dev",
        )
        .await;

        assert!(resp.is_ok());

        let resp = remove_user_from_org("dummy", "user@example.com").await;

        assert!(resp.is_ok());
    }
}
