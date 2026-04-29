# Release Notes

## Version 0.86.0 (29.04.2026)

### Fixed
- **Clipboard unter XRDP / Kitty / Xfce funktioniert wieder** — Auf Debian 13
  (Trixie) mit Kitty als Terminal und XRDP-Zugriff funktionierte das Clipboard
  in beide Richtungen nicht: F9 (Copy Last N Lines) zeigte zwar den Footer-
  Flash, aber das System-Clipboard blieb leer; Paste in die PTY-Panes
  (Claude/LazyGit/Terminal) kam gar nicht erst durch. Ursache war die
  unzuverlässige OSC-52-Brücke unter XRDP und das Fehlen jeglichen Fallbacks
  für `arboard`.

### Added
- **Mehrstufige Clipboard-Fallback-Kette in `src/clipboard.rs`** — Copy
  versucht in dieser Reihenfolge: `arboard` → `xclip` → `xsel` → `wl-copy`
  → OSC 52. Paste analog: `arboard` → `xclip -o` → `xsel -b -o`
  → `wl-paste --no-newline`. Die Subprocess-Helper schreiben direkt in die
  X11-Selection — exakt der Pfad, den `xrdp-chansrv` zum RDP-Kanal synct.
- **`ClipboardOutcome`-Enum** — Statt `bool` liefert `copy_to_clipboard()`
  jetzt `Arboard` / `Xclip` / `Xsel` / `WlCopy` / `Osc52` / `Failed(reason)`.
  Aufrufer können den genutzten Backend für Diagnose und User-Feedback
  auslesen.
- **`ClipboardStrategy` mit Environment-Detection** — `ArboardFirst` (macOS,
  native Linux) vs `SubprocessFirst` (XRDP/X11 mit xclip/xsel verfügbar).
  Detection via `XRDP_SESSION` und `XDG_SESSION_TYPE`, einmalig per
  `OnceLock` cached. In XRDP-Sessions wird arboard übersprungen, weil
  dessen `wayland-data-control`-Feature dort hängen kann.
- **F11 — Universal Paste** — Liest das System-Clipboard via Fallback-Kette
  und schreibt direkt in die aktive Pane: raw bytes für Claude (kein
  bracketed paste), `\x1b[200~…\x1b[201~`-gewrappt für LazyGit/Terminal,
  `editor.insert_str()` für Preview-Edit. **Schlüssel-Workaround für XRDP**,
  weil Kitty's bracketed-paste-Bridge dort versagt — F11 umgeht sie komplett.
- **Startup-Dependency-Check für Clipboard-Helpers** — `DependencyReport`
  in `src/setup/dependency_checker.rs` um `ClipboardHelpers`-Struct
  erweitert (xclip / xsel / wl-copy / wl-paste). Auf Linux ohne Helper
  erscheint 10 Sekunden lang ein gelber Footer-Banner:
  `⚠ xclip / xsel / wl-copy fehlen — Clipboard eingeschränkt. F12 für Details.`
- **Footer-Error-Flash** — Fehlgeschlagene Clipboard-Operationen zeigen
  3 Sekunden lang `❌ Clipboard error: <reason>` (rot). Vorrang vor
  `✓ N Zeilen` (Copy-Flash) und Auto-Save-Flash. Schluss mit stillem
  Versagen bei F9 unter XRDP.
- **CLI-Option `--clipboard-diag`** — `claude-workbench --clipboard-diag`
  startet die TUI nicht, sondern druckt: aktive `ClipboardStrategy`,
  Helper-Pfade (`xclip` / `xsel` / `wl-copy` / `wl-paste`), Environment-
  Variablen (`DISPLAY` / `WAYLAND_DISPLAY` / `XDG_SESSION_TYPE` /
  `XRDP_SESSION` / `SSH_TTY`) und einen Copy/Paste-Roundtrip-Test mit
  Match-Verifikation.
- **F11-Eintrag im F12-Help-Screen** — Neue Zeile dokumentiert F11 als
  Universal Paste mit XRDP-Workaround-Hinweis.

### Changed
- **`paste_from_clipboard()` mit Fallback-Kette** — Bisher nur `arboard.get_text()`
  ohne Fallback, gab `None` ohne Fehler zurück wenn arboard scheiterte.
  Jetzt versucht arboard → xclip → xsel → wl-paste.
