
import shutil
import time
import sys

class Colors:
    # Omarchy-like palette
    # Dark background assumed
    RESET = '\033[0m'
    BOLD = '\033[1m'
    DIM = '\033[2m'
    
    # Colors based on screenshot
    CYAN = '\033[38;5;39m'    # Headers, borders
    BLUE = '\033[38;5;27m'    # Darker accents
    YELLOW = '\033[38;5;220m' # Highlights, warnings
    GREEN = '\033[38;5;82m'   # Success, low bars
    RED = '\033[38;5;196m'    # Errors, high bars
    WHITE = '\033[38;5;255m'  # Main text
    GREY = '\033[38;5;240m'   # Subtle text
    
    # Backgrounds
    BG_BLUE = '\033[48;5;17m'

class Box:
    TL = '┌'
    TR = '┐'
    BL = '└'
    BR = '┘'
    H = '─'
    V = '│'
    T_DOWN = '┬'
    T_UP = '┴'
    T_RIGHT = '├'
    T_LEFT = '┤'
    CROSS = '┼'

def get_width():
    return shutil.get_terminal_size((80, 24)).columns

def draw_bar(percentage, width=20):
    """Draws a progress bar like the screenshot"""
    fill_chars = "▏▎▍▌▋▊▉█"
    filled_len = int(percentage * width)
    bar = ""
    
    # Gradient colors for bar
    for i in range(width):
        if i < filled_len:
            if i < width * 0.6:
                color = Colors.GREEN
            elif i < width * 0.8:
                color = Colors.YELLOW
            else:
                color = Colors.RED
            bar += f"{color}█{Colors.RESET}"
        else:
            bar += f"{Colors.GREY}░{Colors.RESET}"
            
    return bar

def print_panel(title, content, border_color=Colors.CYAN):
    w = get_width() - 2 # Padding
    
    # Top border
    print(f"{border_color}{Box.TL}{Box.H}{title} {Box.H * (w - len(title) - 2)}{Box.TR}{Colors.RESET}")
    
    # Content
    for line in content.split('\n'):
        # Wrap line if too long (simple wrapping)
        while len(line) > w - 2:
            chunk = line[:w-2]
            line = line[w-2:]
            print(f"{border_color}{Box.V}{Colors.RESET} {chunk} {border_color}{Box.V}{Colors.RESET}")
        
        if line:
             print(f"{border_color}{Box.V}{Colors.RESET} {line:<{w-4}} {border_color}{Box.V}{Colors.RESET}")
        
    # Bottom border
    print(f"{border_color}{Box.BL}{Box.H * w}{Box.BR}{Colors.RESET}")

def demo():
    print(f"{Colors.RESET}")
    
    # Top Status Bar
    w = get_width()
    timestamp = "12:46:04"
    header = f" cpu {Box.H} menu {Box.H} preset "
    right_header = f" BAT 100% {Box.H * 5} "
    
    spacer = w - len(header) - len(right_header) - len(timestamp) - 2
    if spacer < 0: spacer = 0
    print(f"{Colors.CYAN}{header}{Colors.RESET}{timestamp}{Colors.CYAN}{' ' * spacer}{right_header}{Colors.RESET}")
    
    print("\n")
    
    # Chat Message Panel
    msg = "Hello! I want to make the ux of this better and I want the theme to be similar to how the task manager looks on omarchy"
    print_panel("USER", msg, Colors.BLUE)
    
    print("\n")
    
    # Assistant Response
    resp = "I can certainly help with that. We can update the color palette to use deep blues and cyans, implement box-drawing characters for borders, and add status bars."
    print_panel("ASSISTANT", resp, Colors.GREEN)
    
    print("\n")
    
    # Stats Demo
    print(f"{Colors.CYAN}{Box.TL} system {Box.H * 10}{Box.TR}{Colors.RESET}")
    print(f"{Colors.CYAN}{Box.V}{Colors.RESET} CPU Usage: {draw_bar(0.45)} 45% {Colors.CYAN}{Box.V}{Colors.RESET}")
    print(f"{Colors.CYAN}{Box.V}{Colors.RESET} Memory:    {draw_bar(0.72)} 72% {Colors.CYAN}{Box.V}{Colors.RESET}")
    print(f"{Colors.CYAN}{Box.BL}{Box.H * 30}{Box.BR}{Colors.RESET}")

if __name__ == "__main__":
    demo()
