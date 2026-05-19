mod cli;
mod context;
mod extract;
mod fetch;
mod llm;
mod render;
mod search;
mod pipeline;
mod chunk;
mod rerank;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    cli::run().await
}