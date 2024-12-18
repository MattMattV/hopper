use anyhow::{anyhow, Result};
use moka::{future::Cache, Expiry};
use std::{
    hash::Hasher,
    time::{Duration, Instant},
};

use crate::{
    model::AtUri,
    webhostmeta::{query, WebHostMeta},
};

struct ResolveWebHostMetaExpiry;

struct ResolveAtUriExpiry;

impl Expiry<String, ResolveWebHostMetaResult> for ResolveWebHostMetaExpiry {
    fn expire_after_create(
        &self,
        _key: &String,
        value: &ResolveWebHostMetaResult,
        _current_time: Instant,
    ) -> Option<Duration> {
        match value {
            ResolveWebHostMetaResult::Found(_) => None,
            ResolveWebHostMetaResult::NotFound(_) => Some(Duration::from_secs(60 * 10)),
        }
    }
}

impl Expiry<String, ResolveAtUriResult> for ResolveAtUriExpiry {
    fn expire_after_create(
        &self,
        _key: &String,
        value: &ResolveAtUriResult,
        _current_time: Instant,
    ) -> Option<Duration> {
        match value {
            ResolveAtUriResult::Found(_) => Some(Duration::from_secs(60 * 30)),
            ResolveAtUriResult::NotFound(_) => Some(Duration::from_secs(60 * 10)),
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub enum ResolveWebHostMetaResult {
    Found(WebHostMeta),
    NotFound(String),
}

#[derive(Clone, PartialEq, Eq)]
pub enum ResolveAtUriResult {
    Found(String),
    NotFound(String),
}

pub fn new_resolve_webhostmeta_cache() -> Cache<String, ResolveWebHostMetaResult> {
    let expiry = ResolveWebHostMetaExpiry;
    Cache::builder()
        .max_capacity(1024 * 20)
        .expire_after(expiry)
        .build()
}

pub fn new_resolve_aturi_cache() -> Cache<String, ResolveAtUriResult> {
    let expiry = ResolveAtUriExpiry;
    Cache::builder()
        .max_capacity(1024 * 20)
        .expire_after(expiry)
        .build()
}

pub(crate) async fn webhostmeta_cached(
    cache: &Cache<String, ResolveWebHostMetaResult>,
    http_client: &reqwest::Client,
    hostname: &str,
) -> Result<WebHostMeta> {
    if let Some(resolve_handle_result) = cache.get(hostname).await {
        return match resolve_handle_result {
            ResolveWebHostMetaResult::Found(webhostmeta) => Ok(webhostmeta),
            ResolveWebHostMetaResult::NotFound(err) => Err(anyhow!(err)),
        };
    }
    let webfinger = query(http_client, hostname).await;

    let cache_value = match webfinger.as_ref() {
        Ok(webfinger) => ResolveWebHostMetaResult::Found(webfinger.clone()),
        Err(err) => ResolveWebHostMetaResult::NotFound(err.to_string()),
    };

    cache.insert(hostname.to_string(), cache_value).await;
    webfinger
}

pub(crate) async fn aturi_cached(
    http_client: &reqwest::Client,
    webfinger_cache: &Cache<String, ResolveWebHostMetaResult>,
    aturi_cache: &Cache<String, ResolveAtUriResult>,
    servers: &Vec<String>,
    aturi_input: &str,
    aturi: &AtUri,
) -> Result<String> {
    let mut hasher = cityhasher::CityHasher::new();
    hasher.write(aturi_input.as_bytes());
    for server in servers {
        hasher.write(server.as_bytes());
    }
    let cache_key = hasher.finish().to_string();

    if let Some(resolve_handle_result) = aturi_cache.get(&cache_key).await {
        return match resolve_handle_result {
            ResolveAtUriResult::Found(destination) => Ok(destination),
            ResolveAtUriResult::NotFound(err) => Err(anyhow!(err)),
        };
    }

    for server in servers {
        let webfinger = webhostmeta_cached(webfinger_cache, http_client, server).await;

        if let Err(err) = webfinger {
            tracing::debug!(error = ?err, "error encountered");
            continue;
        }

        let webfinger = webfinger.unwrap();

        let destination = webfinger.match_uri(server, aturi);
        if destination.is_none() {
            tracing::debug!("no destination found");
            continue;
        }

        let destination = destination.unwrap();

        aturi_cache
            .insert(cache_key, ResolveAtUriResult::Found(destination.clone()))
            .await;
        return Ok(destination);
    }

    let err = anyhow!("error-web-unsupported-aturi Unsupported AT-URI");
    aturi_cache
        .insert(cache_key, ResolveAtUriResult::NotFound(err.to_string()))
        .await;

    Err(err)
}
