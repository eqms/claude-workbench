# Claude Workbench

<p align="center">
  <img src="docs/Claude_Workbench.png" alt="Claude Workbench - Rust-based TUI Multiplexer for AI-Assisted Development" width="800">
</p>

**[English](#english) | [Deutsch](#deutsch)**

---

<a name="english"></a>
## English

A Rust-based TUI (Terminal User Interface) multiplexer designed for AI-assisted development workflows. Provides an integrated development environment with file browser, syntax-highlighted preview pane, and multiple embedded PTY terminals.

![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)
![Platforms](https://img.shields.io/badge/platforms-Linux%20|%20macOS%20|%20Windows-green.svg)

### Features

| Pane | Key | Description |
|------|-----|-------------|
| **File Browser** | F1 | Navigate directories, git status integration, file operations (F9), toggle visibility |
| **Preview** | F2 | Syntax highlighting (500+ languages), Markdown rendering, built-in editor |
| **Claude Code** | F4 | Embedded Claude CLI terminal with startup prefixes |
| **LazyGit** | F5 | Integrated Git TUI (restarts in current directory) |
| **Terminal** | F6 | General-purpose shell (syncs to current directory) |

**Highlights:**
- Full PTY emulation with 256-color support and 1000-line scrollback
- Search & Replace (MC Edit style) with regex support
- **Character-level mouse selection** - click and drag to select text, auto-copies to clipboard
- Keyboard selection mode (Ctrl+S) with intelligent filtering
- Drag & Drop files into terminals
- Git remote change detection with pull prompts
- Claude fullscreen mode when all panes hidden (F1/F2/F5/F6 toggles)
- **Interactive pane resizing** - drag pane borders with mouse
- **Horizontal scrolling** - in Preview and Edit mode for long lines
- **Self-update** - automatic update check from GitHub Releases
- Mouse and keyboard navigation throughout

### Quick Start

```bash
# Download for your platform from Releases
# https://github.com/eqms/claude-workbench/releases

# Or build from source
git clone https://github.com/eqms/claude-workbench.git
cd claude-workbench
cargo build --release
./target/release/claude-workbench
```

**See [INSTALL.md](INSTALL.md) for detailed platform-specific installation instructions.**

### Essential Shortcuts

| Key | Action |
|-----|--------|
| F1-F6 | Switch between panes |
| F9 | File menu (new, rename, delete, etc.) |
| F12 | Help (full shortcut reference) |
| Ctrl+P | Fuzzy file finder |
| Ctrl+Q | Quit |
| E | Edit file (in Preview) |
| Ctrl+S | Selection mode (in Terminal/Preview) |
| Ctrl+C | Copy selection to System Clipboard |

**See [USAGE.md](USAGE.md) for complete keyboard shortcuts and detailed usage guide.**

### Configuration

Configuration files are loaded in priority order:
1. `./config.yaml` (project-local, highest priority)
2. `~/.config/claude-workbench/config.yaml` (user config)

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
claude:
  startup_prefixes:
    - name: "Code Review"
      prefix: "/review"
      description: "Review code changes"
```

### Tech Stack

- **[Ratatui](https://github.com/ratatui/ratatui)** - TUI framework
- **[Crossterm](https://github.com/crossterm-rs/crossterm)** - Terminal handling
- **[portable-pty](https://github.com/wez/wezterm)** - PTY management
- **[vt100](https://github.com/doy/vt100-rust)** - Terminal emulation
- **[syntect](https://github.com/trishume/syntect)** - Syntax highlighting
- **[tui-textarea](https://github.com/rhysd/tui-textarea)** - Text editor widget
- **[tui-markdown](https://github.com/joshka/tui-markdown)** - Markdown rendering

### License

MIT License - Copyright (c) 2025 Martin Schmid

See [LICENSE](LICENSE) for details.

---

<a name="deutsch"></a>
## Deutsch

Ein Rust-basierter TUI (Terminal User Interface) Multiplexer für KI-unterstützte Entwicklungsworkflows. Bietet eine integrierte Entwicklungsumgebung mit Dateibrowser, Syntax-hervorgehobener Vorschau und mehreren eingebetteten PTY-Terminals.

### Funktionen

| Bereich | Taste | Beschreibung |
|---------|-------|--------------|
| **Dateibrowser** | F1 | Verzeichnisnavigation, Git-Status-Integration, Dateioperationen (F9), ein-/ausblenden |
| **Vorschau** | F2 | Syntax-Hervorhebung (500+ Sprachen), Markdown-Rendering, Editor |
| **Claude Code** | F4 | Eingebettetes Claude CLI Terminal mit Startup-Präfixen |
| **LazyGit** | F5 | Integrierte Git-TUI (startet im aktuellen Verzeichnis neu) |
| **Terminal** | F6 | Allgemeines Shell-Terminal (wechselt ins aktuelle Verzeichnis) |

**Highlights:**
- Volle PTY-Emulation mit 256-Farben und 1000 Zeilen Scrollback
- Suchen & Ersetzen (MC Edit Stil) mit Regex-Unterstützung
- **Zeichenweise Mausauswahl** - Klicken und Ziehen zum Markieren, kopiert automatisch ins Clipboard
- Tastatur-Auswahlmodus (Ctrl+S) mit intelligentem Filtering
- Drag & Drop von Dateien in Terminals
- Git Remote-Änderungserkennung mit Pull-Aufforderung
- Claude Vollbildmodus wenn alle Bereiche ausgeblendet (F1/F2/F5/F6 Umschaltung)
- **Interaktives Pane-Resizing** - Bereichsgrenzen per Maus ziehen
- **Horizontales Scrollen** - in Vorschau und Editor für lange Zeilen
- **Selbst-Update** - automatische Update-Prüfung von GitHub Releases
- Maus- und Tastaturnavigation durchgehend

### Schnellstart

```bash
# Binary für Ihre Plattform von Releases herunterladen
# https://github.com/eqms/claude-workbench/releases

# Oder aus Quellcode bauen
git clone https://github.com/eqms/claude-workbench.git
cd claude-workbench
cargo build --release
./target/release/claude-workbench
```

**Siehe [INSTALL.md](INSTALL.md) für detaillierte plattformspezifische Installationsanleitungen.**

### Wichtige Tastenkürzel

| Taste | Aktion |
|-------|--------|
| F1-F6 | Zwischen Bereichen wechseln |
| F9 | Datei-Menü (neu, umbenennen, löschen, etc.) |
| F12 | Hilfe (vollständige Shortcut-Referenz) |
| Ctrl+P | Fuzzy-Dateisuche |
| Ctrl+Q | Beenden |
| E | Datei bearbeiten (in Vorschau) |
| Ctrl+S | Auswahlmodus (in Terminal/Vorschau) |
| Ctrl+C | Auswahl in System-Clipboard kopieren |

**Siehe [USAGE.md](USAGE.md) für alle Tastenkürzel und detaillierte Bedienungsanleitung.**

### Konfiguration

Konfigurationsdateien werden in Prioritätsreihenfolge geladen:
1. `./config.yaml` (projektlokal, höchste Priorität)
2. `~/.config/claude-workbench/config.yaml` (Benutzerkonfiguration)

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

# Optional: Claude Startup-Präfixe
claude:
  startup_prefixes:
    - name: "Code Review"
      prefix: "/review"
      description: "Code-Änderungen überprüfen"
```

### Technologie-Stack

- **[Ratatui](https://github.com/ratatui/ratatui)** - TUI-Framework
- **[Crossterm](https://github.com/crossterm-rs/crossterm)** - Terminal-Handhabung
- **[portable-pty](https://github.com/wez/wezterm)** - PTY-Verwaltung
- **[vt100](https://github.com/doy/vt100-rust)** - Terminal-Emulation
- **[syntect](https://github.com/trishume/syntect)** - Syntax-Hervorhebung
- **[tui-textarea](https://github.com/rhysd/tui-textarea)** - Texteditor-Widget
- **[tui-markdown](https://github.com/joshka/tui-markdown)** - Markdown-Rendering

### Lizenz

MIT-Lizenz - Copyright (c) 2025 Martin Schmid

Siehe [LICENSE](LICENSE) für Details.
