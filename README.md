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
| F11 | Universal Paste — inject system clipboard into active pane (XRDP / broken bracketed-paste workaround) |
| Right-click | Paste from system clipboard into pane under cursor (mirrors Kitty's `mouse_map right press paste`) |

**See [USAGE.md](USAGE.md) for complete keyboard shortcuts and detailed usage guide.**

### What's New in v0.90.0

- **Phase 1 Security Hardening — Wave 1 of 3 shipped.** Eight audit findings closed in this release: predictable temp-path symlink-redirect vector (`tempfile::Builder` with `O_EXCL` in `src/browser/pdf_export.rs`); `--update-to <version>` flag debug-only via `#[cfg(debug_assertions)]` plus `filter_restart_args()` so the restart loop can't be wedged into an infinite downgrade; allow-list validation of the browser/editor config (`validate_program()` in `src/browser/opener.rs`); removal of the `$SHELL -i -c` fallback in `src/setup/dependency_checker.rs`; executable-bit check in `clipboard::which()`; `shlex::try_quote` error propagation from `sync_terminals*`; `semver::Version::max_by` release selection instead of GitHub creation-order `releases[0]`. Self-update signature verification (SEC-01) remains the only Phase 1 item still open — Wave 2 needs an operator-generated `zipsign` ed25519 keypair, Wave 3 needs at least two signed releases shipped before it can be enabled. Tests: 111 → 130.
- **`.planning/` directory introduced** — GSD project planning artifacts (PROJECT.md, ROADMAP.md, REQUIREMENTS.md, per-phase plans/research/reviews under `.planning/phases/`, refreshed codebase maps under `.planning/codebase/`) are now committed. Source of truth for upcoming phases.

### What's New in v0.87.0

- **Async clipboard worker thread** — `copy_to_clipboard()` now dispatches to a dedicated `clipboard-worker` thread and returns immediately with `ClipboardOutcome::Submitted`. The event loop polls `take_pending_outcome()` once per frame and only flashes the footer on real `Failed`. The UI stays responsive even when the X-server clipboard hangs for the full 500 ms timeout from v0.86.4. Paste remains synchronous (callers need the result immediately to inject into PTY/editor).

### What's New in v0.86.4

- **Clipboard subprocess timeout (500 ms)** — Real root cause of the "app frozen, no key/mouse response" symptom on XRDP: `xclip -i` and `xsel -i` block indefinitely under XRDP when the X11 selection-owner negotiation never completes. Because copy/paste run synchronously in the main thread, the entire event loop froze. `run_with_stdin()` and `run_capture()` now wait at most 500 ms and kill the child on timeout, falling back to the next helper or OSC 52 instead of hanging.
- **`CLAUDE_WORKBENCH_CLIPBOARD=osc52` kill-switch** — Forces the new `Osc52Only` strategy: skips arboard, xclip, xsel, wl-copy/-paste entirely and emits OSC 52 only. Use in sessions where the X-server clipboard is completely broken. Other accepted values: `arboard`, `subprocess`. `--clipboard-diag` shows the active override.

### What's New in v0.86.3

- **XRDP/Kitty selection-freeze fix (left-click drag)** — Under XRDP, the RDP transport reliably swallows the `ButtonRelease` event when `EnableMouseCapture` is active, while `ButtonPress` and motion events come through. Result: `mouse_selection.selecting` stays `true`, the highlight visually freezes, and further clicks are interpreted as drag extensions. Two defensive fixes: **Esc** now cancels an active mouse selection (global handler, runs after modal dismissals), and a new `Down(Left)` event clears any stale selection before starting a fresh one — clicks on footer/scrollbar/modal areas no longer leave a frozen highlight behind. Complements v0.86.2 which only addressed right-click.

### What's New in v0.86.2

- **Right-click = Paste in PTY/Preview panes** — `EnableMouseCapture` was swallowing right-clicks, so Kitty's `mouse_map right press ungrabbed paste_from_clipboard` could never fire under XRDP. Right-click now uses the same fallback chain as F11 (arboard → xclip → xsel → wl-paste) and pastes into the pane under the cursor. If an Alt+drag mouse selection is active, right-click clears it instead — fixes the XRDP-only "screen blocked, selection won't go away" bug.

### What's New in v0.86.1

- **macOS boot hang hotfix** — v0.86.0 invoked `check_command()` for the four new clipboard helpers at startup, falling back to `$SHELL -i -c "..."` when direct exec failed. On macOS the helpers are typically absent → 4× `fish -i -c` with job-control init → terminal state corrupted on next launch. Helper detection now uses pure-Rust PATH lookup (`crate::clipboard::which()`), no subprocess.

### What's New in v0.86.0

- **Clipboard fallback chain** — `arboard` → `xclip` → `xsel` → `wl-copy` → OSC 52 for copy; `arboard` → `xclip -o` → `xsel -b -o` → `wl-paste` for paste. Restores clipboard sync under XRDP / Kitty / Xfce where Kitty's bracketed-paste forwarding fails.
- **F11 Universal Paste** — Reads the system clipboard via the fallback chain and injects it directly into the active pane (Claude / LazyGit / Terminal / Preview-Edit). Bypasses Kitty's bracketed-paste bridge entirely — the workaround when Kitty cannot read the system clipboard under XRDP.
- **Startup dependency check** — Detects `xclip`, `xsel`, `wl-copy`, `wl-paste` at launch. On Linux without any helper, a yellow footer banner appears for 10 seconds. F12 (Help) shows the current strategy and detected helpers.
- **`--clipboard-diag` CLI** — `claude-workbench --clipboard-diag` prints the active strategy, helper paths, relevant environment variables (`DISPLAY`, `WAYLAND_DISPLAY`, `XRDP_SESSION`, `XDG_SESSION_TYPE`, `SSH_TTY`) and runs a copy/paste roundtrip test.
- **Footer error flash** — Failed clipboard operations now show `❌ Clipboard error: ...` for 3 seconds, no more silent failure.

#### Clipboard troubleshooting (Debian / Xfce / Kitty / XRDP)

```bash
sudo apt install xclip xsel xfce4-clipman-plugin
pgrep -af xrdp-chansrv      # must be running (default with xrdp)
# ~/.config/kitty/kitty.conf:
clipboard_control write-clipboard write-primary read-clipboard read-primary no-append
```

With `xclip` installed, the app picks `SubprocessFirst` strategy automatically and writes directly into the X11 selection — exactly the path `xrdp-chansrv` syncs to the RDP channel.

### What's New in v0.81.0

- **Claude Code 2.1.117 Startup Dialog** — The permission mode dialog is replaced by a unified multi-section startup dialog covering Permission Mode, Model, Effort, Session Name, Worktree, and Remote Control. Navigation: `Tab`/`Shift+Tab` between sections, `↑↓` in lists, `←→` for radios. All values persist to `config.yaml` and are pre-selected on next launch.
- **Auto Mode** — New `auto` permission mode (`--permission-mode auto`) as the 6th variant, sorted after `acceptEdits` matching the Shift+Tab cycle order of Claude Code. Lets Claude check each tool call for risky actions and prompt injection — ideal for long-running tasks.
- **Model, Effort, Session Name, Worktree Flags** — `--model sonnet|opus`, `--effort low|medium|high|xhigh|max`, `--name <session>`, `--worktree <name>` now configurable from the startup dialog.
- **Remote Control via CLI flag** — The 4-second `/remote-control` slash-command hack is replaced by the official `--remote-control` flag. Starts reliably without timing-dependent race conditions.
- **18 new unit tests** — Test count now 99 (96 unit + 3 integration), up from 81.

### What's New in v0.80.0

- **Internal Refactor Release** — `handle_key_event` (1,375 lines) split into 15 focused methods per overlay/pane; `src/update/mod.rs` (986 lines) split into six submodules (`log`, `state`, `version`, `release_notes`, `check`, `install`). Behavior preserved 1:1.
- **Optional `pdf-export` Feature** — Typst PDF toolchain is now behind a default-enabled Cargo feature. `cargo build --no-default-features` produces a smaller binary without PDF support.
- **Multi-OS CI** — Test job now runs on Linux, macOS, and Windows (previously Linux only).
- **Explicit Style Contract** — `rustfmt.toml` and `clippy.toml` codify formatting and lint thresholds.
- **Integration Tests** — New `tests/cli.rs` exercises `--help`, `--version`, and unknown-flag handling (81 total tests, up from 78).

### What's New in v0.79.0

- **Cross-File Link Resolution** — HTML export now auto-converts referenced `.md` files and rewrites links. When README.md links to USAGE.md or INSTALL.md, all files are converted to HTML with working links.

### What's New in v0.78.0

- **HTML Anchor Links Fixed** — Internal anchor links (`[text](#heading)`) now work in browser preview and HTML export. Headings get auto-generated `id` attributes via shared `slugify()` function.

### What's New in v0.77.0

- **Unified Export System** — All export paths (PDF, Markdown preview, Syntax preview) now share the same configurable values. 14 hardcoded values replaced by config fields.
- **7 New Document Settings** — Table Font Size, Header Font Size, Line Height, Code Block BG, Heading Separator, Table Cell Padding, Blockquote Border now editable in F8.
- **Consistent Preview Filenames** — Browser previews use `{project}-{file}-{date}.html` naming instead of random temp names.
- **Bug Fix** — `pre code` CSS incorrectly used table font size instead of code font size.

### What's New in v0.76.0

- **Configurable Font Sizes** — Body Font Size and Code Font Size are now editable in Document Settings (F8 → Document). Code blocks use their own size independently from tables.

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
| F11 | Universal Paste — System-Clipboard in aktive Pane einfügen (Workaround für XRDP / defektes Bracketed-Paste-Forwarding) |
| Rechtsklick | Paste aus System-Clipboard in Pane unter dem Cursor (entspricht Kittys `mouse_map right press paste`) |

**Siehe [USAGE.md](USAGE.md) für alle Tastenkürzel und detaillierte Bedienungsanleitung.**

### Neu in v0.87.0

- **Async-Clipboard im Worker-Thread** — `copy_to_clipboard()` dispatcht jetzt an einen dedizierten `clipboard-worker`-Thread und returnt sofort mit `ClipboardOutcome::Submitted`. Der Event-Loop pollt einmal pro Frame `take_pending_outcome()` und zeigt nur bei realem `Failed` den Footer-Flash. Die UI bleibt responsiv, selbst wenn der X-Server-Clipboard die volle 500 ms-Timeout-Spanne aus v0.86.4 ausschöpft. Paste bleibt synchron (Caller brauchen das Ergebnis sofort für PTY/Editor-Inject).

### Neu in v0.86.4

- **Clipboard-Subprocess-Timeout (500 ms)** — Wahre Ursache des "App reagiert auf nichts mehr"-Symptoms unter XRDP: `xclip -i` und `xsel -i` blockieren indefinit, wenn die X11-Selection-Owner-Negotiation keinen Empfänger findet. Da Copy/Paste synchron im Main-Thread laufen, fror der gesamte Event-Loop ein. `run_with_stdin()` und `run_capture()` warten jetzt maximal 500 ms und killen den Child bei Timeout — Fallback auf nächsten Helper bzw. OSC 52 statt Hänger.
- **`CLAUDE_WORKBENCH_CLIPBOARD=osc52` Kill-Switch** — Erzwingt die neue `Osc52Only`-Strategy: arboard/xclip/xsel/wl-copy/-paste werden komplett übersprungen, ausschließlich OSC 52 ans Terminal. Für Sessions, in denen der X-Server-Clipboard komplett kaputt ist. Weitere Werte: `arboard`, `subprocess`. `--clipboard-diag` zeigt den aktiven Override.

### Neu in v0.86.3

- **XRDP/Kitty Selection-Freeze Fix (Linksklick-Drag)** — Unter XRDP verschluckt das RDP-Backend bei aktivem `EnableMouseCapture` zuverlässig den `ButtonRelease`-Event, während `ButtonPress` und Bewegung durchkommen. Folge: `mouse_selection.selecting` bleibt `true`, der Highlight friert ein, weitere Klicks werden als Drag-Erweiterung interpretiert. Zwei defensive Fixes: **Esc** cancelt jetzt eine aktive Mausselektion (globaler Handler, greift nach Modal-Dismissals), und ein neues `Down(Left)` clearet stale Selection vorab — Klicks auf Footer/Scrollbar/Modal lassen keinen eingefrorenen Highlight mehr zurück. Ergänzt v0.86.2, das nur Rechtsklick adressierte.

### Neu in v0.86.2

- **Rechtsklick = Paste in PTY/Preview-Panes** — `EnableMouseCapture` schluckte Rechtsklicks, sodass Kittys `mouse_map right press ungrabbed paste_from_clipboard` unter XRDP nie greifen konnte. Rechtsklick nutzt jetzt dieselbe Fallback-Kette wie F11 (arboard → xclip → xsel → wl-paste) und pastet in die Pane unter dem Cursor. Bei aktiver Alt+Drag-Mausselektion clearet Rechtsklick stattdessen die Selektion — behebt den XRDP-only Bug "Bildschirm blockiert, Markierung lässt sich nicht entfernen".

### Neu in v0.86.1

- **macOS Boot-Hänger Hotfix** — v0.86.0 rief beim Start `check_command()` für die vier neuen Clipboard-Helper auf, mit Fallback auf `$SHELL -i -c "..."` wenn direct exec scheiterte. Auf macOS sind die Helper typischerweise nicht installiert → 4× `fish -i -c` mit Job-Control-Init → Terminal-State korrumpiert nach Beenden. Helper-Detection nutzt jetzt pure-Rust PATH-Lookup (`crate::clipboard::which()`), kein Subprocess.

### Neu in v0.86.0

- **Clipboard-Fallback-Kette** — `arboard` → `xclip` → `xsel` → `wl-copy` → OSC 52 für Copy; `arboard` → `xclip -o` → `xsel -b -o` → `wl-paste` für Paste. Stellt Clipboard-Sync unter XRDP / Kitty / Xfce wieder her, wenn Kittys Bracketed-Paste-Forwarding nicht funktioniert.
- **F11 Universal Paste** — Liest das System-Clipboard über die Fallback-Kette und schreibt direkt in die aktive Pane (Claude / LazyGit / Terminal / Preview-Edit). Umgeht Kittys Bracketed-Paste-Bridge komplett — der Workaround, wenn Kitty unter XRDP das System-Clipboard nicht lesen kann.
- **Startup-Dependency-Check** — Erkennt `xclip`, `xsel`, `wl-copy`, `wl-paste` beim Start. Auf Linux ohne Helper erscheint 10 Sekunden lang ein gelbes Banner im Footer. F12 (Hilfe) zeigt die aktive Strategie und gefundene Helper.
- **`--clipboard-diag` CLI** — `claude-workbench --clipboard-diag` druckt die aktive Strategie, Helper-Pfade, relevante Umgebungsvariablen (`DISPLAY`, `WAYLAND_DISPLAY`, `XRDP_SESSION`, `XDG_SESSION_TYPE`, `SSH_TTY`) und führt einen Copy/Paste-Roundtrip aus.
- **Footer-Error-Flash** — Fehlgeschlagene Clipboard-Operationen zeigen jetzt `❌ Clipboard error: ...` für 3 Sekunden, kein stilles Versagen mehr.

#### Clipboard-Troubleshooting (Debian / Xfce / Kitty / XRDP)

```bash
sudo apt install xclip xsel xfce4-clipman-plugin
pgrep -af xrdp-chansrv      # muss laufen (Standard bei xrdp)
# ~/.config/kitty/kitty.conf:
clipboard_control write-clipboard write-primary read-clipboard read-primary no-append
```

Mit `xclip` installiert wählt die App automatisch die `SubprocessFirst`-Strategie und schreibt direkt in die X11-Selection — exakt der Pfad, den `xrdp-chansrv` zum RDP-Kanal synct.

### Neu in v0.81.0

- **Claude Code 2.1.117 Startup-Dialog** — Der Permission-Mode-Dialog wird durch einen vereinheitlichten Multi-Sektion-Startup-Dialog ersetzt: Permission Mode, Model, Effort, Session-Name, Worktree und Remote Control. Navigation: `Tab`/`Shift+Tab` zwischen Sektionen, `↑↓` in Listen, `←→` für Radio-Buttons. Alle Werte werden in `config.yaml` persistiert und beim nächsten Start vorselektiert.
- **Auto Mode** — Neuer `auto` Permission Mode (`--permission-mode auto`) als 6. Variante, nach `acceptEdits` einsortiert entsprechend der Shift+Tab-Reihenfolge von Claude Code. Claude prüft jeden Tool-Call auf riskante Aktionen und Prompt-Injection — ideal für Long-Running Tasks.
- **Model, Effort, Session-Name, Worktree Flags** — `--model sonnet|opus`, `--effort low|medium|high|xhigh|max`, `--name <session>`, `--worktree <name>` jetzt im Startup-Dialog konfigurierbar.
- **Remote Control als CLI-Flag** — Der 4-Sekunden `/remote-control` Slash-Command-Hack ist durch das offizielle `--remote-control` Flag ersetzt. Startet zuverlaessig ohne Timing-abhaengige Race Condition.
- **18 neue Unit-Tests** — Test-Anzahl jetzt 99 (96 unit + 3 integration), vorher 81.

### Neu in v0.80.0

- **Internes Refactor-Release** — `handle_key_event` (1.375 Zeilen) in 15 fokussierte Methoden pro Overlay/Pane aufgeteilt; `src/update/mod.rs` (986 Zeilen) in sechs Submodule (`log`, `state`, `version`, `release_notes`, `check`, `install`) zerlegt. Verhalten bleibt 1:1.
- **Optionales `pdf-export` Feature** — Die Typst-PDF-Toolchain steht jetzt hinter einem default-aktivierten Cargo-Feature. `cargo build --no-default-features` erzeugt ein kleineres Binary ohne PDF-Support.
- **Multi-OS CI** — Der Test-Job laeuft jetzt auf Linux, macOS und Windows (vorher nur Linux).
- **Expliziter Style-Contract** — `rustfmt.toml` und `clippy.toml` dokumentieren Formatierungs- und Lint-Schwellenwerte.
- **Integrationstests** — Neue `tests/cli.rs` prueft `--help`, `--version` und unbekannte Flags (81 Tests gesamt, vorher 78).

### Neu in v0.79.0

- **Cross-File Link-Aufloesung** — Der HTML-Export konvertiert jetzt automatisch referenzierte `.md`-Dateien mit und schreibt die Links um. Wenn README.md auf USAGE.md oder INSTALL.md verlinkt, werden alle Dateien zu HTML konvertiert und die Links funktionieren.

### Neu in v0.78.0

- **HTML Anker-Links repariert** — Interne Anker-Links (`[text](#heading)`) funktionieren jetzt in der Browser-Vorschau und im HTML-Export. Ueberschriften erhalten automatisch `id`-Attribute ueber eine gemeinsame `slugify()`-Funktion.

### Neu in v0.77.0

- **Einheitliches Export-System** — Alle Export-Pfade (PDF, Markdown-Vorschau, Syntax-Vorschau) nutzen jetzt die gleichen konfigurierbaren Werte. 14 hardcoded Werte durch Config-Felder ersetzt.
- **7 neue Document Settings** — Tabellen-Schriftgroesse, Header-Schriftgroesse, Zeilenhoehe, Code-Block-Hintergrund, Ueberschrift-Trennlinie, Tabellen-Zellenabstand, Blockquote-Rahmen jetzt in F8 editierbar.
- **Konsistente Vorschau-Dateinamen** — Browser-Vorschauen verwenden `{Projekt}-{Datei}-{Datum}.html` statt zufaelliger Temp-Namen.
- **Bug-Fix** — `pre code` CSS nutzte faelschlicherweise Tabellen- statt Code-Schriftgroesse.

### Neu in v0.76.0

- **Konfigurierbare Schriftgroessen** — Body Font Size und Code Font Size sind jetzt in den Document Settings (F8 → Document) editierbar. Code-Bloecke verwenden eine eigene Groesse unabhaengig von Tabellen.

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
