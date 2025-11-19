#!/usr/bin/env python3
"""
Universal CLI Chat Tool
Works with: TabbyAPI, SGLang, Ollama, vLLM, or any OpenAI-compatible API
"""

import requests
import json
from datetime import datetime
from pathlib import Path
import shutil
import sys

# ========== CONFIGURATION ==========
# Change these based on which backend you're using:

BACKENDS = {
    "tabbyapi": {
        "url": "http://localhost:5000/v1",
        "model": "default"
    },
    "sglang": {
        "url": "http://localhost:30000/v1",
        "model": "default"
    },
    "ollama": {
        "url": "http://localhost:11434/v1",
        "model": "qwen2.5:1.5b"
    },
    "vllm": {
        "url": "http://localhost:8000/v1",
        "model": "default"
    }
}

# Select your backend here
ACTIVE_BACKEND = "tabbyapi"  # Change to: tabbyapi, sglang, ollama, or vllm

API_URL = BACKENDS[ACTIVE_BACKEND]["url"]
MODEL_NAME = BACKENDS[ACTIVE_BACKEND]["model"]
HISTORY_DIR = Path.home() / ".chat_history"
HISTORY_DIR.mkdir(exist_ok=True)

# ========== OMARCHY AESTHETIC ==========
class Colors:
    RESET = '\033[0m'
    BOLD = '\033[1m'
    DIM = '\033[2m'
    
    # Omarchy Palette
    CYAN = '\033[38;5;39m'    # Headers, borders
    BLUE = '\033[38;5;27m'    # Darker accents
    YELLOW = '\033[38;5;220m' # Highlights, warnings
    GREEN = '\033[38;5;82m'   # Success, low bars
    RED = '\033[38;5;196m'    # Errors, high bars
    WHITE = '\033[38;5;255m'  # Main text
    GREY = '\033[38;5;240m'   # Subtle text
    
    # Aliases for compatibility
    HEADER = CYAN
    ACCENT = BLUE
    USER = BLUE
    ASSISTANT = GREEN
    SUCCESS = GREEN
    WARNING = YELLOW
    ERROR = RED
    BG_HEADER = '' # Not used in this theme

class Box:
    TL = '┌'; TR = '┐'; BL = '└'; BR = '┘'
    H = '─'; V = '│'
    T_DOWN = '┬'; T_UP = '┴'
    T_RIGHT = '├'; T_LEFT = '┤'
    CROSS = '┼'

def get_terminal_width():
    return shutil.get_terminal_size((80, 20)).columns

def draw_separator(char='─'):
    return f"{Colors.DIM}{char * get_terminal_width()}{Colors.RESET}"

def draw_bar(percentage, width=20):
    """Draws a gradient progress bar"""
    filled_len = int(percentage * width)
    bar = ""
    for i in range(width):
        if i < filled_len:
            if i < width * 0.6: color = Colors.GREEN
            elif i < width * 0.8: color = Colors.YELLOW
            else: color = Colors.RED
            bar += f"{color}█{Colors.RESET}"
        else:
            bar += f"{Colors.GREY}░{Colors.RESET}"
    return bar

def print_header():
    """Print Omarchy-style header"""
    width = get_terminal_width()
    
    # Top Bar
    left = f" {Box.TL} UNIVERSAL CLI CHAT {Box.TR} "
    right = f" {Box.TL} {ACTIVE_BACKEND.upper()} {Box.TR} "
    
    spacer_len = width - len(left) - len(right) - 2 # -2 for padding
    if spacer_len < 0: spacer_len = 0
    
    print(f"\n{Colors.CYAN}{left}{Colors.DIM}{Box.H * spacer_len}{Colors.CYAN}{right}{Colors.RESET}\n")
    
    # Commands
    commands = [
        ("/help", "Help"),
        ("/save", "Save"),
        ("/load", "Load"),
        ("/clear", "Clear"),
        ("/stats", "Stats"),
        ("/quit", "Exit")
    ]
    
    cmd_str = ""
    for cmd, desc in commands:
        cmd_str += f"{Colors.CYAN}[{Colors.RESET} {cmd} {Colors.DIM}{desc}{Colors.RESET} {Colors.CYAN}]{Colors.RESET}  "
    
    print(f" {cmd_str}")
    print(f"\n{Colors.CYAN}{Box.H * width}{Colors.RESET}\n")

