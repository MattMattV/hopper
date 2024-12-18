use anyhow::{anyhow, Result};
use moka::{future::Cache, Expiry};
use std::{
    hash::Hasher,
    time::{Duration, Instant},
};

use crate::{
    model::AtUri,
    webfinger::{query, Webfinger},
};

struct ResolveWebfingerExpiry;

struct ResolveAtUriExpiry;

impl Expiry<String, ResolveWebfingerResult> for ResolveWebfingerExpiry {
    fn expire_after_create(
        &self,
        _key: &String,
        value: &ResolveWebfingerResult,
        _current_time: Instant,
    ) -> Option<Duration> {
        match value {
            ResolveWebfingerResult::Found(_) => None,
            // ResolveWebfingerResult::Found(_) => Some(Duration::from_secs(60 * 5)),
            ResolveWebfingerResult::NotFound(_) => Some(Duration::from_secs(60 * 10)),
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
pub enum ResolveWebfingerResult {
    Found(Webfinger),
    NotFound(String),
}

#[derive(Clone, PartialEq, Eq)]
pub enum ResolveAtUriResult {
    Found(String),
    NotFound(String),
}

pub fn new_resolve_webfinger_cache() -> Cache<String, ResolveWebfingerResult> {
    let expiry = ResolveWebfingerExpiry;
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

pub(crate) async fn webfinger_cached(
    cache: &Cache<String, ResolveWebfingerResult>,
    http_client: &reqwest::Client,
    hostname: &str,
) -> Result<Webfinger> {
    if let Some(resolve_handle_result) = cache.get(hostname).await {
        return match resolve_handle_result {
            ResolveWebfingerResult::Found(webfinger) => Ok(webfinger),
            ResolveWebfingerResult::NotFound(err) => Err(anyhow!(err)),
        };
    }
    let webfinger = query(http_client, hostname).await;

    let cache_value = match webfinger.as_ref() {
        Ok(webfinger) => ResolveWebfingerResult::Found(webfinger.clone()),
        Err(err) => ResolveWebfingerResult::NotFound(err.to_string()),
    };

    cache.insert(hostname.to_string(), cache_value).await;
    webfinger
}

pub(crate) async fn aturi_cached(
    http_client: &reqwest::Client,
    webfinger_cache: &Cache<String, ResolveWebfingerResult>,
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
        let webfinger = webfinger_cached(webfinger_cache, http_client, server).await;

        if let Err(err) = webfinger {
            tracing::debug!(error = ?err, "error encountered");
            continue;
        }

        let webfinger = webfinger.unwrap();

        let destination = webfinger.match_uri(server, aturi);
        if destination.is_none() {
            continue;
        }

        let destination = destination.unwrap();

        aturi_cache
            .insert(cache_key, ResolveAtUriResult::Found(destination.clone()))
            .await;
        return Ok(destination);
    }

    let err = anyhow!("unable to resolve at-uri");
    aturi_cache
        .insert(cache_key, ResolveAtUriResult::NotFound(err.to_string()))
        .await;

    Err(err)
}
