# Release Notes

## Version 0.70.0 (25.03.2026)

### Added
- **F9 Menu Export** — New "Export Markdown/PDF" entry (`x` key) in File Menu. Opens the format
  chooser dialog (same as Ctrl+X) for Markdown files shown in Preview pane.
- **Ctrl+X in F12 Help** — Export shortcut now documented in Global Shortcuts section.
  F9 description updated with `x:Export`.

### Changed
- **Ctrl+E Context-Aware** — External editor now opens the Preview pane's current file when
  Preview is active, instead of always using the File Browser's selected file.

### Fixed
- **Settings Persistence** — Settings dialog now auto-saves changes on Esc (previously only
  saved when pressing `s`/`S`, silently discarding changes on Esc).
- **Tab Completion** — Path tab-completion now works in OpenMarkdown and Export dialogs
  (previously only worked in GoToPath dialog).
- **Browser Selection for Export** — Exported files now open with the configured browser
  instead of system default.

## Version 0.69.0 (24.03.2026)

### Added
- **App-Dropdown for Browser/Editor** — Settings → Paths shows auto-detected installed browsers
  and editors in a dropdown menu instead of manual path input. Supports macOS (app bundle detection)
  and Linux (which-based detection). Includes Safari, Firefox, Chrome, Brave, Arc, Zen Browser,
  VS Code, Cursor, Zed, Sublime Text, Neovim, and more. "Custom path..." fallback for manual entry.
- **Ctrl+X: Markdown Export** — Export current Markdown file as Markdown copy or PDF. Two-step dialog:
  format chooser (MD/PDF) followed by target path dialog with tab-completion. PDF generation via
  Chrome headless `--print-to-pdf` or wkhtmltopdf fallback. Print-optimized HTML template with
  document header (title, author, date) and footer.
- **Export Directory** — New `export_dir` config field and Settings → Paths entry. Defaults to
  ~/Downloads. Configurable target directory for Markdown/PDF exports.
- **Ctrl+V Paste in Dialogs** — Clipboard paste support in all input dialogs (Ctrl+O path dialog,
  file operations, settings text fields). Uses existing `paste_from_clipboard()` arboard integration.
- **Ctrl+C Cancel in Dialogs** — Ctrl+C now closes input dialogs (consistent with terminal behavior).

### Changed
- **Settings → Paths**: Expanded from 4 to 5 items (+ Export Directory). Browser and External Editor
  fields now show `▼` dropdown indicator and display friendly app names with command in parentheses.
- **Command Splitting Bugfix**: `open_file_with_browser()` and `open_file_with_editor()` now correctly
  parse commands like `open -a "Brave Browser"` via quote-aware shell word splitting. Previously,
  the entire string was passed as a single binary name to `Command::new()`.
- **About Version**: Settings → About now shows version dynamically via `env!("CARGO_PKG_VERSION")`
  instead of hardcoded string.
- **Chrome --no-sandbox**: Only applied on Linux (containerized environments), not on macOS.
- **Export Error Feedback**: Export failures show a confirm dialog with error message instead of
  being silently swallowed.

### New Files
- `src/app_detector.rs` — Platform-specific browser/editor detection (macOS + Linux)
- `src/browser/pdf_export.rs` — Markdown→PDF export via Chrome headless or wkhtmltopdf

## Version 0.68.0 (24.03.2026)

### Added
- **Ctrl+O: Open Markdown Preview** — Dialog mit Tab-Completion zum Öffnen beliebiger Markdown-Dateien
  als HTML-Preview im Browser. Vorausgefüllt mit `~/.claude/plans/` für Claude-Pläne.
  Unterstützt Tilde-Expansion und alle previewbaren Dateitypen.
- **Ctrl+E: External Editor** — Öffnet die ausgewählte Datei im konfigurierten externen GUI-Editor
  (z.B. VS Code, Sublime Text). Konfigurierbar in Settings (F8) → Paths.
- **Browser-Konfiguration** — Neues `browser`-Feld in Config und Settings → Paths. Die `o`-Taste
  nutzt jetzt den konfigurierten Browser statt nur den System-Default.
- **External Editor-Konfiguration** — Neues `external_editor`-Feld in Config und Settings → Paths.

### Changed
- **Settings → Paths**: Kategorie von 2 auf 4 Items erweitert (+ Browser, External Editor)
- **Footer (FileBrowser)**: Zeigt `^O` (OpenMD) und `^E` (Editor) Shortcuts
- **Help-Screen**: Neue Ctrl+O und Ctrl+E Shortcuts dokumentiert
- **Browser-Opener**: Neues `open_file_with_browser()` und `open_file_with_editor()` in opener.rs
- **Refactoring**: Browser-Preview-Logik in wiederverwendbare `App::open_in_browser()` Methode extrahiert

