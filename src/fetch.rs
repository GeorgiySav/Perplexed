use anyhow::bail;
use futures::future::join_all;
use crate::search::SearchHit;

#[derive(Debug, Clone)]
pub struct FetchedPage {
    pub url: String,
    pub title: String,
    pub html: String,
}

pub async fn fetch_page(
    client: &reqwest::Client,
    search_hit: SearchHit
) -> anyhow::Result<FetchedPage> {

    let response = client.get(&search_hit.url).send().await?.error_for_status()?;
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok()).unwrap_or("");

    if !content_type.starts_with("text/html") {
        bail!("non-html content-type: {}", content_type);
    }

    let body = response.text().await?;

    let fetched_page = FetchedPage{
        url: search_hit.url,
        title: search_hit.title,
        html: body
    };

    Ok(fetched_page)
}

pub async fn fetch_all(
    client: &reqwest::Client,
    search_hits: Vec<SearchHit>
) -> Vec<FetchedPage> {
    join_all(
        search_hits
            .into_iter()
            .map(|hit| fetch_page(client, hit)
        )).await
            .into_iter()
            .filter_map(|h| h.ok())
            .collect()
}