use std::env;

use crate::error::InterpretError;

const GEMINI_MODEL: &str = "gemini-2.0-flash";

fn get_api_key() -> Result<String, InterpretError> {
    env::var("GEMINI_API_KEY").map_err(|_| InterpretError::AiMissingApiKey {
        message: "AI primitives (prompt/verify) require the GEMINI_API_KEY environment variable"
            .to_string(),
    })
}

fn call_gemini(
    prompt: &str,
    system_instruction: Option<&str>,
) -> Result<String, InterpretError> {
    let api_key = get_api_key()?;

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| InterpretError::AiError {
            message: format!("Failed to create async runtime: {}", e),
        })?;

    rt.block_on(async {
        let client = reqwest::Client::new();
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            GEMINI_MODEL, api_key
        );

        let mut body = serde_json::json!({
            "contents": [{
                "parts": [{"text": prompt}]
            }]
        });

        if let Some(instruction) = system_instruction {
            body["systemInstruction"] = serde_json::json!({
                "parts": [{"text": instruction}]
            });
        }

        let response = client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| InterpretError::AiError {
                message: format!("AI API request failed: {}", e),
            })?;

        let json: serde_json::Value =
            response.json().await.map_err(|e| InterpretError::AiError {
                message: format!("Failed to parse AI response: {}", e),
            })?;

        json["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .map(|s| s.trim().to_string())
            .ok_or_else(|| InterpretError::AiError {
                message: format!("AI returned an unexpected response: {}", json),
            })
    })
}

pub fn ai_prompt(prompt: &str) -> Result<String, InterpretError> {
    call_gemini(prompt, None)
}

pub fn ai_verify(prompt: &str) -> Result<bool, InterpretError> {
    let response = call_gemini(
        prompt,
        Some("You are a verification engine. Respond with ONLY the word 'true' or 'false'. Nothing else."),
    )?;
    Ok(response.to_lowercase() == "true")
}
