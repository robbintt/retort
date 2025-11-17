use ::llm::{
    builder::{LLMBackend, LLMBuilder},
    chat::ChatMessage,
};
use anyhow::Result;
use futures::stream::{Stream, StreamExt};

pub async fn get_response_stream(
    messages: &[ChatMessage],
    system_prompt: Option<String>,
) -> Result<std::pin::Pin<Box<dyn Stream<Item = Result<String>> + Send>>> {
    if let Ok(mock_content) = std::env::var("MOCK_LLM_CONTENT") {
        return Ok(Box::pin(futures::stream::once(async { Ok(mock_content) })));
    }
    if std::env::var("MOCK_LLM").is_ok() {
        let response_string = "This is a mocked response.".to_string();
        return Ok(Box::pin(futures::stream::once(async {
            Ok(response_string)
        })));
    }

    // Get Google API key from environment variable.
    let api_key =
        std::env::var("GEMINI_API_KEY").map_err(|_| anyhow::anyhow!("GEMINI_API_KEY not set."))?;

    let mut builder = LLMBuilder::new()
        .backend(LLMBackend::Google)
        .api_key(api_key)
        .model("gemini-2.5-flash");

    if let Some(system) = system_prompt {
        builder = builder.system(system);
    }

    let llm = builder
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to build LLM (Google): {}", e))?;

    let stream = llm.chat_stream(messages).await?;

    Ok(Box::pin(
        stream.map(|item| item.map_err(anyhow::Error::from)),
    ))
}

pub async fn get_response(
    messages: &[ChatMessage],
    system_prompt: Option<String>,
) -> Result<String> {
    // In a test environment, if MOCK_LLM is set, we return a mock response
    // without making a network call.
    if let Ok(mock_content) = std::env::var("MOCK_LLM_CONTENT") {
        println!("{}", mock_content);
        return Ok(mock_content);
    }
    if std::env::var("MOCK_LLM").is_ok() {
        let response_string = "This is a mocked response.".to_string();
        // The real function prints the response, so we do too for consistency.
        println!("{}", response_string);
        return Ok(response_string);
    }

    // Get Google API key from environment variable.
    let api_key =
        std::env::var("GOOGLE_API_KEY").map_err(|_| anyhow::anyhow!("GOOGLE_API_KEY not set."))?;

    let mut builder = LLMBuilder::new()
        .backend(LLMBackend::Google)
        .api_key(api_key)
        .model("gemini-2.5-flash")
        .max_tokens(8512)
        .temperature(0.7);

    if let Some(system) = system_prompt {
        builder = builder.system(system);
    }

    let llm = builder
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
