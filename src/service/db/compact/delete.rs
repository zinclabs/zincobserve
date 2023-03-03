use dashmap::DashSet;
use std::sync::Arc;

use crate::{infra::db::Event, meta::StreamType};

lazy_static! {
    static ref CACHE: DashSet<String> = DashSet::new();
}

// delete data from stream
// if date_range is empty, delete all data
// date_range is a tuple of (start, end), eg: (20230102, 20230103)
#[inline]
pub async fn delete_stream(
    org_id: &str,
    stream_name: &str,
    stream_type: StreamType,
    date_range: Option<(&str, &str)>,
) -> Result<(), anyhow::Error> {
    let db = &crate::infra::db::DEFAULT;
    let key = match date_range {
        Some((start, end)) => format!(
            "{}/{}/{}/{},{}",
            org_id, stream_type, stream_name, start, end
        ),
        None => format!("{}/{}/{}/all", org_id, stream_type, stream_name),
    };

    // write in cache
    if CACHE.contains(&key) {
        return Ok(()); // already in cache, just skip
    }
    CACHE.insert(key.to_string());

    let db_key = format!("/compact/delete/{}", key);
    db.put(&db_key, "OK".into()).await?;

    Ok(())
}

// check if stream is deleting from cache
#[inline]
pub fn is_deleting_stream(
    org_id: &str,
    stream_name: &str,
    stream_type: StreamType,
    date_range: Option<(&str, &str)>,
) -> bool {
    let key = match date_range {
        Some((start, end)) => format!(
            "{}/{}/{}/{},{}",
            org_id, stream_type, stream_name, start, end
        ),
        None => format!("{}/{}/{}/all", org_id, stream_type, stream_name),
    };
    CACHE.contains(&key)
}

#[inline]
pub async fn delete_stream_done(
    org_id: &str,
    stream_name: &str,
    stream_type: StreamType,
    date_range: Option<(&str, &str)>,
) -> Result<(), anyhow::Error> {
    let db = &crate::infra::db::DEFAULT;
    let key = match date_range {
        Some((start, end)) => format!(
            "{}/{}/{}/{},{}",
            org_id, stream_type, stream_name, start, end
        ),
        None => format!("{}/{}/{}/all", org_id, stream_type, stream_name),
    };
    let db_key = format!("/compact/delete/{}", key);
    db.delete(&db_key, false).await?;

    // remove in cache
    CACHE.remove(&key);

    Ok(())
}

pub async fn list() -> Result<Vec<String>, anyhow::Error> {
    let mut items = Vec::new();
    let db = &crate::infra::db::DEFAULT;
    let key = "/compact/delete/";
    let ret = db.list(key).await?;
    for (item_key, _) in ret {
        let item_key = item_key.strip_prefix(key).unwrap();
        items.push(item_key.to_string());
    }
    Ok(items)
}

pub async fn watch() -> Result<(), anyhow::Error> {
    let db = &crate::infra::db::DEFAULT;
    let key = "/compact/delete/";
    let mut events = db.watch(key).await?;
    let events = Arc::get_mut(&mut events).unwrap();
    log::info!("[TRACE] Start watching stream deleting");
    loop {
        let ev = match events.recv().await {
            Some(ev) => ev,
            None => {
                log::error!("watch_stream_deleting: event channel closed");
                break;
            }
        };
        match ev {
            Event::Put(ev) => {
                let item_key = ev.key.strip_prefix(key).unwrap();
                CACHE.insert(item_key.to_string());
            }
            Event::Delete(ev) => {
                let item_key = ev.key.strip_prefix(key).unwrap();
                CACHE.remove(item_key);
            }
        }
    }
    Ok(())
}

pub async fn cache() -> Result<(), anyhow::Error> {
    let db = &crate::infra::db::DEFAULT;
    let key = "/compact/delete/";
    let ret = db.list(key).await?;
    for (item_key, _) in ret {
        let item_key = item_key.strip_prefix(key).unwrap();
        CACHE.insert(item_key.to_string());
    }
    Ok(())
}
