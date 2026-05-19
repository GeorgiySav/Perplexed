use anyhow::bail;
use dom_smoothie::{Config, Readability};

use crate::fetch::FetchedPage;

const MIN_CHARS_PER_PAGE: usize = 200;

#[derive(Debug, Clone)]
pub struct ExtractedPage {
    pub url: String,
    pub title: String,
    pub text: String,
}

pub fn extract_page(
    page: FetchedPage
) -> anyhow::Result<ExtractedPage> {

    let mut readability = Readability::new(
        page.html,
        Some(&page.url),
        Some(Config::default()))?;

    let article = readability.parse()?;

    if article.text_content.chars().filter(|c| !c.is_whitespace()).count() < MIN_CHARS_PER_PAGE {
        bail!("Extraction only provided {} chars", article.length);
    }

    let extracted_page = ExtractedPage{
        url: page.url,
        title: page.title,
        text: article.text_content.to_string()
    };

    Ok(extracted_page)
}

pub fn extract_all(
    pages: Vec<FetchedPage>
) -> Vec<ExtractedPage> {
    pages
        .into_iter()
        .map(extract_page)
        .filter_map(Result::ok)
        .collect()
}