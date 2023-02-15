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

use bytes::{Bytes, BytesMut};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::sync::{Arc, RwLock};

use crate::infra::config::CONFIG;
use crate::infra::ider;
use crate::meta::StreamType;

lazy_static! {
    pub static ref LOCKER: Locker = Locker::new();
    pub static ref FILES: RwLock<HashMap<String, Bytes>> = RwLock::new(HashMap::with_capacity(16));
}

pub struct Locker {
    data: Arc<Vec<RwData>>,
}

type RwData = RwLock<HashMap<String, Arc<RwFile>>>;

#[derive(Debug)]
pub struct RwFile {
    use_cache: bool,
    file: Option<RwLock<File>>,
    cache: Option<RwLock<BytesMut>>,
    dir: String,
    name: String,
    expired: i64,
}

#[inline]
pub fn get_or_create(
    thread_id: usize,
    org_id: &str,
    stream_name: &str,
    stream_type: StreamType,
    key: &str,
    use_cache: bool,
) -> Arc<RwFile> {
    LOCKER.get_or_create(thread_id, org_id, stream_name, stream_type, key, use_cache)
}

#[inline]
pub fn check_in_use(
    org_id: &str,
    stream_name: &str,
    stream_type: StreamType,
    file_name: &str,
) -> bool {
    LOCKER.check_in_use(org_id, stream_name, stream_type, file_name)
}

#[inline]
pub fn flush_all() {
    for data in LOCKER.data.iter() {
        let thread_data = data.read().unwrap();
        for (_, file) in thread_data.iter() {
            file.sync();
        }
    }

    for (file, data) in FILES.read().unwrap().iter() {
        let file_path = format!("{}{}", CONFIG.common.data_wal_dir, file);
        let mut f = OpenOptions::new()
            .write(true)
            .create(true)
            .append(true)
            .open(file_path)
            .unwrap();
        f.write_all(data).unwrap();
    }
}

impl Default for Locker {
    fn default() -> Self {
        Self::new()
    }
}

impl Locker {
    pub fn new() -> Locker {
        let size = CONFIG.limit.cpu_num;
        let mut data = Vec::with_capacity(size);
        for _i in 0..size {
            data.push(RwLock::new(HashMap::<String, Arc<RwFile>>::new()));
        }
        Locker {
            data: Arc::new(data),
        }
    }

    pub fn get(
        &self,
        thread_id: usize,
        org_id: &str,
        stream_name: &str,
        stream_type: StreamType,
        key: &str,
    ) -> Option<Arc<RwFile>> {
        let full_key = format!("{}_{}_{}_{}", org_id, stream_name, stream_type, key);
        let file = match self
            .data
            .get(thread_id)
            .unwrap()
            .read()
            .unwrap()
            .get(&full_key)
        {
            Some(file) => file.clone(),
            None => {
                return None;
            }
        };

        // check size & ttl
        if file.size() >= (CONFIG.limit.max_file_size_on_disk as i64)
            || file.expired() <= chrono::Utc::now().timestamp()
        {
            self.data
                .get(thread_id)
                .unwrap()
                .write()
                .unwrap()
                .remove(&full_key);
            file.sync();
            return None;
        }

        Some(file)
    }

    pub fn create(
        &self,
        thread_id: usize,
        org_id: &str,
        stream_name: &str,
        stream_type: StreamType,
        key: &str,
        use_cache: bool,
    ) -> Arc<RwFile> {
        let full_key = format!("{}_{}_{}_{}", org_id, stream_name, stream_type, key);
        let file = Arc::new(RwFile::new(
            thread_id,
            org_id,
            stream_name,
            stream_type,
            key,
            use_cache,
        ));
        let mut data = self.data.get(thread_id).unwrap().write().unwrap();
        data.insert(full_key, file.clone());
        file
    }

    pub fn get_or_create(
        &self,
        thread_id: usize,
        org_id: &str,
        stream_name: &str,
        stream_type: StreamType,
        key: &str,
        use_cache: bool,
    ) -> Arc<RwFile> {
        if let Some(file) = self.get(thread_id, org_id, stream_name, stream_type, key) {
            file
        } else {
            self.create(thread_id, org_id, stream_name, stream_type, key, use_cache)
        }
    }

    pub fn check_in_use(
        &self,
        org_id: &str,
        stream_name: &str,
        stream_type: StreamType,
        file_name: &str,
    ) -> bool {
        let columns = file_name.split('_').collect::<Vec<&str>>();
        let thread_id: usize = columns[0].parse().unwrap();
        let key = columns[1..columns.len() - 1].join("_");
        if let Some(file) = self.get(thread_id, org_id, stream_name, stream_type, &key) {
            if file.name() == file_name {
                return true;
            }
        }
        false
    }
}

