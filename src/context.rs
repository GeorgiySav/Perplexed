use crate::{chunk::Chunk, extract::ExtractedPage};

pub const SYSTEM_PROMPT: &str = r#"You are a research assistant. Answer the user's question using ONLY the information provided in <source id="N">...</source> blocks in the user message. After each factual claim, cite the source(s) inline using bracketed numbers like [1] or [1][3], where the number matches the source's `id` attribute.

Rules:
- Only cite sources whose `id` appears in the user message. Never invent citation numbers.
- Do not introduce facts that are not present in the provided sources, even when you know them from training.
- Inline bracketed numbers that appear inside source body text (e.g. Wikipedia's own footnote references) are part of those sources and are NOT valid citations for you. Use only the source `id`s shown in the source tags.
- If the provided sources do not contain enough information to answer the question, say so plainly.
"#;


#[derive(Debug, Clone)]
pub struct Citation {
    pub n: usize,
    pub url: String,
    pub title: String,
}

#[derive(Debug)]
pub struct Context {
    pub system_prompt: &'static str,
    pub user_prompt: String,
    pub citations: Vec<Citation>,
}

pub fn build_context(
    query: &str,
    docs: &[ExtractedPage],
    chunks: Vec<Chunk>
) -> Context {
    let mut citations: Vec<Citation> = Vec::new();
    let mut source_blocks: Vec<String> = Vec::new();

    let mut groups: Vec<(usize, Vec<String>)> = Vec::new();
    for chunk in chunks {
        if let Some(g) = groups.iter_mut().find(|(idx, _)| *idx == chunk.source_idx) {
            g.1.push(chunk.text);
        }
        else {
            groups.push((chunk.source_idx, vec![chunk.text]));
        }
    }

    for (citation_idx, (source_idx, chunk_texts)) in groups.into_iter().enumerate() {
        let n = citation_idx + 1;
        let doc = &docs[source_idx];

        let combined = chunk_texts.join("\n\n");

        source_blocks.push(format!(
            "<source id=\"{}\">\nTitle: {}\nURL: {}\n\n{}</source>",
            n, doc.title, doc.url, combined 
        ));
        citations.push(Citation { n: n, url: doc.url.clone(), title: doc.title.clone() });
    }
    let sources = source_blocks.join("\n\n");

    Context{
        system_prompt: SYSTEM_PROMPT,
        user_prompt: format!("Sources are provided below as <source id=\"N\">...</source> blocks.\n\n{}\n\nQuestion: {}", sources, query),
        citations
    }
}