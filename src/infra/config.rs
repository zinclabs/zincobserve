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

use dashmap::DashMap;
use datafusion::arrow::datatypes::Schema;
use dotenv::dotenv;
use dotenv_config::EnvConfig;
use reqwest::Client;
use std::time::Duration;
use sys_info::hostname;

use crate::common::file::get_file_meta;
use crate::meta::alert::{AlertList, Trigger, TriggerTimer};
use crate::meta::functions::{FunctionList, Transform};
use crate::meta::prom::ClusterLeader;
use crate::meta::user::User;

pub static VERSION: &str = env!("GIT_VERSION");
pub static COMMIT_HASH: &str = env!("GIT_COMMIT");
pub static BUILD_DATE: &str = env!("GIT_DATE");
#[cfg(feature = "zo_functions")]
pub static HAS_FUNCTIONS: bool = true;
#[cfg(not(feature = "zo_functions"))]
pub static HAS_FUNCTIONS: bool = false;

lazy_static! {
    pub static ref CONFIG: Config = init();
    pub static ref LOCKER: DashMap<String, std::sync::Mutex<bool>> = DashMap::new();
    pub static ref INSTANCE_ID: DashMap<String, String> = DashMap::new();
    pub static ref TELEMETRY_CLIENT: segment::HttpClient = segment::HttpClient::new(
        Client::builder()
            .connect_timeout(Duration::new(10, 0))
            .build()
            .unwrap(),
        CONFIG.common.telemetry_url.clone()
    );
}

lazy_static! {
    pub static ref STREAM_FUNCTIONS: DashMap<String, FunctionList> = DashMap::new();
    pub static ref STREAM_SCHEMAS: DashMap<String, Vec<Schema>> = DashMap::new();
    pub static ref QUERY_FUNCTIONS: DashMap<String, Transform> = DashMap::new();
    pub static ref USERS: DashMap<String, User> = DashMap::new();
    pub static ref PASSWORD_HASH: DashMap<String, String> = DashMap::new();
    #[derive(Debug)]
    pub static ref METRIC_CLUSTER_MAP: DashMap<String, Vec<String>> = DashMap::new();
    #[derive(Debug)]
    pub static ref METRIC_CLUSTER_LEADER: DashMap<String, ClusterLeader> = DashMap::new();
    pub static ref STREAM_ALERTS: DashMap<String, AlertList> = DashMap::new();
    pub static ref TRIGGERS: DashMap<String, Trigger> = DashMap::new();
    pub static ref TRIGGERS_IN_PROCESS: DashMap<String, TriggerTimer> = DashMap::new();
}

#[derive(Clone, Debug, EnvConfig)]
pub struct Config {
    pub auth: Auth,
    pub http: Http,
    pub grpc: Grpc,
    pub route: Route,
    pub common: Common,
    pub limit: Limit,
    pub compact: Compact,
    pub memory_cache: MemoryCache,
    pub log: Log,
    pub etcd: Etcd,
    pub sled: Sled,
    pub s3: S3,
}

#[derive(Clone, Debug, EnvConfig)]
pub struct Auth {
    #[env_config(name = "ZO_USER_NAME", default = "admin")]
    pub username: String,
    #[env_config(name = "ZO_USER_PASSWORD", default = "Complexpass#123")]
    pub password: String,
}

#[derive(Clone, Debug, EnvConfig)]
pub struct Http {
    #[env_config(name = "ZO_HTTP_PORT", default = 5080)]
    pub port: u16,
}

#[derive(Clone, Debug, EnvConfig)]
pub struct Grpc {
    #[env_config(name = "ZO_GRPC_PORT", default = 5081)]
    pub port: u16,
    #[env_config(name = "ZO_GRPC_TIMEOUT", default = 600)]
    pub timeout: u64,
    #[env_config(name = "ZO_GRPC_ORG_HEADER_KEY", default = "zinc-org-id")]
    pub org_header_key: String,
}

#[derive(Clone, Debug, EnvConfig)]
pub struct Route {
    #[env_config(name = "ZO_ROUTE_TIMEOUT", default = 600)]
    pub timeout: u64,
}

