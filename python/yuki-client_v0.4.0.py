import json
import os
import requests
from prompt_toolkit import PromptSession
from prompt_toolkit.history import FileHistory

SERVER_URL = "http://127.0.0.1:8080/v1/chat/completions"
HISTORY_FILE = "history.json"

def load_history():
    if os.path.exists(HISTORY_FILE):
        try:
            with open(HISTORY_FILE, "r") as f:
                msgs = json.load(f)
                print(f"\033[93m--- Loaded history ({len(msgs)} messages) ---\033[0m")
                return msgs
        except Exception:
            return []
    return []

def save_history(messages):
    with open(HISTORY_FILE, "w") as f:
        json.dump(messages, f, indent=4)

def chat_stream(messages):
    payload = {
        "model": "local",
        "messages": messages,
        "stream": True
    }
    
    full_reply = ""
    try:
        # We use stream=True to handle the SSE (Server-Sent Events)
        response = requests.post(SERVER_URL, json=payload, stream=True)
        response.raise_for_status()

        for line in response.iter_lines():
            if line:
                decoded_line = line.decode('utf-8')
                if not decoded_line.startswith("data: "):
                    continue
                
                data = decoded_line[6:]
                if data == "[DONE]":
                    break
                
                try:
                    chunk = json.loads(data)
                    content = chunk['choices'][0]['delta'].get('content', '')
                    if content:
                        print(content, end='', flush=True)
                        full_reply += content
                except json.JSONDecodeError:
                    continue
                    
        return full_reply
    except Exception as e:
        print(f"\nError: {e}")
        return None

def main():
    messages = load_history()
    # prompt_toolkit provides the 'rustyline' equivalent for Python
    session = PromptSession(history=FileHistory(".yuki_history"))

    print("\033[96mYuki Chat (Python). Type 'exit' to quit or 'clear' to reset.\033[0m")

    while True:
        try:
            user_input = session.prompt("\nYou: ").strip()
            
            if not user_input:
                continue
            if user_input.lower() in ["exit", "quit"]:
                break
            if user_input.lower() == "clear":
                messages = []
                if os.path.exists(HISTORY_FILE):
                    os.remove(HISTORY_FILE)
                print("History cleared.")
                continue

            messages.append({"role": "user", "content": user_input})
            
            print("\nAssistant: ", end='', flush=True)
            reply = chat_stream(messages)
            
            if reply:
                messages.append({"role": "assistant", "content": reply})
                save_history(messages)
            print()

        except (KeyboardInterrupt, EOFError):
            break

if __name__ == "__main__":
    main()
