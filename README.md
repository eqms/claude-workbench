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
- Toggle hidden files with `.` key
- Refresh with F3
- Context menu (F9) for file operations: New File, New Directory, Rename, Duplicate, Copy to, Move to, Delete, Copy Path

#### Preview Pane (F2)
- Syntax highlighting for 500+ languages (via syntect)
- Markdown rendering with formatted display (via tui-markdown)
- Built-in text editor with undo/redo support (via tui-textarea)
- Scrollable preview with keyboard and mouse navigation
- PageUp/PageDown, Home/End for quick navigation
- Line numbers with current line highlighting in edit mode

#### Search & Replace (MC Edit Style)
- `/` or `Ctrl+F` to start search
- `Ctrl+H` to open Search & Replace directly
- `Tab` to switch between Find/Replace fields
- `n`/`N` or `Ctrl+N`/`Ctrl+P` to navigate matches
- `Ctrl+I` to toggle case sensitivity
- `Enter` to replace current match
- `Ctrl+R` to replace all matches
- `Esc` to close search

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
- Multi-line input: Use `\` + Enter in Claude Code pane for newlines
- Word navigation: Alt+Left/Right to move by word
- PageUp/PageDown remapped to Home/End for line navigation

#### Terminal Selection Mode (Ctrl+S or Alt+Click)
Select and copy terminal output to Claude as a code block:

**Keyboard Selection (Ctrl+S):**
- Start with Ctrl+S in any terminal pane
- j/k or arrows to adjust selection
- Shift+Up/Down for 5-line jumps
- g/G to jump to start/end of buffer
- Enter or y to copy to Claude
- Esc to cancel

**Mouse Selection:**
- Alt+Click and drag to select lines in terminal panes
- Release to enter selection mode
- Enter or y to copy to Claude
- Highlighted in yellow during selection
- Note: Regular click only focuses pane (no selection)

**Intelligent Filtering:**
When copying to Claude, the output is automatically filtered:
- Shell prompts removed (user@host$, >, >>>, etc.)
- Error messages and stack traces preserved
- Directory listing noise filtered (drwx, total N)
- Consecutive blank lines collapsed (max 2)
- Syntax auto-detection for code blocks (Python, Rust, JavaScript, Bash, XML)

#### Drag & Drop
- Drag files from File Browser to terminal panes
- Automatically quotes paths with spaces
- Insert file paths directly into Claude or Terminal

#### Git Integration
- Auto-checks for remote changes when entering a different repository
- Prompts to pull if remote is ahead
- Color-coded file status in file browser

#### Additional Features
- **Fuzzy Finder (Ctrl+P)**: Quick file search and navigation
- **Settings Menu (Ctrl+,)**: Configure shell, layout, and more
- **Setup Wizard (Ctrl+Shift+W)**: First-run configuration assistant
- **About Dialog (F10)**: License info and open source components
- **Help (F12)**: Scrollable keyboard shortcuts and usage guide (j/k, arrows, PgUp/PgDn, g/G)
- **Context Menu (F9)**: File operations with full cursor-based input editing

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

#### Global (work everywhere, including terminals)
| Key | Action |
|-----|--------|
| Ctrl+Q | Quit |
| Ctrl+P | Fuzzy Finder |
| Ctrl+, | Settings |
| Ctrl+Shift+W | Setup Wizard |
| F1 | File Browser |
| F2 | Preview Pane |
| F3 | Refresh File Browser |
| F4 | Claude Code |
| F5 | LazyGit |
| F6 | User Terminal |
| F9 | File Menu |
| F10 | About |
| F12 | Help |
| Esc | Close Dialogs/Help |

#### Context-specific (FileBrowser/Preview only)
| Key | Action |
|-----|--------|
| ? | Help (not in terminals) |
| i | About (FileBrowser only) |

#### File Browser
| Key | Action |
|-----|--------|
| j/k, Up/Down | Navigate |
| Enter | Open/Enter |
| Backspace/Left | Parent directory |
| . | Toggle hidden files |
| o | Open in browser |
| O | Open in Finder |

#### Preview Pane (Read-Only)
| Key | Action |
|-----|--------|
| j/k, Up/Down | Scroll 1 line |
| PageUp/Down | Scroll 10 lines |
| Home/End | Jump to start/end |
| E | Enter edit mode |
| Ctrl+S | Enter selection mode |

#### Search & Replace
| Key | Action |
|-----|--------|
| / or Ctrl+F | Start search |
| Ctrl+H | Search & Replace (Edit mode) |
| Tab | Switch Find/Replace fields |
| n / N | Next / Previous match |
| Ctrl+N / Ctrl+P | Navigate while typing |
| Ctrl+I | Toggle case sensitivity |
| Enter | Confirm / Replace current |
| Ctrl+R | Replace all matches |
| Esc | Close search |

#### Editor (Edit Mode)
| Key | Action |
|-----|--------|
| Ctrl+S | Save |
| Ctrl+Z | Undo |
| Esc | Exit (confirm if modified) |

**MC Edit Style Block Operations:**
| Key | Action |
|-----|--------|
| Shift+↑/↓/←/→ | Select text |
| Ctrl+F3 | Toggle block marking |
| Ctrl+F5 | Copy block |
| Ctrl+F6 | Move (cut) block |
| Ctrl+F8 | Delete block |
| Ctrl+Y | Delete current line |

#### Terminal Panes
| Key | Action |
|-----|--------|
| \\ + Enter | Insert newline in Claude Code (F4) |
| Ctrl+S | Start selection |
| Shift+PgUp/PgDn | Scroll 10 lines |
| Shift+Up/Down | Scroll 1 line |
| Alt+Left/Right | Word navigation |
| PageUp | Jump to line start (Home) |
| PageDown | Jump to line end (End) |

#### Selection Mode (Ctrl+S in Terminal/Preview)
| Key | Action |
|-----|--------|
| j/k, Up/Down | Adjust selection |
| Shift+Up/Down | Adjust by 5 lines |
| g / G | Jump to buffer start/end |
| Enter / y | Copy to Claude |
| Esc | Cancel |

#### Dialog Input Fields
| Key | Action |
|-----|--------|
| Left/Right | Move cursor |
| Home/End | Jump to start/end |
| Backspace | Delete before cursor |
| Delete | Delete at cursor |
| Enter | Confirm |
| Esc | Cancel |

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
- Versteckte Dateien mit `.` umschalten
- Aktualisieren mit F3
- Kontextmenü (F9) für Dateioperationen: Neue Datei, Neues Verzeichnis, Umbenennen, Duplizieren, Kopieren nach, Verschieben nach, Löschen, Pfad kopieren

#### Vorschau-Bereich (F2)
- Syntax-Hervorhebung für über 500 Sprachen (via syntect)
- Markdown-Rendering mit formatierter Anzeige (via tui-markdown)
- Integrierter Texteditor mit Undo/Redo-Unterstützung (via tui-textarea)
- Scrollbare Vorschau mit Tastatur- und Mausnavigation
- PageUp/PageDown, Home/End für schnelle Navigation
- Zeilennummern mit Hervorhebung der aktuellen Zeile im Bearbeitungsmodus

#### Suchen & Ersetzen (MC Edit Stil)
- `/` oder `Ctrl+F` zum Starten der Suche
- `Ctrl+H` zum direkten Öffnen von Suchen & Ersetzen
- `Tab` zum Wechseln zwischen Such-/Ersetzungsfeld
- `n`/`N` oder `Ctrl+N`/`Ctrl+P` zum Navigieren der Treffer
- `Ctrl+I` zum Umschalten der Groß-/Kleinschreibung
- `Enter` zum Ersetzen des aktuellen Treffers
- `Ctrl+R` zum Ersetzen aller Treffer
- `Esc` zum Schließen der Suche

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
- Mehrzeilige Eingabe: `\` + Enter für Zeilenumbruch im Claude Code Pane
- Wort-Navigation: Alt+Links/Rechts zum Springen zwischen Wörtern
- PageUp/PageDown umgemappt auf Home/End für Zeilen-Navigation

#### Terminal-Auswahlmodus (Ctrl+S oder Alt+Klick)
Terminal-Ausgabe auswählen und als Code-Block an Claude kopieren:

**Tastatur-Auswahl (Ctrl+S):**
- Start mit Ctrl+S in jedem Terminal-Bereich
- j/k oder Pfeiltasten zur Anpassung der Auswahl
- Shift+Up/Down für 5-Zeilen-Sprünge
- g/G zum Springen an Anfang/Ende des Buffers
- Enter oder y zum Kopieren an Claude
- Esc zum Abbrechen

**Maus-Auswahl:**
- Alt+Klicken und Ziehen um Zeilen in Terminal-Bereichen auszuwählen
- Loslassen zum Betreten des Auswahlmodus
- Enter oder y zum Kopieren an Claude
- Gelb hervorgehoben während der Auswahl
- Hinweis: Normaler Klick fokussiert nur den Bereich (keine Auswahl)

**Intelligentes Filtering:**
Beim Kopieren zu Claude wird die Ausgabe automatisch gefiltert:
- Shell-Prompts entfernt (user@host$, >, >>>, etc.)
- Fehlermeldungen und Stack-Traces bleiben erhalten
- Verzeichnislisten-Rauschen gefiltert (drwx, total N)
- Aufeinanderfolgende Leerzeilen komprimiert (max 2)
- Syntax-Erkennung für Code-Blöcke (Python, Rust, JavaScript, Bash, XML)

#### Drag & Drop
- Dateien aus dem Dateibrowser in Terminal-Bereiche ziehen
- Pfade mit Leerzeichen werden automatisch quotiert
- Dateipfade direkt in Claude oder Terminal einfügen

#### Git-Integration
- Automatische Prüfung auf Remote-Änderungen beim Wechsel in ein anderes Repository
- Aufforderung zum Pullen wenn Remote voraus ist
- Farbcodierte Dateistatus im Dateibrowser

#### Zusätzliche Funktionen
- **Fuzzy-Finder (Ctrl+P)**: Schnelle Dateisuche und Navigation
- **Einstellungsmenü (Ctrl+,)**: Shell, Layout und mehr konfigurieren
- **Setup-Assistent (Ctrl+Shift+W)**: Erstkonfigurationsassistent
- **Über-Dialog (F10)**: Lizenzinfo und Open-Source-Komponenten
- **Hilfe (F12)**: Scrollbare Tastenkürzel und Bedienungsanleitung (j/k, Pfeile, PgUp/PgDn, g/G)
- **Kontextmenü (F9)**: Dateioperationen mit voller Cursor-basierter Eingabebearbeitung

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

#### Global (funktionieren überall, auch in Terminals)
| Taste | Aktion |
|-------|--------|
| Ctrl+Q | Beenden |
| Ctrl+P | Fuzzy-Finder |
| Ctrl+, | Einstellungen |
| Ctrl+Shift+W | Setup-Assistent |
| F1 | Dateibrowser |
| F2 | Vorschau-Bereich |
| F3 | Dateibrowser aktualisieren |
| F4 | Claude Code |
| F5 | LazyGit |
| F6 | Benutzer-Terminal |
| F9 | Datei-Menü |
| F10 | Über |
| F12 | Hilfe |
| Esc | Dialoge/Hilfe schließen |

#### Kontext-spezifisch (nur FileBrowser/Preview)
| Taste | Aktion |
|-------|--------|
| ? | Hilfe (nicht in Terminals) |
| i | Über (nur im Dateibrowser) |

#### Dateibrowser
| Taste | Aktion |
|-------|--------|
| j/k, Up/Down | Navigieren |
| Enter | Öffnen/Betreten |
| Backspace/Left | Übergeordnetes Verzeichnis |
| . | Versteckte Dateien umschalten |
| o | Im Browser öffnen |
| O | Im Finder öffnen |

#### Vorschau-Bereich (Nur-Lesen)
| Taste | Aktion |
|-------|--------|
| j/k, Up/Down | 1 Zeile scrollen |
| PageUp/Down | 10 Zeilen scrollen |
| Home/End | An Anfang/Ende springen |
| E | Bearbeitungsmodus starten |
| Ctrl+S | Auswahlmodus starten |

#### Suchen & Ersetzen
| Taste | Aktion |
|-------|--------|
| / oder Ctrl+F | Suche starten |
| Ctrl+H | Suchen & Ersetzen (Bearbeitungsmodus) |
| Tab | Such-/Ersetzungsfeld wechseln |
| n / N | Nächster / Vorheriger Treffer |
| Ctrl+N / Ctrl+P | Während der Eingabe navigieren |
| Ctrl+I | Groß-/Kleinschreibung umschalten |
| Enter | Bestätigen / Aktuellen ersetzen |
| Ctrl+R | Alle Treffer ersetzen |
| Esc | Suche schließen |

#### Editor (Bearbeitungsmodus)
| Taste | Aktion |
|-------|--------|
| Ctrl+S | Speichern |
| Ctrl+Z | Rückgängig |
| Esc | Beenden (Bestätigung wenn geändert) |

**MC Edit Block-Operationen:**
| Taste | Aktion |
|-------|--------|
| Shift+↑/↓/←/→ | Text auswählen |
| Ctrl+F3 | Block-Markierung umschalten |
| Ctrl+F5 | Block kopieren |
| Ctrl+F6 | Block verschieben (ausschneiden) |
| Ctrl+F8 | Block löschen |
| Ctrl+Y | Aktuelle Zeile löschen |

#### Terminal-Bereiche
| Taste | Aktion |
|-------|--------|
| \\ + Enter | Zeilenumbruch im Claude Code (F4) |
| Ctrl+S | Auswahl starten |
| Shift+PgUp/PgDn | 10 Zeilen scrollen |
| Shift+Up/Down | 1 Zeile scrollen |
| Alt+Links/Rechts | Wort-Navigation |
| PageUp | An Zeilenanfang springen (Home) |
| PageDown | An Zeilenende springen (End) |

#### Auswahlmodus (Ctrl+S in Terminal/Vorschau)
| Taste | Aktion |
|-------|--------|
| j/k, Up/Down | Auswahl anpassen |
| Shift+Up/Down | Um 5 Zeilen anpassen |
| g / G | An Buffer-Anfang/-Ende springen |
| Enter / y | An Claude kopieren |
| Esc | Abbrechen |

#### Dialog-Eingabefelder
| Taste | Aktion |
|-------|--------|
| Links/Rechts | Cursor bewegen |
| Home/End | An Anfang/Ende springen |
| Backspace | Vor Cursor löschen |
| Delete | An Cursor löschen |
| Enter | Bestätigen |
| Esc | Abbrechen |

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
