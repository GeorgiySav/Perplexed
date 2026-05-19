use crate::{chunk, context, extract, fetch, llm::{self, Message}, render, rerank::{self, rerank}, rerank_ce::rerank_ce, search};
use indicatif::{ProgressBar, ProgressStyle};

const SEARXNG_URL: &str = "http://localhost:8888";
const OLLAMA_URL: &str = "http://localhost:11434";
const MODEL: &str = "qwen2.5:7b";
const EMBED_MODEL: &str = "mxbai-embed-large";
const BI_ENCODER_POOL: usize = 20;
const CROSS_ENCODER_TOP_K: usize = 8;
const PER_QUERY_TOP_K: usize = 10;
const REQUEST_TIMEOUT_SECS: u64 = 10;
const USER_AGENT: &str = "Mozilla/5.0 (compatible; Perplexed/0.1)";

fn create_spinner(message: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::default_spinner().template("{spinner:.cyan} {msg}").unwrap());
    pb.set_message(message.to_string());
    pb.enable_steady_tick(std::time::Duration::from_millis(80));
    pb
}

const QUERY_REWRITE_SYSTEM: &str = "You rewrite the user's latest message into a standalone search query, using the conversation history to resolve pronouns and references. When the latest message is ambiguous, prefer the topic of the most recent assistant response. Return ONLY the rewritten query — no explanation, no quotes, no prefix.";

async fn rewrite_query(
    client: &reqwest::Client,
    history: &[Message],
    query: &str
) -> anyhow::Result<String> {
    if history.is_empty() {
        return Ok(query.to_string())
    }

    let mut messages = vec![
        Message{
            role: "system".to_string(),
            content: QUERY_REWRITE_SYSTEM.to_string()
        }
    ];
    messages.extend_from_slice(history);
    messages.push(Message { role: "user".to_string(), content: query.to_string() });

    Ok(
        llm::chat_once(client, OLLAMA_URL, MODEL, &messages).await?.trim().to_string()
    )
}

const QUERY_DECOMPOSE_SYSTEM: &str = r#"You decompose a user question into 1 to 4 standalone search queries that, taken together, cover the question. Output rules:
- Output ONLY the sub-queries, one per line.
- No numbering, no prefix, no explanation, no quotes.
- Use 1 sub-query for simple questions, 2 to 4 for multi-faceted questions.
- Each sub-query must be a complete standalone search query (no pronouns or references to past context).
"#;

async fn decompose_query(
    client: &reqwest::Client,
    query: &str
) -> anyhow::Result<Vec<String>> {
    let messages = vec![
        Message{
            role: "system".to_string(),
            content: QUERY_DECOMPOSE_SYSTEM.to_string()
        },
        Message{
            role: "user".to_string(),
            content: query.to_string()
        }
    ];

    let mut result = llm::chat_once(&client, OLLAMA_URL, MODEL, &messages).await?
    .lines()
    .map(|s| s.trim().to_string())
    .filter(|s| !s.is_empty())
    .take(4)
    .collect::<Vec<String>>();

    if result.is_empty() {
        result.push(query.to_string());
    }

    Ok(result)
}

pub async fn answer(
    query: &str,
    history: &[Message]
) -> anyhow::Result<String> {
    
    let client = reqwest::Client::builder()
    .timeout(std::time::Duration::from_secs(REQUEST_TIMEOUT_SECS))
    .user_agent(USER_AGENT)
    .build()?;

    let search_query = rewrite_query(&client, history, query).await?;
    
    let spinner = create_spinner("Decomposing query...");
    let sub_queries = decompose_query(&client, query).await?;
    spinner.finish_with_message(format!("Decomposed into {} sub-queries", sub_queries.len()));

    let spinner = create_spinner("Searching the web...");
    let search_results = search::search_many(&client, SEARXNG_URL, &sub_queries, PER_QUERY_TOP_K).await;
    spinner.finish_with_message(format!("Found {} results", search_results.len()));

    let spinner = create_spinner("Fetching pages...");
    let fetched_pages = fetch::fetch_all(&client, search_results).await;
    spinner.finish_with_message(format!("Fetched {} results", fetched_pages.len()));

    let spinner = create_spinner("Extracting from pages...");
    let extracted_pages = extract::extract_all(fetched_pages);
    spinner.finish_with_message(format!("Extracted {} pages", extracted_pages.len()));

    let spinner = create_spinner("Chunking sources...");
    let chunks = chunk::chunk(&extracted_pages);
    spinner.finish_with_message(format!("Built {} chunks", chunks.len()));

    let spinner = create_spinner("Bi-encoder ranking chunks...");
    let reranked_chunks = rerank::rerank(&client, OLLAMA_URL, EMBED_MODEL, &search_query, chunks, BI_ENCODER_POOL).await?;
    spinner.finish_with_message(format!("Bi-encoder kept {} chunks", reranked_chunks.len()));

    let spinner = create_spinner("Cross-encoder reranking...");
    let top_chunks = rerank_ce(&search_query, reranked_chunks, CROSS_ENCODER_TOP_K)?;
    spinner.finish_with_message(format!("Select top {} chunks", top_chunks.len()));

    let ctx = context::build_context(query, &extracted_pages, top_chunks);
    
    let mut messages = vec![
        Message{
            role: "system".to_string(),
            content: ctx.system_prompt.to_string()
        }
    ];
    messages.extend_from_slice(history);
    messages.push(Message { role: "user".to_string(), content: ctx.user_prompt });
        
    let mut guard = render::CitationGuard::new(ctx.citations.len());
    let mut response_buffer = String::new();

    println!();
    println!();
    llm::stream_chat(
        &client,
        OLLAMA_URL,
        MODEL,
        &messages,
        |tok| {
            response_buffer.push_str(tok);
            guard.feed(tok);
        }
    ).await?;
    guard.flush();
    println!();
    println!();
    render::print_sources(&ctx.citations);

    Ok(response_buffer)
}