#[derive(Clone, Debug, EnvConfig)]
pub struct Common {
    #[env_config(name = "ZO_LOCAL_MODE", default = true)]
    pub local_mode: bool,
    // ZO_LOCAL_MODE_STORAGE is ignored when ZO_LOCAL_MODE is set to false
    #[env_config(name = "ZO_LOCAL_MODE_STORAGE", default = "disk")]
    pub local_mode_storage: String,
    #[env_config(name = "ZO_NODE_ROLE", default = "all")]
    pub node_role: String,
    #[env_config(name = "ZO_INSTANCE_NAME", default = "")]
    pub instance_name: String,
    #[env_config(name = "ZO_DATA_DIR", default = "./data/")]
    pub data_dir: String,
    #[env_config(name = "ZO_DATA_WAL_DIR", default = "./data/wal/")]
    pub data_wal_dir: String,
    #[env_config(name = "ZO_DATA_STREAM_DIR", default = "./data/stream/")]
    pub data_stream_dir: String,
    #[env_config(name = "ZO_WAL_MEMORY_MODE_ENABLED", default = false)]
    pub wal_memory_mode_enabled: bool,
    #[env_config(name = "ZO_FILE_EXT_JSON", default = ".json")]
    pub file_ext_json: String,
    #[env_config(name = "ZO_FILE_EXT_PARQUET", default = ".parquet")]
    pub file_ext_parquet: String,
    #[env_config(name = "ZO_PARQUET_COMPRESSION", default = "zstd")]
    pub parquet_compression: String,
    #[env_config(name = "ZO_TIME_STAMP_COL", default = "_timestamp")]
    pub time_stamp_col: String,
    #[env_config(name = "ZO_WIDENING_SCHEMA_EVOLUTION", default = false)]
    pub widening_schema_evoluation: bool,
    #[env_config(name = "ZO_FEATURE_PER_THREAD_LOCK", default = false)]
    pub feature_per_thread_lock: bool,
    #[env_config(name = "ZO_FEATURE_FULLTEXT_ON_ALL_FIELDS", default = false)]
    pub feature_fulltext_on_all_fields: bool,
    #[env_config(name = "ZO_UI_ENABLED", default = true)]
    pub ui_enabled: bool,
    #[env_config(name = "ZO_METRICS_DEDUP_ENABLED", default = true)]
    pub metrics_dedup_enabled: bool,
    #[env_config(name = "ZO_TRACING_ENABLED", default = false)]
    pub tracing_enabled: bool,
    #[env_config(
        name = "OTEL_OTLP_HTTP_ENDPOINT",
        default = "http://127.0.0.1:5080/api/nexus/traces"
    )]
    pub otel_otlp_url: String,
    #[env_config(name = "ZO_TRACING_HEADER_KEY", default = "Authorization")]
    pub tracing_header_key: String,
    #[env_config(
        name = "ZO_TRACING_HEADER_VALUE",
        default = "Basic YWRtaW46Q29tcGxleHBhc3MjMTIz"
    )]
    pub tracing_header_value: String,
    #[env_config(name = "ZO_TELEMETRY", default = true)]
    pub enable_telemetry: bool,
    #[env_config(name = "ZO_TELEMETRY_URL", default = "https://e1.zinclabs.dev")]
    pub telemetry_url: String,
}

#[derive(Clone, Debug, EnvConfig)]
pub struct Limit {
    #[env_config(name = "ZO_JSON_LIMIT", default = 209715200)]
    pub req_json_limit: usize,
    #[env_config(name = "ZO_PAYLOAD_LIMIT", default = 209715200)]
    pub req_payload_limit: usize,
    #[env_config(name = "ZO_MAX_FILE_SIZE_ON_DISK", default = 10)] // MB
    pub max_file_size_on_disk: u64,
    #[env_config(name = "ZO_MAX_FILE_RETENTION_TIME", default = 600)] // seconds
    pub max_file_retention_time: u64,
    #[env_config(name = "ZO_FILE_PUSH_INTERVAL", default = 10)] // seconds
    pub file_push_interval: u64,
    #[env_config(name = "ZO_FILE_MOVE_THREAD_NUM", default = 0)]
    pub file_move_thread_num: usize,
    #[env_config(name = "ZO_QUERY_THREAD_NUM", default = 0)]
    pub query_thread_num: usize,
    #[env_config(name = "ZO_TS_ALLOWED_UPTO", default = 5)] // in hours - in past
    pub allowed_upto: i64,
    #[env_config(name = "ZO_METRICS_LEADER_PUSH_INTERVAL", default = 15)]
    pub metrics_leader_push_interval: u64,
    #[env_config(name = "ZO_METRICS_LEADER_ELECTION_INTERVAL", default = 30)]
    pub metrics_leader_election_interval: i64,
    #[env_config(name = "ZO_HEARTBEAT_INTERVAL", default = 30)] // in minutes
    pub hb_interval: i64,
    // no need set by environment
    pub cpu_num: usize,
}

