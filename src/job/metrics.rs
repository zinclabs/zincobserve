use std::collections::HashMap;
use std::{fs, path::Path};
use tokio::time;

use crate::common::file::scan_files;
use crate::infra::config::CONFIG;
use crate::infra::{cluster, metrics};

pub async fn run() -> Result<(), anyhow::Error> {
    if !cluster::is_ingester(&cluster::LOCAL_NODE_ROLE) {
        return Ok(()); // not an ingester, no need to init job
    }

    // create wal dir
    fs::create_dir_all(&CONFIG.common.data_wal_dir)?;
    // load metrics
    load_ingest_wal_used_bytes().await?;

    let mut interval = time::interval(time::Duration::from_secs(5));
    interval.tick().await; // trigger the first run
    loop {
        interval.tick().await;
        // if let Err(e) = update_ingest_wal_used_bytes().await {
        //     log::error!("Error update metrics: {}", e);
        // }
    }
}

async fn load_ingest_wal_used_bytes() -> Result<(), anyhow::Error> {
    let data_dir = Path::new(&CONFIG.common.data_wal_dir)
        .canonicalize()
        .unwrap();
    let pattern = format!("{}/files/*/*/*/*.json", &CONFIG.common.data_wal_dir);
    let files = scan_files(&pattern);
    let mut sizes = HashMap::new();
    for file in files {
        let local_file = file.to_owned();
        let local_path = Path::new(&file).canonicalize().unwrap();
        let file_path = local_path
            .strip_prefix(&data_dir)
            .unwrap()
            .to_str()
            .unwrap()
            .replace('\\', "/");
        let columns = file_path.split('/').collect::<Vec<&str>>();
        let _ = columns[0].to_string();
        let org_id = columns[1].to_string();
        let stream_type = columns[2].to_string();
        let stream_name = columns[3].to_string();
        let entry = sizes.entry((org_id, stream_name, stream_type)).or_insert(0);
        *entry += match std::fs::metadata(local_file) {
            Ok(metadata) => metadata.len(),
            Err(_) => 0,
        };
    }
    for ((org_id, stream_name, stream_type), size) in sizes {
        metrics::INGEST_WAL_USED_BYTES
            .with_label_values(&[&org_id, &stream_name, &stream_type])
            .set(size as i64);
    }
    Ok(())
}
