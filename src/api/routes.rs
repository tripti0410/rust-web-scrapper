use axum::{
    routing::post,
    Router,
    extract::{Json, State},
    response::IntoResponse,
};
use tower_http::cors::{CorsLayer, Any};
use chrono::Utc;

use crate::error::{Result, AppError};
use crate::api::models::{ScrapeRequest, ScrapeResponse};
use crate::api::response;
use crate::scraper::{fetch_html, extract_body, format_html, build_prompt};
use crate::llm::call_openrouter;
use crate::AppState;

pub fn create_router(app_state: AppState) -> Router {
    Router::new()
        .route("/api/scrape", post(scrape_handler))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .with_state(app_state)
}

async fn scrape_handler(
    State(state): State<AppState>,
    Json(req): Json<ScrapeRequest>,
) -> impl IntoResponse {
    match process_scrape_request(&state, &req).await {
        Ok(response_data) => response::success(response_data),
        Err(err) => {
            let (status, msg) = match &err {
                AppError::FetchError(msg) => (axum::http::StatusCode::BAD_REQUEST, msg.clone()),
                AppError::ParseError(msg) => (axum::http::StatusCode::UNPROCESSABLE_ENTITY, msg.clone()),
                AppError::LlmError(msg) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
                AppError::ConfigError(msg) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
            };
            
            response::error(status, msg)
        }
    }
}

async fn process_scrape_request(state: &AppState, req: &ScrapeRequest) -> Result<ScrapeResponse> {
    // Fetch and process HTML
    let html = fetch_html(&req.url).await?;
    
    let raw_body = extract_body(&html)
        .ok_or_else(|| crate::error::AppError::ParseError("No <body> tag found in the HTML".to_string()))?;
    
    let formatted = format_html(&raw_body);
    let prompt = build_prompt(&formatted);

    // Calculate word count
    let word_count = formatted.split_whitespace().count();

    // Call LLM
    let summary = call_openrouter(
        &state.config.openrouter_api_key, 
        &prompt, 
        Some(&req.url), 
        None
    ).await?;

    // Ensure proper Markdown formatting
    let formatted_summary = ensure_markdown_formatting(&summary);

    Ok(ScrapeResponse {
        url: req.url.clone(),
        summary: formatted_summary,
        scraped_at: Utc::now(),
        word_count,
        status: "success".to_string(),
    })
}

/// Ensures the text is properly formatted as Markdown.
/// This function does some basic validation and formatting to improve the Markdown structure.
fn ensure_markdown_formatting(text: &str) -> String {
    let text = text.trim();
    
    // If the text doesn't start with a Markdown heading, add one
    if !text.starts_with('#') {
        // Try to find a good title from the first line
        let first_line = text.lines().next().unwrap_or("Summary");
        let title = if first_line.len() > 50 {
            "Summary"
        } else {
            first_line
        };
        
        return format!("# {}\n\n{}", title, text);
    }
    
    // Text already has Markdown formatting, return as is
    text.to_string()
} 