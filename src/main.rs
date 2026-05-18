mod search;
mod fetch;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent("Mozilla/5.0 (compatible; Perplexed/0.1)")
        .build()?;

    let searxng_base_url = "http://localhost:8888";
    let query = "What is Perplexity";
    let top_k = 5;

    let search_results = search::search(&client, searxng_base_url, query, top_k).await?;

    for r in &search_results {
        println!("[{}] {} - {} (score: {})", r.engine, r.title, r.url, r.score);
    }

    let fetched_pages = fetch::fetch_all(&client, search_results).await;

    for p in &fetched_pages {
        println!("[{} chars] {} - {}", p.html.len(), p.title, p.url);
    }

    Ok(())
}