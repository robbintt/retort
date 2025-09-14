use anyhow::Result;
use llm::{
    builder::{LLMBackend, LLMBuilder},
    chat::ChatMessage,
};

pub async fn get_response(messages: &[ChatMessage]) -> Result<String> {
    // Get Google API key from environment variable.
    let api_key =
        std::env::var("GOOGLE_API_KEY").map_err(|_| anyhow::anyhow!("GOOGLE_API_KEY not set."))?;

    let llm = LLMBuilder::new()
        .backend(LLMBackend::Google)
        .api_key(api_key)
        .model("gemini-2.5-flash")
        .max_tokens(8512)
        .temperature(0.7)
        .system("You are a helpful AI assistant specialized in programming.")
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to build LLM (Google): {}", e))?;

    match llm.chat(messages).await {
        Ok(text) => {
            let response_string = text.to_string();
            println!("{}", response_string);
            Ok(response_string)
        }
        Err(e) => anyhow::bail!("Chat error: {e}"),
    }
}
