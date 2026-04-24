import json
import os
import sys
import requests

SERVER_URL = "http://127.0.0.1:8080/v1/chat/completions"
HISTORY_FILE = "history.json"

def load_history():
    if os.path.exists(HISTORY_FILE):
        try:
            with open(HISTORY_FILE, 'r') as f:
                msgs = json.load(f)
                print(f"\033[93m--- Loaded history ({len(msgs)} messages) ---\033[0m")
                return msgs
        except Exception:
            pass
    return []

def save_history(messages):
    try:
        with open(HISTORY_FILE, 'w') as f:
            json.dump(messages, f, indent=4)
    except Exception as e:
        print(f"Failed to save history: {e}")

def chat(messages):
    payload = {
        "model": "local",
        "messages": messages,
        "stream": True
    }
    
    full_reply = ""
    try:
        # We use stream=True to handle the Server-Sent Events (SSE)
        response = requests.post(SERVER_URL, json=payload, stream=True)
        response.raise_for_status()

        for line in response.iter_lines():
            if not line:
                continue
            
            line = line.decode('utf-8')
            if not line.startswith("data: "):
                continue
            
            data_str = line[6:]
            if data_str == "[DONE]":
                break
                
            try:
                data = json.loads(data_str)
                content = data['choices'][0]['delta'].get('content', '')
                if content:
                    print(content, end='', flush=True)
                    full_reply += content
            except json.JSONDecodeError:
                continue
                
    except Exception as e:
        print(f"\nError during chat: {e}")
        return None
        
    return full_reply

def main():
    messages = load_history()
    print("\033[96mYuki Chat (Python). Type 'exit' to quit or 'clear' to reset.\033[0m")

    while True:
        try:
            user_input = input("\nYou: ").strip()
            
            if not user_input:
                continue
            if user_input.lower() in ['exit', 'quit']:
                break
            
            if user_input.lower() == 'clear':
                messages.clear()
                if os.path.exists(HISTORY_FILE):
                    os.remove(HISTORY_FILE)
                print("History cleared.")
                continue

            messages.append({"role": "user", "content": user_input})

            print("\nAssistant: ", end='', flush=True)
            reply = chat(messages)
            
            if reply is not None:
                messages.append({"role": "assistant", "content": reply})
                save_history(messages)
            print()

        except (KeyboardInterrupt, EOFError):
            break

if __name__ == "__main__":
    main()
