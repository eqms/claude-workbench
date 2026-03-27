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

### Built for Speed. Stripped to the Essentials.

I love efficient coding, but I grew tired of bloated IDEs. Visual Studio Code felt too heavy, and other tools often came with baggage I simply didn't need for my daily workflow. What I truly wanted was an environment as fast as my thought process — built on the stability of Rust and bringing the power of Claude directly into the shell.

Over the 2025/2026 New Year, I turned that vision into reality: **Claude Workbench**.

It's not a traditional IDE; it's a high-performance TUI (Terminal User Interface). Built on the Fish shell and Rust, it seamlessly integrates tools like `lazy-git` and provides everything you need for a frictionless workflow — from an intelligent file browser and live Markdown rendering to direct Claude integration.

No overhead. Maximum performance. Built by a developer, for developers.

#### Why Start Fresh?

- **The Problem:** Modern IDEs have become bloated, filled with features that distract rather than help.
- **The Search:** After testing alternatives like Zed or Google IDX, they lacked the "Shell-First" philosophy I crave.
- **The Goal:** Create a portable, lightning-fast solution that feels like a natural extension of the terminal.

#### The Technical Foundation

- **Rust:** Chosen for uncompromising performance, safety, and stability.
- **Fish Shell (4.x):** The core for a modern, user-friendly command-line experience.
- **Claude Integration:** Deep integration of Claude (e.g., via Claude Code) for AI-assisted development without leaving the terminal.
- **Automation:** Hosted on GitHub with automated release workflows (compiling) and integrated self-update logic.

<p align="center">
  <img src="docs/claude-workbench-tui.png" alt="Claude Workbench - Core Features & Integrated Workflow" width="900">
</p>

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
- **F9 Copy Last N Lines** - copy last N terminal lines to clipboard (configurable, default 50)
- **Self-update** - automatic update check from GitHub Releases
- **App Dropdown** - auto-detect installed browsers/editors in Settings (macOS + Linux)
- **Ctrl+X Markdown Export** - export as Markdown copy or PDF (native Typst engine, no external tools needed)
- **Ctrl+V Paste** - clipboard paste in all input dialogs
- Mouse and keyboard navigation throughout

### Quick Start

```bash
# Install via Homebrew (macOS / Linux)
brew install eqms/claude-workbench/claude-workbench

# Or use the installer script
curl -fsSL https://raw.githubusercontent.com/eqms/claude-workbench/main/scripts/install.sh | bash

# Or build from source
git clone https://github.com/eqms/claude-workbench.git
cd claude-workbench && cargo build --release
./target/release/claude-workbench
```

**See [INSTALL.md](INSTALL.md) for detailed platform-specific installation instructions.**

### Essential Shortcuts

| Key | Action |
|-----|--------|
| F1-F6 | Switch between panes |
| F9 | File menu in File Browser / **Copy last N lines** to clipboard in Terminal panes |
| F12 | Help (full shortcut reference) |
| Ctrl+P | Fuzzy file finder |
| Ctrl+Q | Quit |
| E | Edit file (in Preview) |
| Ctrl+X | Export Markdown/PDF (format chooser) |
| Ctrl+E | Open in External Editor (context-aware) |
| Ctrl+S | Selection mode (in Terminal/Preview) |
| Ctrl+C | Copy selection to System Clipboard |

**See [USAGE.md](USAGE.md) for complete keyboard shortcuts and detailed usage guide.**

### What's New in v0.75.0

- **PDF Code Font from Config** — The `Code Font` setting (F8 → Document) is now applied to code blocks and tables in PDF export. Previously hardcoded to Consolas/Courier New.

### What's New in v0.73.0

- **Async PDF Export** — PDF generation runs in background with yellow progress indicator. UI stays responsive.
- **PDF Internal Links** — Markdown anchor links (`[text](#heading)`) now work as clickable navigation in PDF.
- **PDF Font Fix** — Header/footer now use configured Carlito/Calibri font instead of Typst default.
- **Linux Clipboard** — Wayland support via `wayland-data-control`, OSC 52 dual-terminator (BEL + ST) for broader terminal compatibility.
- **Linux Rendering Fix** — Fixed severe pane overlap/ghosting when scrolling on Linux terminals. Added `Clear` before pane rendering, zero-area guards, and proper None-cell clearing.

### What's New in v0.72.0

- **PDF Export Fix** — Fixed "file not found" error when exporting Markdown with local images. Remote image URLs are now rendered as links in PDF.
- **Export Flash Message** — Footer shows "PDF exported" / "Markdown exported" after successful export.
- **Templates Tab Removed** — Removed non-functional Templates tab from Settings (no persistence, no effect).

