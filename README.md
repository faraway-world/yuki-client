# Yuki — Minimal Streaming Client for llama.cpp

Yuki is a minimal client for interacting with a remote `llama.cpp` inference server.  
It now supports **both Python and Rust clients**, with streaming output, persistent chat memory, and clean separation between interface and inference.

This project exists to understand the system end-to-end — no SDKs, no abstractions, no shortcuts.

## Features

- Token-by-token streaming output
- Persistent chat history (JSON)
- Works over SSH port forwarding
- Compatible with `llama-server`
- Simple, inspectable codebase

## Architecture
```
Local Machine (Client)        Remote Machine (Inference)
┌─────────────────────┐      ┌──────────────────────────┐
│  client.py/ main.rs │ ---> │  llama-server            │
│  chat memory (JSON) │ HTTP │  GGUF model              │
└─────────────────────┘      └──────────────────────────┘
```


Inference runs remotely.  
The client stays local.  
Communication is plain HTTP.  
Rust client uses `tokio` async runtime; Python client uses `requests`.

## Requirements

- **Python client:** Python 3.9+  
- **Rust client:** Rust 1.70+, `cargo`, `tokio`, `reqwest`, `serde`  
- `llama.cpp` built with `llama-server`
- GGUF model
- SSH access to the inference machine

## Setup

### 1. Install dependencies

```bash
pip install -r requirements.txt
```

### 2. Start llama-server (AI machine)
for eg:
```bash
~/llama.cpp/build/bin/llama-server \
  -m ~/models/Llama-3.2-3B-Instruct-Q4_K_M.gguf \
  -c 4096 \
  --port 8080
```

The server listens on `127.0.0.1:8080`.

### 3. Forward the port (local machine)

```bash
ssh -L 8080:127.0.0.1:8080 -C -c aes128-ctr user@REMOTE_IP
```

---

### 4. Run the client

```bash
python3 client.py
```
or
```bash
cargo build --release
```

Type normally. Output streams as tokens are generated.

---

## Chat Memory

Conversation history is stored in:

```
chats/default.json
```

This allows:

* conversation persistence
* manual inspection
* future upgrades (summarization, pruning, embeddings)

## Why this exists

Most “AI apps” hide everything behind SDKs.

This project does the opposite.

It shows:

* how streaming actually works
* how prompts grow and break
* how servers behave under long contexts
* how fragile interfaces really are

## Known Limitations

* No retry logic
* No context pruning
* No role-based formatting enforcement
* Single conversation at a time

Working on fixing these and improving the ux as well progressing with new features.