- **`copy_to_clipboard()`-Aufrufer** in `src/app/clipboard.rs:93,114,193`
  werten den `ClipboardOutcome` aus und setzen den Footer-Error-Flash bei
  `Failed(_)`. F9 setzt `last_copy_time` nur noch bei tatsächlichem Erfolg.
- **README.md (DE+EN) und Skill-Doku aktualisiert** — Neuer Abschnitt
  "Clipboard troubleshooting" mit XRDP/Kitty-Setup-Schritten:
  ```bash
  sudo apt install xclip xsel xfce4-clipman-plugin
  pgrep -af xrdp-chansrv
  # ~/.config/kitty/kitty.conf:
  clipboard_control write-clipboard write-primary read-clipboard read-primary no-append
  ```

### Tests
- 5 neue Unit-Tests in `src/clipboard.rs`: `test_outcome_label_and_success`,
  `test_which_finds_common_binary`, `test_which_returns_none_for_missing`,
  `test_diag_collect_does_not_panic`, plus erhaltener `test_base64_encode`.
- Test-Count: **103 unit + 3 integration = 106 Tests** (vorher 99 / 102).

### Architecture
- Bestehende `osc52_copy()` und `base64_encode()` (Z. 40-81 in der alten
  `src/clipboard.rs`) bleiben als Stufe 5 der Fallback-Kette erhalten.
- `arboard 3.6` mit `wayland-data-control`-Feature unverändert — bleibt
  wertvoll auf nativen Wayland-Sessions, wird in XRDP-Sessions nur
  übersprungen.

## Version 0.85.0 (28.04.2026)

### Fixed
- **Windows Self-Update** — `archive-zip` + `compression-zip-deflate` Features
  für `self_update` aktiviert (Cargo.toml). PowerShell `Compress-Archive`
  erzeugt DEFLATE-komprimierte ZIPs auf Windows; ohne diese Features schlug
  die Extraktion mit `ArchiveNotEnabled: Archive extension 'zip' not supported`
  fehl. Linux/macOS-Pfade unverändert (weiterhin `.tar.gz`).

## Version 0.84.0 (28.04.2026)

### Added
- **Startup-Indikator vor `ratatui::init()`** — Drei Zeilen Status auf Stderr,
  während Config geladen und Panes gespawnt werden:
  ```
  claude-workbench v0.84.0 starting...
    config loaded (12 ms)
    spawning panes...
  ```
  Sichtbar bevor der Alternate-Screen aktiviert wird; Stderr bleibt im
  normalen Buffer und stört die TUI-Ausgabe nicht. Wirkt sich primär unter
  Windows aus, wo ConPTY-Spawn spürbar mehr Zeit kostet als Unix-PTY.
- **Wall-clock-Timing für config-Load** — `Instant::now()` vor `load_config()`,
  Differenz wird ausgegeben. Macht Latenz-Regressionen sichtbar.

### Notes
- Output kommt nur im echten TUI-Modus (kein Output bei `--check-update`,
  `--update-to <version>` oder `--fake-version <v>` falls deren Codepfad
  vor der Stderr-Zeile abbricht — `main()` verlässt diese Modi vor
  `async_main()`).

## Version 0.83.0 (28.04.2026)

### Performance
- **Lazy-Init für LazyGit + Terminal Panes** — Beim App-Start werden nur noch
  PTYs für Panes gespawnt, deren Visibility-Flag (`config.ui.show_lazygit` /
  `config.ui.show_terminal`) auf `true` steht. Bisher wurden alle drei PTYs
  immer beim Start sequenziell gespawnt — auch wenn die Panes laut Config
  unsichtbar waren.
  - **Auswirkung:** Da der Default beider Flags `false` ist, werden statt
    drei nur ein PTY (Claude) beim Start aufgebaut. Der ConPTY-Handshake unter
    Windows ist signifikant langsamer als Unix-PTY, daher ist der Effekt dort
    am größten.

### Added
- **`App::ensure_pty_for_pane(PaneId)`** in `src/app/pty.rs` — Idempotente
  Spawn-Methode für `Terminal` und `LazyGit` Panes. No-op wenn das PTY bereits
  existiert. Spawn-Fehler werden in `terminal_error`/`lazygit_error` erfasst
  (rendert weiterhin als roter Border + Fehlertext, siehe v0.82.0).

### Changed
- **F6 Terminal-Toggle** — Ruft `ensure_pty_for_pane(Terminal)` direkt nach
  dem Visibility-Flip auf, bevor der `cd`-Sync läuft. Beim ersten Toggle
  entsteht das PTY; bei späteren Toggles bleibt der Shell-State (History,
  laufende Prozesse) erhalten.
