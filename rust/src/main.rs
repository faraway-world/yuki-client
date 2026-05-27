use chrono::Local;
use colored::*;
use dirs::home_dir;
use fastembed::{TextEmbedding, InitOptions, EmbeddingModel};
use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH, Instant};
use rustyline::error::ReadlineError;
use rustyline::Editor;
use rustyline::Config;
use rustyline::completion::{Candidate, Completer, FilenameCompleter};
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::Helper;

const SERVER_URL: &str = "http://127.0.0.1:8080/v1/chat/completions";

struct YukiCompleter {
    commands: Vec<String>,
    available_chats: Vec<String>,
    file_completer: FilenameCompleter,
}

impl Helper for YukiCompleter {}

impl Completer for YukiCompleter {
    type Candidate = String;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        ctx: &rustyline::Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Self::Candidate>)> {
        let mut candidates = Vec::new();

        if line.starts_with("/read ") && pos >= 6 {
            let path_part = &line[6..pos];
            let (offset, file_candidates) = self.file_completer.complete(path_part, pos - 6, ctx)?;
            let mut results = Vec::new();
            for fc in file_candidates {
                results.push(fc.replacement().to_string());
            }
            return Ok((offset + 6, results));
        }

        if line.starts_with("/vault add ") && pos >= 11 {
            let path_part = &line[11..pos];
            let (offset, file_candidates) = self.file_completer.complete(path_part, pos - 11, ctx)?;
            let mut results = Vec::new();
            for fc in file_candidates {
                results.push(fc.replacement().to_string());
            }
            return Ok((offset + 11, results));
        }

        if line.starts_with("/vault rm ") && pos >= 10 {
            let sub = &line[10..pos];
            let vault_path = get_root_path().join("vault");
            if let Ok(entries) = fs::read_dir(vault_path) {
                for entry in entries.flatten() {
                    if let Ok(name) = entry.file_name().into_string() {
                        if name.starts_with(sub) {
                            candidates.push(name);
                        }
                    }
                }
            }
            return Ok((10, candidates));
        }

        if line.starts_with("/vault ") {
            let sub = &line[7..];
            for cmd in &["list", "add ", "rm "] {
                if cmd.starts_with(sub) {
                    candidates.push(format!("/vault {}", cmd));
                }
            }
        } else if line.starts_with("/think ") {
            let sub = &line[7..];
            for cmd in &["on", "off"] {
                if cmd.starts_with(sub) {
                    candidates.push(format!("/think {}", cmd));
                }
            }
        } else if line.starts_with("/") {
            for cmd in &self.commands {
                if cmd.starts_with(line) {
                    candidates.push(cmd.clone());
                }
            }
        } else {
            for chat in &self.available_chats {
                if chat.starts_with(line) {
                    candidates.push(chat.clone());
                }
            }
        }
        Ok((0, candidates))
    }
}

impl Hinter for YukiCompleter {
    type Hint = String;
    fn hint(&self, _line: &str, _pos: usize, _ctx: &rustyline::Context<'_>) -> Option<Self::Hint> { None }
}

impl Highlighter for YukiCompleter {}

impl Validator for YukiCompleter {}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Message {
    role: String,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    timestamp: Option<u64>,
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

// --- RAG ENGINE ---

#[derive(Serialize, Deserialize)]
struct IndexMetadata {
    last_indexed: u64,
}

struct RagManager {
    model: TextEmbedding,
    index: Vec<(String, Vec<f32>)>,
}

impl RagManager {
    fn new() -> Self {
        let mut options = InitOptions::default();
        options.model_name = EmbeddingModel::BGESmallENV15; // Lightest model for 8GB RAM
        options.show_download_progress = true;

        let model = TextEmbedding::try_new(options).expect("Failed to load embedding model");

        Self { model, index: Vec::new() }
    }

