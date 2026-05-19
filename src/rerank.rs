use std::iter::zip;

use serde::{Serialize, Deserialize};
use crate::chunk::Chunk;

const MAX_EMBED_CHARS: usize = 1200;

#[derive(Serialize)]
struct EmbedRequest {
    model: String,
    input: Vec<String>
}

#[derive(Deserialize, Debug)]
struct EmbedResponse {
    embeddings: Vec<Vec<f32>>
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot = zip(a.iter(), b.iter()).map(|(x, y)| x*y).sum::<f32>();
    let norm_a = a.iter().map(|a| a*a).sum::<f32>().sqrt();
    let norm_b = b.iter().map(|b| b*b).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a*norm_b)
}

fn truncate_text(s: &str) -> String {
    s.chars().take(MAX_EMBED_CHARS).collect()
}

pub async fn rerank(
    client: &reqwest::Client,
    ollama_url: &str,
    embed_model: &str,
    query: &str,
    chunks: Vec<Chunk>,
    top_k: usize
) -> anyhow::Result<Vec<Chunk>> {

    if chunks.is_empty() {
        return Ok(Vec::<Chunk>::new());
    }

    let mut input: Vec<String> = Vec::new();
    input.push(truncate_text(query));
    input.extend(
        chunks.iter().map(|c| truncate_text(&c.text))
    );

    let request_body = EmbedRequest{
        model: embed_model.to_string(),
        input: input
    };

    let response = client
        .post(format!("{}/api/embed", ollama_url.trim_end_matches("/")))
        .json(&request_body)
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("Embed request failed ({}): {}", status, body);
    }

    let mut response = response.json::<EmbedResponse>().await?;

    let q_embed = response.embeddings.remove(0);
    let mut chunk_scores = zip(
            chunks.into_iter(), 
            response.embeddings.into_iter().map(|e| cosine_similarity(&q_embed, &e))
        )
        .collect::<Vec<(Chunk, f32)>>();

    chunk_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    Ok(chunk_scores.into_iter().take(top_k).map(|(c, _)| c).collect())
}