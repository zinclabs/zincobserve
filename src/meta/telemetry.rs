use segment::{message::Track, Client, Message};
use serde_json::Value;
use std::collections::HashMap;

use crate::infra::config::{CONFIG, INSTANCE_ID, STREAM_SCHEMAS, TELEMETRY_CLIENT, USERS, VERSION};

#[derive(Clone, Debug, Default)]
pub struct Telemetry {
    pub instance_id: String,
    pub event: Track,
    pub base_info: HashMap<String, Value>,
}

impl Telemetry {
    pub fn new() -> Self {
        Telemetry {
            instance_id: "".to_string(),
            event: Track::default(),
            base_info: get_base_info(&mut HashMap::new()),
        }
    }

    pub fn add_event(&mut self, track: Track) {
        self.event = track;
    }

    pub async fn event(
        &mut self,
        event: &str,
        data: Option<HashMap<String, Value>>,
        send_zo_data: bool,
    ) {
        if !CONFIG.common.enable_telemetry {
            return;
        }
        log::info!("[TRACE] sending event {}", event);
        let mut props = self.base_info.clone();
        if data.is_some() {
            for item in data.unwrap() {
                props.insert(item.0, item.1);
            }
        }
        if send_zo_data {
            add_zo_info(&mut props)
        }
        self.add_event(Track {
            user: segment::message::User::UserId {
                user_id: segment::message::User::AnonymousId {
                    anonymous_id: INSTANCE_ID.get("instance_id").unwrap().to_string(),
                }
                .to_string(),
            },
            event: event.to_string(),
            properties: serde_json::to_value(props).unwrap(),
            timestamp: Some(time::OffsetDateTime::now_utc()),
            ..Default::default()
        });

        let res = TELEMETRY_CLIENT
            .send(
                INSTANCE_ID.get("instance_id").unwrap().to_string(),
                Message::from(self.event.clone()),
            )
            .await;

        if res.is_err() {
            log::error!("Error sending event {} , {:?}", event, res);
        }
    }

    pub async fn heart_beat(&mut self, event: &str, data: Option<HashMap<String, Value>>) {
        self.event(event, data, true).await;
    }
}

pub fn get_base_info(data: &mut HashMap<String, Value>) -> HashMap<String, Value> {
    data.insert("cpu_count".to_string(), sys_info::cpu_num().unwrap().into());
    data.insert(
        "total_memory".to_string(),
        sys_info::mem_info().unwrap().total.into(),
    );
    data.insert(
        "free_memory".to_string(),
        sys_info::mem_info().unwrap().free.into(),
    );
    data.insert(
        "avail_memory".to_string(),
        sys_info::mem_info().unwrap().avail.into(),
    );
    data.insert("os".to_string(), sys_info::os_type().unwrap().into());
    data.insert(
        "os_release".to_string(),
        sys_info::os_release().unwrap().into(),
    );
    data.insert(
        "time_zone".to_string(),
        chrono::Local::now().offset().local_minus_utc().into(),
    );
    data.insert(
        "host_name".to_string(),
        CONFIG.common.instance_name.clone().into(),
    );
    //TODO
    data.insert("zo_version".to_string(), VERSION.to_owned().into());
    data.clone()
}

pub fn add_zo_info(data: &mut HashMap<String, Value>) {
    let iter = STREAM_SCHEMAS.iter().clone();
    let mut num_streams = 0;
    for item in iter {
        num_streams += item.value().len()
    }
    data.insert("num_org".to_string(), STREAM_SCHEMAS.len().into());
    data.insert("num_streams".to_string(), num_streams.into());
    data.insert("num_users".to_string(), USERS.len().into());
}

#[cfg(test)]
mod test_telemetry {
    use super::*;
    #[test]
    fn test_telemetry_new() {
        let mut tel = Telemetry::new();
        add_zo_info(&mut tel.base_info);
        assert!(tel.base_info.len() > 0)
    }
}
