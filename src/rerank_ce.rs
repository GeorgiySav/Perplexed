use std::sync::{LazyLock, Mutex};

use fastembed::{RerankInitOptions, TextRerank, RerankerModel};

use crate::chunk::Chunk;


static RERANKER: LazyLock<Mutex<TextRerank>> = LazyLock::new(|| {
    Mutex::new(
        TextRerank::try_new(
            RerankInitOptions::new(RerankerModel::BGERerankerBase)
                .with_show_download_progress(true)
        ).expect("failed to load reranker model")
    )
});

pub fn rerank_ce(
    query: &str,
    chunks: Vec<Chunk>,
    top_k: usize
) -> anyhow::Result<Vec<Chunk>> {
    if chunks.is_empty() {
        return Ok(Vec::<Chunk>::new())
    }

    let docs: Vec<&str> = chunks.iter()
        .map(|c| c.text.as_str())
        .collect();

    let mut guard = RERANKER.lock().unwrap();
    let results = guard.rerank(query, docs, false, None)?;

    let top = results.into_iter()
        .take(top_k)
        .map(|r| chunks[r.index].clone())
        .collect::<Vec<Chunk>>();

    Ok(top)
}