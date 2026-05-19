use crate::{search, fetch, extract, context, llm, render};
use indicatif::{ProgressBar, ProgressStyle};

const SEARXNG_URL: &str = "http://localhost:8888";
const OLLAMA_URL: &str = "http://localhost:11434";
const MODEL: &str = "qwen2.5:7b";
const TOP_K: usize = 5;
const REQUEST_TIMEOUT_SECS: u64 = 10;
const USER_AGENT: &str = "Mozilla/5.0 (compatible; Perplexed/0.1)";

fn create_spinner(message: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::default_spinner().template("{spinner:.cyan} {msg}").unwrap());
    pb.set_message(message.to_string());
    pb.enable_steady_tick(std::time::Duration::from_millis(80));
    pb
}

pub async fn answer(query: &str) -> anyhow::Result<()> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .user_agent(USER_AGENT)
        .build()?;

    let spinner = create_spinner("Searching the web...");
    let search_results = search::search(&client, SEARXNG_URL, query, TOP_K).await?;
    spinner.finish_with_message(format!("Found {} results", search_results.len()));

    let spinner = create_spinner("Fetching pages...");
    let fetched_pages = fetch::fetch_all(&client, search_results).await;
    spinner.finish_with_message(format!("Fetched {} results", fetched_pages.len()));

    let spinner = create_spinner("Extracting from pages...");
    let extracted_pages = extract::extract_all(fetched_pages);
    spinner.finish_with_message(format!("Extracted {} pages", extracted_pages.len()));

    let ctx = context::build_context(query, extracted_pages);

    let mut guard = render::CitationGuard::new(ctx.citations.len());

    println!();
    println!();
    llm::stream_chat(
        &client,
        OLLAMA_URL,
        MODEL,
        ctx.system_prompt,
        &ctx.user_prompt,
        |tok| guard.feed(tok)
    ).await?;
    guard.flush();
    println!();
    println!();
    render::print_sources(&ctx.citations);

    Ok(())
}