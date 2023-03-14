use std::sync::Arc;

use crate::common::json;
use crate::infra::config::ALERTS_TEMPLATES;
use crate::infra::db::Event;
use crate::meta::alert::DestinationTemplate;

pub async fn get(org_id: &str, name: &str) -> Result<Option<DestinationTemplate>, anyhow::Error> {
    let map_key = format!("{}/{}", org_id, name);
    let value: Option<DestinationTemplate> = if ALERTS_TEMPLATES.contains_key(&map_key) {
        Some(ALERTS_TEMPLATES.get(&map_key).unwrap().clone())
    } else {
        let db = &crate::infra::db::DEFAULT;
        let key = format!("/templates/{}/{}", org_id, name);
        match db.get(&key).await {
            Ok(val) => json::from_slice(&val).unwrap(),
            Err(_) => None,
        }
    };
    Ok(value)
}

pub async fn set(
    org_id: &str,
    name: &str,
    template: DestinationTemplate,
) -> Result<(), anyhow::Error> {
    let db = &crate::infra::db::DEFAULT;
    let key = format!("/templates/{}/{}", org_id, name);
    db.put(&key, json::to_vec(&template).unwrap().into())
        .await?;
    Ok(())
}

pub async fn delete(org_id: &str, name: &str) -> Result<(), anyhow::Error> {
    let db = &crate::infra::db::DEFAULT;
    let key = format!("/templates/{}/{}", org_id, name);
    match db.delete(&key, false).await {
        Ok(_) => Ok(()),
        Err(e) => Err(anyhow::anyhow!(e)),
    }
}

pub async fn list(org_id: &str) -> Result<Vec<DestinationTemplate>, anyhow::Error> {
    let db = &crate::infra::db::DEFAULT;

    let key = format!("/templates/{}", org_id);
    let ret = db.list_values(&key).await?;
    let mut temp_list: Vec<DestinationTemplate> = Vec::new();
    for item_value in ret {
        let json_val = json::from_slice(&item_value).unwrap();
        temp_list.push(json_val)
    }
    Ok(temp_list)
}

pub async fn watch() -> Result<(), anyhow::Error> {
    let db = &crate::infra::db::DEFAULT;
    let key = "/templates/";
    let mut events = db.watch(key).await?;
    let events = Arc::get_mut(&mut events).unwrap();
    log::info!("[TRACE] Start watching alert templates");
    loop {
        let ev = match events.recv().await {
            Some(ev) => ev,
            None => {
                log::error!("watch_alert_templates: event channel closed");
                break;
            }
        };
        match ev {
            Event::Put(ev) => {
                let item_key = ev.key.strip_prefix(key).unwrap();
                let item_value: DestinationTemplate = json::from_slice(&ev.value.unwrap()).unwrap();
                ALERTS_TEMPLATES.insert(item_key.to_owned(), item_value);
            }
            Event::Delete(ev) => {
                let item_key = ev.key.strip_prefix(key).unwrap();
                ALERTS_TEMPLATES.remove(item_key);
            }
        }
    }
    Ok(())
}

pub async fn cache() -> Result<(), anyhow::Error> {
    let db = &crate::infra::db::DEFAULT;
    let key = "/templates/";
    let ret = db.list(key).await?;
    for (item_key, item_value) in ret {
        let item_key = item_key.strip_prefix(key).unwrap();
        let json_val: DestinationTemplate = json::from_slice(&item_value).unwrap();
        ALERTS_TEMPLATES.insert(item_key.to_owned(), json_val);
    }
    log::info!("[TRACE] Alert templates Cached");
    Ok(())
}
