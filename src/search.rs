use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct SearchHit {
    pub url: String,
    pub title: String,
    pub engine: String,
    pub score: f64,
}

#[derive(Deserialize, Debug)]
struct SearchHits {
    results: Vec<SearchHit>,
}