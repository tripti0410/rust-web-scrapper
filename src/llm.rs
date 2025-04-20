use serde::Serialize;
use reqwest::{Client, ClientBuilder};
use std::time::Duration;
use once_cell::sync::Lazy;
use crate::error::{Result, AppError};

#[derive(Serialize, Debug)]
struct Message {
    role: String,
    content: String,
}

#[derive(Serialize, Debug)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
    timeout: Option<u32>,
}

// Create a static client to reuse connections with shorter timeout
static CLIENT: Lazy<Client> = Lazy::new(|| {
    ClientBuilder::new()
        .timeout(Duration::from_secs(15))
        .connect_timeout(Duration::from_secs(3))   
        .pool_max_idle_per_host(5)
        .pool_idle_timeout(Duration::from_secs(30))
        .build()
        .expect("Failed to build HTTP client")
});

pub async fn call_openrouter(
    api_key: &str, 
    input_markdown: &str,
    site_url: Option<&str>,
    site_name: Option<&str>
) -> Result<String> {
    println!("🔍 Preparing LLM request");
    
    // Aggressively limit input size - reduced to 1000 characters
    let truncated_input = if input_markdown.len() > 1000 {
        println!("✂️ Truncating prompt from {} to 1000 chars", input_markdown.len());
        &input_markdown[0..1000]
    } else {
        println!("📏 Prompt size: {} chars", input_markdown.len());
        input_markdown
    };
    
    let body = ChatRequest {
        model: "deepseek/deepseek-chat-v3-0324".into(),
        messages: vec![
            Message {
                role: "user".into(),
                content: truncated_input.into(),
            }
        ],
        max_tokens: Some(400),        // Reduced further
        temperature: Some(0.1),
        timeout: Some(10),
    };
    
    println!("📦 Request payload: model={}, max_tokens={}, temperature={}", 
             body.model, body.max_tokens.unwrap_or(0), body.temperature.unwrap_or(0.0));
    
    // Try up to 3 times with exponential backoff
    let max_retries = 2;
    let mut last_error = String::from("Unknown error");
    
    for attempt in 0..=max_retries {
        if attempt > 0 {
            println!("🔄 Retry attempt {} for LLM API request", attempt);
            // Exponential backoff between retries
            tokio::time::sleep(Duration::from_millis(500 * 2u64.pow(attempt as u32))).await;
        }
        
        let mut request = CLIENT
            .post("https://openrouter.ai/api/v1/chat/completions")
            .bearer_auth(api_key)
            .timeout(Duration::from_secs(10))
            .json(&body);
        
        // Add optional headers if provided
        if let Some(url) = site_url {
            request = request.header("HTTP-Referer", url);
        }
        
        if let Some(name) = site_name {
            request = request.header("X-Title", name);
        }
        
        println!("📤 Sending request to OpenRouter API (attempt {})", attempt + 1);
        
        match request.send().await {
            Ok(res) => {
                let status = res.status();
                println!("📥 Received response with status: {}", status);
                
                if status.is_success() {
                    match res.json::<serde_json::Value>().await {
                        Ok(json) => {
                            if let Some(content) = json["choices"][0]["message"]["content"].as_str() {
                                println!("✅ Successfully received LLM response ({} chars)", content.len());
                                return Ok(content.to_string());
                            } else {
                                println!("❌ Invalid response format: {:?}", json);
                                last_error = "Invalid response format".to_string();
                            }
                        },
                        Err(e) => {
                            println!("❌ Failed to parse JSON: {}", e);
                            last_error = format!("JSON parse error: {}", e);
                        }
                    }
                } else {
                    // Try to get error message from response
                    match res.text().await {
                        Ok(text) => {
                            println!("❌ Error response: {}", text);
                            last_error = format!("API error ({}): {}", status, text);
                        },
                        Err(e) => {
                            last_error = format!("HTTP error ({}): {}", status, e);
                        }
                    };
                    
                    // Don't retry on certain status codes
                    if status.as_u16() == 401 || status.as_u16() == 403 {
                        println!("⛔ Not retrying due to authentication error");
                        break;
                    }
                }
            },
            Err(e) => {
                println!("❌ Request error: {}", e);
                last_error = format!("Request error: {}", e);
                
                if e.is_timeout() {
                    println!("⏱️ Request timed out");
                    last_error = "Request timed out".to_string();
                }
            }
        }
    }
    
    println!("❌ All retry attempts failed");
    Err(AppError::LlmError(last_error))
}
