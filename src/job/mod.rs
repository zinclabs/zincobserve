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

use crate::infra::config::{CONFIG, INSTANCE_ID};
use crate::infra::{cluster, ider};
use crate::meta::organization::DEFAULT_ORG;
use crate::meta::user::User;
use crate::service::{db, users};
use rand::distributions::{Alphanumeric, DistString};

mod alert_manager;
mod compact;
mod file_list;
mod files;
mod prom;
mod telemetry;

pub async fn init() -> Result<(), anyhow::Error> {
    let res = db::user::get_root_user(&CONFIG.auth.username).await;
    if res.is_err() || res.unwrap().is_none() {
        let token = Alphanumeric.sample_string(&mut rand::thread_rng(), 16);
        let _ = users::post_user(
            DEFAULT_ORG,
            User {
                name: CONFIG.auth.username.clone(),
                password: CONFIG.auth.password.clone(),
                role: crate::meta::user::UserRole::Root,
                salt: String::new(),
                ingestion_token: token,
            },
        )
        .await;
    }

    // telemetry run
    if CONFIG.common.enable_telemetry {
        let res = db::get_instance().await;
        let instance_id;
        if res.as_ref().is_err() || res.as_ref().unwrap().is_none() {
            instance_id = ider::generate();
            let _ = db::set_instance(&instance_id).await;
        } else {
            instance_id = res.unwrap().unwrap();
        }
        INSTANCE_ID.insert("instance_id".to_owned(), instance_id);
        tokio::task::spawn(async move { telemetry::run().await });
    }

    tokio::task::spawn(async move { db::functions::watch().await });
    tokio::task::spawn(async move { db::user::watch().await });
    tokio::task::spawn(async move { db::schema::watch().await });
    tokio::task::spawn(async move { db::watch_prom_cluster_leader().await });
    tokio::task::spawn(async move { db::alerts::watch().await });
    tokio::task::spawn(async move { db::triggers::watch().await });
    tokio::task::yield_now().await; // yield let other tasks run
    db::functions::cache().await?;
    db::user::cache().await?;
    db::schema::cache().await?;
    db::cache_prom_cluster_leader().await?;
    db::alerts::cache().await?;
    db::triggers::cache().await?;

    // cache file list
    db::file_list::local::cache().await?;
    db::file_list::remote::cache().await?;

    // Shouldn't serve request until initialization finishes
    log::info!("[TRACE] Start job");

    // compactor run
    tokio::task::spawn(async move { compact::run().await });

    // alert manager run
    tokio::task::spawn(async move { alert_manager::run().await });

    // ingester run
    tokio::task::spawn(async move { files::disk::run().await });
    tokio::task::spawn(async move { files::memory::run().await });
    tokio::task::spawn(async move { file_list::run().await });
    tokio::task::spawn(async move { prom::run().await });

    Ok(())
}

#[cfg(test)]
mod test_utils {
    use super::*;
    use std::env;
    #[actix_web::test]
    #[ignore]
    async fn test_init() {
        env::set_var("ZO_LOCAL_MODE", "true");
        env::set_var("ZO_NODE_ROLE", "all");
        let _ = init().await;
        //assert_eq!(fs::metadata(&CONFIG.common.data_wal_dir).is_ok(), true)
    }
}
