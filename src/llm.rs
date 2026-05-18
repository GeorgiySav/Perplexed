use serde::{Deserialize, Serialize};
use serde_json::from_slice;


#[derive(Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    stream: bool,
}


#[derive(Deserialize, Debug)]
struct ChunkMessage {
    content: String
}

#[derive(Deserialize, Debug)]
struct StreamChunk {
    message: ChunkMessage,
    done: bool
}


pub async fn stream_chat(
    client: &reqwest::Client,
    ollama_url: &str,
    model: &str,
    system_prompt: &str,
    user_prompt: &str,
    mut on_token: impl FnMut(&str)
) -> anyhow::Result<()> {

    let request_body = ChatRequest{
        model: model.to_string(),
        messages: vec![Message{
            role: "system".to_string(),
            content: system_prompt.to_string()
        }, Message{
            role: "user".to_string(),
            content: user_prompt.to_string()
        }],
        stream: true,
    };

    let request_url = format!("{}/api/chat", ollama_url.trim_end_matches("/"));

    let mut response = client.post(request_url).json(&request_body).send().await?.error_for_status()?;

    let mut buffer: Vec<u8> = Vec::new();

    while let Some(chunk) = response.chunk().await? {
        buffer.extend_from_slice(&chunk);

        while let Some(pos) = buffer.iter().position(|&b| b == b'\n') {
            let line: Vec<u8> = buffer.drain(..=pos).collect();

            if !line.iter().all(|c| c.is_ascii_whitespace()) {
                let parsed = from_slice::<StreamChunk>(&line)?;

                if !parsed.message.content.is_empty() {
                    on_token(&parsed.message.content);
                }

                if parsed.done {
                    return Ok(())
                }
            }
        }
    }

    Ok(())
}