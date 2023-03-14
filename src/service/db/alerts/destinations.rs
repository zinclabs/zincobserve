use std::sync::Arc;

use crate::common::json;
use crate::infra::config::ALERTS_DESTINATIONS;
use crate::infra::db::Event;
use crate::meta::alert::{AlertDestination, AlertDestinationResponse};
use crate::service::db;

pub async fn get(
    org_id: &str,
    name: &str,
) -> Result<Option<AlertDestinationResponse>, anyhow::Error> {
    let map_key = format!("{}/{}", org_id, name);
    let value: Option<AlertDestinationResponse> = if ALERTS_DESTINATIONS.contains_key(&map_key) {
        let dest = ALERTS_DESTINATIONS.get(&map_key).unwrap().clone();
        let template = db::alerts::templates::get(org_id, &dest.template).await?;
        Some(dest.to_dest_resp(template))
    } else {
        let db = &crate::infra::db::DEFAULT;
        let key = format!("/destinations/{}/{}", org_id, name);
        match db.get(&key).await {
            Ok(val) => {
                let dest: AlertDestination = json::from_slice(&val).unwrap();
                let template = db::alerts::templates::get(org_id, &dest.template).await?;
                Some(dest.to_dest_resp(template))
            }
            Err(_) => None,
        }
    };

    Ok(value)
}

pub async fn set(
    org_id: &str,
    name: &str,
    destination: AlertDestination,
) -> Result<(), anyhow::Error> {
    let db = &crate::infra::db::DEFAULT;
    let key = format!("/destinations/{}/{}", org_id, name);
    db.put(&key, json::to_vec(&destination).unwrap().into())
        .await?;
    Ok(())
}

pub async fn delete(org_id: &str, name: &str) -> Result<(), anyhow::Error> {
    let db = &crate::infra::db::DEFAULT;
    let key = format!("/destinations/{}/{}", org_id, name);
    match db.delete(&key, false).await {
        Ok(_) => Ok(()),
        Err(e) => Err(anyhow::anyhow!(e)),
    }
}

pub async fn list(org_id: &str) -> Result<Vec<AlertDestinationResponse>, anyhow::Error> {
    let db = &crate::infra::db::DEFAULT;

    let key = format!("/destinations/{}", org_id);
    let ret = db.list_values(&key).await?;
    let mut temp_list: Vec<AlertDestinationResponse> = Vec::new();
    for item_value in ret {
        let dest: AlertDestination = json::from_slice(&item_value).unwrap();
        let template = db::alerts::templates::get(org_id, &dest.template).await?;
        temp_list.push(dest.to_dest_resp(template))
    }
    Ok(temp_list)
}

pub async fn watch() -> Result<(), anyhow::Error> {
    let db = &crate::infra::db::DEFAULT;
    let key = "/destinations/";
    let mut events = db.watch(key).await?;
    let events = Arc::get_mut(&mut events).unwrap();
    log::info!("[TRACE] Start watching alert destinations");
    loop {
        let ev = match events.recv().await {
            Some(ev) => ev,
            None => {
                log::error!("watch_alert_destinations: event channel closed");
                break;
            }
        };
        match ev {
            Event::Put(ev) => {
                let item_key = ev.key.strip_prefix(key).unwrap();
                let item_value: AlertDestination = json::from_slice(&ev.value.unwrap()).unwrap();
                ALERTS_DESTINATIONS.insert(item_key.to_owned(), item_value);
            }
            Event::Delete(ev) => {
                let item_key = ev.key.strip_prefix(key).unwrap();
                ALERTS_DESTINATIONS.remove(item_key);
            }
        }
    }
    Ok(())
}

pub async fn cache() -> Result<(), anyhow::Error> {
    let db = &crate::infra::db::DEFAULT;
    let key = "/destinations/";
    let ret = db.list(key).await?;
    for (item_key, item_value) in ret {
        let item_key = item_key.strip_prefix(key).unwrap();
        let json_val: AlertDestination = json::from_slice(&item_value).unwrap();
        ALERTS_DESTINATIONS.insert(item_key.to_owned(), json_val);
    }
    log::info!("[TRACE] Alert destinations Cached");
    Ok(())
}
