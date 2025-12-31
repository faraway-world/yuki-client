# Yuki — Minimal Streaming Client for llama.cpp

Yuki is a high-performance, minimal client designed for interacting with a `llama.cpp` inference server. It provides both **Python** and **Rust** implementations, focusing on transparency and raw system understanding rather than heavy abstractions.

This project exists to understand the LLM system end-to-end—no SDKs, no complex frameworks, no shortcuts.

## Features

- **True Streaming:** Token-by-token output using Server-Sent Events (SSE) for a zero-latency "live" feel.
- **Named Chat Sessions:** Support for multiple separate conversations with tab-completion for existing logs.
- **Persistent Memory:** - **History:** Automatic conversation saving to `history/`.
  - **Summarization:** Use the `summarize` command to compress long histories into a "Memory Block" stored in `chats/`, saving context window space.
  - **Backups:** Wiping a chat via `clear` automatically moves the history to `backups/` with a timestamp.
- **Enhanced UX:** Full arrow-key support and command history (powered by `rustyline` in Rust).
- **Zero SDKs:** Built using standard HTTP requests to demonstrate how OpenAI-compatible LLM APIs actually work.

## Architecture



```text
Local Machine (Client)                 Remote Machine (Inference)
┌────────────────────────────────┐      ┌──────────────────────────┐
│  Yuki (Rust/Python)            │      │  llama-server            │
│  ├─ history/ (Full logs)       │ ---> │  GGUF Model              │
│  ├─ chats/   (Summaries)       │ HTTP │  (Port 8080)             │
│  └─ backups/ (Archived)        │      │                          │
└────────────────────────────────┘      └──────────────────────────┘

```

Inference runs on the machine with the GPU; the client stays on your local machine. Communication is plain HTTP, typically tunneled through SSH.

## Project Structure

```text
yuki_client/
├── history/           # Persistent JSON conversation logs
├── chats/             # Compressed session summaries
├── backups/           # Archived logs created upon 'clear'
├── python/
│   ├── client.py      # Simple implementation using requests
│   └── requirements.txt
└── rust/
    ├── Cargo.toml     # Rust manifest (reqwest, tokio, rustyline)
    └── src/
        └── main.rs    # Async implementation with custom tab-completion

```

## Setup & Usage

### 1. Start the Server (Remote Machine)

Run your `llama-server` on your inference machine (example using the model Llama 3.2):

```bash
~/llama.cpp/build/bin/llama-server \
  -m ~/models/Llama-3.2-3B-Instruct-Q4_K_M.gguf \
  -c 4096 

```

### 2. Port Forwarding (If hosted remotely)

If the server is remote, tunnel the port to your local machine:

```bash
ssh -L 8080:127.0.0.1:8080 -C user@REMOTE_IP

```

### 3. Running the Client (Rust)

```bash
cd rust
# The client will prompt for a Chat Name. Use Tab to see existing chats.
cargo run --release

```

## Interactive Commands

* **`Enter Chat Name`**: Type a new name to start fresh or an old name to resume. Use **Tab** to cycle through existing history files.
* **`summarize`**: Forces the model to condense the current chat into 4 bullet points. It then clears the active history and saves the summary to `chats/` as the new starting context.
* **`clear`**: Wipes the current session memory from the model and moves your current `history.json` to the `backups/` folder.
* **`exit` or `quit**`: Safely closes the session.

## Why Yuki Exists

Most AI applications hide the complexity behind massive SDKs. Yuki does the opposite. It is a "glass box" project designed to show:

* **SSE Parsing:** How `data: ` chunks are handled in real-time.
* **Context Loading:** How the client checks for summaries vs. full history on boot.
* **Memory Management:** The difference between raw history and summarized "Memory Blocks."
* **System Design:** How to build a robust CLI with path-aware storage (absolute paths in `~/yuki_client`).

## License

MIT - Feel free to use, study, break it, or fix it.