### What's New in v0.71.0

- **Native Typst PDF Engine** — PDF export now uses pure Rust Typst rendering. No external tools (Chrome, wkhtmltopdf) required. Bundled Carlito font (Calibri-compatible) for consistent cross-platform rendering.
- **Page Numbers** — Every PDF page shows "Seite X von Y" with three-column footer (Company | Date | Page) and header with document title.
- **Central DocumentConfig** — New `document:` section in `config.yaml` for unified branding: company name, fonts (Calibri default), colors (#D5E8F0 table headers), font sizes (Word-standard hierarchy), PDF page settings (A4, 2.5cm margins).
- **CSS Template Module** — Shared styling across all HTML preview and PDF templates via configurable `TemplateContext`.

### What's New in v0.70.0

- **F9 Menu Export** — File Menu (F9 in File Browser) now has an "Export Markdown/PDF" entry (`x` key) as a direct alternative to Ctrl+X.
- **Ctrl+E Context-Aware** — Opens the file currently shown in the Preview pane when Preview is active.

### What's New in v0.60.1

**Remote Control Fix** — The `remote-control` subcommand was removed (not a valid Claude CLI command). When Remote Control is enabled, the workbench now automatically sends a Space key 2 seconds after Claude starts, triggering the QR code display for remote access.

### What's New in v0.60.0

**Remote Control Toggle** — The Permission Mode dialog now includes a Remote Control checkbox below the 5 permission modes. When enabled, the QR code for remote access is automatically displayed after Claude starts. Toggle with Space, the setting is persisted in `config.yaml`.

```yaml
claude:
  remote_control: true  # Enable remote control mode (default: false)
```

### What's New in v0.59.0

**F9 „Copy Last N Lines"** — Press F9 in any terminal pane (Claude, LazyGit, Terminal) to copy the last N lines of output to the system clipboard. Configurable via `pty.copy_lines_count` (default: 50). Footer shows a green „✓ N lines" flash for 2 seconds. F9 in the File Browser still opens the file menu as before.

```yaml
pty:
  copy_lines_count: 50  # Increase for longer outputs, e.g. 100 or 200
```

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

# Optional: Claude startup prefixes and remote control
claude:
  remote_control: false  # Auto-show QR code for remote access after start
  startup_prefixes:
    - name: "Code Review"
      prefix: "/review"
      description: "Review code changes"

# Document export settings (PDF uses native Typst, no external tools needed)
document:
  company:
    name: "My Company"                    # Shown in PDF footer and author
    footer_text: "Generated by {company_name}"
  fonts:
    body: "Calibri, -apple-system, sans-serif"
  colors:
    table_header_bg: "#D5E8F0"
  pdf:
    page_size: "A4"
    margin: "2.5cm"
```

### Tech Stack

- **[Ratatui](https://github.com/ratatui/ratatui)** - TUI framework
- **[Crossterm](https://github.com/crossterm-rs/crossterm)** - Terminal handling
- **[portable-pty](https://github.com/wez/wezterm)** - PTY management
- **[vt100](https://github.com/doy/vt100-rust)** - Terminal emulation
- **[syntect](https://github.com/trishume/syntect)** - Syntax highlighting
- **[Typst](https://github.com/typst/typst)** - Native PDF generation (pure Rust, no external binaries)
- **[tui-textarea](https://github.com/rhysd/tui-textarea)** - Text editor widget
- **[tui-markdown](https://github.com/joshka/tui-markdown)** - Markdown rendering

### License

MIT License - Copyright (c) 2025 Martin Schmid

See [LICENSE](LICENSE) for details.

---

<a name="deutsch"></a>
## Deutsch

Ein Rust-basierter TUI (Terminal User Interface) Multiplexer für KI-unterstützte Entwicklungsworkflows. Bietet eine integrierte Entwicklungsumgebung mit Dateibrowser, Syntax-hervorgehobener Vorschau und mehreren eingebetteten PTY-Terminals.

### Gebaut für Geschwindigkeit. Reduziert auf das Wesentliche.

Ich liebe effizientes Programmieren, aber ich hatte genug von aufgeblähten IDEs. Visual Studio Code fühlte sich zu schwer an, und andere Tools brachten oft Ballast mit, den ich für meinen täglichen Workflow schlicht nicht brauchte. Was ich wirklich wollte, war eine Umgebung, die so schnell ist wie mein Denkprozess — aufgebaut auf der Stabilität von Rust und mit der Kraft von Claude direkt in der Shell.

Über Silvester 2025/2026 habe ich diese Vision Wirklichkeit werden lassen: **Claude Workbench**.

Es ist keine traditionelle IDE; es ist ein hochperformantes TUI (Terminal User Interface). Aufgebaut auf der Fish Shell und Rust, integriert es nahtlos Werkzeuge wie `lazy-git` und bietet alles, was man für einen reibungslosen Workflow braucht — von einem intelligenten Dateibrowser und Live-Markdown-Rendering bis hin zur direkten Claude-Integration.

Kein Overhead. Maximale Performance. Von einem Entwickler, für Entwickler.

#### Warum von Grund auf neu?

- **Das Problem:** Moderne IDEs sind aufgebläht, vollgestopft mit Features die ablenken statt zu helfen.
- **Die Suche:** Nach dem Testen von Alternativen wie Zed oder Google IDX fehlte ihnen die "Shell-First"-Philosophie, die ich brauche.
- **Das Ziel:** Eine portable, blitzschnelle Lösung schaffen, die sich wie eine natürliche Erweiterung des Terminals anfühlt.

#### Das technische Fundament

- **Rust:** Gewählt für kompromisslose Performance, Sicherheit und Stabilität.
- **Fish Shell (4.x):** Der Kern für ein modernes, benutzerfreundliches Kommandozeilen-Erlebnis.
- **Claude-Integration:** Tiefe Integration von Claude (z.B. via Claude Code) für KI-unterstützte Entwicklung ohne das Terminal zu verlassen.
- **Automatisierung:** Gehostet auf GitHub mit automatisierten Release-Workflows (Kompilierung) und integrierter Selbst-Update-Logik.

<p align="center">
  <img src="docs/claude-workbench-tui_de.png" alt="Claude Workbench - Kern-Features & Integrierter Workflow" width="900">
</p>

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
- **F9 Letzte N Zeilen kopieren** - letzten N Terminal-Zeilen ins Clipboard kopieren (konfigurierbar, Standard 50)
- **Selbst-Update** - automatische Update-Prüfung von GitHub Releases
- **App-Dropdown** - automatische Erkennung installierter Browser/Editoren in Settings (macOS + Linux)
- **Ctrl+X Markdown-Export** - Export als Markdown-Kopie oder PDF (native Typst-Engine, keine externen Tools nötig)
- **Ctrl+V Einfügen** - Clipboard-Paste in allen Eingabedialogen
- Maus- und Tastaturnavigation durchgehend

### Schnellstart

```bash
# Installation via Homebrew (macOS / Linux)
brew install eqms/claude-workbench/claude-workbench

# Oder Installer-Skript verwenden
curl -fsSL https://raw.githubusercontent.com/eqms/claude-workbench/main/scripts/install.sh | bash

# Oder aus Quellcode bauen
git clone https://github.com/eqms/claude-workbench.git
cd claude-workbench && cargo build --release
./target/release/claude-workbench
```

**Siehe [INSTALL.md](INSTALL.md) für detaillierte plattformspezifische Installationsanleitungen.**

### Wichtige Tastenkürzel

| Taste | Aktion |
|-------|--------|
| F1-F6 | Zwischen Bereichen wechseln |
| F9 | Datei-Menü im Dateibrowser / **Letzte N Zeilen kopieren** in Terminal-Bereichen |
| F12 | Hilfe (vollständige Shortcut-Referenz) |
| Ctrl+P | Fuzzy-Dateisuche |
| Ctrl+Q | Beenden |
| E | Datei bearbeiten (in Vorschau) |
| Ctrl+X | Markdown/PDF exportieren (Format-Auswahl) |
| Ctrl+E | In externem Editor öffnen (kontextabhängig) |
| Ctrl+S | Auswahlmodus (in Terminal/Vorschau) |
| Ctrl+C | Auswahl in System-Clipboard kopieren |

**Siehe [USAGE.md](USAGE.md) für alle Tastenkürzel und detaillierte Bedienungsanleitung.**

### Neu in v0.75.0

- **PDF Code-Font aus Config** — Die `Code Font`-Einstellung (F8 → Document) wird jetzt auf Code-Blöcke und Tabellen im PDF-Export angewendet. Zuvor fest auf Consolas/Courier New verdrahtet.

### Neu in v0.73.0

- **Async PDF-Export** — PDF-Generierung im Hintergrund mit gelbem Fortschrittsindikator. UI bleibt reaktiv.
- **PDF Interne Links** — Markdown-Anker-Links (`[text](#heading)`) funktionieren als klickbare Navigation im PDF.
- **PDF Schriftart-Fix** — Header/Footer verwenden jetzt konfigurierte Carlito/Calibri-Schrift statt Typst-Default.
- **Linux Clipboard** — Wayland-Support via `wayland-data-control`, OSC 52 Dual-Terminator (BEL + ST) fuer breitere Terminal-Kompatibilitaet.
- **Linux Rendering-Fix** — Schweres Pane-Overlap/Ghosting beim Scrollen unter Linux behoben. Clear vor Pane-Rendering, Zero-Area-Guards, und korrektes None-Cell-Clearing.

### Neu in v0.72.0

- **PDF-Export Fix** — Fehler „file not found" bei Markdown-Dateien mit lokalen Bildern behoben. Remote-Bild-URLs werden als Links im PDF dargestellt.
- **Export Flash-Nachricht** — Footer zeigt nach erfolgreichem Export „PDF exported" / „Markdown exported".
- **Templates-Tab entfernt** — Nicht-funktionaler Templates-Tab aus Settings entfernt (keine Persistierung, keine Wirkung).

### Neu in v0.70.0

- **F9-Menü Export** — Das Datei-Menü (F9 im Dateibrowser) hat jetzt einen „Export Markdown/PDF"-Eintrag (`x`-Taste) als direkte Alternative zu Ctrl+X.
- **Ctrl+E kontextabhängig** — Öffnet die aktuell in der Vorschau angezeigte Datei, wenn der Vorschau-Bereich aktiv ist (bisher nur die im Dateibrowser markierte Datei).
- **Behoben: Settings Auto-Save bei Esc** — Der Settings-Dialog speichert Änderungen jetzt korrekt beim Schließen mit Esc.
- **Behoben: Tab-Vervollständigung in allen Pfad-Dialogen** — Tab vervollständigt Pfade in allen Dialogen (Ctrl+O, Export-Verzeichnis, Browser/Editor-Pfade).
- **Behoben: Browser-Auswahl für Exporte** — Der konfigurierte Browser wird jetzt korrekt für die Markdown-zu-HTML-Export-Vorschau verwendet.

### Neu in v0.69.0

- **App-Dropdown für Browser/Editor** — Settings → Paths erkennt jetzt automatisch installierte Browser und Editoren. Auswahl per Dropdown statt manuelle Pfadeingabe. macOS (App-Bundle-Erkennung) und Linux (which-basiert). "Custom path..." Fallback für manuelle Eingabe.
- **Ctrl+X: Markdown-Export** — Aktuelle Markdown-Datei als Markdown-Kopie oder PDF exportieren. Format-Auswahl-Dialog, konfigurierbares Export-Verzeichnis (Settings → Paths → Export Directory, Standard: ~/Downloads). PDF via Chrome headless oder wkhtmltopdf.
- **Ctrl+V Einfügen in Dialogen** — Clipboard-Paste funktioniert jetzt in allen Eingabedialogen (Ctrl+O, Dateioperationen, Settings-Felder).
- **Command-Splitting Bugfix** — Browser/Editor-Befehle wie `open -a "Brave Browser"` funktionieren jetzt korrekt.

### Neu in v0.60.1

**Remote Control Fix** — Der `remote-control` Subcommand wurde entfernt (kein gültiger Claude CLI-Befehl). Bei aktiviertem Remote Control wird nun automatisch 2 Sekunden nach Claude-Start die Leertaste gesendet, um den QR-Code für den Remote-Zugriff anzuzeigen.

### Neu in v0.60.0

**Remote Control Toggle** — Der Berechtigungsmodus-Dialog enthält nun eine Remote Control Checkbox unterhalb der 5 Modi. Wenn aktiviert, wird der QR-Code für den Remote-Zugriff automatisch nach dem Claude-Start angezeigt. Umschalten mit Leertaste, die Einstellung wird in `config.yaml` gespeichert.

```yaml
claude:
  remote_control: true  # Remote Control Modus aktivieren (Standard: false)
```

### Neu in v0.59.0

**F9 „Letzte N Zeilen kopieren"** — F9 in einem Terminal-Bereich (Claude, LazyGit, Terminal) kopiert die letzten N Ausgabe-Zeilen in die Zwischenablage. Konfigurierbar über `pty.copy_lines_count` (Standard: 50). Der Footer zeigt 2 Sekunden lang einen grünen „✓ N Zeilen"-Flash. F9 im Dateibrowser öffnet weiterhin das Datei-Menü.

```yaml
pty:
  copy_lines_count: 50  # Für längere Ausgaben erhöhen, z.B. 100 oder 200
```

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

# Optional: Claude Startup-Präfixe und Remote Control
claude:
  remote_control: false  # QR-Code für Remote-Zugriff automatisch anzeigen
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
