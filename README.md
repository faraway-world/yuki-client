# Yuki — Minimal Streaming Client for llama.cpp

Yuki is a high-performance, minimal client designed for interacting with a `llama.cpp` inference server. It provides both **Python** and **Rust** implementations, focusing on transparency and raw system understanding rather than heavy abstractions.

This project exists to understand the LLM system end-to-end—no SDKs, no complex frameworks, no shortcuts.

## Features

- **True Streaming:** Token-by-token output for a zero-latency "live" feel.
- **Persistent Memory:** Automatic conversation saving and loading via `history.json`.
- **Enhanced UX:** Full arrow-key support and command history (powered by `readline` in Python and `rustyline` in Rust).
- **Zero SDKs:** Built using standard HTTP requests to demonstrate how LLM APIs actually work.
- **Cross-Platform:** Works locally or over SSH port forwarding for remote inference.

## Architecture



```text
Local Machine (Client)               Remote Machine (Inference)
┌───────────────────────────────┐      ┌──────────────────────────┐
│  Python/Rust Client           │ ---> │  llama-server            │
│  History (history.json)       │ HTTP │  GGUF Model              │
└───────────────────────────────┘      └──────────────────────────┘

```

Inference runs on the machine with the GPU; the client stays on your local machine. Communication is plain HTTP, typically tunneled through SSH.

## Project Structure

```text
yuki/
├── python/
│   ├── client.py            # Simple implementation using requests
│   └── requirements.txt     # Python dependencies
├── rust/
│   ├── Cargo.toml           # Rust manifest and dependencies
│   └── src/
│       └── main.rs          # High-performance async implementation
└── README.md

```

## Setup & Usage

### 1. Start the Server (Remote Machine)

Run your `llama-server` on your inference machine (example using Llama 3.2):

```bash
~/llama.cpp/build/bin/llama-server \
  -m ~/models/Llama-3.2-3B-Instruct-Q4_K_M.gguf \
  -c 4096 

```

### Port Forwarding (if ai hosted on remote machine)

If the server is remote, tunnel the port to your local machine:

```bash
ssh -L 8080:127.0.0.1:8080 -C user@REMOTE_IP

```

### 2. Running the Clients

#### Python Client

```bash
cd python
pip install -r requirements.txt
python3 client.py

```

#### Rust Client

```bash
cd rust
cargo run --release

```

## Interactive Commands

* **Type normally** to chat.
* **Arrow Keys:** Move the cursor to fix typos or press **Up** to see previous messages.
* **`clear`**: Wipes the current session memory and deletes the local `history.json`.
* **`exit` or `quit**`: Safely closes the session.

## Why Yuki Exists

Most AI applications hide the complexity behind massive SDKs. Yuki does the opposite. It is a "glass box" project designed to show:

* How **Server-Sent Events (SSE)** stream tokens in real-time.
* How the `messages` array grows and maintains state.
* How simple it is to interact with GGUF models directly via HTTP.
* The performance and safety differences between Python and Rust.

## Known Limitations

* **Context Limit:** No automatic context pruning; the message list grows until the model's limit is reached.
* **Single Threaded:** Designed for one conversation at a time.
* **Error Handling:** Basic retry logic for network interruptions is currently being improved.

Fixing these as you read.

## License

MIT - Feel free to use, study, break it, fix it.