- **F5 LazyGit-Toggle** — Verhalten unverändert: nutzt weiterhin
  `restart_lazygit_in_current_dir()`, das jeden Toggle frisch im aktuellen
  Verzeichnis startet (gewollt, damit LazyGit immer das richtige Repo zeigt).
- **`check_and_restart_exited_ptys` und `restart_single_pty`** prüfen jetzt,
  ob das Pane sichtbar ist, bevor sie ein abgestürztes PTY restarten —
  unsichtbare Panes (z. B. Terminal nach `lazygit` Crash bei `auto_restart:
  true`) bleiben tot bis zum nächsten F-Key-Toggle.

## Version 0.82.0 (28.04.2026)

### Fixed
- **Windows: Terminal-Pane bleibt tot unter PowerShell 7.6** — Bislang setzte
  `Config::default()` den `shell_path` hardcoded auf `/bin/bash`; auf Windows
  existiert dieser Pfad nicht, der PTY-Spawn schlug fehl, und der Fehler
  wurde still verschluckt — F6 zeigte ein unbenutzbares Pane.
- **PTY-Spawn-Fehler werden jetzt sichtbar** — Nicht nur Claude, sondern auch
  LazyGit und Terminal zeigen bei Spawn-Fehler einen roten Border + Fehlertext
  inklusive verwendetem Kommando. Bisheriges Verhalten (`if let Ok(pty) = …`
  ohne Else-Zweig) ließ Fehler unbemerkt verpuffen.

### Added
- **`config::default_shell_path()` plattform-bewusst** — Windows-Lookup-Reihenfolge
  `%COMSPEC%` → `pwsh.exe` (PATH-Probe via `-NoLogo -NoProfile -Command "exit 0"`)
  → `powershell.exe` → `C:\Windows\System32\cmd.exe`. Unix-Lookup: `$SHELL` →
  `/bin/bash`. Wird in `Config::default()` für `terminal.shell_path` verwendet.
- **`lazygit_error` und `terminal_error` Felder** in `App` (analog zu
  `claude_error`); rendert in `terminal_pane.rs` als roter Border + ⚠-Header
  + Kommando + Original-Fehlertext.

### Changed
- **`setup/dependency_checker.rs`: Windows-Pfade** — `find_executable_path()`
  nutzt `where` auf Windows statt `which`; mehrzeilige Ausgabe wird per
  `lines().next()` korrekt verarbeitet. `check_available_shells()` prüft
  `pwsh`/`powershell`/`cmd` auf Windows, `bash`/`zsh`/`fish`/`sh` sonst —
  `cmd` mit `/?` statt `--version`. Shell-Fallback (`-i -c "<cmd>"`) wird auf
  Windows übersprungen, weil `cmd.exe` keine Entsprechung kennt.
- **macOS/Linux: `$SHELL` wird respektiert** — User mit explizit gesetztem
  `$SHELL` (z. B. Fish via Homebrew) bekommen ihren Shell als Default; Konfigs
  mit explizitem `terminal.shell_path` bleiben unangetastet.

### Tests
- 3 neue Unit-Tests in `src/config.rs`: `default_shell_path_is_nonempty`,
  `default_shell_path_unix_is_absolute_or_shell_env`,
  `config_default_terminal_shell_path_is_set`. Plus `#[cfg(windows)]`
  `default_shell_path_windows_is_not_unix`. Suite jetzt 99 Tests (+3
  Integration), alle grün.

## Version 0.81.0 (22.04.2026)

### Added
- **Auto Mode for Claude Code 2.1.117** — New `auto` permission mode
  (`--permission-mode auto`) as the 6th variant in `ClaudePermissionMode`,
  sorted right after `acceptEdits` matching the Shift+Tab cycle order of
  Claude Code itself. Auto Mode lets Claude check each tool call for risky
  actions and prompt injection before executing, ideal for long-running
  tasks.
- **Unified Claude Startup Dialog** — The permission mode dialog is replaced
  by a multi-section startup dialog covering Permission Mode, Model, Effort,
  Session Name, Worktree and Remote Control. Navigation: `Tab`/`Shift+Tab`
  between sections, `↑↓` in lists, `←→` for radios, Home/End/Left/Right for
  text input. All values persist to `~/.config/claude-workbench/config.yaml`
  and get pre-selected on the next launch.