    fn should_rebuild(&self) -> bool {
        let vault_path = get_root_path().join("vault");
        let meta_path = get_root_path().join("index_metadata.json");

        let last_indexed = if let Ok(data) = fs::read_to_string(&meta_path) {
            serde_json::from_str::<IndexMetadata>(&data).map(|m| m.last_indexed).unwrap_or(0)
        } else {
            0
        };

        if let Ok(entries) = fs::read_dir(vault_path) {
            for entry in entries.flatten() {
                if let Ok(meta) = entry.metadata() {
                    if let Ok(modified) = meta.modified() {
                        let ts = modified.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
                        if ts > last_indexed {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    fn rebuild_index(&mut self) {
        let vault_path = get_root_path().join("vault");
        self.index.clear();

        let mut total_chunks = 0;
        if let Ok(entries) = fs::read_dir(vault_path) {
            for entry in entries.flatten() {
                if let Ok(content) = fs::read_to_string(entry.path()) {
                    let chunks: Vec<String> = content.chars()
                        .collect::<Vec<char>>()
                        .chunks(1000)
                        .map(|c| c.iter().collect())
                        .collect();

                    if let Ok(embeddings) = self.model.embed(chunks.clone(), None) {
                        for (text, vec) in chunks.into_iter().zip(embeddings) {
                            self.index.push((text, vec));
                            total_chunks += 1;
                        }
                    }
                }
            }
        }

        let meta_path = get_root_path().join("index_metadata.json");
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let meta = IndexMetadata { last_indexed: now };
        fs::write(meta_path, serde_json::to_string(&meta).unwrap()).ok();

        println!("{}", format!("[System] RAG Index Rebuilt: {} chunks loaded.", total_chunks).magenta());
    }

    fn get_context(&mut self, query: &str, limit: usize) -> String {
        if self.index.is_empty() { return String::new(); }

        let query_vec = self.model.embed(vec![query.to_string()], None)
            .unwrap_or_default()
            .pop()
            .unwrap_or_default();

        let mut scores: Vec<(f32, &String)> = self.index.iter()
            .map(|(text, vec)| (dot_product(&query_vec, vec), text))
            .collect();

        scores.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        
        scores.into_iter()
            .take(limit)
            .map(|(_, text)| text.clone())
            .collect::<Vec<String>>()
            .join("\n---\n")
    }
}

fn dot_product(v1: &[f32], v2: &[f32]) -> f32 {
    v1.iter().zip(v2).map(|(a, b)| a * b).sum()
}

// --- UTILS ---

fn get_root_path() -> PathBuf {
    home_dir().unwrap().join("yuki_client")
}

fn expand_path(path: &str) -> PathBuf {
    if path.starts_with("~/") {
        if let Some(home) = home_dir() {
            return home.join(&path[2..]);
        }
    }
    PathBuf::from(path)
}

fn ensure_dirs() {
    let root = get_root_path();
    fs::create_dir_all(root.join("history")).ok();
    fs::create_dir_all(root.join("chats")).ok();
    fs::create_dir_all(root.join("backups")).ok();
    fs::create_dir_all(root.join("vault")).ok();
    fs::create_dir_all(root.join("memories")).ok();
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
        fs::copy(hist_path, &backup_path).ok();
        println!("{}", format!("[System] Archive created: {:?}", backup_path).blue());
    }
}

fn get_available_chats() -> Vec<String> {
    let root = get_root_path();
    let history_dir = root.join("history");
    let chats_dir = root.join("chats");
    let mut chats = HashSet::new();

    if let Ok(entries) = fs::read_dir(history_dir) {
        for entry in entries.flatten() {
            if let Ok(name) = entry.file_name().into_string() {
                if name.starts_with("history_") && name.ends_with(".json") {
                    chats.insert(name.replace("history_", "").replace(".json", ""));
                }
            }
        }
    }

    if let Ok(entries) = fs::read_dir(chats_dir) {
        for entry in entries.flatten() {
            if let Ok(name) = entry.file_name().into_string() {
                if name.starts_with("summary_") && name.ends_with(".json") {
                    chats.insert(name.replace("summary_", "").replace(".json", ""));
                }
            }
        }
    }
    let mut sorted_chats: Vec<_> = chats.into_iter().collect();
    sorted_chats.sort();
    sorted_chats
}

fn list_available_chats() {
    let chats = get_available_chats();
    if !chats.is_empty() {
        println!("{}", "--- Available Chats ---".magenta());
        for chat in chats {
            println!(" • {}", chat.cyan());
        }
        println!();
    }
}

fn get_vault_manifest() -> String {
    let vault_path = get_root_path().join("vault");
    if let Ok(entries) = fs::read_dir(vault_path) {
        let files: Vec<String> = entries.flatten()
            .filter_map(|e| e.file_name().into_string().ok())
            .collect();
        if files.is_empty() { "None".to_string() } else { files.join(", ") }
    } else {
        "None".to_string()
    }
}

fn get_memories_manifest() -> String {
    let memories_path = get_root_path().join("memories");
    if let Ok(entries) = fs::read_dir(memories_path) {
        let files: Vec<String> = entries.flatten()
            .filter_map(|e| e.file_name().into_string().ok())
            .filter(|n| n.ends_with(".md"))
            .collect();
        if files.is_empty() { "None".to_string() } else { files.join(", ") }
    } else {
        "None".to_string()
    }
}

fn print_help() {
    println!("{}", "--- Command Help ---".magenta());
    println!("  /exit              - Exit the application");
    println!("  /clear             - Clear current chat history and archive it");
    println!("  /summarize         - Summarize current conversation and compress history");
    println!("  /vault list        - List files in the RAG vault");
    println!("  /vault add <path>  - Add a file to the RAG vault");
    println!("  /vault rm <name>   - Remove a file from the RAG vault");
    println!("  /refresh           - Rebuild the RAG index");
    println!("  /read <path>       - Read content of a local file into the chat");
    println!("  /memories          - List all persistent memory files");
    println!("  /think <on/off>    - Toggle visibility of model reasoning");
    println!("  /help              - Show this help message");
    println!();
}

async fn chat_request(mut messages: Vec<Message>, show_thinking: bool, rag: &mut RagManager) -> Option<String> {
    let client = Client::new();
    let start_time = Instant::now();
    
    loop {
        let request = ChatRequest { model: "local".to_string(), messages: messages.clone(), stream: true };
        let res = client.post(SERVER_URL).json(&request).send().await.ok()?;
        let mut stream = res.bytes_stream();
        let mut full_reply = String::new();
        let mut is_thinking = false;
        let mut tag_buffer = String::new();
        let mut tool_call: Option<(String, String, Option<String>)> = None; // (ToolName, Argument, Option<Content>)

        while let Some(item) = stream.next().await {
            let chunk = item.ok()?;
            let text = String::from_utf8_lossy(&chunk);
            for line in text.lines() {
                if line.starts_with("data: ") {
                    let data_str = &line[6..];
                    if data_str == "[DONE]" { break; }
                    if let Ok(data) = serde_json::from_str::<ChatResponse>(data_str) {
                        if let Some(content) = &data.choices[0].delta.content {
                            full_reply.push_str(content);

                            for c in content.chars() {
                                if c == '<' || !tag_buffer.is_empty() {
                                    tag_buffer.push(c);
                                    
                                    if tag_buffer == "<think>" {
                                        if show_thinking {
                                            println!("{}", "\n[Thinking...]".dimmed().italic());
                                            io::stdout().flush().ok();
                                        }
                                        is_thinking = true;
                                        tag_buffer.clear();
                                    } else if tag_buffer == "</think>" {
                                        if show_thinking {
                                            println!();
                                            io::stdout().flush().ok();
                                        }
                                        is_thinking = false;
                                        tag_buffer.clear();
                                    } else if tag_buffer.contains("/>") {
                                        if tag_buffer.contains("<read name=") || tag_buffer.contains("<read path=") {
                                            let name = tag_buffer.replace("<read name=", "").replace("<read path=", "")
                                                .replace("/>", "")
                                                .replace("'", "")
                                                .replace("\"", "");
                                            tool_call = Some(("read".to_string(), name.trim().to_string(), None));
                                            tag_buffer.clear();
                                            break; 
                                        } else if tag_buffer.contains("<read_memory name=") {
                                            let name = tag_buffer.replace("<read_memory name=", "")
                                                .replace("/>", "")
                                                .replace("'", "")
                                                .replace("\"", "");
                                            tool_call = Some(("read_memory".to_string(), name.trim().to_string(), None));
                                            tag_buffer.clear();
                                            break;
                                        } else if tag_buffer.contains("<search_vault query=") {
                                            let query = tag_buffer.replace("<search_vault query=", "")
                                                .replace("/>", "")
                                                .replace("'", "")
                                                .replace("\"", "");
                                            tool_call = Some(("search".to_string(), query.trim().to_string(), None));
                                            tag_buffer.clear();
                                            break;
                                        } else {
                                            // Flush tag_buffer if it's not a recognized tool
                                            for buffered_char in tag_buffer.chars() {
                                                if is_thinking {
                                                    if show_thinking { print!("{}", buffered_char.to_string().truecolor(100, 100, 100)); }
                                                } else {
                                                    print!("{}", buffered_char);
                                                }
                                            }
                                            io::stdout().flush().ok();
                                            tag_buffer.clear();
                                        }
                                    } else if tag_buffer.ends_with("</write_memory>") {
                                        let start_tags = ["<write_memory name='", "<write_memory name=\""];
                                        let mut found = false;
                                        for start_tag in start_tags {
                                            if let Some(start_idx) = tag_buffer.find(start_tag) {
                                                let rest = &tag_buffer[start_idx + start_tag.len()..];
                                                let end_quote = if start_tag.contains("'") { "'" } else { "\"" };
                                                let closing_tag = format!("{}>", end_quote);
                                                if let Some(end_name_idx) = rest.find(&closing_tag) {
                                                    let name = &rest[..end_name_idx];
                                                    let content_start = end_name_idx + closing_tag.len();
                                                    let content_end = rest.len() - "</write_memory>".len();
                                                    
                                                    if content_end >= content_start {
                                                        let content = &rest[content_start..content_end];
                                                        tool_call = Some(("write_memory".to_string(), name.to_string(), Some(content.to_string())));
                                                        tag_buffer.clear();
                                                        found = true;
                                                        break;
                                                    }
                                                }
                                            }
                                        }
                                        if !found { 
                                            // Only clear if it's clearly NOT going to be a write_memory tag
                                            if tag_buffer.len() > 8000 {
                                                for buffered_char in tag_buffer.chars() {
                                                    if is_thinking {
                                                        if show_thinking { print!("{}", buffered_char.to_string().truecolor(100, 100, 100)); }
                                                    } else {
                                                        print!("{}", buffered_char);
                                                    }
                                                }
                                                io::stdout().flush().ok();
                                                tag_buffer.clear();
                                            }
                                        }
                                        else { break; }
                                    } else if tag_buffer.len() > 100 && !tag_buffer.starts_with("<write_memory") { 
                                        // If it's not a write_memory tag and it's getting long, it's probably just text
                                        for buffered_char in tag_buffer.chars() {
                                            if is_thinking {
                                                if show_thinking { print!("{}", buffered_char.to_string().truecolor(100, 100, 100)); }
                                            } else {
                                                print!("{}", buffered_char);
                                            }
                                        }
                                        io::stdout().flush().ok();
                                        tag_buffer.clear();
                                    }
                                } else {
                                    if is_thinking {
                                        if show_thinking {
                                            print!("{}", c.to_string().truecolor(100, 100, 100));
                                        }
                                    } else {
                                        print!("{}", c);
                                    }
                                    io::stdout().flush().ok();
                                }
                            }
                            if tool_call.is_some() { break; }
                        }
                    }
                }
            }
            if tool_call.is_some() { break; }
        }

        if let Some((tool, arg, content)) = tool_call {
            messages.push(Message { role: "assistant".to_string(), content: full_reply.clone(), timestamp: Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()) });
            
            if tool == "read" {
                let vault_path = get_root_path().join("vault").join(&arg);
                match fs::read_to_string(vault_path) {
                    Ok(content) => {
                        println!("{}", format!("  ✓ ReadVault {}", arg).green());
                        messages.push(Message { 
                            role: "system".to_string(), 
                            content: format!("### VAULT DATA START ###\n[FILE: {}]\n{}\n### VAULT DATA END ###\n\nInstructions: Vault data provided. Proceed.", arg, content),
                            timestamp: Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs())
                        });
                    }
                    Err(_) => {
                        println!("{}", format!("  ✗ Failed to read vault file {}", arg).red());
                        messages.push(Message { role: "system".to_string(), content: format!("TOOL_RESULT: Error reading vault file {}", arg), timestamp: Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()) });
                    }
                }
            } else if tool == "read_memory" {
                let memory_path = get_root_path().join("memories").join(&arg);
                match fs::read_to_string(memory_path) {
                    Ok(content) => {
                        let line_count = content.lines().count();
                        println!("{}", format!("  ✓ ReadMemory {} ({} lines)", arg, line_count).green());
                        messages.push(Message { 
                            role: "system".to_string(), 
                            content: format!("### MEMORY DATA START ###\n[MEMORY: {}]\n{}\n### MEMORY DATA END ###\n\nInstructions: Memory provided. Proceed.", arg, content),
                            timestamp: Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs())
                        });
                    }
                    Err(_) => {
                        println!("{}", format!("  ✗ Failed to read memory {}", arg).red());
                        messages.push(Message { role: "system".to_string(), content: format!("TOOL_RESULT: Error reading memory {}", arg), timestamp: Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()) });
                    }
                }
            } else if tool == "write_memory" {
                if let Some(c) = content {
                    let memory_path = get_root_path().join("memories").join(&arg);
                    let old_size = if memory_path.exists() {
                        fs::read_to_string(&memory_path).map(|s| s.len()).unwrap_or(0)
                    } else {
                        0
                    };
                    let new_size = c.len();
                    let line_count = c.lines().count();
                    
                    match fs::write(&memory_path, &c) {
                        Ok(_) => {
                            let action = if old_size == 0 { 
                                "CreatedMemory" 
                            } else if new_size < old_size {
                                "CuratedMemory"
                            } else {
                                "UpdatedMemory"
                            };
                            println!("{}", format!("  ✓ {} {} ({} lines)", action, arg, line_count).green());
                            messages.push(Message { 
                                role: "system".to_string(), 
                                content: format!("SUCCESS: Memory [{}] has been {}. You can continue with your response.", arg, action.to_lowercase()),
                                timestamp: Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs())
                            });
                        }
                        Err(e) => {
                            println!("{}", format!("  ✗ Failed to write memory {}: {}", arg, e).red());
                            messages.push(Message { role: "system".to_string(), content: format!("TOOL_RESULT: Error writing memory {}: {}", arg, e), timestamp: Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()) });
                        }
                    }
                }
            } else if tool == "search" {
                let context = rag.get_context(&arg, 6);
                println!("{}", "  ✓ SearchVault".blue());
                messages.push(Message { 
                    role: "system".to_string(), 
                    content: format!("SEARCH_RESULTS for [{}]: \n\n{}\n\nInstructions: MANDATORY: Once a tool result is provided, your NEXT response MUST start with 'HANDSHAKE: [Search] processed.' followed by your analysis.", arg, context),
                    timestamp: Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs())
                });
            }
        } else {
            let duration = start_time.elapsed();
            println!("{}", format!("\n[Response took: {:.2}s]", duration.as_secs_f64()).dimmed());
            return Some(full_reply);
        }
    }
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

    print_help();

    let mut rag = RagManager::new();
    if rag.should_rebuild() {
        rag.rebuild_index();
    } else {
        println!("{}", "[System] Vault unchanged. Index loaded from memory.".green());
    }

    list_available_chats();

    let config = Config::builder().build();
    let mut rl: Editor<YukiCompleter, _> = Editor::with_config(config).expect("Failed to initialize line editor");
    let completer = YukiCompleter {
        commands: vec![
            "/exit".to_string(),
            "/clear".to_string(),
            "/summarize".to_string(),
            "/vault".to_string(),
            "/refresh".to_string(),
            "/help".to_string(),
            "/read".to_string(),
            "/think".to_string(),
        ],
        available_chats: get_available_chats(),
        file_completer: FilenameCompleter::new(),
    };
    rl.set_helper(Some(completer));

    let current_chat_input = rl.readline("Enter Chat Name: ").expect("Failed to read chat name");
    let current_chat = current_chat_input.trim();

    let (hp, sp) = get_file_paths(current_chat);
    let mut messages: Vec<Message> = Vec::new();
    let mut show_thinking = true;
    let session_start = Instant::now();

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
        let context_usage = (char_count as f64 / 65536.0) * 100.0;
        let prompt = format!("\n[{} | {:.2}% context used]: ", current_chat, context_usage);
        
        match rl.readline(&prompt) {
            Ok(line) => {
                let input = line.trim();
                if input.is_empty() { continue; }
                rl.add_history_entry(input).ok();

                if input == "/exit" { break; }
                if input == "/help" { print_help(); continue; }

                if input == "/memories" {
                    println!("{}", "--- Persistent Memories ---".magenta());
                    let manifest = get_memories_manifest();
                    if manifest == "None" { println!(" No memories saved yet."); }
                    else {
                        for m in manifest.split(", ") { println!(" • {}", m.cyan()); }
                    }
                    continue;
                }

                if input.starts_with("/think ") {
                    let arg = &input[7..];
                    if arg == "on" { show_thinking = true; println!("{}", "[System] Reasoning visibility: ON".green()); }
                    else if arg == "off" { show_thinking = false; println!("{}", "[System] Reasoning visibility: OFF".yellow()); }
                    else { println!("{}", "[System] Usage: /think <on/off>".cyan()); }
                    continue;
                }

                if input.starts_with("/read ") {
                    let path_str = &input[6..];
                    let expanded_path = expand_path(path_str);
                    match fs::read_to_string(&expanded_path) {
                        Ok(content) => {
                            println!("{}", format!("[System] Reading file: {:?}", expanded_path).magenta());
                            let mut active_context = messages.clone();
                            let filename = expanded_path.file_name().and_then(|n| n.to_str()).unwrap_or(path_str);

                            let sys_msg = Message {
                                role: "system".to_string(),
                                content: format!("### SOURCE DATA START ###\n[FILE: {}]\n{}\n### SOURCE DATA END ###\n\nInstructions: You have just been provided with the data above. Proceed with your analysis or response.", filename, content),
                                timestamp: Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs())
                            };
                            active_context.insert(0, sys_msg);
                            active_context.push(Message { role: "user".to_string(), content: format!("Please analyze the file: {}", filename), timestamp: Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()) });

                            if let Some(reply) = chat_request(active_context, show_thinking, &mut rag).await {
                                println!("{}", "[System] Direct file upload complete. Model is now aware of the full content.".green());
                                messages.push(Message { role: "user".to_string(), content: format!("[File: {} read]", filename), timestamp: Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()) });
                                messages.push(Message { role: "assistant".to_string(), content: reply, timestamp: Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()) });
                                fs::write(&hp, serde_json::to_string_pretty(&messages).unwrap()).ok();
                            }
                        }
                        Err(e) => println!("{}: {}", "[Error] Could not read file".red(), e),
                    }
                    continue;
                }


                if input == "/refresh" {
                    println!("{}", "[System] Manually refreshing RAG index...".magenta());
                    rag.rebuild_index();
                    continue;
                }

                if input.starts_with("/vault ") {
                    let args = &input[7..];
                    let vault_path = get_root_path().join("vault");
                    if args == "list" {
                        println!("{}", "--- Vault Contents ---".magenta());
                        if let Ok(entries) = fs::read_dir(&vault_path) {
                            for entry in entries.flatten() {
                                if let Ok(name) = entry.file_name().into_string() { println!(" - {}", name); }
                            }
                        }
                    } else if args.starts_with("add ") {
                        let source_str = &args[4..];
                        let source_path = expand_path(source_str);
                        if let Some(filename) = source_path.file_name() {
                            let dest_path = vault_path.join(filename);
                            if let Err(e) = fs::copy(&source_path, dest_path) {
                                println!("{}: {}", "[Error] Failed to copy file to vault".red(), e);
                            } else {
                                println!("{}", "[System] File added. Run /refresh to update memory.".green());
                            }
                        }
                    } else if args.starts_with("rm ") {
                        let filename_input = &args[3..];
                        let filename = Path::new(filename_input).file_name()
                            .and_then(|n| n.to_str()).unwrap_or(filename_input);
                        let target_path = vault_path.join(filename);
                        if let Err(e) = fs::remove_file(target_path) {
                            println!("{}: {}", "[Error] Failed to remove file from vault".red(), e);
                        } else {
                            println!("{}", "[System] File removed. Run /refresh to update memory.".yellow());
                        }
                    } else { println!("{}", "[System] Usage: /vault <list|add <path>|rm <filename>>".cyan()); }
                    continue;
                }

                if input == "/clear" {
                    create_backup(current_chat, &hp);
                    messages.clear();
                    if hp.exists() { fs::remove_file(&hp).ok(); }
                    if sp.exists() { fs::remove_file(&sp).ok(); }
                    println!("{}", "[System] Memory wiped and archived.".yellow());
                    continue;
                }
                
                if input == "/summarize" {
                    println!("{}", "[System] Summarizing...".magenta());
                    let mut req = messages.clone();
                    req.push(Message { role: "user".to_string(), content: "Summarize into 4 technical points.".to_string(), timestamp: Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()) });
                    if let Some(reply) = chat_request(req, show_thinking, &mut rag).await {
                        messages = vec![Message { role: "system".to_string(), content: format!("SUMMARY:\n{}", reply), timestamp: Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()) }];
                        fs::write(&sp, serde_json::to_string_pretty(&messages).unwrap()).ok();
                        if hp.exists() { fs::remove_file(&hp).ok(); }
                    }
                    continue;
                }

                let now_ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
                let last_ts = messages.last().and_then(|m| m.timestamp).unwrap_or(now_ts);
                let elapsed_since_last = now_ts - last_ts;
                
                let time_info = if elapsed_since_last > 60 {
                    let mins = elapsed_since_last / 60;
                    let hours = mins / 60;
                    if hours > 0 {
                        format!("It has been {} hours and {} minutes since the last interaction.", hours, mins % 60)
                    } else {
                        format!("It has been {} minutes since the last interaction.", mins)
                    }
                } else {
                    "Just a moment has passed since the last interaction.".to_string()
                };

                let cwd = std::env::current_dir().unwrap_or_default().display().to_string();
                let local_time = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
                let vault_manifest = get_vault_manifest();
                let memory_manifest = get_memories_manifest();
                let session_elapsed = session_start.elapsed().as_secs();

                // v0.9.0 Sustainable Memory Counter
                let turn_count = messages.len() / 2;
                let curation_trigger = if turn_count > 0 && turn_count % 10 == 0 {
                    "\n[SYSTEM NOTIFICATION: Memory maintenance cycle. Review your Persistent Memories and compress/curate any that are nearing the 2000 character limit, especially user_preferences.md.]"
                } else {
                    ""
                };

                let awareness_msg = Message {
                    role: "system".to_string(),
                    content: format!(
                        "SYSTEM NOTIFICATION:
[IDENTITY & MISSION]
You are the Yuki AI. You have a dual-layer knowledge system:
1. VAULT (RAG): Static project files. Use for technical reference.
2. PERSISTENT MEMORY: Long-term files in 'memories/'. Use for user traits, project status, and continuity.

[SUSTAINABILITY RULES]
- HARD LIMIT: Memory files MUST NOT exceed 2000 characters.
- CURATION: If a memory file is large, you MUST compress it (remove old/irrelevant info) when updating.
- PREFERENCES: Actively maintain 'user_preferences.md' to store user style, recurring tasks, and traits.

[ENVIRONMENT]
Time: {}
Location: {}
Session Start: {}s ago
{}

[MEMORY & KNOWLEDGE]
Vault (RAG) Files: [{}]
Persistent Memory Files: [{}]
{}

[CRITICAL TOOLS]
- READ VAULT: To read a file from the 'Vault (RAG) Files' list, you MUST use <read name='filename.ext'/>.
- CHECK MEMORY: To read a file from 'Persistent Memory Files', you MUST use <read_memory name='filename.md'/>.
- SAVE/CURATE MEMORY: To save or compress memory, use <write_memory name='filename.md'>Content</write_memory>.
- SEARCH VAULT: Use <search_vault query='...'/> for broad RAG searches.

[DIRECTIVE]
DO NOT claim you don't know something if there are files in your manifests that you haven't read yet. Read them first using the correct tool.

MANDATORY: If you use a tool, you must acknowledge the result naturally in your response.", 
                        local_time, cwd, session_elapsed, time_info, vault_manifest, memory_manifest, curation_trigger),
                    timestamp: Some(now_ts)
                };

                let mut active_context = messages.clone();
                active_context.insert(0, awareness_msg);
                active_context.push(Message { role: "user".to_string(), content: input.to_string(), timestamp: Some(now_ts) });
                
                if let Some(reply) = chat_request(active_context, show_thinking, &mut rag).await {
                    messages.push(Message { role: "user".to_string(), content: input.to_string(), timestamp: Some(now_ts) });
                    messages.push(Message { role: "assistant".to_string(), content: reply, timestamp: Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()) });
                    fs::write(&hp, serde_json::to_string_pretty(&messages).unwrap()).ok();
                }
            },
            Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => break,
            Err(err) => { println!("Error: {:?}", err); break; }
        }
    }
}