impl RwFile {
    fn new(
        thread_id: usize,
        org_id: &str,
        stream_name: &str,
        stream_type: StreamType,
        key: &str,
        use_cache: bool,
    ) -> RwFile {
        let mut dir_path = format!(
            "{}files/{}/{}/{}/",
            &CONFIG.common.data_wal_dir, org_id, stream_type, stream_name
        );
        // Hack for file_list
        let file_list_prefix = "/files//file_list//";
        if dir_path.contains(file_list_prefix) {
            dir_path = dir_path.replace(file_list_prefix, "/file_list/");
        }
        let id = ider::generate();
        let file_name = format!(
            "{}_{}_{}{}",
            thread_id, key, id, &CONFIG.common.file_ext_json
        );
        let file_path = format!("{}{}", dir_path, file_name);
        std::fs::create_dir_all(&dir_path).unwrap();

        let (file, cache) = match use_cache {
            true => (
                None,
                Some(RwLock::new(BytesMut::with_capacity(
                    CONFIG.limit.max_file_size_on_disk as usize / 16,
                ))),
            ),
            false => {
                let f = OpenOptions::new()
                    .write(true)
                    .create(true)
                    .append(true)
                    .open(file_path)
                    .unwrap();
                (Some(RwLock::new(f)), None)
            }
        };
        RwFile {
            use_cache,
            file,
            cache,
            dir: dir_path,
            name: file_name,
            expired: chrono::Utc::now().timestamp() + CONFIG.limit.max_file_retention_time as i64,
        }
    }

    #[inline]
    pub fn write(&self, data: &[u8]) {
        match self.use_cache {
            true => {
                self.cache
                    .as_ref()
                    .unwrap()
                    .write()
                    .unwrap()
                    .extend_from_slice(data);
            }
            false => {
                self.file
                    .as_ref()
                    .unwrap()
                    .write()
                    .unwrap()
                    .write_all(data)
                    .unwrap();
            }
        }
    }

    #[inline]
    pub fn sync(&self) {
        match self.use_cache {
            true => {
                let file_path = format!("{}{}", self.dir, self.name);
                let file_path = file_path.strip_prefix(&CONFIG.common.data_wal_dir).unwrap();
                FILES.write().unwrap().insert(
                    file_path.to_string(),
                    self.cache
                        .as_ref()
                        .unwrap()
                        .read()
                        .unwrap()
                        .to_owned()
                        .freeze(),
                );
            }
            false => self
                .file
                .as_ref()
                .unwrap()
                .write()
                .unwrap()
                .sync_all()
                .unwrap(),
        };
    }

    #[inline]
    pub fn size(&self) -> i64 {
        match self.use_cache {
            true => self.cache.as_ref().unwrap().write().unwrap().len() as i64,
            false => self
                .file
                .as_ref()
                .unwrap()
                .read()
                .unwrap()
                .metadata()
                .unwrap()
                .len() as i64,
        }
    }

    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[inline]
    pub fn full_name(&self) -> String {
        format!("{}{}", self.dir, self.name)
    }

    #[inline]
    pub fn expired(&self) -> i64 {
        self.expired
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_loc_with_stream() {
        let locker = Locker::new();
        let thread_id = 1;
        let org_id = "org1";
        let stream_name = "stream1";
        let stream_type = StreamType::Logs;
        let key = "2022_10_17_10";
        let file = locker.get_or_create(thread_id, org_id, stream_name, stream_type, key, false);
        file.write(b"hello world");
        assert_eq!(file.size(), 11);
        assert_eq!(file.name().contains(key), true);
        assert_eq!(file.expired() > 0, true);
        assert_eq!(
            locker
                .get(1, org_id, stream_name, stream_type, key)
                .unwrap()
                .name(),
            file.name()
        );
    }

    #[test]
    fn test_file_loc_without_stream() {
        let locker = Locker::new();
        let thread_id = 1;
        let org_id = "";
        let stream_name = "";
        let stream_type = StreamType::Filelist;
        let key = "2022_10_17_10";
        let file = locker.get_or_create(thread_id, org_id, stream_name, stream_type, key, false);
        file.write(b"hello world");
        assert_eq!(file.size(), 11);
        assert_eq!(file.name().contains(key), true);
        assert_eq!(file.expired() > 0, true);
        assert_eq!(
            locker
                .get(1, org_id, stream_name, stream_type, key)
                .unwrap()
                .name(),
            file.name()
        );
    }
}
