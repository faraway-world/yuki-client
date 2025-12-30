use std::io::{self, Write};
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};
use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use rustyline::DefaultEditor;
use rustyline::completion::{Completer, Pair};
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{Context, Helper, Editor, Config};

const SERVER_URL: &str = "http://127.0.0.1:8080/v1/chat/completions";

// --- FOLDER STRUCTURE ---
const ROOT_DIR: &str = "yuki_client";
const HIST_DIR: &str = "yuki_client/history";
const CHAT_DIR: &str = "yuki_client/chats";
const BACKUP_DIR: &str = "yuki_client/backups";

#[derive(Serialize, Deserialize, Clone)]
struct Message {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: &'a Vec<Message>,
    stream: bool,
}

#[derive(Deserialize)]
struct StreamChunk { choices: Vec<Choice> }
#[derive(Deserialize)]
struct Choice { delta: Delta }
#[derive(Deserialize)]
struct Delta { content: Option<String> }

// --- TAB COMPLETION SETUP ---
struct ChatCompleter;
impl Helper for ChatCompleter {}
impl Hinter for ChatCompleter { type Hint = String; }
impl Highlighter for ChatCompleter {}
impl Validator for ChatCompleter {}

impl Completer for ChatCompleter {
    type Candidate = Pair;
    fn complete(&self, line: &str, _pos: usize, _ctx: &Context<'_>) -> rustyline::Result<(usize, Vec<Pair>)> {
        let mut completions = Vec::new();
        if let Ok(entries) = fs::read_dir(HIST_DIR) {
            for entry in entries.flatten() {
                let filename = entry.file_name().to_string_lossy().into_owned();
                if filename.starts_with("history_") && filename.ends_with(".json") {
                    let name = &filename[8..filename.len() - 5];
                    if name.starts_with(line) {
                        completions.push(Pair { display: name.to_string(), replacement: name.to_string() });
                    }
                }
            }
        }
        Ok((0, completions))
    }
}

// --- LOGIC FUNCTIONS ---

fn get_file_paths(chat_name: &str) -> (String, String) {
    let history_path = format!("{}/history_{}.json", HIST_DIR, chat_name);
    let summary_path = format!("{}/summary_{}.json", CHAT_DIR, chat_name);
    (history_path, summary_path)
}

fn ensure_dirs() -> io::Result<()> {
    fs::create_dir_all(HIST_DIR)?;
    fs::create_dir_all(CHAT_DIR)?;
    fs::create_dir_all(BACKUP_DIR)?;
    Ok(())
}

fn create_backup(chat_name: &str, hist_path: &str) {
    if fs::metadata(hist_path).is_ok() {
        if let Ok(ts) = SystemTime::now().duration_since(UNIX_EPOCH) {
            let backup_path = format!("{}/log_{}_{}.json", BACKUP_DIR, chat_name, ts.as_secs());
            if fs::copy(hist_path, &backup_path).is_ok() {
                println!("\x1b[94m[System] Archive created: {}\x1b[0m", backup_path);
            }
        }
    }
}

fn list_existing_chats() {
    println!("\x1b[94mExisting Chats:\x1b[0m");
    if let Ok(entries) = fs::read_dir(HIST_DIR) {
        let mut found = false;
        for entry in entries.flatten() {
            let filename = entry.file_name().to_string_lossy().into_owned();
            if filename.starts_with("history_") && filename.ends_with(".json") {
                let name = &filename[8..filename.len() - 5];
                println!("  \x1b[93m- {}\x1b[0m", name);
                found = true;
            }
        }
        if !found { println!("  (No existing chats found in {})", HIST_DIR); }
    }
    println!();
}

fn load_initial_messages(chat_name: &str) -> Vec<Message> {
    let (hist_path, summ_path) = get_file_paths(chat_name);
    if let Ok(data) = fs::read_to_string(&summ_path) {
        if let Ok(msgs) = serde_json::from_str::<Vec<Message>>(&data) {
            println!("\x1b[93m--- Loaded summary memory ({} messages) ---\x1b[0m", msgs.len());
            return msgs;
        }
    }
    if let Ok(data) = fs::read_to_string(&hist_path) {
        if let Ok(msgs) = serde_json::from_str::<Vec<Message>>(&data) {
            println!("\x1b[93m--- Loaded history ({} messages) ---\x1b[0m", msgs.len());
            return msgs;
        }
    }
    println!("\x1b[92m--- Starting new chat: {} ---\x1b[0m", chat_name);
    Vec::new()
}

