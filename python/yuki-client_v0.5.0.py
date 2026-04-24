import os
import sys
import json
import time
import requests
from colorama import init, Fore, Style

# Initialize Colorama for Windows ANSI support
init(autoreset=True)

# --- CONFIGURATION ---
SERVER_URL = "http://127.0.0.1:8080/v1/chat/completions"
ROOT_DIR = "yuki_client"
HIST_DIR = os.path.join(ROOT_DIR, "history")
CHAT_DIR = os.path.join(ROOT_DIR, "chats")
BACKUP_DIR = os.path.join(ROOT_DIR, "backups")

# --- TAB COMPLETION SETUP ---
try:
    import readline
except ImportError:
    # This will load pyreadline3 on Windows
    import pyreadline3 as readline

class ChatCompleter:
    def __init__(self, options):
        self.options = options

    def complete(self, text, state):
        response = None
        # Refresh options from the directory
        if os.path.exists(HIST_DIR):
            files = os.listdir(HIST_DIR)
            self.options = [f[8:-5] for f in files if f.startswith("history_") and f.endswith(".json")]
        
        matches = [option for option in self.options if option.startswith(text)]
        if state < len(matches):
            response = matches[state]
        return response

# --- LOGIC FUNCTIONS ---

def ensure_dirs():
    for d in [HIST_DIR, CHAT_DIR, BACKUP_DIR]:
        os.makedirs(d, exist_ok=True)

def get_file_paths(chat_name):
    history_path = os.path.join(HIST_DIR, f"history_{chat_name}.json")
    summary_path = os.path.join(CHAT_DIR, f"summary_{chat_name}.json")
    return history_path, summary_path

def create_backup(chat_name, hist_path):
    if os.path.exists(hist_path):
        ts = int(time.time())
        backup_path = os.path.join(BACKUP_DIR, f"log_{chat_name}_{ts}.json")
        try:
            import shutil
            shutil.copy(hist_path, backup_path)
            print(f"{Fore.BLUE}[System] Archive created: {backup_path}")
        except Exception:
            pass

def list_existing_chats():
    print(f"{Fore.BLUE}Existing Chats:")
    found = False
    if os.path.exists(HIST_DIR):
        for filename in os.listdir(HIST_DIR):
            if filename.startswith("history_") and filename.endswith(".json"):
                print(f"  {Fore.YELLOW}- {filename[8:-5]}")
                found = True
    if not found:
        print(f"  (No existing chats found in {HIST_DIR})")
    print()

def load_initial_messages(chat_name):
    hist_path, summ_path = get_file_paths(chat_name)
    
    # Try summary first
    if os.path.exists(summ_path):
        with open(summ_path, 'r', encoding='utf-8') as f:
            msgs = json.load(f)
            print(f"{Fore.YELLOW}--- Loaded summary memory ({len(msgs)} messages) ---")
            return msgs
            
    # Try full history
    if os.path.exists(hist_path):
        with open(hist_path, 'r', encoding='utf-8') as f:
            msgs = json.load(f)
            print(f"{Fore.YELLOW}--- Loaded history ({len(msgs)} messages) ---")
            return msgs
            
    print(f"{Fore.GREEN}--- Starting new chat: {chat_name} ---")
    return []

def chat_request(messages, is_summary=False):
    payload = {
        "model": "local",
        "messages": messages,
        "stream": True
    }
    
    full_reply = ""
    try:
        response = requests.post(SERVER_URL, json=payload, stream=True, timeout=60)
        response.raise_for_status()

        for line in response.iter_lines():
            if not line: continue
            line = line.decode('utf-8')
            if not line.startswith("data: "): continue
            
            data_str = line[6:]
            if data_str == "[DONE]": break
            
            try:
                data = json.loads(data_str)
                content = data['choices'][0]['delta'].get('content', '')
                if content:
                    print(content, end='', flush=True)
                    full_reply += content
            except json.JSONDecodeError:
                continue
        print()
        return full_reply
    except Exception as e:
        print(f"\n{Fore.RED}Error: Server unreachable ({e})")
        return None

def main():
    ensure_dirs()
    
    logo = r"""
 █████ █████ █████  █████ █████   ████ █████
▒▒███ ▒▒███ ▒▒███  ▒▒███ ▒▒███   ███▒ ▒▒███ 
 ▒▒███ ███   ▒███   ▒███  ▒███  ███    ▒███ 
  ▒▒█████    ▒███   ▒███  ▒███████     ▒███ 
   ▒▒███     ▒███   ▒███  ▒███▒▒███    ▒███ 
    ▒███     ▒███   ▒███  ▒███ ▒▒███   ▒███ 
    █████    ▒▒████████   █████ ▒▒████ █████
   ▒▒▒▒▒      ▒▒▒▒▒▒▒▒   ▒▒▒▒▒   ▒▒▒▒ ▒▒▒▒▒ 
    """
    print(Fore.CYAN + logo)
    print(f"{Fore.CYAN}Yuki Client (Python) Started.")
    print(f"{Fore.WHITE}Data Folder: {ROOT_DIR}\n")
    
    list_existing_chats()

    # Set up Autocomplete
    completer = ChatCompleter([])
    readline.set_completer(completer.complete)
    readline.parse_and_bind("tab: complete")

    try:
        chat_name = input("Enter Chat Name (Tab): ").strip()
        if not chat_name: return
    except (KeyboardInterrupt, EOFError): return

    # Turn off autocomplete for the main chat
    readline.set_completer(None)

    messages = load_initial_messages(chat_name)
    hist_path, summ_path = get_file_paths(chat_name)
    
    print(f"{Fore.WHITE}Commands: 'exit', 'clear', 'summarize'")

    while True:
        try:
            char_count = sum(len(m['content']) for m in messages)
            prompt = f"\n[{chat_name} | ~{char_count} chars]: "
            user_input = input(prompt).strip()
            
            if not user_input: continue
            if user_input in ['exit', 'quit']: break
            
            if user_input == "clear":
                create_backup(chat_name, hist_path)
                messages = []
                if os.path.exists(hist_path): os.remove(hist_path)
                if os.path.exists(summ_path): os.remove(summ_path)
                print(f"{Fore.RED}Memory wiped. History archived to {ROOT_DIR}/backups")
                continue

            if user_input == "summarize":
                create_backup(chat_name, hist_path)
                print(f"{Fore.MAGENTA}\n[System] Compressing memory...")
                summary_req = messages + [{"role": "user", "content": "Summarize our conversation into 4 bullet points for your memory."}]
                
                print(f"{Fore.GREEN}Assistant (Summarizing):{Style.RESET_ALL} ", end="")
                reply = chat_request(summary_req)
                if reply:
                    messages = [{"role": "assistant", "content": f"MEMORY_BLOCK:\n{reply}"}]
                    with open(summ_path, 'w', encoding='utf-8') as f:
                        json.dump(messages, f, indent=4)
                    if os.path.exists(hist_path): os.remove(hist_path)
                    print(f"{Fore.GREEN}[Done] Summary saved.")
                continue

            # Standard Chat
            messages.append({"role": "user", "content": user_input})
            print(f"{Fore.GREEN}Assistant:{Style.RESET_ALL} ", end="", flush=True)
            
            reply = chat_request(messages)
            if reply:
                messages.append({"role": "assistant", "content": reply})
                with open(hist_path, 'w', encoding='utf-8') as f:
                    json.dump(messages, f, indent=4)

        except (KeyboardInterrupt, EOFError):
            break

if __name__ == "__main__":
    main()
