use actix_web_prometheus::{PrometheusMetrics, PrometheusMetricsBuilder};
use prometheus::{
    CounterVec, HistogramOpts, HistogramVec, IntCounterVec, IntGaugeVec, Opts, Registry,
};
use std::collections::HashMap;

use super::config::CONFIG;

pub const NAMESPACE: &str = "zo";

lazy_static! {
    // http latency
    pub static ref HTTP_INCOMING_REQUESTS: IntCounterVec = IntCounterVec::new(
        Opts::new( "http_incoming_requests",  "HTTP incoming requests")
            .namespace(NAMESPACE)
            .const_labels(create_const_labels()),
        &["endpoint", "status", "organization", "stream", "stream_type"],
    ).expect("Metric created");
    pub static ref HTTP_RESPONSE_TIME: HistogramVec = HistogramVec::new(
        HistogramOpts::new("http_response_time", "HTTP response time")
            .namespace(NAMESPACE)
            .const_labels(create_const_labels()),
        &["endpoint","status", "organization", "stream", "stream_type"],
    ).expect("Metric created");

    // grpc latency
    pub static ref GRPC_INCOMING_REQUESTS: IntCounterVec = IntCounterVec::new(
        Opts::new("grpc_incoming_requests", "gRPC incoming requests")
            .namespace(NAMESPACE)
            .const_labels(create_const_labels()),
        &["endpoint", "status", "organization", "stream", "stream_type"],
    ).expect("Metric created");
    pub static ref GRPC_RESPONSE_TIME: HistogramVec = HistogramVec::new(
        HistogramOpts::new("grpc_response_time", "gRPC response time")
            .namespace(NAMESPACE)
            .const_labels(create_const_labels()),
        &["endpoint", "status", "organization", "stream", "stream_type"],
    ).expect("Metric created");

    // ingester stats
    pub static ref INGEST_RECORDS: IntGaugeVec = IntGaugeVec::new(
        Opts::new("ingest_records", "Ingested records")
            .namespace(NAMESPACE)
            .const_labels(create_const_labels()),
        &["organization", "stream", "stream_type"],
    ).expect("Metric created");
    pub static ref INGEST_BYTES: IntGaugeVec = IntGaugeVec::new(
        Opts::new("ingest_bytes", "Ingested bytes")
            .namespace(NAMESPACE)
            .const_labels(create_const_labels()),
        &["organization", "stream", "stream_type"],
    ).expect("Metric created");
    pub static ref INGEST_WAL_USED_BYTES: IntGaugeVec = IntGaugeVec::new(
        Opts::new("ingest_wal_used_bytes", "Ingestor WAL used bytes")
            .namespace(NAMESPACE)
            .const_labels(create_const_labels()),
        &["organization", "stream", "stream_type"],
    ).expect("Metric created");
    pub static ref INGEST_WAL_WRITE_BYTES: IntCounterVec = IntCounterVec::new(
        Opts::new("ingest_wal_write_bytes", "Ingestor WAL write bytes")
            .namespace(NAMESPACE)
            .const_labels(create_const_labels()),
        &["organization", "stream", "stream_type"],
    ).expect("Metric created");
    pub static ref INGEST_WAL_READ_BYTES: IntCounterVec = IntCounterVec::new(
        Opts::new("ingest_wal_read_bytes", "Ingestor WAL read bytes")
            .namespace(NAMESPACE)
            .const_labels(create_const_labels()),
        &["organization", "stream", "stream_type"],
    ).expect("Metric created");

    // querier stats
    pub static ref QUERY_CACHE_LIMIT_BYTES: IntGaugeVec = IntGaugeVec::new(
        Opts::new("query_cache_limit_bytes", "Querier cache limit bytes")
            .namespace(NAMESPACE)
            .const_labels(create_const_labels()),
        &["organization", "stream", "stream_type"],
    ).expect("Metric created");
    pub static ref QUERY_CACHE_USED_BYTES: IntGaugeVec = IntGaugeVec::new(
        Opts::new("query_cache_used_bytes", "Querier cache used bytes")
            .namespace(NAMESPACE)
            .const_labels(create_const_labels()),
        &["organization", "stream", "stream_type"],
    ).expect("Metric created");
    pub static ref QUERY_CACHE_FILES: IntGaugeVec = IntGaugeVec::new(
        Opts::new("query_cache_files", "Querier cached files")
            .namespace(NAMESPACE)
            .const_labels(create_const_labels()),
        &["organization", "stream", "stream_type"],
    ).expect("Metric created");
    pub static ref QUERY_CACHE_RECORDS: IntGaugeVec = IntGaugeVec::new(
        Opts::new("query_cache_records", "Querier cached records")
            .namespace(NAMESPACE)
            .const_labels(create_const_labels()),
        &["organization", "stream", "stream_type"],
    ).expect("Metric created");

    // compactor stats
    pub static ref COMPACT_USED_TIME: CounterVec = CounterVec::new(
        Opts::new("compact_used_time", "Compactor used time")
            .namespace(NAMESPACE)
            .const_labels(create_const_labels()),
        &["organization", "stream", "stream_type"],
    ).expect("Metric created");
    pub static ref COMPACT_MERGED_FILES: IntCounterVec = IntCounterVec::new(
        Opts::new("compact_merged_files", "Compactor merged files")
            .namespace(NAMESPACE)
            .const_labels(create_const_labels()),
        &["organization", "stream", "stream_type"],
    ).expect("Metric created");
    pub static ref COMPACT_MERGED_BYTES: IntCounterVec = IntCounterVec::new(
        Opts::new("compact_merged_bytes", "Compactor merged bytes")
            .namespace(NAMESPACE)
            .const_labels(create_const_labels()),
        &["organization", "stream", "stream_type"],
    ).expect("Metric created");
    pub static ref COMPACT_DELAY_HOURS: IntGaugeVec = IntGaugeVec::new(
        Opts::new("compact_delay_hours", "Compactor delay hours")
            .namespace(NAMESPACE)
            .const_labels(create_const_labels()),
        &["organization", "stream", "stream_type"],
    ).expect("Metric created");
    // TODO deletion / archiving stats

    // storage stats
    pub static ref STORAGE_ORIGINAL_BYTES: IntGaugeVec = IntGaugeVec::new(
        Opts::new("storage_original_bytes", "Storage original bytes")
            .namespace(NAMESPACE)
            .const_labels(create_const_labels()),
        &["organization", "stream", "stream_type"],
    ).expect("Metric created");
    pub static ref STORAGE_COMPRESSED_BYTES: IntGaugeVec = IntGaugeVec::new(
        Opts::new("storage_compressed_bytes", "Storage compressed bytes")
            .namespace(NAMESPACE)
            .const_labels(create_const_labels()),
        &["organization", "stream", "stream_type"],
    ).expect("Metric created");
    pub static ref STORAGE_FILES: IntGaugeVec = IntGaugeVec::new(
        Opts::new("storage_files", "Storage files")
            .namespace(NAMESPACE)
            .const_labels(create_const_labels()),
        &["organization", "stream", "stream_type"],
    ).expect("Metric created");
    pub static ref STORAGE_RECORDS: IntGaugeVec = IntGaugeVec::new(
        Opts::new("storage_records", "Storage records")
            .namespace(NAMESPACE)
            .const_labels(create_const_labels()),
        &["organization", "stream", "stream_type"],
    ).expect("Metric created");
    pub static ref STORAGE_WRITE_BYTES: IntGaugeVec = IntGaugeVec::new(
        Opts::new("storage_write_bytes", "Storage write bytes")
            .namespace(NAMESPACE)
            .const_labels(create_const_labels()),
        &["organization", "stream", "stream_type"],
    ).expect("Metric created");
    pub static ref STORAGE_READ_BYTES: IntGaugeVec = IntGaugeVec::new(
        Opts::new("storage_read_bytes", "Storage read bytes")
            .namespace(NAMESPACE)
            .const_labels(create_const_labels()),
        &["organization", "stream", "stream_type"],
    ).expect("Metric created");

    // metadata stats
    pub static ref META_STORAGE_BYTES: IntGaugeVec = IntGaugeVec::new(
        Opts::new("meta_storage_bytes", "Metadata storage used bytes")
            .namespace(NAMESPACE)
            .const_labels(create_const_labels()),
        &[],
    ).expect("Metric created");
    pub static ref META_STORAGE_KEYS: IntGaugeVec = IntGaugeVec::new(
        Opts::new("meta_storage_keys", "Metadata storage item keys")
            .namespace(NAMESPACE)
            .const_labels(create_const_labels()),
        &[],
    ).expect("Metric created");
    pub static ref META_NUM_NODES: IntGaugeVec = IntGaugeVec::new(
        Opts::new("meta_num_nodes", "Metadata node nums")
            .namespace(NAMESPACE)
            .const_labels(create_const_labels()),
        &["node_role"],
    ).expect("Metric created");
    pub static ref META_NUM_ORGANIZATIONS: IntGaugeVec = IntGaugeVec::new(
        Opts::new("meta_num_organizations", "Metadata organization nums")
            .namespace(NAMESPACE)
            .const_labels(create_const_labels()),
        &[],
    ).expect("Metric created");
    pub static ref META_NUM_STREAMS: IntGaugeVec = IntGaugeVec::new(
        Opts::new("meta_num_streams", "Metadata stream nums")
            .namespace(NAMESPACE)
            .const_labels(create_const_labels()),
        &["organization", "stream_type"],
    ).expect("Metric created");
    pub static ref META_NUM_USERS_TOTAL: IntGaugeVec = IntGaugeVec::new(
        Opts::new("meta_num_users_total", "Metadata user total")
            .namespace(NAMESPACE)
            .const_labels(create_const_labels()),
        &[],
    ).expect("Metric created");
    pub static ref META_NUM_USERS: IntGaugeVec = IntGaugeVec::new(
        Opts::new("meta_num_users", "Metadata user nums")
            .namespace(NAMESPACE)
            .const_labels(create_const_labels()),
        &["organization"],
    ).expect("Metric created");
    pub static ref META_NUM_FUNCTIONS: IntGaugeVec = IntGaugeVec::new(
        Opts::new("meta_num_functions", "Metadata function nums")
            .namespace(NAMESPACE)
            .const_labels(create_const_labels()),
        &["organization", "stream", "stream_type", "function_type"],
    ).expect("Metric created");
    pub static ref META_NUM_ALERTS: IntGaugeVec = IntGaugeVec::new(
        Opts::new("meta_num_alerts", "Metadata alert nums")
            .namespace(NAMESPACE)
            .const_labels(create_const_labels()),
        &["organization", "stream", "stream_type"],
    ).expect("Metric created");
    pub static ref META_NUM_DASHBOARDS: IntGaugeVec = IntGaugeVec::new(
        Opts::new("meta_num_dashboards", "Metadata dashboard nums")
            .namespace(NAMESPACE)
            .const_labels(create_const_labels()),
        &["organization"],
    ).expect("Metric created");
}

