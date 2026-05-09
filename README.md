# Yuki — Minimal Streaming Client for llama.cpp/kobold.cpp

Yuki is a high-performance, minimal client designed for interacting with a `llama.cpp` inference server. It provides both **Python** and **Rust** implementations, focusing on transparency and raw system understanding rather than heavy abstractions.

This project exists to understand the LLM system end-to-end—no SDKs, no complex frameworks, no shortcuts.

## Features

* **True Streaming:** Token-by-token output using Server-Sent Events (SSE) for a zero-latency "live" feel.
* **Smart Session Management:** - **`/load`**: Switch between different chat sessions on the fly with session-name tab completion.
* **`/delete`**: Delete chat histories permanently at startup using the `<name> /delete` syntax.


* **Terminal-Style File Injection:**
* **`/read`**: Full filesystem navigation with tab-completion, tilde (`~`) expansion, and directory tunneling to feed any file's content directly into the AI context.


* **Persistent Memory:**
* **History:** Automatic conversation saving to `~/yuki_client/history/`.
* **Summarization:** Use the `/summarize` command to compress long histories into a "Memory Block" stored in `chats/`, saving context window space.
* **Backups:** Wiping a chat via `/clear` automatically archives the history to `backups/` with a Unix timestamp.


* **Zero SDKs:** Built using standard HTTP requests to demonstrate how OpenAI-compatible LLM APIs actually work.

## Architecture

```text
Local Machine (Client)                Remote Machine (Inference)
┌────────────────────────────────┐      ┌────────────────────────────────┐
│  Yuki (Rust/Python)            │      │  llama-server/kobold.server    │
│  ├─ history/ (Full logs)       │ ---> │  GGUF Model                    │
│  ├─ chats/   (Summaries)       │ HTTP │  (Port 8080 for llama          │
│  └─ backups/ (Archived)        │      │  or 5001 ofr kobold)           │
└────────────────────────────────┘      └────────────────────────────────┘

```

## Project Structure

The Rust client centralizes all data in `~/yuki_client/` to ensure persistence regardless of where the binary is executed.

```text
~/yuki_client/
├── history/           # Persistent JSON conversation logs
├── chats/             # Compressed session summaries (Loaded as initial context)
└── backups/           # Archived logs created upon '/clear' or '/summarize'

[Source]
├── python/
│   └── client.py      # Simple implementation using requests
└── rust/
    ├── Cargo.toml     # Rust manifest (reqwest, tokio, rustyline)
    └── src/
        └── main.rs    # Async implementation with custom path-aware tab-completion

```

## Setup & Usage

### 1. Start the Server (Remote Machine)

```bash
~/llama.cpp/build/bin/llama-server \
  -m ~/models/Llama-3.2-3B-Instruct-Q4_K_M.gguf \
  -c 4096 

```

### 2. Running the Client (Rust)

```bash
cd rust
cargo run --release

```

## Interactive Commands

| Command | Description |
| --- | --- |
| `/vault list` | List files currently in the RAG vault |
| `/vault add <path>` | Copy a local file (supports `~/`) into the vault |
| `/vault rm <name>` | Remove a file from the vault (safely handles full paths) |
| `/refresh` | Rebuild the vector index following vault changes |
| `/read <path>` | Ingest a local file directly into the session |
| `/think <on/off>` | Toggle terminal visibility of reasoning tokens |
| `/summarize` | Distill history into a persistent technical summary |
| `/help` | View the command menu |

## Why Yuki Exists

Most AI applications hide complexity behind massive SDKs. Yuki is a "glass box" project designed to show:

* **SSE Parsing:** Handling `data: ` chunks in real-time.
* **Context Loading:** The logic of checking for summaries vs. full history on boot.
* **Filesystem Integration:** Implementing a custom `rustyline` Completer for terminal-like path traversal.

## License

MIT - Feel free to use, study, break it, or fix it.
