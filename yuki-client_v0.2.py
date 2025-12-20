import json
import requests

SERVER_URL = "http://127.0.0.1:8080/v1/chat/completions"

session = requests.Session()
session.headers.update({
    "Content-Type": "application/json",
    "Accept": "text/event-stream",
    "Connection": "keep-alive",
})

def chat(messages):
    payload = {
        "model": "local",
        "messages": messages,
        "stream": True,
        "temperature": 0.6,
        "max_tokens": 256,
    }

    assistant = []

    with session.post(
        SERVER_URL,
        json=payload,
        stream=True,
        timeout=300
    ) as r:
        r.raise_for_status()

        for line in r.iter_lines(chunk_size=1, decode_unicode=True):
            if not line:
                continue

            if not line.startswith("data:"):
                continue

            data = line[5:].strip()

            if data == "[DONE]":
                break

            try:
                obj = json.loads(data)
            except json.JSONDecodeError:
                continue

            delta = obj["choices"][0].get("delta", {})
            text = delta.get("content")

            if text:
                print(text, end="", flush=True)
                assistant.append(text)

    print()
    return "".join(assistant)


def main():
    messages = []

    print("Local chat (working). Type exit to quit.")

    while True:
        user = input("\nYou: ").strip()
        if user in {"exit", "quit"}:
            break

        messages.append({"role": "user", "content": user})
        print("\nAssistant: ", end="", flush=True)

        reply = chat(messages)
        
        messages.append({"role": "assistant", "content": reply})


if __name__ == "__main__":
    main()
