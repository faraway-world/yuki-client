use std::io::{self, Write};

use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};

const SERVER_URL: &str = "http://127.0.0.1:8080/v1/chat/completions";

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: &'a Vec<Message>,
    stream: bool,
    temperature: f32,
    max_tokens: u32,
}

#[derive(Serialize, Deserialize, Clone)]
struct Message {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct StreamChunk {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    delta: Delta,
}

#[derive(Deserialize)]
struct Delta {
    content: Option<String>,
}

async fn chat(client: &Client, messages: &Vec<Message>) -> anyhow::Result<String> {
    let payload = ChatRequest {
        model: "local",
        messages,
        stream: true,
        temperature: 0.8,
        max_tokens: 256,
    };

    let mut response = client
        .post(SERVER_URL)
        .json(&payload)
        .send()
        .await?
        .bytes_stream();

    let mut assistant = String::new();

    while let Some(chunk) = response.next().await {
        let chunk = chunk?;
        let text = String::from_utf8_lossy(&chunk);

        for line in text.lines() {
            if !line.starts_with("data:") {
                continue;
            }

            let data = line.trim_start_matches("data:").trim();

            if data == "[DONE]" {
                println!();
                return Ok(assistant);
            }

            let parsed: Result<StreamChunk, _> = serde_json::from_str(data);
            if let Ok(obj) = parsed {
                if let Some(choice) = obj.choices.first() {
                    if let Some(content) = &choice.delta.content {
                        print!("{}", content);
                        io::stdout().flush().unwrap();
                        assistant.push_str(content);
                    }
                }
            }
        }
    }

    println!();
    Ok(assistant)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .default_headers({
            let mut h = reqwest::header::HeaderMap::new();
            h.insert("Content-Type", "application/json".parse().unwrap());
            h.insert("Accept", "text/event-stream".parse().unwrap());
            h.insert("Connection", "keep-alive".parse().unwrap());
            h
        })
        .build()?;

    let mut messages: Vec<Message> = Vec::new();

    println!("Local chat (Rust). Type exit to quit.");

    loop {
        print!("\nYou: ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let user = input.trim();

        if user == "exit" || user == "quit" {
            break;
        }

        messages.push(Message {
            role: "user".to_string(),
            content: user.to_string(),
        });

        print!("\nAssistant: ");
        io::stdout().flush().unwrap();

        let reply = chat(&client, &messages).await?;

        messages.push(Message {
            role: "assistant".to_string(),
            content: reply,
        });
    }

    Ok(())
}