## Version 0.67.0 (24.03.2026)

### Changed
- **F3 True Fullscreen**: Preview übernimmt jetzt das **gesamte Terminal** (inkl. Claude-Bereich und Footer).
  Vorher wurde nur die obere Hälfte maximiert, Claude blieb sichtbar.
  - `compute_layout` erhält neuen `preview_maximized` Parameter für Early Return
  - Footer wird im Fullscreen-Modus ausgeblendet (Guard: `footer.height > 0`)
  - Alle PTYs laufen im Hintergrund weiter, nur die Darstellung wird ausgeblendet
- **Footer-Label**: F3-Button zeigt "Restore" statt "MaxPrev" wenn Preview maximiert ist

## Version 0.66.0 (23.03.2026)

### Added
- **F3 Preview Maximize/Restore**: Neuer Fullscreen-Editor-Modus — F3 blendet alle anderen Panes aus
  und maximiert die Preview-Pane. Erneutes F3 stellt das vorherige Layout wieder her.
  - `SavedLayout` Struct speichert Sichtbarkeits-Flags vor dem Maximieren
  - F1/F2/F5/F6 räumen den Maximize-State automatisch auf
  - Preview wird eingeblendet + maximiert, falls vorher versteckt
  - Maximize-State ist transient (wird nicht in Config persistiert)

### Changed
- **F3 Tastenbelegung**: Von "Refresh File Browser" (redundant, Auto-Refresh vorhanden) zu "Maximize/Restore Preview"
- **FooterAction::Refresh** → `FooterAction::MaximizePreview` umbenannt
- **Help-Screen**: F3-Beschreibung in allen 3 Abschnitten aktualisiert (Global, Navigation, File Browser)
- **USAGE.md**: F3-Shortcuts in EN und DE Sektion aktualisiert

## Version 0.65.0 (23.03.2026)

### Changed
- **Architektur-Refactoring**: Monolithische `app.rs` (4.276 Zeilen) in 9 Module aufgeteilt:
  `mod.rs` (359), `keyboard.rs` (1275), `mouse.rs` (1000), `file_ops.rs` (500),
  `pty.rs` (266), `drawing.rs` (207), `clipboard.rs` (195), `git_ops.rs` (148), `update.rs` (144).
  Die `run()`-Methode ist jetzt ein dünner Orchestrator (~80 Zeilen).
- **tokio Features**: `"full"` auf `"rt-multi-thread"` reduziert (schnellere Builds)

### Fixed
- **Fish-Shell venv-Bug**: Terminal-Pane wechselte beim Start fälschlich das Verzeichnis, weil
  `sync_terminals_initial()` ein redundantes `cd` sendete, das Fish-Shell-Hooks (venv auto-activate)
  triggerte. Das `cd` war unnötig — PTYs starten bereits mit dem korrekten CWD.
- **Security Audit** (7 Findings behoben):
  1. Mutex-Poisoning: `lock().unwrap()` durch poison-resiliente `lock_or_recover()` ersetzt
  2. Unsafe libc: `localtime_r` mit Timestamp-Validierung und Return-Value-Check
  3. Path-Check: `canonicalize()` + `is_safe_destination()` für CopyFileTo/MoveFileTo
  4. Temp-File TOCTOU: Schreibt via File-Handle statt Pfad
  5. `is_safe_filename`: Char-Level Unicode-Check für `/` und `\`
  6. `WORKBENCH_FAKE_VERSION`: Nur in Debug-Builds verfügbar (`#[cfg(debug_assertions)]`)
  7. Regex in `fix_image_paths`: `LazyLock` statt Runtime-`unwrap()`

## Version 0.64.0 (22.03.2026)

### Added
- **Clipboard OSC 52 Fallback**: Clipboard funktioniert jetzt über SSH und auf headless
  Linux-Systemen (Debian 13 etc.). Arboard wird als Primary verwendet, bei Fehler automatisch
  OSC 52 Escape Sequences als Fallback. Neues Modul `src/clipboard.rs` konsolidiert alle
  Clipboard-Operationen.
- **UI-State Persistenz**: Pane-Sichtbarkeit (F1 FileBrowser, F2 Preview, F5 LazyGit,
  F6 Terminal) wird in `config.yaml` gespeichert und beim nächsten Start wiederhergestellt.
- **Ctrl+F9 Alias**: Interaktiver "Copy Last N Lines" Dialog wird jetzt sowohl mit
  Shift+F9 als auch Ctrl+F9 geöffnet.

### Fixed
- F9 Copy Flash-Indikator erscheint jetzt immer (auch bei OSC 52 Fallback)
- Clipboard-Aufrufe in Preview-Editor (copy_block, move_block, paste) nutzen
  zentrale Clipboard-Utility

