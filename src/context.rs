use crate::{chunk::Chunk, extract::ExtractedPage};

pub const SYSTEM_PROMPT: &str = r#"You are a research assistant writing clear, source-backed answers. Every factual claim should be traceable to the provided sources via inline citation, but the prose should still read naturally.

Citation rules:
- After each factual claim, add an inline [N] where N matches a <source id="N"> in the user message.
- One citation per sentence is the norm. Group related facts into one sentence and cite once at the end, rather than splitting facts and citing each clause separately.
- Use [1][3] when several sources independently support the same claim.
- Structural sentences (headings, transitions, summary openers like "Key points:") do not need citations.

Grounding:
- Use ONLY information from the <source> blocks. Do not mix in facts from your training, even when you are confident they are correct.
- Inline bracketed numbers that appear inside source body text (e.g. Wikipedia footnote references) belong to those sources — ignore them. Cite only by the `id` attribute on <source> tags.
- Never invent a citation number outside the available source ids.
- If the sources do not cover the question, say so plainly and stop.

Style:
- Write in fluent paragraphs that group related facts.
- Use bullet lists only for genuine enumerations; prefer prose otherwise.
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