fn save_to_file(path: &str, messages: &Vec<Message>) {
    if let Ok(json) = serde_json::to_string_pretty(messages) {
        let _ = fs::write(path, json);
    }
}

async fn chat_request(client: &Client, messages: &Vec<Message>) -> anyhow::Result<String> {
    let payload = ChatRequest { model: "local", messages, stream: true };
    let response = client.post(SERVER_URL).json(&payload).send().await?;
    let mut stream = response.bytes_stream();
    let mut full_reply = String::new();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        let text = String::from_utf8_lossy(&chunk);
        for line in text.lines() {
            if !line.starts_with("data: ") { continue; }
            let data = &line[6..];
            if data == "[DONE]" { 
                println!(); 
                return Ok(full_reply); 
            }
            if let Ok(obj) = serde_json::from_str::<StreamChunk>(data) {
                if let Some(choice) = obj.choices.first() {
                    if let Some(content) = &choice.delta.content {
                        print!("{}", content);
                        io::stdout().flush()?;
                        full_reply.push_str(content);
                    }
                }
            }
        }
    }
    println!(); 
    Ok(full_reply)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    ensure_dirs()?;
    let client = Client::new();
    
    let config = Config::builder().build();
    let mut rl_name = Editor::<ChatCompleter, rustyline::history::DefaultHistory>::with_config(config)?;
    rl_name.set_helper(Some(ChatCompleter));
    
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

    println!("{}", logo);
    println!("\x1b[96mYuki Client (Rust) Started.\x1b[0m");
    println!("\x1b[90mData Folder: {}\x1b[0m\n", ROOT_DIR);
    list_existing_chats();
    
    let chat_name = match rl_name.readline("Enter Chat Name (Tab): ") {
        Ok(line) => {
            let val = line.trim().to_string();
            if val.is_empty() { return Ok(()); }
            val
        },
        Err(_) => return Ok(()),
    };

    let mut rl = DefaultEditor::new()?;
    let (hist_path, summ_path) = get_file_paths(&chat_name);
    let mut messages = load_initial_messages(&chat_name);
    
    println!("\x1b[90mCommands: 'exit', 'clear', 'summarize'\x1b[0m");

    loop {
        let char_count: usize = messages.iter().map(|m| m.content.len()).sum();
        let prompt = format!("\n[{} | ~{} chars]: ", chat_name, char_count);
        
        match rl.readline(&prompt) {
            Ok(line) => {
                let user_input = line.trim();
                if user_input.is_empty() { continue; }
                if user_input == "exit" || user_input == "quit" { break; }
                
                if user_input == "clear" {
                    create_backup(&chat_name, &hist_path);
                    messages.clear();
                    let _ = fs::remove_file(&hist_path);
                    let _ = fs::remove_file(&summ_path);
                    println!("\x1b[91mMemory wiped. History archived to {}/backups\x1b[0m", ROOT_DIR);
                    continue;
                }

                if user_input == "summarize" {
                    create_backup(&chat_name, &hist_path);
                    println!("\x1b[95m\n[System] Compressing memory...\x1b[0m");
                    let mut summary_req = messages.clone();
                    summary_req.push(Message { 
                        role: "user".to_string(), 
                        content: "Summarize our conversation into 4 bullet points for your memory.".to_string() 
                    });

                    print!("\x1b[92mAssistant (Summarizing):\x1b[0m ");
                    io::stdout().flush()?;
                    if let Ok(reply) = chat_request(&client, &summary_req).await {
                        messages = vec![Message { 
                            role: "assistant".to_string(), 
                            content: format!("MEMORY_BLOCK:\n{}", reply) 
                        }];
                        save_to_file(&summ_path, &messages);
                        let _ = fs::remove_file(&hist_path);
                        println!("\x1b[92m[Done] Summary saved.\x1b[0m");
                    }
                    continue;
                }

                let user_msg = Message { role: "user".to_string(), content: user_input.to_string() };
                let mut temp_messages = messages.clone();
                temp_messages.push(user_msg.clone());

                print!("\x1b[92mAssistant:\x1b[0m ");
                io::stdout().flush()?;

                match chat_request(&client, &temp_messages).await {
                    Ok(reply) => {
                        let _ = rl.add_history_entry(user_input);
                        messages.push(user_msg);
                        messages.push(Message { role: "assistant".to_string(), content: reply });
                        save_to_file(&hist_path, &messages);
                    }
                    Err(_) => println!("\n\x1b[91mError: Server unreachable.\x1b[0m"),
                }
            },
            Err(_) => break,
        }
    }
    Ok(())
}
