# Yuki — Minimal Streaming Client for llama.cpp

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
┌────────────────────────────────┐      ┌──────────────────────────┐
│  Yuki (Rust/Python)            │      │  llama-server            │
│  ├─ history/ (Full logs)       │ ---> │  GGUF Model              │
│  ├─ chats/   (Summaries)       │ HTTP │  (Port 8080)             │
│  └─ backups/ (Archived)        │      │                          │
└────────────────────────────────┘      └──────────────────────────┘

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

### Startup Screen

* **`Tab`**: Cycle through existing chat names.
* **`<name>`**: Enter a name to load or create a session.
* **`<name> /delete`**: Deletes the specified chat files permanently from the system.

### In-Chat Slash Commands

* **`/load <name>`**: Instantly switch to another chat session (supports Tab-autocomplete).
* **`/read <path>`**: Injects file content into the chat. Features **full terminal-style path completion** (e.g., `/read ~/Down[TAB]` -> `/read ~/Downloads/`).
* **`/summarize`**: Condenses history into 4 bullet points, clearing the active history and saving the summary to `chats/`.
* **`/clear`**: Wiped current session memory and moves `history.json` to `backups/`.
* **`/exit` or `/quit**`: Safely closes the session.

## Why Yuki Exists

Most AI applications hide complexity behind massive SDKs. Yuki is a "glass box" project designed to show:

* **SSE Parsing:** Handling `data: ` chunks in real-time.
* **Context Loading:** The logic of checking for summaries vs. full history on boot.
* **Filesystem Integration:** Implementing a custom `rustyline` Completer for terminal-like path traversal.

## License

MIT - Feel free to use, study, break it, or fix it.