#[derive(Clone, Debug, EnvConfig)]
pub struct Compact {
    #[env_config(name = "ZO_COMPACT_ENABLED", default = true)]
    pub enabled: bool,
    #[env_config(name = "ZO_COMPACT_INTERVAL", default = 600)] // seconds
    pub interval: u64,
    #[env_config(name = "ZO_COMPACT_MAX_FILE_SIZE", default = 256)] // MB
    pub max_file_size: u64,
}

#[derive(Clone, Debug, EnvConfig)]
pub struct MemoryCache {
    #[env_config(name = "ZO_MEMORY_CACHE_ENABLED", default = true)]
    pub enabled: bool,
    #[env_config(name = "ZO_MEMORY_CACHE_CACHE_LATEST_FILES", default = false)]
    pub cache_latest_files: bool,
    #[env_config(name = "ZO_MEMORY_CACHE_MAX_SIZE", default = 0)]
    // MB, default is half of system memory
    pub max_size: usize,
    #[env_config(name = "ZO_MEMORY_CACHE_RELEASE_SIZE", default = 0)]
    // MB, default is 1% of max_size
    pub release_size: usize,
}

#[derive(Clone, Debug, EnvConfig)]
pub struct Log {
    #[env_config(name = "RUST_LOG", default = "info")]
    pub level: String,
}

#[derive(Clone, Debug, EnvConfig)]
pub struct Etcd {
    #[env_config(name = "ZO_ETCD_ADDR", default = "localhost:2379")]
    pub addr: String,
    #[env_config(name = "ZO_ETCD_PREFIX", default = "/zinc/observe/")]
    pub prefix: String,
    #[env_config(name = "ZO_ETCD_CONNECT_TIMEOUT", default = 2)]
    pub connect_timeout: u64,
    #[env_config(name = "ZO_ETCD_COMMAND_TIMEOUT", default = 5)]
    pub command_timeout: u64,
    #[env_config(name = "ZO_ETCD_LOCK_WAIT_TIMEOUT", default = 600)]
    pub lock_wait_timeout: u64,
    #[env_config(name = "ZO_ETCD_USER", default = "")]
    pub user: String,
    #[env_config(name = "ZO_ETCD_PASSWORD", default = "")]
    pub password: String,
    #[env_config(name = "ZO_ETCD_CLIENT_CERT_AUTH", default = false)]
    pub cert_auth: bool,
    #[env_config(name = "ZO_ETCD_TRUSTED_CA_FILE", default = "")]
    pub ca_file: String,
    #[env_config(name = "ZO_ETCD_CERT_FILE", default = "")]
    pub cert_file: String,
    #[env_config(name = "ZO_ETCD_KEY_FILE", default = "")]
    pub key_file: String,
    #[env_config(name = "ZO_ETCD_DOMAIN_NAME", default = "")]
    pub domain_name: String,
    #[env_config(name = "ZO_ETCD_LOAD_PAGE_SIZE", default = 10000)]
    pub load_page_size: i64,
}

#[derive(Clone, Debug, EnvConfig)]
pub struct Sled {
    #[env_config(name = "ZO_SLED_DATA_DIR", default = "./data/db/")]
    pub data_dir: String,
    #[env_config(name = "ZO_SLED_PREFIX", default = "/zinc/observe/")]
    pub prefix: String,
}

#[derive(Clone, Debug, EnvConfig)]
pub struct S3 {
    #[env_config(name = "ZO_S3_SERVER_URL", default = "")]
    pub server_url: String,
    #[env_config(name = "ZO_S3_REGION_NAME", default = "")]
    pub region_name: String,
    #[env_config(name = "ZO_S3_ACCESS_KEY", default = "")]
    pub access_key: String,
    #[env_config(name = "ZO_S3_SECRET_KEY", default = "")]
    pub secret_key: String,
    #[env_config(name = "ZO_S3_BUCKET_NAME", default = "")]
    pub bucket_name: String,
}

