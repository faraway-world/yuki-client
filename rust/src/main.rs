use colored::*;
use dirs::home_dir;
use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const SERVER_URL: &str = "http://127.0.0.1:8080/v1/chat/completions";

#[derive(Serialize, Deserialize, Clone, Debug)]
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

#[derive(Deserialize)]
struct ChatResponse {
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

// --- UTILS ---

fn get_root_path() -> PathBuf {
    home_dir().unwrap().join("yuki_client")
}

fn ensure_dirs() {
    let root = get_root_path();
    fs::create_dir_all(root.join("history")).ok();
    fs::create_dir_all(root.join("chats")).ok();
    fs::create_dir_all(root.join("backups")).ok();
}

fn get_file_paths(chat_name: &str) -> (PathBuf, PathBuf) {
    let root = get_root_path();
    let hp = root.join("history").join(format!("history_{}.json", chat_name));
    let sp = root.join("chats").join(format!("summary_{}.json", chat_name));
    (hp, sp)
}

fn create_backup(chat_name: &str, hist_path: &Path) {
    if hist_path.exists() {
        let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let backup_path = get_root_path().join("backups").join(format!("log_{}_{}.json", chat_name, ts));
        fs::copy(hist_path, backup_path).ok();
    }
}

async fn chat_request(messages: Vec<Message>) -> Option<String> {
    let client = Client::new();
    let request = ChatRequest {
        model: "local".to_string(),
        messages,
        stream: true,
    };

    let res = client.post(SERVER_URL).json(&request).send().await.ok()?;
    let mut stream = res.bytes_stream();
    let mut full_reply = String::new();

    while let Some(item) = stream.next().await {
        let chunk = item.ok()?;
        let text = String::from_utf8_lossy(&chunk);
        
        for line in text.lines() {
            if line.starts_with("data: ") {
                let data_str = &line[6..];
                if data_str == "[DONE]" { break; }
                if let Ok(data) = serde_json::from_str::<ChatResponse>(data_str) {
                    if let Some(content) = &data.choices[0].delta.content {
                        print!("{}", content);
                        io::stdout().flush().ok();
                        full_reply.push_str(content);
                    }
                }
            }
        }
    }
    println!();
    Some(full_reply)
}

#[tokio::main]
async fn main() {
    ensure_dirs();
    
    let logo = r#"
 █████ █████ █████  █████ █████   ████ █████
▒▒███ ▒▒███ ▒▒███  ▒▒███ ▒▒███   ███▒ ▒▒███ 
 ▒▒███ ███   ▒███   ▒███  ▒███  ███    ▒███ 
  ▒▒█████    ▒███   ▒███  ▒███████     ▒███ 
   ▒▒███     ▒███   ▒███  ▒███▒▒███    ▒███ 
    ▒███     ▒███   ▒███  ▒███ ▒▒███   ▒███ 
    █████    ▒▒████████   █████ ▒▒████ █████
   ▒▒▒▒▒      ▒▒▒▒▒▒▒▒   ▒▒▒▒▒   ▒▒▒▒ ▒▒▒▒▒ 
    "#;

    println!("{}", logo.cyan());
    println!("{}", "Yuki Client (Rust Native) Started.".cyan());

    // Abbreviated Logic for TUI Loop
    let mut current_chat = String::new();
    let mut messages: Vec<Message> = Vec::new();

    print!("Enter Chat Name: ");
    io::stdout().flush().ok();
    io::stdin().read_line(&mut current_chat).ok();
    let current_chat = current_chat.trim();

    let (hp, sp) = get_file_paths(current_chat);

    // Hybrid Load Logic
    if sp.exists() {
        if let Ok(data) = fs::read_to_string(&sp) {
            messages = serde_json::from_str(&data).unwrap_or_default();
            println!("{}", "--- Base Summary Loaded ---".yellow());
        }
    }
    if hp.exists() {
        if let Ok(data) = fs::read_to_string(&hp) {
            let hist: Vec<Message> = serde_json::from_str(&data).unwrap_or_default();
            messages.extend(hist);
            println!("{}", "--- Incremental History Appended ---".yellow());
        }
    }

    loop {
        let char_count: usize = messages.iter().map(|m| m.content.len()).sum();
        print!("\n[{} | ~{} chars]: ", current_chat, char_count);
        io::stdout().flush().ok();

        let mut input = String::new();
        io::stdin().read_line(&mut input).ok();
        let input = input.trim();

        if input == "/exit" { break; }
        
        if input.starts_with("/summarize") {
            println!("{}", "[System] Summarizing...".magenta());
            let mut req = messages.clone();
            req.push(Message { role: "user".to_string(), content: "Summarize into 4 technical points.".to_string() });
            if let Some(reply) = chat_request(req).await {
                messages = vec![Message { role: "system".to_string(), content: format!("SUMMARY:\n{}", reply) }];
                fs::write(&sp, serde_json::to_string_pretty(&messages).unwrap()).ok();
                if hp.exists() { fs::remove_file(&hp).ok(); }
            }
            continue;
        }

        // Regular turn
        messages.push(Message { role: "user".to_string(), content: input.to_string() });
        if let Some(reply) = chat_request(messages.clone()).await {
            messages.push(Message { role: "assistant".to_string(), content: reply });
            fs::write(&hp, serde_json::to_string_pretty(&messages).unwrap()).ok();
        }
    }
}