class ChatSession:
    """Universal chat session - works with any OpenAI-compatible API"""
    
    def __init__(self, system_prompt=None):
        self.conversation = []
        self.system_prompt = system_prompt or "You are a helpful AI assistant."
        self.session_start = datetime.now()
        self.message_count = 0
        
        # Test connection
        print(f"{Colors.DIM}Connecting to {ACTIVE_BACKEND}...{Colors.RESET}")
        
        try:
            # Test if API is responding
            response = requests.get(
                f"{API_URL.replace('/v1', '')}/health" 
                if "/v1" in API_URL else API_URL,
                timeout=2
            )
            print(f"{Colors.SUCCESS}✓ Connected{Colors.RESET} {Colors.DIM}│ {API_URL}{Colors.RESET}")
            print(f"{Colors.DIM}  Model: {MODEL_NAME}{Colors.RESET}\n")
        except Exception as e:
            # Try alternative health check
            try:
                response = requests.get(f"{API_URL}/models", timeout=2)
                print(f"{Colors.SUCCESS}✓ Connected{Colors.RESET} {Colors.DIM}│ {API_URL}{Colors.RESET}\n")
            except:
                print(f"{Colors.WARNING}⚠ Cannot verify connection{Colors.RESET}")
                print(f"{Colors.DIM}  Will attempt to use anyway...{Colors.RESET}\n")
    
    def send_message(self, message):
        """Send message via OpenAI-compatible API with streaming"""
        try:
            # Print User Message in Box
            width = get_terminal_width() - 4
            print(f"{Colors.BLUE}{Box.TL}{Box.H} USER {Box.H * (width - 6)}{Box.TR}{Colors.RESET}")
            
            # Wrap user message
            for line in message.split('\n'):
                while len(line) > width - 2:
                    chunk = line[:width-2]
                    line = line[width-2:]
                    print(f"{Colors.BLUE}{Box.V}{Colors.RESET} {chunk} {Colors.BLUE}{Box.V}{Colors.RESET}")
                print(f"{Colors.BLUE}{Box.V}{Colors.RESET} {line:<{width-2}} {Colors.BLUE}{Box.V}{Colors.RESET}")
                
            print(f"{Colors.BLUE}{Box.BL}{Box.H * width}{Box.BR}{Colors.RESET}\n")

            # Build messages
            messages = [{"role": "system", "content": self.system_prompt}]
            messages.extend(self.conversation)
            messages.append({"role": "user", "content": message})
            
            # Start Assistant Box
            print(f"{Colors.GREEN}{Box.TL}{Box.H} ASSISTANT {Box.H * (width - 11)}{Box.TR}{Colors.RESET}")
            print(f"{Colors.GREEN}{Box.V}{Colors.RESET} ", end='', flush=True)
            
            # Make API request with streaming
            response = requests.post(
                f"{API_URL}/chat/completions",
                json={
                    "model": MODEL_NAME,
                    "messages": messages,
                    "max_tokens": 1000,
                    "temperature": 0.7,
                    "stream": True
                },
                headers={"Content-Type": "application/json"},
                timeout=120,
                stream=True
            )
            
            if response.status_code != 200:
                print(f"\n{Colors.RED}Error: API returned status {response.status_code}{Colors.RESET}")
                return "Error"
            
            assistant_msg = ""
            line_buffer = ""
            
            for line in response.iter_lines():
                if line:
                    line = line.decode('utf-8')
                    if line.startswith("data: "):
                        data_str = line[6:]
                        if data_str == "[DONE]":
                            break
                        try:
                            data = json.loads(data_str)
                            if "choices" in data and len(data["choices"]) > 0:
                                delta = data["choices"][0].get("delta", {})
                                if "content" in delta:
                                    content = delta["content"]
                                    assistant_msg += content
                                    
                                    # Handle printing with basic wrapping/newlines
                                    # This is a simplified stream printer that doesn't perfectly box right-side
                                    # because we can't predict line breaks easily in a stream without buffering.
                                    # Compromise: Print content, handle newlines by re-printing left border.
                                    
                                    for char in content:
                                        if char == '\n':
                                            print(f"{Colors.GREEN}{Box.V}{Colors.RESET}") # Close previous line visually (sort of)
                                            print(f"{Colors.GREEN}{Box.V}{Colors.RESET} ", end='', flush=True)
                                        else:
                                            print(char, end='', flush=True)
                                            
                        except json.JSONDecodeError:
                            pass
            
            # Close Assistant Box
            print(f"\n{Colors.GREEN}{Box.BL}{Box.H * width}{Box.BR}{Colors.RESET}\n")
            
            # Save to history
            self.conversation.append({"role": "user", "content": message})
            self.conversation.append({"role": "assistant", "content": assistant_msg})
            self.message_count += 1
            
            return assistant_msg
            
        except Exception as e:
            print(f"\n{Colors.RED}Error: {str(e)}{Colors.RESET}")
            return f"Error: {str(e)}"
    
    def clear_conversation(self):
        """Clear conversation history"""
        self.conversation = []
        self.message_count = 0
        print(f"\n{Colors.SUCCESS}✓ Conversation cleared{Colors.RESET}\n")
    
    def save_conversation(self, filename=None):
        """Save conversation to file"""
        if not self.conversation:
            print(f"{Colors.WARNING}⚠ No conversation to save{Colors.RESET}\n")
            return
        
        if filename is None:
            timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
            filename = f"chat_{timestamp}.json"
        
        filepath = HISTORY_DIR / filename
        
        data = {
            "backend": ACTIVE_BACKEND,
            "system_prompt": self.system_prompt,
            "conversation": self.conversation,
            "session_start": self.session_start.isoformat(),
            "saved_at": datetime.now().isoformat(),
            "message_count": self.message_count
        }
        
        with open(filepath, 'w') as f:
            json.dump(data, f, indent=2)
        
        size = filepath.stat().st_size / 1024
        print(f"\n{Colors.SUCCESS}✓ Saved{Colors.RESET} {Colors.DIM}│ {filepath.name}{Colors.RESET}")
        print(f"{Colors.DIM}  └─ {self.message_count} messages, {size:.1f}KB{Colors.RESET}\n")
    
    def load_conversation(self, filename):
        """Load conversation from file"""
        filepath = HISTORY_DIR / filename
        
        if not filepath.exists():
            print(f"{Colors.ERROR}✗ File not found{Colors.RESET} {Colors.DIM}│ {filename}{Colors.RESET}\n")
            return
        
        with open(filepath, 'r') as f:
            data = json.load(f)
        
        self.system_prompt = data.get("system_prompt", self.system_prompt)
        self.conversation = data.get("conversation", [])
        self.message_count = len(self.conversation) // 2
        
        backend = data.get("backend", "unknown")
        print(f"\n{Colors.SUCCESS}✓ Loaded{Colors.RESET} {Colors.DIM}│ {filename}{Colors.RESET}")
        print(f"{Colors.DIM}  └─ {self.message_count} messages (saved from {backend}){Colors.RESET}\n")
    
    def list_saved_conversations(self):
        """List all saved conversations"""
        files = sorted(HISTORY_DIR.glob("chat_*.json"), reverse=True)
        
        if not files:
            print(f"{Colors.WARNING}No saved conversations{Colors.RESET}\n")
            return
        
        print(f"\n{Colors.HEADER}{Colors.BOLD}Saved Conversations{Colors.RESET}")
        print(draw_separator())
        
        for i, file in enumerate(files[:10], 1):
            size = file.stat().st_size / 1024
            mtime = datetime.fromtimestamp(file.stat().st_mtime)
            print(f"{Colors.ACCENT}{i:2d}.{Colors.RESET} {file.name}")
            print(f"{Colors.DIM}    {mtime.strftime('%Y-%m-%d %H:%M')} │ {size:.1f}KB{Colors.RESET}")
        
        print()
    
    def show_stats(self):
        """Show conversation statistics with Omarchy style"""
        if not self.conversation:
            print(f"{Colors.WARNING}No conversation data{Colors.RESET}\n")
            return
        
        user_msgs = [t for t in self.conversation if t["role"] == "user"]
        assistant_msgs = [t for t in self.conversation if t["role"] == "assistant"]
        
        total_user_chars = sum(len(m["content"]) for m in user_msgs)
        total_assistant_chars = sum(len(m["content"]) for m in assistant_msgs)
        total_chars = total_user_chars + total_assistant_chars
        
        duration = datetime.now() - self.session_start
        minutes = duration.seconds // 60
        
        width = get_terminal_width() - 4
        
        print(f"{Colors.CYAN}{Box.TL}{Box.H} SESSION STATISTICS {Box.H * (width - 20)}{Box.TR}{Colors.RESET}")
        
        # Helper to print a stat line
        def print_stat(label, value, bar_percent=None):
            val_str = str(value)
            if bar_percent is not None:
                bar = draw_bar(bar_percent, width=20)
                content = f"{label:15s} {bar} {val_str}"
            else:
                content = f"{label:15s} {val_str}"
                
            print(f"{Colors.CYAN}{Box.V}{Colors.RESET} {content:<{width-2}} {Colors.CYAN}{Box.V}{Colors.RESET}")

        print_stat("Backend", ACTIVE_BACKEND.upper())
        print_stat("Model", MODEL_NAME)
        print(f"{Colors.CYAN}{Box.T_RIGHT}{Box.H * width}{Box.T_LEFT}{Colors.RESET}")
        
        print_stat("Messages", len(self.conversation))
        print_stat("Duration", f"{minutes} min")
        
        # Calculate percentages for bars (arbitrary scaling for demo)
        # Assuming 100 messages is "full" for the bar
        msg_cap = 50
        char_cap = 10000
        
        print(f"{Colors.CYAN}{Box.T_RIGHT}{Box.H * width}{Box.T_LEFT}{Colors.RESET}")
        print_stat("User Msgs", len(user_msgs), min(len(user_msgs)/msg_cap, 1.0))
        print_stat("Asst Msgs", len(assistant_msgs), min(len(assistant_msgs)/msg_cap, 1.0))
        print_stat("User Chars", total_user_chars, min(total_user_chars/char_cap, 1.0))
        print_stat("Asst Chars", total_assistant_chars, min(total_assistant_chars/char_cap, 1.0))
        
        print(f"{Colors.CYAN}{Box.BL}{Box.H * width}{Box.BR}{Colors.RESET}\n")