pub fn init() -> Config {
    dotenv().ok();
    let mut cfg = Config::init().unwrap();
    // set cpu num
    let cpu_num = sys_info::cpu_num().unwrap() as usize;
    cfg.limit.cpu_num = cpu_num;
    // HACK for thread_num equal to CPU core * 4
    if cfg.limit.query_thread_num == 0 {
        cfg.limit.query_thread_num = cpu_num * 4;
    }
    // HACK for move_file_thread_num equal to CPU core
    if cfg.limit.file_move_thread_num == 0 {
        cfg.limit.file_move_thread_num = cpu_num;
    }
    if cfg.limit.file_push_interval == 0 {
        cfg.limit.file_push_interval = 10;
    }
    // HACK instance_name
    if cfg.common.instance_name.is_empty() {
        cfg.common.instance_name = hostname().unwrap();
    }
    // check max_file_size_on_disk to MB
    cfg.limit.max_file_size_on_disk *= 1024 * 1024;
    // check compact_max_file_size to MB
    cfg.compact.max_file_size *= 1024 * 1024;
    if cfg.compact.interval == 0 {
        cfg.compact.interval = 600;
    }
    // format local_mode_storage
    cfg.common.local_mode_storage = cfg.common.local_mode_storage.to_lowercase();

    // check data path config
    if let Err(e) = check_path_config(&mut cfg) {
        panic!("data path config error: {}", e);
    }

    // check memeory cache
    if let Err(e) = check_memory_cache_config(&mut cfg) {
        panic!("memory cache config error: {}", e);
    }

    // check etcd config
    if let Err(e) = check_etcd_config(&mut cfg) {
        panic!("etcd config error: {}", e);
    }
    cfg
}

fn check_path_config(cfg: &mut Config) -> Result<(), anyhow::Error> {
    if !cfg.common.data_wal_dir.ends_with('/') {
        cfg.common.data_wal_dir = format!("{}/", cfg.common.data_wal_dir);
    }
    if !cfg.common.data_stream_dir.ends_with('/') {
        cfg.common.data_stream_dir = format!("{}/", cfg.common.data_stream_dir);
    }
    if !cfg.sled.data_dir.ends_with('/') {
        cfg.sled.data_dir = format!("{}/", cfg.sled.data_dir);
    }
    Ok(())
}

fn check_etcd_config(cfg: &mut Config) -> Result<(), anyhow::Error> {
    if !cfg.etcd.cert_auth {
        return Ok(());
    }
    if let Err(e) = get_file_meta(&cfg.etcd.ca_file) {
        return Err(anyhow::anyhow!("ZO_ETCD_TRUSTED_CA_FILE check err: {}", e));
    }
    if let Err(e) = get_file_meta(&cfg.etcd.cert_file) {
        return Err(anyhow::anyhow!("ZO_ETCD_TRUSTED_CA_FILE check err: {}", e));
    }
    if let Err(e) = get_file_meta(&cfg.etcd.key_file) {
        return Err(anyhow::anyhow!("ZO_ETCD_TRUSTED_CA_FILE check err: {}", e));
    }

    // check domain name
    if cfg.etcd.domain_name.is_empty() {
        let mut name = cfg.etcd.addr.clone();
        if name.contains("//") {
            name = name.split("//").collect::<Vec<&str>>()[1].to_string();
        }
        if name.contains(':') {
            name = name.split(':').collect::<Vec<&str>>()[0].to_string();
        }
        cfg.etcd.domain_name = name;
    }

    Ok(())
}

fn check_memory_cache_config(cfg: &mut Config) -> Result<(), anyhow::Error> {
    if cfg.memory_cache.max_size == 0 {
        // meminfo unit is KB
        let meminfo = sys_info::mem_info()?;
        cfg.memory_cache.max_size = meminfo.total as usize * 1024 / 2;
    } else {
        cfg.memory_cache.max_size *= 1024 * 1024;
    }
    if cfg.memory_cache.release_size == 0 {
        // when memory cache is full will release 1% (default)
        cfg.memory_cache.release_size = cfg.memory_cache.max_size / 100;
    } else {
        cfg.memory_cache.release_size *= 1024 * 1024;
    }
    Ok(())
}

#[inline]
pub fn get_parquet_compression() -> parquet::basic::Compression {
    match CONFIG.common.parquet_compression.to_lowercase().as_str() {
        "snappy" => parquet::basic::Compression::SNAPPY,
        "gzip" => parquet::basic::Compression::GZIP,
        "brotli" => parquet::basic::Compression::BROTLI,
        "lz4" => parquet::basic::Compression::LZ4_RAW,
        "zstd" => parquet::basic::Compression::ZSTD,
        _ => parquet::basic::Compression::ZSTD,
    }
}
