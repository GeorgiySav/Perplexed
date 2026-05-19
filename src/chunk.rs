use crate::extract::ExtractedPage;

const TARGET_CHUNK_CHARS: usize = 2000;

#[derive(Debug, Clone)]
pub struct Chunk {
    pub source_idx: usize,
    pub text: String
}

pub fn chunk(docs: &[ExtractedPage]) -> Vec<Chunk> {
    let mut chunks: Vec<Chunk> = Vec::new();

    for (idx, doc) in docs.iter().enumerate() {
        let mut current = String::new();

        for p in doc.text.split("\n\n")
                            .map(|p| p.trim())
                            .filter(|p| !p.is_empty()) {
            if (p.len() + current.len() + (if !current.is_empty() {2} else {0})) > TARGET_CHUNK_CHARS && !current.is_empty() {
                chunks.push(Chunk { source_idx: idx, text: current });
                current = String::new();
            }
            if !current.is_empty() {
                current += "\n\n";
            }
            current += p;
        }

        if !current.is_empty() {
            chunks.push(Chunk { source_idx: idx, text: current });
        } 
    }

    chunks
}
