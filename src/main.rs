mod cli;
mod http;
mod context;
mod extract;
mod fetch;
mod llm;
mod render;
mod search;
mod pipeline;
mod chunk;
mod rerank;
mod rerank_ce;
mod markdown;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    cli::run().await
}