use std::sync::Arc;
use tracing::info_span;

use crate::common::json;
use crate::infra::config::USERS;
use crate::infra::db::Event;
use crate::meta::user::User;

pub async fn get(org_id: Option<&str>, name: &str) -> Result<Option<User>, anyhow::Error> {
    let db_span = info_span!("db:user:get");
    let _guard = db_span.enter();
    let db = &crate::infra::db::DEFAULT;
    let key = match org_id {
        Some(org) => format!("/user/{}/{}", org, name),
        None => format!("/user/{}", name),
    };
    let ret = db.get(&key).await?;
    let loc_value = json::from_slice(&ret).unwrap();
    let value = Some(loc_value);
    Ok(value)
}

pub async fn set(org_id: &str, user: User) -> Result<(), anyhow::Error> {
    let db_span = info_span!("db:user:set");
    let _guard = db_span.enter();
    let db = &crate::infra::db::DEFAULT;

    let key = match user.role {
        crate::meta::user::Role::Admin => format!("/user/{}/{}", org_id, user.name),
        crate::meta::user::Role::User => format!("/user/{}/{}", org_id, user.name),
        crate::meta::user::Role::Root => format!("/user/{}", user.name),
    };
    db.put(&key, json::to_vec(&user).unwrap().into()).await?;
    Ok(())
}

pub async fn delete(org_id: &str, name: &str) -> Result<(), anyhow::Error> {
    let db_span = info_span!("db:user:delete");
    let _guard = db_span.enter();
    let db = &crate::infra::db::DEFAULT;
    let key = format!("/user/{}/{}", org_id, name);
    match db.delete(&key, false).await {
        Ok(_) => Ok(()),
        Err(e) => Err(anyhow::anyhow!(e)),
    }
}

pub async fn watch() -> Result<(), anyhow::Error> {
    let db = &crate::infra::db::DEFAULT;
    let key = "/user/";
    let mut events = db.watch(key).await?;
    let events = Arc::get_mut(&mut events).unwrap();
    log::info!("[TRACE] Start watching user");
    loop {
        let ev = match events.recv().await {
            Some(ev) => ev,
            None => {
                log::error!("watch_users: event channel closed");
                break;
            }
        };
        match ev {
            Event::Put(ev) => {
                let item_key = ev.key.strip_prefix(key).unwrap();
                let item_value: User = json::from_slice(&ev.value.unwrap()).unwrap();
                USERS.insert(item_key.to_owned(), item_value);
            }
            Event::Delete(ev) => {
                let item_key = ev.key.strip_prefix(key).unwrap();
                USERS.remove(item_key);
            }
        }
    }
    Ok(())
}

pub async fn cache() -> Result<(), anyhow::Error> {
    let db = &crate::infra::db::DEFAULT;
    let key = "/user/";
    let ret = db.list(key).await?;
    for (item_key, item_value) in ret {
        let item_key = item_key.strip_prefix(key).unwrap();
        let json_val: User = json::from_slice(&item_value).unwrap();
        USERS.insert(item_key.to_string(), json_val);
    }
    log::info!("[TRACE] Users Cached");
    Ok(())
}

pub async fn get_root_user(name: &str) -> Result<Option<User>, anyhow::Error> {
    let db_span = info_span!("db:user:get_root");
    let _guard = db_span.enter();
    let db = &crate::infra::db::DEFAULT;
    let key = format!("/user/{}", name);
    let ret = db.get(&key).await?;
    let loc_value = json::from_slice(&ret).unwrap();
    let value = Some(loc_value);
    Ok(value)
}