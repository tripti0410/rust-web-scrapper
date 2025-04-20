use reqwest::{Client, ClientBuilder};
use scraper::{Html, Selector};
use std::time::Duration;
use once_cell::sync::Lazy;
use crate::error::Result;

// Create a static client to reuse connections
static CLIENT: Lazy<Client> = Lazy::new(|| {
    ClientBuilder::new()
        .timeout(Duration::from_secs(10))
        .connect_timeout(Duration::from_secs(5))
        .pool_max_idle_per_host(10)
        .build()
        .expect("Failed to build HTTP client")
});

// Create static selectors to avoid recompiling them each time
static BODY_SELECTOR: Lazy<Selector> = Lazy::new(|| {
    Selector::parse("body").expect("Failed to parse body selector")
});

pub async fn fetch_html(url: &str) -> Result<String> {
    let response = CLIENT.get(url).send().await?;
    let html = response.text().await?;
    Ok(html)
}

pub fn extract_body(html: &str) -> Option<String> {
    let document = Html::parse_document(html);
    
    document.select(&BODY_SELECTOR)
        .next()
        .map(|element| element.inner_html())
}

pub fn format_html(html: &str) -> String {
    // More efficient whitespace handling
    let mut result = String::with_capacity(html.len());
    let mut last_was_whitespace = true;

    for line in html.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            if !last_was_whitespace {
                result.push('\n');
            }
            result.push_str(trimmed);
            last_was_whitespace = false;
        }
    }

    result
}

pub fn build_prompt(content: &str) -> String {
    // Use a more efficient string format that pre-allocates approximately the right amount of space
    let mut result = String::with_capacity(content.len() + 150);
    result.push_str("The following is the content of a webpage. Please provide a concise summary formatted in Markdown. Use headers, bullet points, and other Markdown formatting to make the summary structured and readable:\n\n");
    result.push_str(content);
    result
} 