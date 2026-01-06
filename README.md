# Claude Workbench

**[English](#english) | [Deutsch](#deutsch)**

---

<a name="english"></a>
## English

A Rust-based TUI (Terminal User Interface) multiplexer designed for AI-assisted development workflows. Provides an integrated development environment with file browser, syntax-highlighted preview pane, and multiple embedded PTY terminals.

### Features

#### File Browser (F1)
- Navigate directories with keyboard (j/k, arrows) or mouse
- Git status integration with color-coded indicators:
  - Yellow: Untracked (?)
  - Orange: Modified (M)
  - Green: Staged (+)
  - Gray: Ignored (·)
  - Red: Conflict (!)
- Status bar shows file size, modification date, and git branch info
- Double-click to open files or enter directories
- Context menu (F9) for file operations: New, Rename, Delete, Copy Path

#### Preview Pane (F2)
- Syntax highlighting for 500+ languages (via syntect)
- Markdown rendering with formatted display (via tui-markdown)
- Built-in text editor with undo/redo support (via tui-textarea)
- Scrollable preview with keyboard and mouse navigation
- PageUp/PageDown, Home/End for quick navigation

#### Browser Preview (o key)
- **HTML/HTM**: Direct browser opening
- **Markdown**: Converts to styled HTML with dark mode support
- **PDF**: Opens in default PDF viewer
- **Images**: PNG, JPG, GIF, SVG, WebP in system image viewer
- **O (Shift+O)**: Open current directory in Finder/file manager

#### Terminal Panes
- **Claude Code (F4)**: Embedded Claude CLI terminal with optional startup prefixes
- **LazyGit (F5)**: Integrated Git TUI for version control
- **User Terminal (F6)**: General-purpose shell

All terminal panes support:
- Full PTY emulation with 256-color support
- Scrollback history (1000 lines by default)
- Mouse wheel scrolling
- Keyboard scrolling (Shift+PgUp/PgDn, Shift+Up/Down)

#### Terminal Selection Mode (Ctrl+S)
Select and copy terminal output to Claude as a code block:
- Start with Ctrl+S in any terminal pane
- j/k or arrows to adjust selection
- Shift+Up/Down for 5-line jumps
- g/G to jump to start/end of buffer
- Enter or y to copy to Claude
- Esc to cancel

#### Drag & Drop
- Drag files from File Browser to terminal panes
- Automatically quotes paths with spaces
- Insert file paths directly into Claude or Terminal

#### Additional Features
- **Fuzzy Finder (Ctrl+P)**: Quick file search and navigation
- **Settings Menu (Ctrl+,)**: Configure shell, layout, and more
- **Setup Wizard (Ctrl+Shift+W)**: First-run configuration assistant
- **About Dialog (i)**: License info and open source components
- **Context Menu (F9)**: File operations (New, Rename, Delete, Copy Path)

### Installation

#### From Source
```bash
# Clone the repository
git clone https://github.com/yourusername/claude-workbench.git
cd claude-workbench

# Build release version
cargo build --release

# Run
./target/release/claude-workbench
```

#### Requirements
- Rust 1.70+
- macOS, Linux, or Windows
- Terminal with 256-color support recommended

### Configuration

Configuration is stored in:
1. `./config.yaml` (project-local, highest priority)
2. `~/.config/claude-workbench/config.yaml` (user config)

#### Example config.yaml
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
    - name: "Explain Code"
      prefix: "/explain"
      description: "Explain selected code"
```

### Keyboard Shortcuts

#### Global
| Key | Action |
|-----|--------|
| Ctrl+Q | Quit |
| Ctrl+P | Fuzzy Finder |
| Ctrl+, | Settings |
| Ctrl+Shift+W | Setup Wizard |
| F1-F6 | Switch panes |
| F9 | Context Menu |
| ? | Help |
| i | About |

#### File Browser
| Key | Action |
|-----|--------|
| j/k, Up/Down | Navigate |
| Enter | Open/Enter |
| Backspace/Left | Parent directory |
| o | Open in browser |
| O | Open in Finder |

#### Editor (Preview Pane)
| Key | Action |
|-----|--------|
| E | Enter edit mode |
| Ctrl+S | Save |
| Ctrl+Z | Undo |
| Ctrl+Y | Redo |
| PageUp/Down | Scroll 10 lines |
| Home/End | Jump to start/end |
| Esc | Exit editor |

#### Terminal Panes
| Key | Action |
|-----|--------|
| Ctrl+S | Start selection |
| Shift+PgUp/PgDn | Scroll 10 lines |
| Shift+Up/Down | Scroll 1 line |

### Tech Stack

- **[Ratatui](https://github.com/ratatui/ratatui)** - TUI framework
- **[Crossterm](https://github.com/crossterm-rs/crossterm)** - Terminal handling
- **[portable-pty](https://github.com/wez/wezterm)** - PTY management
- **[vt100](https://github.com/doy/vt100-rust)** - Terminal emulation
- **[syntect](https://github.com/trishume/syntect)** - Syntax highlighting
- **[tui-textarea](https://github.com/rhysd/tui-textarea)** - Text editor widget
- **[tui-markdown](https://github.com/joshka/tui-markdown)** - Markdown rendering
- **[pulldown-cmark](https://github.com/pulldown-cmark/pulldown-cmark)** - Markdown to HTML

### License

MIT License - Copyright (c) 2025 Martin Schmid

See [LICENSE](LICENSE) for details.

---

<a name="deutsch"></a>
## Deutsch

Ein Rust-basierter TUI (Terminal User Interface) Multiplexer, entwickelt für KI-unterstützte Entwicklungsworkflows. Bietet eine integrierte Entwicklungsumgebung mit Dateibrowser, Syntax-hervorgehobener Vorschau und mehreren eingebetteten PTY-Terminals.

### Funktionen

#### Dateibrowser (F1)
- Verzeichnisnavigation mit Tastatur (j/k, Pfeiltasten) oder Maus
- Git-Status-Integration mit farbcodierten Indikatoren:
  - Gelb: Ungetrackt (?)
  - Orange: Modifiziert (M)
  - Grün: Staged (+)
  - Grau: Ignoriert (·)
  - Rot: Konflikt (!)
- Statusleiste zeigt Dateigröße, Änderungsdatum und Git-Branch-Info
- Doppelklick zum Öffnen von Dateien oder Betreten von Verzeichnissen
- Kontextmenü (F9) für Dateioperationen: Neu, Umbenennen, Löschen, Pfad kopieren

#### Vorschau-Bereich (F2)
- Syntax-Hervorhebung für über 500 Sprachen (via syntect)
- Markdown-Rendering mit formatierter Anzeige (via tui-markdown)
- Integrierter Texteditor mit Undo/Redo-Unterstützung (via tui-textarea)
- Scrollbare Vorschau mit Tastatur- und Mausnavigation
- PageUp/PageDown, Home/End für schnelle Navigation

#### Browser-Vorschau (o-Taste)
- **HTML/HTM**: Direkte Browser-Öffnung
- **Markdown**: Konvertierung zu gestyltem HTML mit Dark-Mode-Unterstützung
- **PDF**: Öffnet im Standard-PDF-Viewer
- **Bilder**: PNG, JPG, GIF, SVG, WebP im System-Bildbetrachter
- **O (Shift+O)**: Aktuelles Verzeichnis im Finder/Dateimanager öffnen

#### Terminal-Bereiche
- **Claude Code (F4)**: Eingebettetes Claude CLI Terminal mit optionalen Startup-Präfixen
- **LazyGit (F5)**: Integrierte Git-TUI für Versionskontrolle
- **Benutzer-Terminal (F6)**: Allgemeines Shell-Terminal

Alle Terminal-Bereiche unterstützen:
- Volle PTY-Emulation mit 256-Farben-Unterstützung
- Scrollback-Verlauf (standardmäßig 1000 Zeilen)
- Mausrad-Scrolling
- Tastatur-Scrolling (Shift+PgUp/PgDn, Shift+Up/Down)

#### Terminal-Auswahlmodus (Ctrl+S)
Terminal-Ausgabe auswählen und als Code-Block an Claude kopieren:
- Start mit Ctrl+S in jedem Terminal-Bereich
- j/k oder Pfeiltasten zur Anpassung der Auswahl
- Shift+Up/Down für 5-Zeilen-Sprünge
- g/G zum Springen an Anfang/Ende des Buffers
- Enter oder y zum Kopieren an Claude
- Esc zum Abbrechen

#### Drag & Drop
- Dateien aus dem Dateibrowser in Terminal-Bereiche ziehen
- Pfade mit Leerzeichen werden automatisch quotiert
- Dateipfade direkt in Claude oder Terminal einfügen

#### Zusätzliche Funktionen
- **Fuzzy-Finder (Ctrl+P)**: Schnelle Dateisuche und Navigation
- **Einstellungsmenü (Ctrl+,)**: Shell, Layout und mehr konfigurieren
- **Setup-Assistent (Ctrl+Shift+W)**: Erstkonfigurationsassistent
- **Über-Dialog (i)**: Lizenzinfo und Open-Source-Komponenten
- **Kontextmenü (F9)**: Dateioperationen (Neu, Umbenennen, Löschen, Pfad kopieren)

### Installation

#### Aus dem Quellcode
```bash
# Repository klonen
git clone https://github.com/yourusername/claude-workbench.git
cd claude-workbench

# Release-Version bauen
cargo build --release

# Ausführen
./target/release/claude-workbench
```

#### Anforderungen
- Rust 1.70+
- macOS, Linux oder Windows
- Terminal mit 256-Farben-Unterstützung empfohlen

### Konfiguration

Die Konfiguration wird gespeichert in:
1. `./config.yaml` (projektlokal, höchste Priorität)
2. `~/.config/claude-workbench/config.yaml` (Benutzerkonfiguration)

#### Beispiel config.yaml
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
    - name: "Code erklären"
      prefix: "/explain"
      description: "Ausgewählten Code erklären"
```

### Tastenkürzel

#### Global
| Taste | Aktion |
|-------|--------|
| Ctrl+Q | Beenden |
| Ctrl+P | Fuzzy-Finder |
| Ctrl+, | Einstellungen |
| Ctrl+Shift+W | Setup-Assistent |
| F1-F6 | Bereiche wechseln |
| F9 | Kontextmenü |
| ? | Hilfe |
| i | Über |

#### Dateibrowser
| Taste | Aktion |
|-------|--------|
| j/k, Up/Down | Navigieren |
| Enter | Öffnen/Betreten |
| Backspace/Left | Übergeordnetes Verzeichnis |
| o | Im Browser öffnen |
| O | Im Finder öffnen |

#### Editor (Vorschau-Bereich)
| Taste | Aktion |
|-------|--------|
| E | Bearbeitungsmodus starten |
| Ctrl+S | Speichern |
| Ctrl+Z | Rückgängig |
| Ctrl+Y | Wiederholen |
| PageUp/Down | 10 Zeilen scrollen |
| Home/End | An Anfang/Ende springen |
| Esc | Editor verlassen |

#### Terminal-Bereiche
| Taste | Aktion |
|-------|--------|
| Ctrl+S | Auswahl starten |
| Shift+PgUp/PgDn | 10 Zeilen scrollen |
| Shift+Up/Down | 1 Zeile scrollen |

### Technologie-Stack

- **[Ratatui](https://github.com/ratatui/ratatui)** - TUI-Framework
- **[Crossterm](https://github.com/crossterm-rs/crossterm)** - Terminal-Handhabung
- **[portable-pty](https://github.com/wez/wezterm)** - PTY-Verwaltung
- **[vt100](https://github.com/doy/vt100-rust)** - Terminal-Emulation
- **[syntect](https://github.com/trishume/syntect)** - Syntax-Hervorhebung
- **[tui-textarea](https://github.com/rhysd/tui-textarea)** - Texteditor-Widget
- **[tui-markdown](https://github.com/joshka/tui-markdown)** - Markdown-Rendering
- **[pulldown-cmark](https://github.com/pulldown-cmark/pulldown-cmark)** - Markdown zu HTML

### Lizenz

MIT-Lizenz - Copyright (c) 2025 Martin Schmid

Siehe [LICENSE](LICENSE) für Details.
