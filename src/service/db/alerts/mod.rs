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
use std::sync::Arc;

use crate::common::json;
use crate::infra::config::STREAM_ALERTS;
use crate::infra::db::Event;
use crate::meta::alert::{Alert, AlertList};

pub mod destinations;
pub mod templates;

pub async fn get(
    org_id: &str,
    stream_name: &str,
    name: &str,
) -> Result<Option<Alert>, anyhow::Error> {
    let map_key = format!("{}/{}", org_id, stream_name);
    let value: Option<Alert> = if STREAM_ALERTS.contains_key(&map_key) {
        let mut val = STREAM_ALERTS.get(&map_key).unwrap().clone();
        val.list.retain(|alert| alert.name.eq(name));
        val.list.first().cloned()
    } else {
        let db = &crate::infra::db::DEFAULT;
        let key = format!("/alerts/{}/{}/{}", org_id, stream_name, name);
        match db.get(&key).await {
            Ok(val) => json::from_slice(&val).unwrap(),
            Err(_) => None,
        }
    };
    Ok(value)
}

pub async fn set(
    org_id: &str,
    stream_name: &str,
    name: &str,
    alert: Alert,
) -> Result<(), anyhow::Error> {
    let db = &crate::infra::db::DEFAULT;
    let key = format!("/alerts/{}/{}/{}", org_id, stream_name, name);
    db.put(&key, json::to_vec(&alert).unwrap().into()).await?;
    Ok(())
}

pub async fn delete(org_id: &str, stream_name: &str, name: &str) -> Result<(), anyhow::Error> {
    let db = &crate::infra::db::DEFAULT;
    let key = format!("/alerts/{}/{}/{}", org_id, stream_name, name);
    match db.delete(&key, false).await {
        Ok(_) => Ok(()),
        Err(e) => Err(anyhow::anyhow!(e)),
    }
}

pub async fn list(org_id: &str, stream_name: Option<&str>) -> Result<Vec<Alert>, anyhow::Error> {
    let db = &crate::infra::db::DEFAULT;
    let key = match stream_name {
        Some(val) => format!("/alerts/{}/{}", org_id, val),
        None => format!("/alerts/{}", org_id),
    };
    //let key = format!("/alerts/{}", org_id);
    let ret = db.list_values(&key).await?;
    let mut alerts_list: Vec<Alert> = Vec::new();
    for item_value in ret {
        let json_val = json::from_slice(&item_value).unwrap();
        alerts_list.push(json_val)
    }
    Ok(alerts_list)
}

pub async fn watch() -> Result<(), anyhow::Error> {
    let db = &crate::infra::db::DEFAULT;
    let key = "/alerts/";
    let mut events = db.watch(key).await?;
    let events = Arc::get_mut(&mut events).unwrap();
    log::info!("[TRACE] Start watching alerts");
    loop {
        let ev = match events.recv().await {
            Some(ev) => ev,
            None => {
                log::error!("watch_alerts: event channel closed");
                break;
            }
        };
        match ev {
            Event::Put(ev) => {
                let item_key = ev.key.strip_prefix(key).unwrap();
                let alert_key = item_key[0..item_key.rfind('/').unwrap()].to_string();
                let item_value: Alert = json::from_slice(&ev.value.unwrap()).unwrap();

                let mut group = STREAM_ALERTS
                    .entry(alert_key.to_string())
                    .or_insert(AlertList { list: vec![] });
                if group.list.contains(&item_value) {
                    let stream_name = group.list.iter().position(|x| x.eq(&item_value)).unwrap();
                    let _ = std::mem::replace(&mut group.list[stream_name], item_value);
                } else {
                    group.list.push(item_value);
                }
            }
            Event::Delete(ev) => {
                let item_key = ev.key.strip_prefix(key).unwrap();
                let alert_key = item_key[0..item_key.rfind('/').unwrap()].to_string();
                let item_name = item_key[item_key.rfind('/').unwrap() + 1..].to_string();
                if alert_key.contains('/') {
                    let mut group = STREAM_ALERTS.get(&alert_key.to_string()).unwrap().clone();
                    group.list.retain(|trans| !trans.name.eq(&item_name));
                    STREAM_ALERTS.insert(alert_key.to_string(), group);
                } else {
                    STREAM_ALERTS.remove(item_key);
                }
            }
        }
    }
    Ok(())
}

pub async fn cache() -> Result<(), anyhow::Error> {
    let db = &crate::infra::db::DEFAULT;
    let key = "/alerts/";
    let ret = db.list(key).await?;
    for (item_key, item_value) in ret {
        let item_key = item_key.strip_prefix(key).unwrap();
        let json_val: Alert = json::from_slice(&item_value).unwrap();
        let stream_key = &item_key[0..item_key.rfind('/').unwrap()];

        let mut group = STREAM_ALERTS
            .entry(stream_key.to_string())
            .or_insert(AlertList { list: vec![] });
        group.list.push(json_val);
    }
    log::info!("[TRACE] Alerts Cached");
    Ok(())
}
