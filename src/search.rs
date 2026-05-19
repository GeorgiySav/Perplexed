use std::{collections::HashSet, hash::Hash};

use futures::future::join_all;
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct SearchHit {
    pub url: String,
    pub title: String,
    pub engine: String,
    pub score: f64,
}

#[derive(Deserialize, Debug)]
struct SearchHits {
    results: Vec<SearchHit>,
}


pub async fn search(
    client: &reqwest::Client,
    base_url: &str,
    user_query: &str,
    result_cap: usize
) -> anyhow::Result<Vec<SearchHit>> {

    let url_endpoint = format!("{}/search", base_url);
    let search_hits = client
        .get(url_endpoint)
        .query(&[("q", user_query), ("format", "json")])
        .send().await?
        .error_for_status()?
        .json::<SearchHits>().await?;

    let mut results = search_hits.results;
    results.truncate(result_cap);
    
    Ok(results)
}

pub async fn search_many(
    client: &reqwest::Client,
    base_url: &str,
    queries: &[String],
    per_query_top_k: usize
) -> Vec<SearchHit> {
    let mut seen_urls: HashSet<String> = HashSet::new();
    
    let results = join_all(
        queries.iter().map(|q| search(client, base_url, q, per_query_top_k))
    ).await
    .into_iter()
    .filter_map(|r| r.ok())
    .flatten()
    .filter(|h| seen_urls.insert(h.url.clone()))
    .collect::<Vec<SearchHit>>(); 

    results
}