use crate::extract::ExtractedPage;


pub const SYSTEM_PROMPT: &str = "You are a research assistant. Answer the user's question using ONLY the
numbered sources provided in the user message. After each factual claim,
cite the source(s) inline using bracketed numbers like [1] or [1][3]. If
the sources do not contain enough information to answer the question, say
so plainly. Do not invent citations or facts.";

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
    docs: Vec<ExtractedPage>
) -> Context {
    let mut citations: Vec<Citation> = Vec::new();
    let mut source_blocks: Vec<String> = Vec::new();

    for (i, doc) in docs.into_iter().enumerate() {
        source_blocks.push(format!(
            "[{}] {}\n{}\n\n{}", i+1, doc.title, doc.url, doc.text
        ));
        citations.push(Citation { n: i+1, url: doc.url, title: doc.title });
    }
    let sources = source_blocks.join("\n\n");

    Context{
        system_prompt: SYSTEM_PROMPT,
        user_prompt: format!("Sources:\n\n{}\n\nQuestion: {}", sources, query),
        citations
    }
}