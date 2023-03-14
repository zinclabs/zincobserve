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

use serde_json::Value;
use std::error::Error as StdError;

use crate::{
    meta::alert::{self, Alert},
    service::db,
};

pub async fn send_notification(
    alert: &Alert,
    trigger: &alert::Trigger,
) -> Result<(), Box<dyn StdError>> {
    let client = reqwest::Client::new();

    let alert_type = match &trigger.is_ingest_time {
        true => "Real time",
        false => "Scheduled",
    };

    let curr_ts = chrono::Utc::now().timestamp_micros();
    match db::alerts::destinations::get(&trigger.org, &alert.destination).await {
        Ok(dest) => match dest {
            Some(local_dest) => {
                let body = local_dest.template.unwrap().body;
                let resp_str = serde_json::to_string(&body).unwrap();

                let mut resp = resp_str
                    .replace("{stream_name}", &trigger.stream)
                    .replace("{org_name}", &trigger.org)
                    .replace("{alert_name}", &trigger.alert_name)
                    .replace("{alert_type}", alert_type)
                    .replace("{timestamp}", &curr_ts.to_string());

                // Replace contextual information with values if any from alert
                if alert.context_attributes.is_some() {
                    for (key, value) in alert.context_attributes.as_ref().unwrap() {
                        resp = resp.replace(&format!("{{{}}}", key), value)
                    }
                }

                let msg: Value = serde_json::from_str(&resp).unwrap();
                let url = url::Url::parse(&local_dest.url);
                let mut req = match local_dest.method {
                    alert::AlertHTTPType::POST => client.post(url.unwrap()),
                    alert::AlertHTTPType::PUT => client.put(url.unwrap()),
                    alert::AlertHTTPType::GET => client.get(url.unwrap()),
                }
                .header("Content-type", "application/json");

                // Add additional headers if any from destination description
                if local_dest.headers.is_some() {
                    for (key, value) in local_dest.headers.unwrap() {
                        req = req.header(key, value);
                    }
                };

                let resp = req.json(&msg).send().await;
                match resp {
                    Ok(_) => log::info!("Notification Sent"),
                    Err(err) => log::info!("Error sending notification {:?}", err),
                }
            }

            None => log::info!("Destination Not found"),
        },
        Err(err) => log::info!("Error sending notification {:?}", err),
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::meta::{
        alert::{AlertDestination, Condition, DestinationTemplate, Trigger},
        search::Query,
    };

    use super::*;

    #[actix_web::test]
    async fn test_send_notification() {
        let template = DestinationTemplate {
            name: Some("testTemplate".to_string()),
            body: "Test Body".into(),
        };
        let _ = db::alerts::templates::set("default", "testTemplate", template);

        let destination = AlertDestination {
            url: "http://dummy/alert".to_string(),
            method: alert::AlertHTTPType::POST,
            headers: None,
            template: "testTemplate".to_string(),
        };
        let _ = db::alerts::destinations::set("default", "testDest", destination);

        let obj: Trigger = Trigger {
            timestamp: chrono::Utc::now().timestamp_micros(),
            is_valid: true,
            alert_name: "Test Alert".to_string(),
            stream: "olympics".to_string(),
            org: "default".to_string(),
            last_sent_at: chrono::Utc::now().timestamp_micros(),
            count: 1,
            is_ingest_time: true,
        };
        let alert = Alert {
            name: "testAlert".to_string(),
            stream: "olympics".to_string(),
            query: Some(Query {
                sql: "select * from olympics".to_string(),
                from: 0,
                size: 0,
                start_time: 0,
                end_time: 0,
                sql_mode: "full".to_string(),
                track_total_hits: false,
            }),
            condition: Condition {
                column: "Country".to_string(),
                operator: alert::AllOperator::EqualTo,
                ignore_case: Some(false),
                value: serde_json::Value::String("USA".to_string()),
                is_numeric: Some(false),
            },
            duration: 5,
            frequency: 1,
            time_between_alerts: 10,
            destination: "testDest".to_string(),
            is_real_time: true,
            context_attributes: None,
        };

        send_notification(&alert, &obj).await.unwrap();
    }
}