- **`--model` flag** — New `ClaudeModel` enum with `Unset` (CLI default),
  `Sonnet`, `Opus`. Emitted as `--model sonnet` / `--model opus` when set.
- **`--effort` flag** — New `ClaudeEffort` enum with 6 levels
  (Unset/Low/Medium/High/XHigh/Max) emitted as `--effort <level>`.
- **`--name` and `--worktree` flags** — Free-text input fields with full
  UTF-8-safe cursor navigation (char-index based, same pattern as the MC Edit
  search dialog from v0.20.0).
- **`StartupOptions` struct** — Bundles permission mode, model, effort,
  session name, worktree and remote control into one value passed to
  `App::build_claude_command()` and `App::init_claude_pty()`.
- **4 new `ClaudeConfig` fields** — `default_model`, `default_effort`,
  `default_session_name`, `default_worktree` with `#[serde(default)]` for
  forward-compatible YAML migration.
- **18 new unit tests** — `src/types.rs` adds 8 tests covering `Auto`
  variant position/flag/name, `ClaudeModel` and `ClaudeEffort` CLI flags
  and unique-name invariants. `src/app/pty.rs` adds 10 tests covering
  `build_claude_command` across all flag combinations (shell fallback, auto
  mode, model, effort, session name, worktree, remote control, YOLO mode,
  empty values, all-flags combined). Test count now 99 (96 unit + 3
  integration) up from 81.

### Changed
- **Remote Control uses CLI flag instead of slash-command hack** — The
  4-second `/remote-control` slash-command timer is replaced by the official
  `--remote-control` CLI flag introduced in Claude Code 2.1.x. Eliminates
  the timing-dependent race condition and starts reliably. Removes
  `remote_control_send_time` field from `App` and the `poll_remote_control_send`
  method from `src/app/update.rs`.
- **`build_claude_command` signature** — Now takes `&StartupOptions` instead
  of a single `ClaudePermissionMode`. Flags are emitted in order:
  `--permission-mode` / `--dangerously-skip-permissions` → `--model` →
  `--effort` → `--name` → `--worktree` → `--remote-control`.
- **`PermissionModeState` rewritten** — Replaces the single-list dialog with
  a `DialogSection` enum (6 variants) and per-section indices plus text
  fields. Legacy `open()` / `open_with_default()` replaced by
  `open_with_defaults()` taking all 6 default values.

## Version 0.80.0 (21.04.2026)

### Added
- **`pdf-export` Cargo Feature (default-enabled)** — The typst-based PDF pipeline
  (`typst`, `typst-pdf`, `typst-library`, `typst-kit`, `comemo`, `ecow`) is now gated
  behind the `pdf-export` feature. `cargo build --no-default-features` produces a
  smaller binary without the PDF toolchain, while `cargo build` keeps the default
  behavior. Callers of `export_markdown()` receive a descriptive error when PDF
  export is requested in a build without the feature.
- **CLI integration tests (`tests/cli.rs`)** — First integration-test suite verifies
  `--help` shows usage including `--check-update`, `--version` prints the Cargo
  package version, and unknown flags are rejected. Runs alongside the 78 unit tests
  via `cargo test`.
- **`rustfmt.toml` and `clippy.toml`** — Style/lint contract is now explicit:
  edition 2021, Unix newlines, reordered imports/modules, cognitive-complexity
  threshold 30, too-many-arguments threshold 8.
- **Multi-OS test coverage in CI** — `ci.yml` test job now matrixes over
  `ubuntu-latest`, `macos-latest`, `windows-latest` (previously only Linux),
  matching the release-build target platforms.

### Changed
- **`src/app/keyboard.rs` split into 15 focused methods** — `handle_key_event`
  shrunk from 1,375 lines (the single function body) to a ~180-line orchestrator
  that dispatches to dedicated handlers per overlay and pane: `handle_fuzzy_finder_key`,
  `handle_update_dialog_key`, `handle_export_chooser_key`, `handle_active_dialog_key`,
  `handle_menu_key`, `handle_about_key`, `handle_help_key`,
  `handle_permission_mode_dialog_key`, `handle_claude_startup_key`,
  `handle_global_shortcut`, `handle_pane_resize_key`,
  `handle_file_browser_pane_key`, `handle_preview_pane_key`,
  `handle_preview_edit_key`, `handle_preview_readonly_key`,
  `handle_terminal_pane_key`. Behavior is preserved 1:1.
