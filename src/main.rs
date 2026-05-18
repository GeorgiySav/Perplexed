mod search;
mod fetch;
mod extract;
mod context;

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
    println!();

    let fetched_pages = fetch::fetch_all(&client, search_results).await;
    for p in &fetched_pages {
        println!("[{} chars] {} - {}", p.html.len(), p.title, p.url);
    }
    println!();

    let extracted_pages = extract::extract_all(fetched_pages);
    for (i, e) in extracted_pages.iter().enumerate() {
        println!("-- {}. {} --", i+1, e.title);
        println!("{}", e.url);
        println!("[{} chars] {}", e.text.len(), e.text.chars().take(300).collect::<String>());
        println!();
    }
    println!();

    let ctx = context::build_context(query, extracted_pages);

    println!("System Prompt");
    println!("{}", ctx.system_prompt);
    println!();

    println!("User Prompt");
    println!("{}", ctx.user_prompt.chars().take(800).collect::<String>());
    println!();
    println!("{}", ctx.user_prompt.chars().rev().take(300).collect::<Vec<_>>().into_iter().rev().collect::<String>());
    println!("Number of chars: {}", ctx.user_prompt.len());
    println!();

    println!("Citations");
    for c in &ctx.citations {
        println!("[{}] {} - {}", c.n, c.title, c.url);
    }
    println!(); 

    Ok(())
}