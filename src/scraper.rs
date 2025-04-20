use reqwest::Client;
use scraper::{Html, Selector};
use crate::error::Result;

pub async fn fetch_html(url: &str) -> Result<String> {
    let client = Client::new();
    let response = client.get(url).send().await?;
    let html = response.text().await?;
    Ok(html)
}

pub fn extract_body(html: &str) -> Option<String> {
    let document = Html::parse_document(html);
    let body_selector = Selector::parse("body").ok()?;
    
    document.select(&body_selector)
        .next()
        .map(|element| element.inner_html())
}

pub fn format_html(html: &str) -> String {
    // Simple formatting: remove extra whitespace and limit length
    html.lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn build_prompt(content: &str) -> String {
    format!(
        "The following is the content of a webpage. Please provide a summary formatted in Markdown. Use headers, bullet points, and other Markdown formatting to make the summary structured and readable:\n\n{}", 
        content
    )
} 