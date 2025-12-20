import json
import requests
from pathlib import Path

SERVER_URL = "http://127.0.0.1:8080/v1/chat/completions"
CHAT_FILE = Path("chats/default.json")

MAX_CONTEXT_MESSAGES = 6
MAX_GENERATION_TOKENS = 256

def load_chat():
    if CHAT_FILE.exists():
        with open(CHAT_FILE, "r", encoding="utf-8") as f:
            return json.load(f)
    return {"messages": []}

def save_chat(chat):
    CHAT_FILE.parent.mkdir(parents=True, exist_ok=True)
    with open(CHAT_FILE, "w", encoding="utf-8") as f:
        json.dump(chat, f, indent=2, ensure_ascii=False)

def inference_messages(full_messages):
    return full_messages[-MAX_CONTEXT_MESSAGES:]

def send_stream(messages):
    payload = {
        "model": "local",
        "messages": messages,
        "stream": True,
        "temperature": 0.7,
        "max_tokens": MAX_GENERATION_TOKENS,
    }

    assistant_text = ""

    with requests.post(
        SERVER_URL,
        json=payload,
        stream=True,
        timeout=300
    ) as r:
        r.raise_for_status()

        for line in r.iter_lines(decode_unicode=True):
            if not line or not line.startswith("data:"):
                continue

            data = line[5:].strip()
            if data == "[DONE]":
                break

            try:
                obj = json.loads(data)
            except json.JSONDecodeError:
                continue

            delta = obj["choices"][0]["delta"]
            chunk = delta.get("content")
            if chunk:
                print(chunk, end="", flush=True)
                assistant_text += chunk

    print()
    return assistant_text

def main():
    full_chat = load_chat()

    while True:
        user_input = input("\nYou: ").strip()
        if user_input.lower() in {"exit", "quit"}:
            break

        full_chat["messages"].append({
            "role": "user",
            "content": user_input
        })

        print("\nAssistant: ", end="", flush=True)

        context = inference_messages(full_chat["messages"])
        assistant_msg = send_stream(context)

        full_chat["messages"].append({
            "role": "assistant",
            "content": assistant_msg
        })

        save_chat(full_chat)

if __name__ == "__main__":
    main()
