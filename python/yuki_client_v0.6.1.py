import os
import sys
import json
import time
import requests
from pathlib import Path
from colorama import init, Fore, Style
from prompt_toolkit import PromptSession
from prompt_toolkit.completion import Completer, Completion
from prompt_toolkit.shortcuts import CompleteStyle

# Initialize Windows ANSI support
init(autoreset=True)

SERVER_URL = "http://127.0.0.1:8080/v1/chat/completions"

def get_root_path():
    # Windows equivalent of ~/yuki_client
    path = Path.home() / "yuki_client"
    return path

# --- ADVANCED COMPLETION LOGIC ---
class YukiCompleter(Completer):
    def get_completions(self, document, complete_event):
        text = document.text_before_cursor
        root = get_root_path()

        # 1. Path Completion for /read
        if text.startswith("/read "):
            path_str = text[6:]
            expanded = os.path.expanduser(path_str)
            dirname = os.path.dirname(expanded) or "."
            basename = os.path.basename(expanded)

            try:
                if os.path.isdir(dirname):
                    for entry in os.listdir(dirname):
                        if entry.startswith(basename):
                            full_entry = os.path.join(dirname, entry)
                            display = entry + "/" if os.path.isdir(full_entry) else entry
                            yield Completion(entry[len(basename):], start_position=0, display=display)
            except Exception:
                pass

        # 2. Chat Name Completion for /load and Startup
        elif not text.startswith("/"):
            search = text if not text.startswith("/load ") else text[6:]
            hist_dir = root / "history"
            if hist_dir.exists():
                for f in hist_dir.glob("history_*.json"):
                    name = f.stem[8:] # Remove 'history_'
                    if name.startswith(search):
                        yield Completion(name[len(search):], start_position=0)

# --- LOGIC HELPERS ---

def ensure_dirs():
    root = get_root_path()
    (root / "history").mkdir(parents=True, exist_ok=True)
    (root / "chats").mkdir(parents=True, exist_ok=True)
    (root / "backups").mkdir(parents=True, exist_ok=True)

def get_file_paths(chat_name):
    root = get_root_path()
    hp = root / "history" / f"history_{chat_name}.json"
    sp = root / "chats" / f"summary_{chat_name}.json"
    return hp, sp

def create_backup(chat_name, hist_path):
    if hist_path.exists():
        ts = int(time.time())
        backup_path = get_root_path() / "backups" / f"log_{chat_name}_{ts}.json"
        try:
            import shutil
            shutil.copy(hist_path, backup_path)
        except Exception: pass

def list_existing_chats():
    root = get_root_path()
    chats = set()
    # Check both potential sources
    if (root / "history").exists():
        chats.update(f.stem[8:] for f in (root / "history").glob("history_*.json"))
    if (root / "chats").exists():
        chats.update(f.stem[8:] for f in (root / "chats").glob("summary_*.json"))
    
    print(f"{Fore.BLUE}Existing Chats:")
    if not chats:
        print("  (No existing chats found)")
    else:
        for name in sorted(chats):
            print(f"  {Fore.YELLOW}- {name}")
    print()

def chat_request(messages):
    payload = {"model": "local", "messages": messages, "stream": True}
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
            except: continue
        print()
        return full_reply
    except Exception as e:
        print(f"\n{Fore.RED}Error: Server unreachable.")
        return None

# --- MAIN EXECUTION ---

