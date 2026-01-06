# Claude Workbench

A Rust-based TUI (Terminal User Interface) multiplexer designed for AI-assisted development workflows. Provides an integrated development environment with file browser, preview pane, and multiple embedded PTY terminals.

## Features

### File Browser (F1)
- Navigate directories with keyboard (j/k, arrows) or mouse
- Git status integration with color-coded indicators:
  - Yellow: Untracked (?)
  - Orange: Modified (M)
  - Green: Staged (+)
  - Gray: Ignored (·)
  - Red: Conflict (!)
- Status bar shows file size, modification date, and git branch info
- Double-click to open files or enter directories

### Preview Pane (F2)
- Syntax highlighting for 500+ languages (via syntect)
- Markdown rendering with formatted display (via tui-markdown)
- Built-in text editor with undo/redo support (via tui-textarea)
- Scrollable preview with keyboard navigation

### Browser Preview (o key)
- **HTML/HTM**: Direct browser opening
- **Markdown**: Converts to styled HTML with dark mode support
- **PDF**: Opens in default PDF viewer
- **Images**: PNG, JPG, GIF, SVG, WebP in system image viewer
- **O (Shift+O)**: Open current directory in Finder/file manager

### Terminal Panes
- **Claude Code (F4)**: Embedded Claude CLI terminal
- **LazyGit (F5)**: Integrated Git TUI
- **User Terminal (F6)**: General-purpose shell

All terminal panes support:
- Full PTY emulation with 256-color support
- Scrollback history (1000 lines by default)
- Mouse wheel scrolling
- Keyboard scrolling (Shift+PgUp/PgDn, Shift+Up/Down)

### Terminal Selection Mode (Ctrl+S)
- Select lines from terminal output
- Copy selected text to Claude pane as code block
- Navigate with j/k or arrow keys
- Press Enter or y to copy

### Drag & Drop
- Drag files from File Browser to terminal panes
- Automatically quotes paths with spaces
- Insert file paths directly into Claude or Terminal

### Additional Features
- **Fuzzy Finder (Ctrl+P)**: Quick file search and navigation
- **Settings Menu (Ctrl+,)**: Configure shell, layout, and more
- **Setup Wizard (Ctrl+Shift+W)**: First-run configuration assistant
- **About Dialog (i)**: License info and open source components

## Installation

### From Source
```bash
# Clone the repository
git clone https://github.com/yourusername/claude-workbench.git
cd claude-workbench

# Build release version
cargo build --release

# Run
./target/release/claude-workbench
```

### Requirements
- Rust 1.70+
- macOS, Linux, or Windows
- Terminal with 256-color support recommended

## Configuration

Configuration is stored in:
1. `./config.yaml` (project-local, highest priority)
2. `~/.config/claude-workbench/config.yaml` (user config)

### Example config.yaml
```yaml
terminal:
  shell_path: "/opt/homebrew/bin/fish"
  shell_args: ["-l"]

ui:
  theme: "default"

layout:
  claude_height_percent: 40
  file_browser_width_percent: 20
  preview_width_percent: 50
  right_panel_width_percent: 30

file_browser:
  show_hidden: false
  show_file_info: true
  date_format: "%d.%m.%Y %H:%M:%S"
  auto_refresh_ms: 2000

# Optional: Claude startup prefixes
# claude:
#   startup_prefixes:
#     - name: "Code Review"
#       prefix: "/review"
#       description: "Review code changes"
```

## Keyboard Shortcuts

### Global
| Key | Action |
|-----|--------|
| Ctrl+Q | Quit |
| Ctrl+P | Fuzzy Finder |
| Ctrl+, | Settings |
| F1-F6 | Switch panes |
| ? | Help |
| i | About |

### File Browser
| Key | Action |
|-----|--------|
| j/k, ↑/↓ | Navigate |
| Enter | Open/Enter |
| Backspace | Parent |
| o | Open in browser |
| O | Open in Finder |

### Editor (Preview Pane)
| Key | Action |
|-----|--------|
| E | Enter edit mode |
| Ctrl+S | Save |
| Ctrl+Z | Undo |
| Ctrl+Y | Redo |
| Esc | Exit editor |

### Terminal Panes
| Key | Action |
|-----|--------|
| Ctrl+S | Start selection |
| Shift+PgUp/PgDn | Scroll 10 lines |
| Shift+↑/↓ | Scroll 1 line |

## Tech Stack

- **[Ratatui](https://github.com/ratatui/ratatui)** - TUI framework
- **[Crossterm](https://github.com/crossterm-rs/crossterm)** - Terminal handling
- **[portable-pty](https://github.com/wez/wezterm)** - PTY management
- **[vt100](https://github.com/doy/vt100-rust)** - Terminal emulation
- **[syntect](https://github.com/trishume/syntect)** - Syntax highlighting
- **[tui-textarea](https://github.com/rhysd/tui-textarea)** - Text editor widget
- **[tui-markdown](https://github.com/joshka/tui-markdown)** - Markdown rendering
- **[pulldown-cmark](https://github.com/pulldown-cmark/pulldown-cmark)** - Markdown to HTML

## License

MIT License - Copyright (c) 2025 Martin Schmid

See [LICENSE](LICENSE) for details.
