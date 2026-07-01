# Usage Guide / Bedienungsanleitung

**[English](#english) | [Deutsch](#deutsch)**

---

<a name="english"></a>
## English

### Keyboard Shortcuts

#### Global (work everywhere, including terminals)
| Key | Action |
|-----|--------|
| Ctrl+Q/C | Quit |
| Ctrl+P | Fuzzy Finder |
| Ctrl+O | Open Markdown Preview (path dialog with tab-complete) |
| Ctrl+X | Export Markdown as MD/PDF (format chooser + target path) |
| Ctrl+Alt+E (Ctrl+Option+E on macOS) | Open in External Editor (Preview file when Preview active, otherwise FileBrowser selection; configure in Settings F8) |
| Ctrl+V | Paste from clipboard (in input dialogs) |
| Ctrl+Shift+W | Setup Wizard |
| F1 | Toggle File Browser |
| F2 | Toggle Preview Pane |
| F3 | Maximize/Restore Preview (fullscreen editor) |
| F4 | Focus Claude Code |
| F5 | Toggle LazyGit (restarts in current directory) |
| F6 | Toggle User Terminal (syncs to current directory) |
| F7 | Claude Settings (~/.claude) |
| F8 | Settings |
| F9 | File Menu (File Browser) / Copy last command block (Terminal) or last N lines (Claude, LazyGit) |
| Shift+F9 | Copy last N lines with interactive count input (Terminal panes) |
| F10 | About |
| F12 | Help |
| Esc | Close Dialogs/Help |
| Alt+Shift+Left | Shrink File Browser width |
| Alt+Shift+Right | Grow File Browser width |
| Alt+Shift+Up | Shrink Claude pane height |
| Alt+Shift+Down | Grow Claude pane height |

#### Pane Resizing (Mouse)
Drag pane borders to resize interactively. Changes are saved automatically.

#### Context-specific (FileBrowser/Preview only)
| Key | Action |
|-----|--------|
| ? | Help (not in terminals) |

#### File Browser
| Key | Action |
|-----|--------|
| j/k, Up/Down | Navigate |
| Enter | Open/Enter |
| Backspace/Left | Parent directory |
| . | Toggle hidden files |
| o | Open in browser |
| O | Open in Finder |
| g | Go to path |

#### Preview Pane (Read-Only)
| Key | Action |
|-----|--------|
| j/k, Up/Down | Scroll 1 line |
| PageUp/Down | Scroll 10 lines |
| Home/End | Jump to start/end |
| h/l | Scroll horizontally (left/right) |
| Shift+Scroll | Horizontal scroll with mouse wheel |
| E | Enter edit mode |
| Ctrl+S | Enter selection mode |

#### Search & Replace
| Key | Action |
|-----|--------|
| / or Ctrl+F | Start search |
| Ctrl+H | Search & Replace (Edit mode) |
| Tab | Switch Find/Replace fields |
| Ctrl+N / Ctrl+P | Navigate matches |
| Ctrl+I | Toggle case sensitivity |
| Enter | Confirm / Replace current |
| Ctrl+R | Replace all matches |
| Esc | Close search |

#### Editor (Edit Mode)
| Key | Action |
|-----|--------|
| Ctrl+S | Save |
| Ctrl+A | Toggle Autosave on/off |
| Ctrl+Z | Undo last change |
| Ctrl+Shift+Z | Redo |
| Ctrl+C / Cmd+C | Copy selection, or current line if nothing selected |
| Ctrl+X / Cmd+X | Cut selection, or current line if nothing selected |
| Ctrl+V / Cmd+V | Paste from clipboard |
| Ctrl+Y | Delete current line |
| Shift+Arrow | Extend selection |
| Esc | Exit (autosave if enabled, otherwise confirm) |

Note: `Ctrl+C` and `Ctrl+X` without an active selection operate on the current line. `Ctrl+X` only triggers the export dialog in ReadOnly mode or FileBrowser — not in Edit mode.

**Autosave Behavior:**
- When autosave is ON, changes are saved automatically on: Esc (exit edit), file switch, directory change
- Preview title shows `[AUTO]` tag when autosave is active in edit mode
- Footer button shows `Auto:ON` / `Auto:OFF` to indicate current state
- Footer right side shows `AUTO:ON` (green) or `AUTO:OFF` (dim) permanently
- After autosave triggers, footer briefly shows `✓ SAVED` (green flash, 2s)
- Toggle via Ctrl+A in edit mode or F8 Settings

**Horizontal Scrolling:** Use `h`/`l` keys or `Shift+Scroll` for horizontal scrolling in Edit mode.

**Mouse:** Click/drag vertical and horizontal scrollbars for navigation.

#### Terminal Panes
| Key | Action |
|-----|--------|
| \\ + Enter | Insert newline in Claude Code (F4) |
| Ctrl+S | Start selection |
| F9 | Copy last command block (Terminal, full scrollback) or last N visible lines (Claude/LazyGit, N = `pty.copy_lines_count`, default 50) |
| Shift+F9 | Copy last N lines with interactive count input |
| Shift+PgUp/PgDn | Scroll 10 lines |
| Shift+Up/Down | Scroll 1 line |
| Alt+Left/Right | Word navigation |
| PageUp | Jump to line start (Home) |
| PageDown | Jump to line end (End) |

**Mouse-wheel scrolling:** When the inner application has mouse tracking enabled (Claude Code fullscreen renderer, LazyGit), wheel events are forwarded to the application so it scrolls its own view. On the alternate screen without mouse tracking (e.g. `less`, `vim`), arrow keys are sent instead. Otherwise (plain shell) the wheel scrolls the local scrollback buffer as before. `Shift+PgUp/PgDn` always scrolls the local scrollback.

#### User Terminal Prefix Key (F6, Ctrl+B)

In the **User Terminal pane (F6)** all keys — including **F1–F12** and **Ctrl+X / Ctrl+S / Ctrl+O / Ctrl+P / Ctrl+E** — are sent straight to the running program, so full-screen TUIs like **nano**, **mc** (Midnight Commander) and **vim** work correctly. Workbench commands are reached through a tmux-style prefix:

| Key | Action |
|-----|--------|
| Ctrl+B 1 / 2 / 3 | Toggle File Browser / Preview / Maximize Preview |
| Ctrl+B 4 / 5 / 6 | Focus Claude / Toggle LazyGit / Toggle Terminal |
| Ctrl+B ? (or h) | Help |
| Ctrl+B s | Start terminal selection |
| Ctrl+B c | Copy last command output |
| Ctrl+B Ctrl+B | Send a literal Ctrl+B to the terminal |
| Ctrl+Q | Quit Workbench (always reserved, never passed through) |

While the prefix is armed, the footer shows the available commands. This only affects the **User Terminal** — the **Claude** and **LazyGit** panes keep the regular shortcuts. Set `pty.terminal_prefix: ""` (or `"none"`) in `config.yaml` to disable passthrough and restore the legacy behavior; the default is `"ctrl+b"`.

#### Selection Mode (Ctrl+S in Terminal/Preview)
| Key | Action |
|-----|--------|
| j/k, Up/Down | Adjust selection |
| Shift+Up/Down | Adjust by 5 lines |
| g / G | Jump to buffer start/end |
| Enter / y | Copy to Claude |
| Ctrl+C | Copy to System Clipboard |
| Esc | Cancel |

#### F9 Copy Output (Terminal Panes)

Press **F9** to copy terminal output directly to the system clipboard. A green **„✓ N lines"** flash appears in the footer for 2 seconds.

- **Terminal pane (F6):** copies the **whole last command block** — from the last shell prompt to the bottom — read from the *full scrollback*, so older lines that scrolled off-screen are included. When no prompt boundary can be detected, it falls back to the last N lines of the full buffer.
- **Claude Code (F4) / LazyGit (F5):** copies the last **N visible lines** (these panes are TUI apps without a regular scrollback).

Configure the fallback/visible line count in `config.yaml`:
```yaml
pty:
  copy_lines_count: 50  # Default: 50. Increase for longer outputs (e.g. 100, 200)
```

Press **Shift+F9** to open an input dialog where you can enter a custom line count. The dialog pre-fills with the configured default value.

**Note:** F9 in the File Browser still opens the File Menu — the key is context-sensitive.

#### Dialog Input Fields
| Key | Action |
|-----|--------|
| Left/Right | Move cursor |
| Home/End | Jump to start/end |
| Backspace | Delete before cursor |
| Delete | Delete at cursor |
| Enter | Confirm |
| Esc | Cancel |

### Claude Fullscreen Mode

When all upper panes (File Browser, Preview, LazyGit, Terminal) are hidden, Claude Code automatically uses the full screen. Toggle panes with F1, F2, F5, F6 to enter/exit fullscreen mode.

### Claude Startup Dialog

At startup, a unified multi-section dialog lets you pre-configure Claude Code. Six sections: **Permission Mode** (6 modes incl. new `auto`), **Model** (sonnet/opus), **Effort** (low…max), **Session Name**, **Worktree**, and **Remote Control**.

| Key | Action |
|-----|--------|
| Tab / Shift+Tab | Switch between sections |
| ↑/↓ or k/j | Navigate items in Permission list |
| ←/→ or ↑/↓ | Select radio option in Model/Effort |
| Left/Right/Home/End | Cursor in text fields (Session/Worktree) |
| Backspace/Delete | Edit text fields |
| Space | Toggle Remote Control (when focused) |
| Enter | Confirm selection and persist to config |
| Esc | Use saved defaults |

All values persist to `~/.config/claude-workbench/config.yaml` under `claude.*` (`default_permission_mode`, `default_model`, `default_effort`, `default_session_name`, `default_worktree`, `remote_control`) and are pre-selected on next launch.

**Permission Modes (6):** `default`, `acceptEdits`, `auto`, `plan`, `bypassPermissions`, `dangerouslySkip`. The new `auto` mode lets Claude check each tool call for risky actions and prompt injection before executing — ideal for long-running tasks.

**Remote Control** now uses the official `--remote-control` CLI flag (replaces the former 4-second slash-command hack). Setting persists under `claude.remote_control`.

### File Browser Features

#### Git Status Integration
Color-coded indicators show file status:
- **Yellow (?)**: Untracked
- **Orange (M)**: Modified
- **Green (+)**: Staged
- **Gray (·)**: Ignored
- **Red (!)**: Conflict

The status bar shows file size, modification date, and git branch info.

#### File Menu (F9)
| Key | Action |
|-----|--------|
| n | New File |
| N | New Directory |
| r | Rename |
| u | Duplicate |
| c | Copy to... |
| m | Move to... |
| d | Delete |
| x | Export Markdown/PDF (format chooser for Markdown files) |
| y | Copy absolute path |
| Y | Copy relative path |
| g | Go to path |
| i | Add to .gitignore |

### Browser Preview (o key)
Open files in external applications:
- **HTML/HTM**: Direct browser opening
- **Markdown**: Converts to styled HTML with dark mode
- **PDF**: Opens in default PDF viewer
- **Images**: PNG, JPG, GIF, SVG, WebP in system viewer
- **Code files**: 500+ languages with syntax highlighting (Rust, Python, JS, Go, etc.)
- **Config files**: TOML, INI, CONF, CFG, ENV, YAML, JSON, XML, Properties, and more
- **Text files**: LOG, CSV, TSV, and other plain text files
- **O (Shift+O)**: Open directory in Finder/file manager

### Terminal Selection Mode

#### Keyboard Selection (Ctrl+S)
1. Press Ctrl+S in any terminal pane
2. Use j/k or arrows to adjust selection
3. Shift+Up/Down for 5-line jumps
4. g/G to jump to start/end of buffer
5. Enter or y to copy to Claude, Ctrl+C to copy to System Clipboard
6. Esc to cancel

#### Mouse Selection (Click & Drag)
1. Click and drag in Terminal or Preview panes to select text character-by-character
2. Release to automatically copy selection to System Clipboard
3. Selection is constrained to pane boundaries
4. Yellow highlight shows selected characters

**Note:** Mouse selection is character-level (not line-based like keyboard selection).

#### Intelligent Filtering
When copying to Claude, output is automatically filtered:
- Shell prompts removed (user@host$, >, >>>, etc.)
- Error messages and stack traces preserved
- Directory listing noise filtered
- Consecutive blank lines collapsed (max 2)
- Syntax auto-detection (Python, Rust, JavaScript, Bash, XML)

### Drag & Drop
- Drag files from File Browser to terminal panes
- Paths with spaces are automatically quoted
- Insert file paths directly into Claude or Terminal

### Git Integration
- Auto-checks for remote changes when entering a different repository
- Prompts to pull if remote is ahead
- Color-coded file status in file browser

### SSH Image Paste

When you connect from a Mac via SSH (e.g. iTerm2 → Linux server) and run
claude-workbench remotely, **text paste (`Cmd+V`) works** but **image paste
(`Ctrl+V`)** in the Claude pane does **not** — Claude Code on the remote
host can only see the local Linux clipboard, not your Mac pasteboard.

claude-workbench detects this scenario and integrates with
[`cc-clip`](https://github.com/ShunmeiCho/cc-clip), an external bridge that
forwards images from your Mac pasteboard to the remote host over an SSH
reverse tunnel.

**Setup (one-time):**

```bash
# 1. On your Mac
brew install shunmeicho/tap/cc-clip
cc-clip-daemon &     # or as a LaunchAgent

# 2. ~/.ssh/config (Mac side)
Host my-server
    RemoteForward 9998 localhost:9998

# 3. On the remote server
cargo install cc-clip
```

**Verification:**

```bash
claude-workbench --ssh-paste-diag
```

The diagnostic checks (1) SSH session detection, (2) `cc-clip` on `$PATH`,
and (3) reverse-tunnel reachability of port 9998. All three must pass for
image paste to work.

**Behavior in the TUI:**

- The first `Ctrl+V` in the Claude pane during an SSH session triggers a
  one-time yellow footer hint pointing here.
- The wizard (first run) shows an SSH-specific step with detection status
  and setup instructions when started over SSH.
- Settings (F8) → SSH lets you toggle the feature, override the helper
  path, or reset the dismissed-hint flag.

---

<a name="deutsch"></a>
## Deutsch

### Tastenkürzel

#### Global (funktionieren überall, auch in Terminals)
| Taste | Aktion |
|-------|--------|
| Ctrl+Q/C | Beenden |
| Ctrl+P | Fuzzy-Finder |
| Ctrl+O | Markdown-Preview öffnen (Pfad-Dialog mit Tab-Vervollständigung) |
| Ctrl+X | Markdown als MD/PDF exportieren (Formatwahl + Zielpfad) |
| Ctrl+Alt+E (Ctrl+Option+E auf macOS) | In externem Editor öffnen (Vorschau-Datei wenn Vorschau aktiv, sonst Dateibrowser-Auswahl; konfigurierbar in Einstellungen F8) |
| Ctrl+V | Aus Zwischenablage einfügen (in Eingabedialogen) |
| Ctrl+Shift+W | Setup-Assistent |
| F1 | Dateibrowser ein-/ausblenden |
| F2 | Vorschau-Bereich umschalten |
| F3 | Vorschau maximieren/wiederherstellen (Fullscreen-Editor) |
| F4 | Claude Code fokussieren |
| F5 | LazyGit umschalten (startet im aktuellen Verzeichnis neu) |
| F6 | Benutzer-Terminal umschalten (wechselt ins aktuelle Verzeichnis) |
| F7 | Claude Einstellungen (~/.claude) |
| F8 | Einstellungen |
| F9 | Datei-Menü (Dateibrowser) / Letzten Kommando-Block (Terminal) bzw. letzte N Zeilen (Claude, LazyGit) kopieren |
| Shift+F9 | Letzte N Zeilen mit interaktiver Eingabe kopieren (Terminal-Bereiche) |
| F10 | Über |
| F12 | Hilfe |
| Esc | Dialoge/Hilfe schließen |
| Alt+Shift+Links | Dateibrowser-Breite verkleinern |
| Alt+Shift+Rechts | Dateibrowser-Breite vergrößern |
| Alt+Shift+Oben | Claude-Bereich verkleinern |
| Alt+Shift+Unten | Claude-Bereich vergrößern |

#### Bereich-Größenänderung (Maus)
Ziehen Sie Bereichsgrenzen zum interaktiven Ändern der Größe. Änderungen werden automatisch gespeichert.

#### Kontext-spezifisch (nur FileBrowser/Preview)
| Taste | Aktion |
|-------|--------|
| ? | Hilfe (nicht in Terminals) |

#### Dateibrowser
| Taste | Aktion |
|-------|--------|
| j/k, Up/Down | Navigieren |
| Enter | Öffnen/Betreten |
| Backspace/Left | Übergeordnetes Verzeichnis |
| . | Versteckte Dateien umschalten |
| o | Im Browser öffnen |
| O | Im Finder öffnen |
| g | Zu Pfad springen |

#### Vorschau-Bereich (Nur-Lesen)
| Taste | Aktion |
|-------|--------|
| j/k, Up/Down | 1 Zeile scrollen |
| PageUp/Down | 10 Zeilen scrollen |
| Home/End | An Anfang/Ende springen |
| h/l | Horizontal scrollen (links/rechts) |
| Shift+Scroll | Horizontales Scrollen mit Mausrad |
| E | Bearbeitungsmodus starten |
| Ctrl+S | Auswahlmodus starten |

#### Suchen & Ersetzen
| Taste | Aktion |
|-------|--------|
| / oder Ctrl+F | Suche starten |
| Ctrl+H | Suchen & Ersetzen (Bearbeitungsmodus) |
| Tab | Such-/Ersetzungsfeld wechseln |
| Ctrl+N / Ctrl+P | Treffer navigieren |
| Ctrl+I | Groß-/Kleinschreibung umschalten |
| Enter | Bestätigen / Aktuellen ersetzen |
| Ctrl+R | Alle Treffer ersetzen |
| Esc | Suche schließen |

#### Editor (Bearbeitungsmodus)
| Taste | Aktion |
|-------|--------|
| Ctrl+S | Speichern |
| Ctrl+A | Autosave ein-/ausschalten |
| Ctrl+Z | Rückgängig |
| Ctrl+C / Cmd+C | Auswahl in Zwischenablage kopieren |
| Ctrl+X / Cmd+X | Auswahl in Zwischenablage ausschneiden |
| Ctrl+V / Cmd+V | Aus Zwischenablage einfügen |
| Ctrl+Y | Aktuelle Zeile löschen |
| Esc | Beenden (Autosave wenn aktiviert, sonst Bestätigung) |

**Autosave-Verhalten:**
- Bei aktiviertem Autosave werden Änderungen automatisch gespeichert bei: Esc (Editor verlassen), Dateiwechsel, Verzeichniswechsel
- Preview-Titel zeigt `[AUTO]` Tag wenn Autosave im Edit-Modus aktiv ist
- Footer-Button zeigt `Auto:ON` / `Auto:OFF` für den aktuellen Status
- Footer rechts zeigt permanent `AUTO:ON` (grün) oder `AUTO:OFF` (gedimmt)
- Nach Autosave-Auslösung zeigt Footer kurz `✓ SAVED` (grüner Flash, 2s)
- Umschaltbar via Ctrl+A im Edit-Modus oder F8 Einstellungen

**Block-Auswahl & MC Edit Legacy:**
| Taste | Aktion |
|-------|--------|
| Shift+Pfeiltaste | Text auswählen |
| Ctrl+F3 | Block-Markierung umschalten |
| Ctrl+F5 | Block kopieren (Legacy) |
| Ctrl+F6 | Block verschieben (Legacy) |
| Ctrl+F8 | Block löschen |

**Horizontales Scrollen:** `h`/`l` Tasten oder `Shift+Scroll` für horizontales Scrollen im Bearbeitungsmodus.

**Maus:** Vertikale und horizontale Scrollbars per Klick/Drag bedienbar.

#### Terminal-Bereiche
| Taste | Aktion |
|-------|--------|
| \\ + Enter | Zeilenumbruch im Claude Code (F4) |
| Ctrl+S | Auswahl starten |
| F9 | Letzten Kommando-Block (Terminal, voller Scrollback) bzw. letzte N sichtbare Zeilen (Claude/LazyGit, N = `pty.copy_lines_count`, Standard 50) kopieren |
| Shift+F9 | Letzte N Zeilen mit interaktiver Eingabe kopieren |
| Shift+PgUp/PgDn | 10 Zeilen scrollen |
| Shift+Up/Down | 1 Zeile scrollen |
| Alt+Links/Rechts | Wort-Navigation |
| PageUp | An Zeilenanfang springen (Home) |
| PageDown | An Zeilenende springen (End) |

**Mausrad-Scrollen:** Hat die innere Anwendung Mouse-Tracking aktiviert (Claude Code Fullscreen-Renderer, LazyGit), werden Wheel-Events an die Anwendung weitergeleitet, die dann selbst scrollt. Im Alternate Screen ohne Mouse-Tracking (z. B. `less`, `vim`) werden stattdessen Pfeiltasten gesendet. Andernfalls (normale Shell) scrollt das Mausrad wie bisher den lokalen Scrollback-Puffer. `Shift+PgUp/PgDn` scrollt immer den lokalen Scrollback.

#### Benutzer-Terminal Prefix-Taste (F6, Ctrl+B)

Im **Benutzer-Terminal-Bereich (F6)** gehen alle Tasten — inklusive **F1–F12** und **Ctrl+X / Ctrl+S / Ctrl+O / Ctrl+P / Ctrl+E** — direkt an das laufende Programm, sodass Vollbild-TUIs wie **nano**, **mc** (Midnight Commander) und **vim** korrekt funktionieren. Workbench-Befehle erreicht man über eine tmux-artige Prefix-Taste:

| Taste | Aktion |
|-------|--------|
| Ctrl+B 1 / 2 / 3 | Dateibrowser / Vorschau / Vorschau maximieren umschalten |
| Ctrl+B 4 / 5 / 6 | Claude fokussieren / LazyGit / Terminal umschalten |
| Ctrl+B ? (oder h) | Hilfe |
| Ctrl+B s | Terminal-Auswahl starten |
| Ctrl+B c | Letzte Kommando-Ausgabe kopieren |
| Ctrl+B Ctrl+B | Ein literales Ctrl+B an das Terminal senden |
| Ctrl+Q | Workbench beenden (immer reserviert, wird nie durchgereicht) |

Solange der Prefix „scharf" ist, zeigt die Fußzeile die verfügbaren Befehle. Dies betrifft nur das **Benutzer-Terminal** — die Bereiche **Claude** und **LazyGit** behalten die gewohnten Shortcuts. Mit `pty.terminal_prefix: ""` (oder `"none"`) in der `config.yaml` lässt sich das Durchreichen abschalten und das alte Verhalten wiederherstellen; Standard ist `"ctrl+b"`.

#### Auswahlmodus (Ctrl+S in Terminal/Vorschau)
| Taste | Aktion |
|-------|--------|
| j/k, Up/Down | Auswahl anpassen |
| Shift+Up/Down | Um 5 Zeilen anpassen |
| g / G | An Buffer-Anfang/-Ende springen |
| Enter / y | An Claude kopieren |
| Ctrl+C | Ins System-Clipboard kopieren |
| Esc | Abbrechen |

#### F9 Ausgabe kopieren (Terminal-Bereiche)

**F9** drücken, um Terminal-Ausgabe direkt in die Zwischenablage zu kopieren. Im Footer erscheint 2 Sekunden lang ein grüner **„✓ N Zeilen"**-Flash.

- **Terminal-Bereich (F6):** kopiert den **ganzen letzten Kommando-Block** — vom letzten Shell-Prompt bis zum Ende — aus dem *vollen Scrollback*, sodass auch ältere, weggescrollte Zeilen enthalten sind. Wird keine Prompt-Grenze erkannt, greift ein Fallback auf die letzten N Zeilen des vollen Puffers.
- **Claude Code (F4) / LazyGit (F5):** kopiert die letzten **N sichtbaren Zeilen** (diese Bereiche sind TUI-Apps ohne regulären Scrollback).

Fallback-/Sichtbar-Zeilenanzahl in `config.yaml` konfigurieren:
```yaml
pty:
  copy_lines_count: 50  # Standard: 50. Für längere Ausgaben erhöhen (z.B. 100, 200)
```

Mit **Shift+F9** öffnet sich ein Eingabedialog, in dem eine eigene Zeilenanzahl eingegeben werden kann. Der Dialog ist mit dem konfigurierten Standardwert vorausgefüllt.

**Hinweis:** F9 im Dateibrowser öffnet weiterhin das Datei-Menü — die Taste ist kontextsensitiv.

#### Dialog-Eingabefelder
| Taste | Aktion |
|-------|--------|
| Links/Rechts | Cursor bewegen |
| Home/End | An Anfang/Ende springen |
| Backspace | Vor Cursor löschen |
| Delete | An Cursor löschen |
| Enter | Bestätigen |
| Esc | Abbrechen |

### Claude Vollbildmodus

Wenn alle oberen Bereiche (Dateibrowser, Vorschau, LazyGit, Terminal) ausgeblendet sind, nutzt Claude Code automatisch den gesamten Bildschirm. Bereiche mit F1, F2, F5, F6 ein-/ausblenden um den Vollbildmodus zu aktivieren/deaktivieren.

### Claude Startup-Dialog

Beim Start erscheint ein vereinheitlichter Multi-Sektion-Dialog zur Vorkonfiguration von Claude Code. Sechs Sektionen: **Permission Mode** (6 Modi inkl. neuem `auto`), **Model** (sonnet/opus), **Effort** (low…max), **Session-Name**, **Worktree** und **Remote Control**.

| Taste | Aktion |
|-------|--------|
| Tab / Shift+Tab | Zwischen Sektionen wechseln |
| ↑/↓ oder k/j | Items in Permission-Liste navigieren |
| ←/→ oder ↑/↓ | Radio-Option in Model/Effort waehlen |
| Links/Rechts/Home/End | Cursor in Textfeldern (Session/Worktree) |
| Backspace/Entf | Textfelder editieren |
| Leertaste | Remote Control umschalten (wenn fokussiert) |
| Enter | Auswahl bestaetigen und in Config speichern |
| Esc | Gespeicherte Defaults verwenden |

Alle Werte werden in `~/.config/claude-workbench/config.yaml` unter `claude.*` persistiert (`default_permission_mode`, `default_model`, `default_effort`, `default_session_name`, `default_worktree`, `remote_control`) und beim naechsten Start vorselektiert.

**Permission Modes (6):** `default`, `acceptEdits`, `auto`, `plan`, `bypassPermissions`, `dangerouslySkip`. Der neue `auto`-Modus laesst Claude jeden Tool-Call auf riskante Aktionen und Prompt-Injection pruefen — ideal fuer Long-Running Tasks.

**Remote Control** nutzt jetzt das offizielle `--remote-control` CLI-Flag (ersetzt den frueheren 4-Sekunden-Slash-Command-Hack). Einstellung wird unter `claude.remote_control` gespeichert.

### Dateibrowser-Funktionen

#### Git-Status-Integration
Farbcodierte Indikatoren zeigen den Dateistatus:
- **Gelb (?)**: Ungetrackt
- **Orange (M)**: Modifiziert
- **Grün (+)**: Staged
- **Grau (·)**: Ignoriert
- **Rot (!)**: Konflikt

Die Statusleiste zeigt Dateigröße, Änderungsdatum und Git-Branch-Info.

#### Datei-Menü (F9)
| Taste | Aktion |
|-------|--------|
| n | Neue Datei |
| N | Neues Verzeichnis |
| r | Umbenennen |
| u | Duplizieren |
| c | Kopieren nach... |
| m | Verschieben nach... |
| d | Löschen |
| x | Markdown/PDF exportieren (Formatwahl für Markdown-Dateien) |
| y | Absoluten Pfad kopieren |
| Y | Relativen Pfad kopieren |
| g | Zu Pfad springen |
| i | Zur .gitignore hinzufügen |

### Browser-Vorschau (o-Taste)
Dateien in externen Anwendungen öffnen:
- **HTML/HTM**: Direkte Browser-Öffnung
- **Markdown**: Konvertierung zu gestyltem HTML mit Dark-Mode
- **PDF**: Öffnet im Standard-PDF-Viewer
- **Bilder**: PNG, JPG, GIF, SVG, WebP im System-Viewer
- **Code-Dateien**: 500+ Sprachen mit Syntax-Highlighting (Rust, Python, JS, Go, etc.)
- **Config-Dateien**: TOML, INI, CONF, CFG, ENV, YAML, JSON, XML, Properties u.v.m.
- **Text-Dateien**: LOG, CSV, TSV und andere Textdateien
- **O (Shift+O)**: Verzeichnis im Finder/Dateimanager öffnen

### Terminal-Auswahlmodus

#### Tastatur-Auswahl (Ctrl+S)
1. Ctrl+S in einem Terminal-Bereich drücken
2. j/k oder Pfeiltasten zur Anpassung der Auswahl
3. Shift+Up/Down für 5-Zeilen-Sprünge
4. g/G zum Springen an Anfang/Ende des Buffers
5. Enter oder y zum Kopieren an Claude, Ctrl+C ins System-Clipboard
6. Esc zum Abbrechen

#### Maus-Auswahl (Klicken & Ziehen)
1. In Terminal- oder Vorschau-Bereichen klicken und ziehen für zeichenweise Auswahl
2. Loslassen kopiert automatisch ins System-Clipboard
3. Auswahl ist auf Bereichsgrenzen beschränkt
4. Gelbe Hervorhebung zeigt ausgewählte Zeichen

**Hinweis:** Maus-Auswahl ist zeichenweise (nicht zeilenbasiert wie Tastatur-Auswahl).

#### Intelligentes Filtering
Beim Kopieren zu Claude wird die Ausgabe automatisch gefiltert:
- Shell-Prompts entfernt (user@host$, >, >>>, etc.)
- Fehlermeldungen und Stack-Traces bleiben erhalten
- Verzeichnislisten-Rauschen gefiltert
- Aufeinanderfolgende Leerzeilen komprimiert (max 2)
- Syntax-Erkennung (Python, Rust, JavaScript, Bash, XML)

### Drag & Drop
- Dateien aus dem Dateibrowser in Terminal-Bereiche ziehen
- Pfade mit Leerzeichen werden automatisch quotiert
- Dateipfade direkt in Claude oder Terminal einfügen

### Git-Integration
- Automatische Prüfung auf Remote-Änderungen beim Repository-Wechsel
- Aufforderung zum Pullen wenn Remote voraus ist
- Farbcodierte Dateistatus im Dateibrowser

### Bild-Paste über SSH

Wenn Du Dich von einem Mac via SSH (z. B. iTerm2 → Linux-Server) verbindest
und claude-workbench dort startest, funktioniert **Text-Paste (`Cmd+V`)**
problemlos, aber **Bild-Paste (`Ctrl+V`)** im Claude-Pane **nicht** —
Claude Code auf dem Server liest nur das lokale Linux-Clipboard, nicht
das Mac-Pasteboard.

claude-workbench erkennt diese Situation und integriert mit
[`cc-clip`](https://github.com/ShunmeiCho/cc-clip), einer externen Bridge
die Bilder vom Mac-Pasteboard über einen SSH-Reverse-Tunnel zum Server
überträgt.

**Einmaliges Setup:**

```bash
# 1. Auf dem Mac
brew install shunmeicho/tap/cc-clip
cc-clip-daemon &     # oder als LaunchAgent

# 2. ~/.ssh/config (Mac-Seite)
Host mein-server
    RemoteForward 9998 localhost:9998

# 3. Auf dem Server
cargo install cc-clip
```

**Verifikation:**

```bash
claude-workbench --ssh-paste-diag
```

Die Diagnose prüft (1) SSH-Session-Erkennung, (2) `cc-clip` auf `$PATH`
und (3) Reverse-Tunnel-Erreichbarkeit von Port 9998. Alle drei müssen
grün sein, damit Bild-Paste funktioniert.

**Verhalten im TUI:**

- Beim ersten `Ctrl+V` im Claude-Pane während einer SSH-Session erscheint
  ein einmaliger gelber Footer-Hinweis mit Verweis auf diese Doku.
- Der Setup-Wizard (Erststart) zeigt einen SSH-spezifischen Schritt mit
  Detection-Status und Setup-Anleitung, sofern er über SSH läuft.
- Einstellungen (F8) → SSH erlauben das Feature zu deaktivieren, den
  Helper-Pfad zu überschreiben oder den Hinweis-Status zurückzusetzen.