## Version 0.63.1 (22.03.2026)

### Fixed
- **Security Audit**: 5 Findings behoben:
  1. Shell Injection in `dependency_checker.rs` (shell_escape für Argumente)
  2. Temp File Leakage (HTML-Preview-Dateien werden bei App-Exit gelöscht)
  3. Panicking `unwrap()` auf User-Input-Pfad durch `if let` ersetzt
  4. Path Traversal in New File/Directory/Rename Dialog (Dateinamen-Validierung)
  5. Supply Chain: `cross` auf v0.2.5 gepinnt in `release.yml`
- Clippy-Fix: `saturating_sub` in `terminal.rs`

## Version 0.63.0 (06.03.2026)

### Fixed
- **Doppelklick-Focus-Bug (Claude Pane)**: Klick auf das Claude-Pane setzt jetzt sofort
  den Fokus, auch wenn der Startup-Dialog angezeigt wird. Bisher blieb der Fokus auf dem
  vorherigen Pane, bis ein zweiter Klick erfolgte.
- **Startup-Dialog blockiert Pane-Wechsel**: Wenn der Claude Startup-Dialog sichtbar war
  und auf ein anderes Pane (z.B. Terminal) geklickt wurde, erzwang der Dialog-Dismissal
  den Fokus auf Claude. Jetzt schließt der Klick den Dialog und der Fokus geht korrekt
  an das angeklickte Pane.

### Added
- **Shift+F9 Interactive Copy**: Neuer Input-Dialog zur Laufzeit-Eingabe der Zeilenanzahl
  für "Copy Last N Lines". F9 bleibt schnell (Default aus config.yaml), Shift+F9 öffnet
  Dialog mit editierbarem Default-Wert.

## Version 0.62.0 (28.02.2026)

### Fixed
- **Remote Control**: Claude wird jetzt normal interaktiv gestartet (mit `--permission-mode`),
  anstatt als `claude remote-control` Server-Modus. Nach 4 Sekunden Startup-Delay wird
  `/remote-control` als Slash-Command an das Claude PTY gesendet. So wird Remote Control
  innerhalb einer interaktiven Session aktiviert, ohne "Start Session Block" Error.

## Version 0.60.1 (28.02.2026)

### Fixed
- **Remote Control Toggle**: `claude remote-control` ist kein gültiger CLI-Subcommand.
  Der ungültige Subcommand wurde entfernt. Stattdessen wird nach dem Claude-Start
  automatisch nach 2 Sekunden die Leertaste gesendet, um den QR-Code für den
  Remote-Zugriff anzuzeigen.

## Version 0.60.0 (28.02.2026)

### Added
- **Remote Control Toggle** im Permission Mode Dialog: Neue Checkbox unterhalb der
  5 Permission-Modi erlaubt es, Claude Code im Remote Control Modus zu starten.
  Session kann dann von anderen Geräten (Browser, Handy) weitergenutzt werden.
- **Space-Taste** schaltet den Remote Control Toggle im Dialog um.
- **Config-Persistierung**: `remote_control: true/false` wird in `config.yaml` gespeichert
  und beim nächsten Start wiederhergestellt.

## Version 0.59.1 (26.02.2026)

### Fixed
- **Terminal-Kopie verliert Leerzeichen**: Beim Kopieren von Text aus Terminal-Panes
  (Maus-Selektion und F9 „Copy Last N Lines") wurden Leerzeichen zwischen Wörtern
  entfernt. Ursache: Die vt100-Crate gibt für Space-Zellen `""` statt `" "` zurück.
  Fix: Helper-Methode `push_cell_content()` ersetzt leere Zellinhalte durch Leerzeichen
  in `extract_lines()`, `extract_last_n_lines()` und `extract_char_range()`.

## Version 0.59.0 (25.02.2026)

### Added
- **F9 „Copy Last N Lines"** in Terminal-Panes (Claude, LazyGit, Terminal):
  Kopiert die letzten N Zeilen des aktiven Terminal-Fensters in die Zwischenablage.
  F9 bleibt kontextsensitiv: im FileBrowser öffnet F9 weiterhin das File-Menü.
- **Footer-Flash** „✓ N Zeilen" (grün, 2 Sekunden) nach erfolgreichem Kopiervorgang.
- **F9-Button** im Footer-Kontext für Terminal-Panes (CopyLast).
- **Konfigurierbares `copy_lines_count`** in `config.yaml` (Standard: 50):
  ```yaml
  pty:
    copy_lines_count: 50   # Increase for longer outputs
  ```
- **F9-Shortcut** in der Hilfe (F12) unter dem Terminal-Abschnitt dokumentiert.
