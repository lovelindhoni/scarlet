use std::env;

use crate::error::InterpretError;

const CEREBRAS_MODEL: &str = "llama-4-scout-17b-16e-instruct";
const CEREBRAS_API_URL: &str = "https://api.cerebras.ai/v1/chat/completions";

fn get_api_key() -> Result<String, InterpretError> {
    env::var("CEREBRAS_API_KEY").map_err(|_| InterpretError::AiMissingApiKey {
        message: "AI primitives (generate/verify/classify/extract) require the CEREBRAS_API_KEY environment variable"
            .to_string(),
    })
}

fn call_llm(user_prompt: &str, system_prompt: Option<&str>) -> Result<String, InterpretError> {
    let api_key = get_api_key()?;

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| InterpretError::AiError {
            message: format!("Failed to create async runtime: {}", e),
        })?;

    rt.block_on(async {
        let client = reqwest::Client::new();

        let mut messages = Vec::new();
        if let Some(sys) = system_prompt {
            messages.push(serde_json::json!({"role": "system", "content": sys}));
        }
        messages.push(serde_json::json!({"role": "user", "content": user_prompt}));

        let body = serde_json::json!({
            "model": CEREBRAS_MODEL,
            "messages": messages,
            "stream": false
        });

        let response = client
            .post(CEREBRAS_API_URL)
            .bearer_auth(&api_key)
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

        json["choices"][0]["message"]["content"]
            .as_str()
            .map(|s| s.trim().to_string())
            .ok_or_else(|| InterpretError::AiError {
                message: format!("AI returned an unexpected response: {}", json),
            })
    })
}

pub fn ai_generate(prompt: &str) -> Result<String, InterpretError> {
    call_llm(
        prompt,
        Some("You should never use markdown or any rich formatting text like bolds or italics in your text response"),
    )
}

pub fn ai_verify(prompt: &str) -> Result<bool, InterpretError> {
    let response = call_llm(
        prompt,
        Some("You are a verification engine. Respond with ONLY the word 'true' or 'false'. Nothing else."),
    )?;
    Ok(response.to_lowercase() == "true")
}

pub fn ai_classify(text: &str, labels: &[String]) -> Result<String, InterpretError> {
    let labels_list = labels.join(", ");
    let prompt = format!(
        "Classify the following text into exactly one of these categories: [{}]\n\nText: {}",
        labels_list, text
    );
    let response = call_llm(
        &prompt,
        Some("You are a classification engine. Respond with ONLY the category label, nothing else. The label must be exactly one of the provided categories."),
    )?;
    Ok(response)
}

pub fn ai_extract(query: &str, source: &str) -> Result<String, InterpretError> {
    let prompt = format!(
        "Extract the following from the text below: {}\n\nText: {}",
        query, source
    );
    call_llm(
        &prompt,
        Some("You are an extraction engine. Respond with ONLY the extracted information, nothing else. No explanation, no formatting."),
    )
}
