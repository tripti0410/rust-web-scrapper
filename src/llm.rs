use serde::Serialize;
use reqwest::Client;
use crate::error::{Result, AppError};

#[derive(Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
}

pub async fn call_openrouter(
    api_key: &str, 
    input_markdown: &str,
    site_url: Option<&str>,
    site_name: Option<&str>
) -> Result<String> {
    let client = Client::new();
    let body = ChatRequest {
        model: "deepseek/deepseek-chat-v3-0324".into(),
        messages: vec![
            Message {
                role: "user".into(),
                content: input_markdown.into(),
            }
        ],
    };

    let mut request = client
        .post("https://openrouter.ai/api/v1/chat/completions")
        .bearer_auth(api_key)
        .json(&body);
    
    // Add optional headers if provided
    if let Some(url) = site_url {
        request = request.header("HTTP-Referer", url);
    }
    
    if let Some(name) = site_name {
        request = request.header("X-Title", name);
    }

    let res = request.send().await?;

    let json: serde_json::Value = res.json().await?;
    let reply = json["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| AppError::LlmError("Invalid response format from LLM".to_string()))?
        .to_string();

    Ok(reply)
}
