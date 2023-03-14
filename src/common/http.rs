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

use actix_web::web::Query;
use std::collections::HashMap;
use std::io::{Error, ErrorKind};

use crate::meta::StreamType;

#[inline(always)]
pub fn get_stream_type_from_request(
    query: &Query<HashMap<String, String>>,
) -> Result<Option<StreamType>, Error> {
    let stream_type = match query.get("type") {
        Some(s) => match s.to_lowercase().as_str() {
            "logs" => Some(StreamType::Logs),
            "metrics" => Some(StreamType::Metrics),
            "traces" => Some(StreamType::Traces),
            _ => {
                return Err(Error::new(
                    ErrorKind::Other,
                    "'type' query param with value 'logs', 'metrics' or 'traces' allowed",
                ));
            }
        },
        None => None,
    };

    Ok(stream_type)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_get_file_from_cache() {
        let key = "type".to_string();

        let mut map: HashMap<String, String> = HashMap::new();
        map.insert(key.clone(), key.clone());

        let resp = get_stream_type_from_request(&Query { 0: map.clone() });
        assert!(resp.is_err());

        map.insert(key.clone(), "LOGS".to_string());
        let resp = get_stream_type_from_request(&Query { 0: map.clone() });
        assert_eq!(resp.unwrap(), Some(StreamType::Logs));

        map.insert(key.clone(), "METRICS".to_string());
        let resp = get_stream_type_from_request(&Query { 0: map.clone() });
        assert_eq!(resp.unwrap(), Some(StreamType::Metrics));

        map.insert(key.clone(), "TRACES".to_string());
        let resp = get_stream_type_from_request(&Query { 0: map.clone() });
        assert_eq!(resp.unwrap(), Some(StreamType::Traces));
    }
}
