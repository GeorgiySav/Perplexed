mod search;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = reqwest::Client::new();

    let searxng_base_url = "http://localhost:8888";
    let query = "What is Perplexity";
    let top_k = 5;

    let search_results = search::search(&client, searxng_base_url, query, top_k).await?;

    for r in &search_results {
        println!("[{}] {} - {} (score: {})", r.engine, r.title, r.url, r.score);
    }

    Ok(())
}