fn register_metrics(registry: &Registry) {
    // http latency
    registry
        .register(Box::new(HTTP_INCOMING_REQUESTS.clone()))
        .expect("Metric registered");
    registry
        .register(Box::new(HTTP_RESPONSE_TIME.clone()))
        .expect("Metric registered");

    // grpc latency
    registry
        .register(Box::new(GRPC_INCOMING_REQUESTS.clone()))
        .expect("Metric registered");
    registry
        .register(Box::new(GRPC_RESPONSE_TIME.clone()))
        .expect("Metric registered");

    // ingester stats
    registry
        .register(Box::new(INGEST_RECORDS.clone()))
        .expect("Metric registered");
    registry
        .register(Box::new(INGEST_BYTES.clone()))
        .expect("Metric registered");
    registry
        .register(Box::new(INGEST_WAL_USED_BYTES.clone()))
        .expect("Metric registered");
    registry
        .register(Box::new(INGEST_WAL_WRITE_BYTES.clone()))
        .expect("Metric registered");
    registry
        .register(Box::new(INGEST_WAL_READ_BYTES.clone()))
        .expect("Metric registered");

    // querier stats
    registry
        .register(Box::new(QUERY_CACHE_LIMIT_BYTES.clone()))
        .expect("Metric registered");
    registry
        .register(Box::new(QUERY_CACHE_USED_BYTES.clone()))
        .expect("Metric registered");
    registry
        .register(Box::new(QUERY_CACHE_FILES.clone()))
        .expect("Metric registered");
    registry
        .register(Box::new(QUERY_CACHE_RECORDS.clone()))
        .expect("Metric registered");

    // compactor stats
    registry
        .register(Box::new(COMPACT_USED_TIME.clone()))
        .expect("Metric registered");
    registry
        .register(Box::new(COMPACT_MERGED_FILES.clone()))
        .expect("Metric registered");
    registry
        .register(Box::new(COMPACT_MERGED_BYTES.clone()))
        .expect("Metric registered");
    registry
        .register(Box::new(COMPACT_DELAY_HOURS.clone()))
        .expect("Metric registered");

    // storage stats
    registry
        .register(Box::new(STORAGE_ORIGINAL_BYTES.clone()))
        .expect("Metric registered");
    registry
        .register(Box::new(STORAGE_COMPRESSED_BYTES.clone()))
        .expect("Metric registered");
    registry
        .register(Box::new(STORAGE_FILES.clone()))
        .expect("Metric registered");
    registry
        .register(Box::new(STORAGE_RECORDS.clone()))
        .expect("Metric registered");
    registry
        .register(Box::new(STORAGE_WRITE_BYTES.clone()))
        .expect("Metric registered");
    registry
        .register(Box::new(STORAGE_READ_BYTES.clone()))
        .expect("Metric registered");

    // metadata stats
    registry
        .register(Box::new(META_STORAGE_BYTES.clone()))
        .expect("Metric registered");
    registry
        .register(Box::new(META_STORAGE_KEYS.clone()))
        .expect("Metric registered");
    registry
        .register(Box::new(META_NUM_NODES.clone()))
        .expect("Metric registered");
    registry
        .register(Box::new(META_NUM_ORGANIZATIONS.clone()))
        .expect("Metric registered");
    registry
        .register(Box::new(META_NUM_STREAMS.clone()))
        .expect("Metric registered");
    registry
        .register(Box::new(META_NUM_USERS_TOTAL.clone()))
        .expect("Metric registered");
    registry
        .register(Box::new(META_NUM_USERS.clone()))
        .expect("Metric registered");
    registry
        .register(Box::new(META_NUM_FUNCTIONS.clone()))
        .expect("Metric registered");
    registry
        .register(Box::new(META_NUM_ALERTS.clone()))
        .expect("Metric registered");
    registry
        .register(Box::new(META_NUM_DASHBOARDS.clone()))
        .expect("Metric registered");
}

fn create_const_labels() -> HashMap<String, String> {
    let mut labels = HashMap::new();
    labels.insert("cluster".to_string(), CONFIG.common.cluster_name.clone());
    labels.insert("instance".to_string(), CONFIG.common.instance_name.clone());
    labels.insert("role".to_string(), CONFIG.common.node_role.clone());
    labels
}

pub fn create_prometheus_handler() -> PrometheusMetrics {
    let registry = prometheus::Registry::new();
    register_metrics(&registry);

    PrometheusMetricsBuilder::new(NAMESPACE)
        .endpoint(format!("{}/metrics", CONFIG.common.base_uri).as_str())
        .const_labels(create_const_labels())
        .registry(registry)
        .build()
        .expect("Failed to build prometheus")
}
