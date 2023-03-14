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

use segment::{message::Track, Client, Message};
use serde_json::Value;
use std::collections::HashMap;

use crate::infra::config::{
    CONFIG, INSTANCE_ID, ROOT_USER, STREAM_SCHEMAS, TELEMETRY_CLIENT, USERS, VERSION,
};

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
        if !CONFIG.common.telemetry_enabled {
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
            log::error!("Error sending event {}, {:?}", event, res);
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

    data.insert("zo_version".to_string(), VERSION.to_owned().into());
    data.insert(
        "root_user".to_string(),
        serde_json::Value::String(ROOT_USER.get("root").unwrap().clone().email),
    );
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
    data.insert(
        "is_local_mode".to_string(),
        serde_json::Value::Bool(CONFIG.common.local_mode),
    );
    if CONFIG.common.local_mode {
        data.insert(
            "local_mode_storage".to_string(),
            CONFIG.common.local_mode_storage.clone().into(),
        );
    }

    let nodes = crate::infra::cluster::load_local_node_role();
    if !crate::infra::cluster::is_single_node(nodes) {
        match crate::infra::cluster::get_cached_online_nodes() {
            Some(nodes) => {
                data.insert("is_HA_mode".to_string(), serde_json::Value::Bool(true));
                data.insert("number_of_nodes".to_string(), nodes.len().into());
            }
            None => {
                data.insert("is_HA_mode".to_string(), serde_json::Value::Bool(false));
            }
        }
    }
}

#[cfg(test)]
mod test_telemetry {
    use super::*;
    #[test]
    fn test_telemetry_new() {
        ROOT_USER.insert(
            "root".to_string(),
            crate::meta::user::User {
                email: "admin@zo.dev".to_string(),
                password: "pass#123".to_string(),
                role: crate::meta::user::UserRole::Root,
                salt: String::new(),
                token: "token".to_string(),
                first_name: "admin".to_owned(),
                last_name: "".to_owned(),
                org: "default".to_string(),
            },
        );
        let tel = Telemetry::new();
        let mut props = tel.base_info.clone();
        add_zo_info(&mut props);
        assert!(tel.base_info.len() > 0)
    }
}
