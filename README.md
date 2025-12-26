To provide the README as a file, I have formatted the content below. You can copy this text, save it as a file named `README.md` in your main project folder, and it will look professional on GitHub.

```markdown
# Yuki — Minimal Streaming Client for llama.cpp

Yuki is a high-performance, minimal client designed for interacting with a `llama.cpp` inference server. It provides both **Python** and **Rust** implementations, focusing on transparency and raw system understanding rather than heavy abstractions.

This project exists to understand the system end-to-end — no SDKs, no abstractions, no shortcuts.

## Features

- **True Streaming:** Token-by-token output for zero-latency feel.
- **Persistent Memory:** Automatic conversation saving/loading via `history.json`.
- **Enhanced UX:** Full arrow-key support and command history (powered by `readline`/`rustyline`).
- **Zero SDKs:** Built using standard HTTP requests to show how LLM APIs actually work.
- **Cross-Platform:** Works locally or over SSH port forwarding.

## Architecture



```text
Local Machine (Client)               Remote Machine (Inference)
┌───────────────────────────────┐      ┌──────────────────────────┐
│  Python/Rust Client           │ ---> │  llama-server            │
│  History (history.json)       │ HTTP │  GGUF Model              │
└───────────────────────────────┘      └──────────────────────────┘

```

## Project Structure

* `/python`: Lightweight implementation using `requests` and `readline`.
* `/rust`: High-performance implementation using `tokio`, `reqwest`, and `rustyline`.

## Setup & Usage

### 1. Start the Server (Remote Machine)

Run your `llama-server` on your inference machine (example using Llama 3.2 3B):

```bash
~/llama.cpp/build/bin/llama-server \
  -m ~/models/Llama-3.2-3B-Instruct-Q4_K_M.gguf \
  -c 4096 \
  --port 8080

```

### 2. Port Forwarding (Local Machine)

If the server is remote, tunnel the port to your local machine:

```bash
ssh -L 8080:127.0.0.1:8080 -C -c aes128-ctr user@REMOTE_IP

```

### 3. Running the Clients

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
* **Arrow Keys:** Move cursor to fix typos or press **Up** to see message history.
* **`clear`**: Wipes the current session and deletes the history file.
* **`exit` or `quit**`: Safely closes the session.

## Why Yuki?

Most AI applications hide complexity behind massive SDKs. Yuki does the opposite. It is a "glass box" project designed to show:

* How **Server-Sent Events (SSE)** stream tokens in real-time.
* How the **messages array** grows over time to provide "memory."
* How the server behaves under long contexts.
* How to build a professional CLI experience from scratch.

## Known Limitations

* **Context Pruning:** No automatic windowing; history will grow until it hits the model's limit.
* **Error Handling:** Minimal retry logic for network interruptions.
* **Single Session:** Designed for one-on-one conversation tracking.

Fixing these as you read.
---

*Created as a learning journey from CLI to TUI.*

```

---

### How to use this:
1.  **Open a text editor** (like VS Code, Nano, or TextEdit).
2.  **Paste the block above.**
3.  **Save the file** as `README.md` in the root of your `yuki` folder.

**Would you like me to help you create a `.gitignore` file so that your `target/` folders and `history.json` don't get accidentally uploaded to GitHub?**

```