def main():
    """Main chat loop"""
    print_header()
    
    session = ChatSession()
    
    while True:
        try:
            # Prompt
            print(f"{Colors.CYAN}[{Colors.RESET} USER {Colors.CYAN}]{Colors.RESET} ", end='')
            user_input = input().strip()
            
            if not user_input:
                continue
            
            # Handle commands
            if user_input.startswith('/'):
                command = user_input.lower().split()[0]
                
                if command in ['/quit', '/exit', '/q']:
                    print(f"\n{Colors.DIM}Session ended. Goodbye!{Colors.END}\n")
                    break
                
                elif command in ['/help', '/h']:
                    print_header()
                
                elif command == '/clear':
                    session.clear_conversation()
                
                elif command == '/save':
                    session.save_conversation()
                
                elif command == '/load':
                    session.list_saved_conversations()
                    filename = input(f"{Colors.DIM}Enter filename (or press Enter to cancel): {Colors.END}").strip()
                    if filename:
                        session.load_conversation(filename)
                
                elif command == '/stats':
                    session.show_stats()
                
                elif command == '/system':
                    print(f"\n{Colors.DIM}Current: {session.system_prompt}{Colors.END}")
                    new_prompt = input(f"{Colors.DIM}New prompt (or Enter to cancel): {Colors.END}").strip()
                    if new_prompt:
                        session.system_prompt = new_prompt
                        session.clear_conversation()
                        print(f"{Colors.SUCCESS}✓ System prompt updated{Colors.END}\n")
                
                else:
                    print(f"{Colors.ERROR}Unknown command: {command}{Colors.END}\n")
                
                continue
            
            # Send message and display response
            print()
            # Response is handled inside send_message now due to streaming
            session.send_message(user_input)
            # print(f"\n{draw_separator()}\n") # Separator not needed with boxes
        
        except KeyboardInterrupt:
            print(f"\n{Colors.DIM}Press Ctrl+C again to exit{Colors.END}\n")
        
        except EOFError:
            print(f"\n{Colors.DIM}Session ended. Goodbye!{Colors.END}\n")
            break
        
        except Exception as e:
            print(f"{Colors.ERROR}✗ Error: {str(e)}{Colors.END}\n")

if __name__ == "__main__":
    main()