def main():
    ensure_dirs()
    session = PromptSession(completer=YukiCompleter(), complete_style=CompleteStyle.READLINE_LIKE)

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
    print(f"{Fore.CYAN}Yuki Client (Python Native) Started.")
    list_existing_chats()

    # Startup Loop
    current_chat = ""
    while not current_chat:
        try:
            line = session.prompt("Enter Chat Name (or '<name> /delete'): ").strip()
            if not line: continue
            if " /delete" in line:
                target = line.replace(" /delete", "").strip()
                hp, sp = get_file_paths(target)
                if hp.exists(): hp.unlink()
                if sp.exists(): sp.unlink()
                print(f"{Fore.RED}[System] Chat '{target}' deleted.")
                list_existing_chats()
                continue
            current_chat = line
        except (KeyboardInterrupt, EOFError): return

    # Load logic
    hp, sp = get_file_paths(current_chat)
    messages = []

    # 1. Load the Base (Summary)
    if sp.exists():
        try:
            messages = json.loads(sp.read_text(encoding='utf-8'))
            print(f"{Fore.YELLOW}--- Base Summary Loaded ---")
        except Exception as e:
            print(f"{Fore.RED}Error loading summary: {e}")

    # 2. Append the Incremental (History)
    if hp.exists():
        try:
            hist_messages = json.loads(hp.read_text(encoding='utf-8'))
            messages.extend(hist_messages)
            print(f"{Fore.YELLOW}--- Incremental History Appended ---")
        except Exception as e:
            print(f"{Fore.RED}Error loading history: {e}")

    if not messages:
        print(f"{Fore.GREEN}--- Starting new chat: {current_chat} ---")

    print(f"{Fore.BLACK}{Style.BRIGHT}Commands: /exit, /clear, /summarize, /read <path>, /load <name>")

    while True:
        try:
            char_count = sum(len(m['content']) for m in messages)
            prompt = f"\n[{current_chat} | ~{char_count} chars]: "
            user_input = session.prompt(prompt).strip()
            
            if not user_input: continue
            
            if user_input.startswith('/'):
                cmd = user_input.split()[0]
                if cmd in ["/exit", "/quit"]: break
                elif cmd == "/clear":
                    create_backup(current_chat, hp)
                    messages = []
                    # Nuke both files
                    if hp.exists(): hp.unlink()
                    if sp.exists(): sp.unlink()
                    print(f"{Fore.YELLOW}Memory and Summary wiped.")
                elif cmd == "/summarize":
                    if not messages:
                        print(f"{Fore.RED}Nothing to summarize.")
                        continue
                    print(f"{Fore.MAGENTA}[System] Summarizing...")
                    req = messages + [{"role":"user", "content":"Summarize the above conversation into 4 direct technical bullet points."}]
                    reply = chat_request(req)
                    if reply and len(reply) > 50:
                        # Save new base context
                        messages = [{"role":"system", "content": f"PREVIOUS_CONTEXT_SUMMARY:\n{reply}"}]
                        sp.write_text(json.dumps(messages, indent=4), encoding='utf-8')
                        # Delete raw history so it doesn't duplicate on next load
                        if hp.exists(): hp.unlink()
                        print(f"{Fore.GREEN}[System] Conversation compressed to Summary.")
                    else:
                        print(f"{Fore.RED}[System] Summarization failed. History preserved.")
                elif cmd == "/read":
                    path_to_read = user_input.split(None, 1)[1] if len(user_input.split()) > 1 else ""
                    expanded = Path(os.path.expanduser(path_to_read))
                    if expanded.exists():
                        content = expanded.read_text(encoding='utf-8', errors='replace')
                        messages.append({"role":"user", "content": f"Analyze this file content:\n\n{content}"})
                        print(f"{Fore.BLUE}[System] File loaded.")
                        reply = chat_request(messages)
                        if reply:
                            messages.append({"role":"assistant", "content": reply})
                            hp.write_text(json.dumps(messages, indent=4), encoding='utf-8')
                    else: print(f"{Fore.RED}Error: Path not found.")
                elif cmd == "/load":
                    new_name = user_input.split(None, 1)[1] if len(user_input.split()) > 1 else ""
                    if new_name:
                        current_chat = new_name
                        hp, sp = get_file_paths(current_chat)
                        # Re-run load logic (abbreviated here)
                        print(f"{Fore.CYAN}Switched to {current_chat}")
                continue

            # Regular Message
            messages.append({"role": "user", "content": user_input})
            print(f"{Fore.GREEN}Assistant: {Style.RESET_ALL}", end="", flush=True)
            reply = chat_request(messages)
            if reply:
                messages.append({"role": "assistant", "content": reply})
                hp.write_text(json.dumps(messages, indent=4), encoding='utf-8')

        except (KeyboardInterrupt, EOFError): break

if __name__ == "__main__":
    main()