- **`src/update/mod.rs` split into submodules** — The 986-line update module is
  now composed of six focused files: `log.rs` (file-based update log),
  `state.rs` (`UpdateState`, `UpdateCheckResult`, `UpdateResult`),
  `version.rs` (`version_newer`, `CURRENT_VERSION`),
  `release_notes.rs` (fetch + platform filtering),
  `check.rs` (sync/async update checks, `get_target`),
  `install.rs` (`perform_update_*`, `restart_application`). All public items are
  re-exported from `update/` for source-compatibility.
- **Regex `unwrap()` audit** — The 49 `Regex::new(…).unwrap()` sites in
  `src/filter.rs` are now `.expect("static regex pattern must compile")`, so any
  future pattern-compilation failure emits a descriptive panic instead of a bare
  `called Option::unwrap() on None`.
- **Single-file modules flattened** — `src/input/mod.rs` → `src/input.rs` and
  `src/session/mod.rs` → `src/session.rs`. Module paths and imports are unchanged.

### Internal
- This is a pure refactor/infrastructure release — no user-facing behavior
  changes beyond the optional `--no-default-features` build. Existing
  shortcuts, dialogs, and workflows are untouched.

## Version 0.79.0 (30.03.2026)

### Added
- **HTML Export Cross-File Link Resolution** — When a Markdown file references other `.md`
  files (e.g., `[Usage](USAGE.md)`), the HTML export now automatically converts all
  referenced Markdown files to HTML and rewrites the links to point to the generated
  HTML files. This ensures link integrity when previewing documentation in the browser.
- 6 new unit tests for `collect_md_links()` and `fix_md_links()`: simple links, fragment
  preservation, absolute URL filtering, dot-slash normalization, unknown link passthrough.

### Changed
- **`markdown_to_html()` signature** — Now returns `Vec<PathBuf>` instead of `PathBuf`,
  containing the primary HTML file and all linked dependency files for proper cleanup tracking.
- **Internal refactoring** — Extracted `convert_single_md()` helper for single-file conversion
  without disk I/O, enabling the two-phase convert-then-rewrite approach.

### Security
- Path traversal guard on linked `.md` files — resolved paths must remain under the source
  file's directory (same guard as used for image paths).

## Version 0.78.0 (30.03.2026)

### Fixed
- **Internal Anchor Links in HTML Export** — Clicking `[text](#section)` links in the
  browser preview (`o` key) and HTML export now correctly jumps to the target heading.
  Root cause: `pulldown-cmark` does not auto-generate `id` attributes on headings.
  New `inject_heading_ids()` function walks the event stream, collects heading text,
  slugifies it, and injects `id="slug"` into each heading tag.

### Changed
- **Shared `slugify()` function** — The heading-to-slug conversion (used for anchor links)
  is now shared between the HTML exporter (`markdown.rs`) and the Typst/PDF exporter
  (`typst_pdf.rs`) via `browser::slugify()`, ensuring consistent behavior across both
  export paths.

### Added
- 6 new unit tests for heading ID injection: basic heading, spaces, special characters,
  anchor link resolution, multiple heading levels, and inline code in headings.

## Version 0.77.0 (28.03.2026)

### Added
- **Unified Export/Preview System** — All export paths (PDF, HTML Markdown preview, Syntax
  preview) now share the same configurable values. 14 new config fields replace hardcoded values
  across `template.rs`, `typst_pdf.rs`, and `syntax.rs`.
- **7 New Document Settings (F8)** — Table Font Size, Header Font Size, Line Height,
  Code Block BG color, Heading Separator color, Table Cell Padding, and Blockquote Border
  are now editable in the Document Settings dialog.
- **Consistent Preview Filenames** — Browser previews (`o` key) now use the same naming
  convention as PDF export: `{project}-{filename}-{dd.mm.yyyy}.html` instead of random
  `cwb-preview-XXXXXXXX.html` / `cwb-syntax-XXXXXXXX.html` temp names.

### Changed
- **TemplateContext Unification** — `syntax.rs` (code file preview) now uses `TemplateContext`
  for footer styling and config-driven font sizes/line heights, consistent with `markdown.rs`.
- **Heading Separator Consistency** — H1 separator line now uses the same color (`#cccccc`)
  in both HTML preview and PDF export. Previously inconsistent (`#eee` vs `#cccccc`).

### Fixed
- **Pre-code font-size** — `pre code` blocks now correctly inherit the configured
  `code_font_size` instead of using a hardcoded value.
