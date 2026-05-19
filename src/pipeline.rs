use crate::{chunk, context::{self, Citation}, extract, fetch, llm::{self, Message}, rerank, rerank_ce::rerank_ce, search};
use indicatif::{ProgressBar, ProgressStyle};
use tokio::sync::mpsc::UnboundedSender;

const SEARXNG_URL: &str = "http://localhost:8888";
const OLLAMA_URL: &str = "http://localhost:11434";
const MODEL: &str = "qwen2.5:7b";
const EMBED_MODEL: &str = "mxbai-embed-large";
const BI_ENCODER_POOL: usize = 20;
const CROSS_ENCODER_TOP_K: usize = 8;
const PER_QUERY_TOP_K: usize = 10;
const REQUEST_TIMEOUT_SECS: u64 = 10;
const USER_AGENT: &str = "Mozilla/5.0 (compatible; Perplexed/0.1)";

pub enum PipelineEvent {
    Sources(Vec<Citation>),
    Token(String),
    Done,
    Error(String)
}

fn create_spinner(message: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::default_spinner().template("{spinner:.cyan} {msg}").unwrap());
    pb.set_message(message.to_string());
    pb.enable_steady_tick(std::time::Duration::from_millis(80));
    pb
}

const QUERY_REWRITE_SYSTEM: &str = r#"You rewrite the user's latest message into a standalone search query that a search engine could answer without any prior context.

Rules:
- The rewrite MUST be self-contained. Anyone reading just the rewrite — without the conversation history — should understand the subject.
- Identify the main subject of the conversation (usually from the first user message) and include it explicitly in the rewrite, even when the user's latest message omits it.
- Resolve all pronouns ("it", "they", "them") AND definite references ("the plan", "the company", "this feature") to their concrete subjects.
- If the latest message is already a complete standalone query, return it unchanged.
- Return ONLY the rewritten query — no explanation, no quotes, no prefix.
"#;


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


async fn run_pipeline(
    query: &str,
    history: &[Message],
    tx: &UnboundedSender<PipelineEvent>
) -> anyhow::Result<()> { 
    let client = reqwest::Client::builder()
    .timeout(std::time::Duration::from_secs(REQUEST_TIMEOUT_SECS))
    .user_agent(USER_AGENT)
    .build()?;

    let search_query = rewrite_query(&client, history, query).await?;
    
    let sub_queries = decompose_query(&client, &search_query).await?;

    let search_results = search::search_many(&client, SEARXNG_URL, &sub_queries, PER_QUERY_TOP_K).await;

    let fetched_pages = fetch::fetch_all(&client, search_results).await;

    let extracted_pages = extract::extract_all(fetched_pages);

    let chunks = chunk::chunk(&extracted_pages);

    let reranked_chunks = rerank::rerank(&client, OLLAMA_URL, EMBED_MODEL, &search_query, chunks, BI_ENCODER_POOL).await?;

    let top_chunks = rerank_ce(&search_query, reranked_chunks, CROSS_ENCODER_TOP_K)?;

    let ctx = context::build_context(query, &extracted_pages, top_chunks);
    let _ = tx.send(PipelineEvent::Sources(ctx.citations.clone()));
    
    let mut messages = vec![
        Message{
            role: "system".to_string(),
            content: ctx.system_prompt.to_string()
        }
    ];
    messages.extend_from_slice(history);
    messages.push(Message { role: "user".to_string(), content: ctx.user_prompt });
        
    llm::stream_chat(
        &client,
        OLLAMA_URL,
        MODEL,
        &messages,
        |tok| {
            let _ = tx.send(PipelineEvent::Token(tok.to_string()));
        }
    ).await?;

    Ok(())
}

pub async fn answer_streaming(
    query: String,
    history: Vec<Message>,
    tx: UnboundedSender<PipelineEvent>
) {
    match run_pipeline(&query, &history, &tx).await {
        Ok(()) => {
            let _ = tx.send(PipelineEvent::Done);
        }
        Err(e) => {
            let _ = tx.send(PipelineEvent::Error(format!("{:#}", e)));
        }
    }
}