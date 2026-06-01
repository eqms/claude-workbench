# Release Notes

## Version 0.90.2 (01.06.2026)

### Fixed

- **File-browser scroll position no longer snaps back during auto-refresh** —
  The v0.90.1 mouse-wheel fix exposed a pre-existing bug: the periodic
  file-browser auto-refresh (default every 2 s, `auto_refresh_ms`) called
  `refresh()` → `load_tree()` → `ListState::select(None)`, and ratatui resets
  a list's `offset` to 0 on `select(None)`. `refresh()` restored the selection
  by path but not the scroll offset, so the viewport jumped back to the top a
  couple of seconds after scrolling stopped. `refresh()` now saves the scroll
  offset before the tree rebuild and restores it afterward (clamped to the new
  entry count), so the scroll position survives auto-refresh. Covered by two
  new unit tests in `src/ui/file_browser.rs`.

## Version 0.90.1 (01.06.2026)

### Fixed

- **File-browser mouse-wheel now scrolls the viewport, not the selection** —
  Scrolling the wheel over the file-browser pane previously called
  `FileBrowserState::down()`/`up()`, which moved the *selection* instead of
  the viewport. Because ratatui couples a `ListState`'s `offset` to its
  `selected`, the highlight wandered while scrolling and the click hit-test
  (`idx = list_state.offset() + relative_y`) landed on the wrong row after a
  scroll. The wheel now adjusts the viewport offset directly (±3 rows, clamped
  to `[0, item_count - visible_height]`) and clamps the selection into the new
  visible window so ratatui does not snap the offset back on the next render.
  Visible height is cached each frame as `files.height - 3` (top border +
  bottom border + 1-line info bar), matching the click hit-test geometry. New
  pure helpers `scroll_files_pane()` and `clamp_selected_to_window()` in
  `src/app/mouse.rs`, covered by 10 co-located unit tests. The click hit-test
  itself is unchanged.

## Version 0.90.0 (11.05.2026)

### Fixed

- **Self-update `--update-to <version>` flag gated to debug builds only** —
  Release binaries now reject `--update-to` with `unexpected argument`,
  closing the unauthenticated downgrade vector (CR-02). The flag and its
  handler `run_update_to_version_cli` are wrapped in
  `#[cfg(debug_assertions)]` so they don't even compile into release
  artifacts. Paired with new `filter_restart_args()` in
  `src/update/install.rs` that strips one-shot flags
  (`--check-update`, `--update-to`, etc.) from `args[]` before
  `restart_application()` re-execs the new binary — preventing the
  infinite-downgrade-loop that an attacker could otherwise wedge a user
  into (IN-02 backport).
- **`tempfile::Builder` replaces predictable PDF/HTML temp paths** —
  `src/browser/pdf_export.rs` no longer constructs
  `$TMPDIR/{stem}-{dd.mm.yyyy}.html` by hand. `default_preview_file()`
  now returns `Result<tempfile::NamedTempFile>` opened with `O_EXCL`,
  and `App::temp_preview_files: Vec<NamedTempFile>` handles deletion
  via RAII (the manual `cleanup_temp_files()` is gone). Closes the
  symlink-to-`~/.ssh/authorized_keys` write-to-arbitrary-file attack
  vector on multi-user XRDP hosts (SEC-04 / CR-03).
- **Browser/editor allow-list in `src/browser/opener.rs`** — new
  `validate_program()` helper rejects anything that doesn't match
  `^[A-Za-z0-9_./-]+$` before reaching `std::process::Command::new()`.
  Called from both `open_file_with_browser` and `open_file_with_editor`.
  Hand-rolled `split_command()` removed in favour of `shlex::split` for
  POSIX-correct tokenization (SEC-02 / WR-01).
- **`$SHELL -i -c "<cmd>"` fallback removed from dependency probe** —
  `src/setup/dependency_checker.rs::check_command` no longer invokes
  an interactive shell to look up binaries. Direct `Command::new(name)`
  is sufficient because every probed dependency (`git`, `claude`,
  `lazygit`, shells themselves) is a real executable on supported
  systems, not a shell function. Removes both the injection-adjacent
  pattern and the fish job-control init side effect that caused
  macOS-startup hangs in earlier patch releases (SEC-03 / WR-02).
- **`clipboard::which()` checks the executable bit** — new
  `is_executable()` helper (`#[cfg(unix)]`, `mode() & 0o111 != 0`)
  augments the existing `is_file()` check. Non-executable PATH entries
  that happen to share a name with a real binary no longer cause
  "Permission denied" deep inside subprocess spawn — they're rejected
  during PATH resolution. Linux/macOS only; no-op on platforms without
  Unix file modes (WR-03).
- **`shlex::try_quote` errors propagate from `sync_terminals*`** —
  `src/app/pty.rs` extracted a `quote_path_for_cd()` helper; all three
  `sync_terminals*` call sites now `match` on its `Option<String>` and
  call `log_update(...)` on `None` instead of silently falling back to
  the unescaped path. Unescaped path bytes can no longer reach the PTY
  shell via this code path (WR-04).
- **Release selection uses `semver::Version::max_by`, not creation
  order** — `src/update/check.rs` now picks the highest-semver tag
  across all GitHub releases instead of trusting `releases[0]`
  (which is creation order). Tie-breaker: most recent `published_at`.
  A backdated patch release on an old branch can no longer suppress
  legitimate newer updates (WR-05). `semver = "1"` promoted from
  transitive to direct dependency.

### Changed

- **Phase 1 Security Hardening — Wave 1 of 3 shipped.** Wave 2
  (CI-side `zipsign` signing pipeline) and Wave 3 (client-side
  `.verifying_keys()` wiring on `Update::configure`) remain open. They
  close the HIGH-severity self-update finding (SEC-01 / CR-01) and are
  intentionally gated: Wave 2 requires an operator to generate an
  ed25519 keypair and add it to GitHub Actions secrets; Wave 3 must
  wait until at least two signed releases have shipped, or the
  verification step bricks auto-update for every existing user.
- **`.planning/` directory introduced** — GSD project planning
  artifacts are now committed to the repository: `PROJECT.md`,
  `ROADMAP.md`, `REQUIREMENTS.md`, `STATE.md`, `codebase/*.md`
  (refreshed structural maps), and per-phase plans/research/review
  files under `.planning/phases/`. Source of truth for what comes
  next — Phase 2 (Test Coverage + Reliability), Phase 3 (Refactor +
  Dependency Strategy), Phase 4 (Session Persistence).
- **Test coverage:** 111 → 130 passing tests. New coverage includes
  the `tempfile::NamedTempFile` lifecycle (suffix/prefix/uniqueness/
  auto-delete), CLI integration test
  `update_to_flag_not_present_in_release_build`, three
  `filter_restart_args` unit tests, `validate_program` (~12
  assertions), `check_command` after shell-fallback removal,
  `is_executable` rejection paths, `sync_terminals*` error
  propagation, and `semver` max-selection over creation-order.

## Version 0.89.0 (11.05.2026)

### Fixed
- **Mouse focus on first click for borders and scrollbars** — Clicking a
  pane border (resize handle, ±1 px tolerance) or scrollbar handle no
  longer requires a second click inside the pane content to actually
  focus that pane. New helper `pane_at_position()` resolves the click
  to the underlying pane and updates `active_pane` eagerly, before the
  resize/scrollbar special-cases short-circuit the event handler.

### Changed
- **Async job lifecycle modelled explicitly** — Replaced four
  `Option<Receiver<T>>` fields on `App` (`git_check_receiver`,
  `update_check_receiver`, `update_receiver`, `export_receiver`) with
  a `JobState<T> { Idle, Running(Receiver<T>) }` enum and a new
  `PollOutcome<T>` returned by `poll()`. Disconnected channels are now
  surfaced separately from "no result yet" so worker-thread death is
  no longer silently conflated with normal idle state.
- **`src/app/keyboard.rs` split into per-context submodules** —
  1421-line file decomposed into `keyboard/{mod, dialogs, global,
  preview, terminal, file_browser}.rs`. The dispatcher (`mod.rs`)
  shrinks to ~300 lines; each submodule owns a coherent slice of the
  input surface. No behavior change.
- **Dependency hardening based on audit 2026-05-11**:
  - `shell-escape 0.1.5` (unmaintained since 2017) replaced with
    `shlex 1.3` in `src/app/pty.rs` (4 call sites) and
    `src/setup/dependency_checker.rs`.
  - `pulldown-cmark 0.10` bumped to `0.13` to deduplicate with the
    transitive version pulled by `tui-markdown`. API migration in
    `src/browser/typst_pdf.rs`: `TagEnd::BlockQuote` is now a tuple
    variant; `Event::InlineMath` / `DisplayMath` are handled as raw
    text; new `Event::Start(Tag::DefinitionList…)` variants
    pass through via wildcard arm.
  - `self_update`'s `signatures` feature flag enabled in `Cargo.toml`
    (capability only; full signing rollout tracked in `SECURITY-NOTES.md`).

### Added
- **`SECURITY-NOTES.md`** — Operational security playbook for
  claude-workbench. Tracks the open audit findings (HIGH: self-update
  has no checksum/signature verification; tar-slip protection
  delegated; MEDIUM: browser-injection latent risk, shell `-i -c`
  fragility, predictable temp-file path) and lays out a two-half
  rollout plan for ed25519 signing via `zipsign`: CI workflow signs
  release archives with a key from GitHub Secrets first, then a
  later client release enables `verifying_keys()` once releases
  reliably ship `.sig` sidecars.

### Notes
- `crossterm 0.28 → 0.29` was attempted and reverted: the
  `tui-textarea` fork on `update-ratatui` is built against
  crossterm 0.28's `Event` types and its `From<Event> for Input`
  impl. Bumping breaks `editor.input(Event::Key(key))` call sites in
  the preview-edit handler. Fork must be rebased on crossterm 0.29
  (or upstream `tui-textarea` must support ratatui 0.30) before this
  dedupe can land.

---

## Version 0.88.0 (05.05.2026)

### Added
- **SSH image-paste hint + cc-clip integration** — Adresses the long-standing
  pain point that `Ctrl+V` (image paste) in the Claude pane silently fails
  when claude-workbench runs on a remote Linux host reached over SSH from a
  Mac. The remote Claude CLI reads the local X11 clipboard, which does not
  contain the Mac pasteboard image, so the keystroke ends in nothing.
  - New helper `clipboard::is_ssh_session()` checks `SSH_TTY` /
    `SSH_CONNECTION` (cached via `OnceLock`).
  - New `App` field `ssh_image_paste_hint: Option<(String, Instant)>` and
    matching `Footer` rendering branch (10 s, yellow ℹ banner, identical
    look to the existing `clipboard_warning` flash).
  - `Ctrl+V` in the Claude pane during an SSH session triggers the hint
    *once*; the keystroke is not consumed (`0x16` still flows to the PTY
    so the Claude CLI's own paste path is unchanged).
  - Persisted dismissed flag (`config.ssh.notification_dismissed`) silences
    the hint on subsequent presses; resettable from Settings.
- **Wizard step `SshImagePaste`** — Conditionally rendered between
  `ClaudeConfig` and `Confirmation` when `is_ssh_session()` returns true.
  Shows live `cc-clip` detection (`$PATH` lookup) and a 4-line setup
  recipe (`brew install shunmeicho/tap/cc-clip`, daemon, `~/.ssh/config`
  RemoteForward, `cargo install cc-clip`). The "[m] mark as configured"
  shortcut persists `notification_dismissed = true` via `generate_config()`.
  Total step count adapts dynamically (5 vs 6).
- **Settings category `SSH`** (F8 → Tab to "SSH") with three items:
  enable/disable toggle, helper-path override, and reset-hint action.
  Live banner shows whether the current process is in an SSH session.
- **`--ssh-paste-diag` CLI flag** — Stderr-only diagnostic before the TUI
  starts. Reports SSH-session detection (`SSH_TTY` / `SSH_CONNECTION`),
  `cc-clip` on `$PATH`, and TCP reachability of `127.0.0.1:9998` (the
  `ssh -R 9998:localhost:9998` reverse tunnel target). Pattern mirrors
  `--clipboard-diag`.
- **`SshConfig` struct** in `config.rs` with serde-default forward-compat:
  fields `enabled`, `image_paste_helper: Option<String>`,
  `notification_dismissed`. Old config files without an `ssh:` section
  load cleanly.
- **8 new unit tests**: 5 for `detect_ssh_session()` (env-var matrix,
  empty-string handling, no-panic guard), 3 for the SSH wizard step
  (chain wiring, mark-configured persistence, default not persisted).
  Total now **111 unit + 3 integration = 114 tests**.

### Architecture notes

- New submodule `src/app/ssh_paste.rs` keeps SSH-specific App glue
  isolated from the core keyboard router. The flash-trigger method
  `App::show_ssh_image_paste_hint()` lives there.
- claude-workbench cannot modify Claude Code (separate Anthropic CLI),
  so the integration deliberately stops at "detect, hint, recommend
  cc-clip". OSC 5522 (Kitty image-clipboard protocol) is intentionally
  not implemented — iTerm2 does not support it on the Mac side.

### Why cc-clip?

cc-clip is a small, focused, well-maintained external tool that solves
the SSH pasteboard-bridge problem cleanly: a daemon on the Mac reads
the pasteboard, an SSH `RemoteForward` exposes the daemon's TCP socket
on the remote host, and the `cc-clip` client on the server fetches the
image. Bundling the same logic into claude-workbench would duplicate
maintenance burden for very narrow win.

## Version 0.87.0 (30.04.2026)

### Added
- **Async-Clipboard im Worker-Thread** — Alle `copy_to_clipboard()`-Aufrufe
  werden jetzt an einen dedizierten Background-Thread (`clipboard-worker`)
  delegiert. Der Main-Loop returnt sofort mit `ClipboardOutcome::Submitted`
  (neue Variante), der Worker führt die echte X11/Wayland/OSC-52-Sequenz
  aus. Der App-Event-Loop pollt einmal pro Frame `take_pending_outcome()`
  und zeigt nur bei `Failed(reason)` den Footer-Error-Flash. Folge: Die
  UI bleibt **immer** responsiv, selbst wenn der X-Server-Clipboard
  hängt — die 500 ms-Wartezeit aus v0.86.4 ist jetzt komplett unsichtbar
  für den User.
- **`copy_to_clipboard_sync()`** — Synchroner Pfad als öffentliche API
  erhalten, wird intern vom Worker aufgerufen und vom `--clipboard-diag`
  genutzt (damit der Roundtrip-Report das echte Backend-Ergebnis zeigt
  statt nur "Submitted").
- **`take_pending_outcome()`** — Public API zum Abholen des zuletzt
  fertig gestellten Worker-Outcomes (None bei leerer Queue).

### Changed
- **Architektur**: Single-Worker-Thread + `mpsc::channel<ClipboardJob>` +
  `Mutex<Option<ClipboardOutcome>>`-Slot. Lazy-Init via `OnceLock`. Worker
  lebt bis Process-Exit (kein expliziter Shutdown — joinen würde bei
  noch hängenden Helper-Subprocessen deadlocken). Aktuelle Queue-Politik:
  jeder neue Outcome überschreibt den alten Slot (relevant ist immer der
  letzte Copy für die User-Wahrnehmung).
- **Paste bleibt synchron** — Caller brauchen den Text sofort zum
  PTY-/Editor-Inject, asynchron würde keinen Mehrwert bieten. Das
  500 ms-Subprocess-Timeout aus v0.86.4 reicht hier.

### Notes
- Die `Submitted`-Variante zählt für `is_success()` als true → Caller
  sehen sofort den grünen Copy-Flash, bevor das echte Outcome kommt.
- Worker-Thread ist nach Name auffindbar (`pthread_setname_np`-äquivalent
  via `Thread::Builder::name`) — hilft beim Debugging mit `top -H`.

## Version 0.86.4 (30.04.2026)

### Fixed
- **Clipboard-Subprocess-Hänger unter XRDP** — Wahre Wurzel des "App
  reagiert auf nichts mehr"-Symptoms: `xclip -selection clipboard -i`
  und `xsel --clipboard --input` blockieren unter XRDP indefinit, wenn
  die X11-Selection-Owner-Negotiation mit `xrdp-xorgxrdp` keinen
  Empfänger findet. Da `copy_to_clipboard()` und `paste_from_clipboard()`
  synchron im Main-Thread laufen, friert der gesamte Event-Loop ein
  (kein Render, kein Tastatur-/Maus-Input) — passt zum Symptom
  "Selektion bleibt, Esc/Pfeiltasten reagieren nicht". `--clipboard-diag`
  hing aus demselben Grund im Roundtrip-Test. Diagnose-bestätigt durch
  `^C`-Abbruch des Roundtrips.

### Changed
- **Subprocess-Timeout (500 ms) für alle Clipboard-Helper** —
  `run_with_stdin()` und `run_capture()` in `src/clipboard.rs` nutzen
  jetzt `wait_or_kill()`: Polling von `try_wait()` mit 20 ms-Intervall,
  nach 500 ms wird der Child gekillt und gereapt. Statt sekundenlang zu
  hängen, fällt die App jetzt sofort auf den nächsten Helper bzw. OSC 52
  zurück. `stderr` wird auf `Stdio::null()` gestellt, um Pipe-Deadlocks
  bei vollem stderr-Buffer zu verhindern.

### Added
- **`CLAUDE_WORKBENCH_CLIPBOARD`-ENV-Override** — Kill-Switch für
  Sessions, in denen die X-Server-Clipboard-Negotiation komplett kaputt
  ist:
  - `CLAUDE_WORKBENCH_CLIPBOARD=osc52` → neue `ClipboardStrategy::Osc52Only`
    überspringt arboard, xclip, xsel, wl-copy/-paste komplett. Copy
    sendet ausschließlich OSC 52 ans Terminal (Kitty/XTerm/iTerm
    übernehmen). Paste liefert `None` (OSC 52 hat keinen Read-Pfad).
  - `CLAUDE_WORKBENCH_CLIPBOARD=arboard` / `subprocess` für manuelles
    Strategy-Pinning, falls Auto-Detection daneben liegt.
  - Unbekannte Werte fallen zurück auf Auto-Detection (warning-frei).
- **`--clipboard-diag` zeigt ENV-Override** — Neuer "ENV override:"-Eintrag
  in der Diag-Ausgabe. Hilft beim Erkennen, ob der User im Override-Modus
  läuft.

### Notes
- v0.86.3 hat zwei Selection-Cleanup-Pfade hinzugefügt (Esc + Down(Left)
  vorab-Clear), aber das war nicht das Hauptproblem — der UI-Freeze kam
  vom hängenden xclip im Main-Thread. Beide Fixes bleiben aktiv und
  nützlich, sobald das Subprocess-Timeout-Problem behoben ist.
- Async-Clipboard im Worker-Thread (Main-Loop bleibt responsiv selbst
  wenn der Helper die volle 500 ms wartet) ist für v0.87.0 vorgesehen.

## Version 0.86.3 (30.04.2026)

### Fixed
- **Eingefrorene Mausselektion unter XRDP/Kitty** — Selektionen per
  Linksklick-Drag blieben nach dem Loslassen permanent stehen, der Pane
  fror visuell ein und reagierte nicht mehr auf weitere Klicks. Ursache:
  XRDPs RDP-Backend (`xrdp-xorgxrdp`) verschluckt bei aktivem
  `EnableMouseCapture` zuverlässig den `ButtonRelease`-Event,
  während `ButtonPress` und `ButtonMotion` durchkommen. Der Inhalt landet
  zwar korrekt in der Zwischenablage (Drag-Update läuft), aber
  `mouse_selection.selecting` bleibt `true`, weil `clear()` nur im
  `Up(Left)`-Match-Arm aufgerufen wird. Folge: Jeder weitere Klick wird
  als Drag-Erweiterung interpretiert, der Pane scheint blockiert. v0.86.2
  hatte das nur für Rechtsklick adressiert; Linksklick-Selektion blieb
  betroffen. Tritt nicht in nativen Shells (iTerm, Kitty lokal) auf — dort
  liefert das OS Up-Events zuverlässig.

### Added
- **Esc cancelt aktive Mausselektion** — Globaler Tasten-Handler in
  `src/app/keyboard.rs` ruft `mouse_selection.clear()` auf, wenn Esc
  gedrückt wird und `selecting=true` ist. Modal-Handler (Help, Settings,
  Dialoge etc.) werden nicht beeinflusst, da der Esc-Selection-Cancel
  erst nach allen Modal-Returns greift.

### Changed
- **`Down(Left)` clearet stale Selection vorab** — In `src/app/mouse.rs`
  wird beim Empfang eines neuen Linksklick-Down-Events zuerst eine
  hängengebliebene Selection geclearet, bevor (ggf.) eine neue gestartet
  wird. Klickt der User in einen Pane, beginnt `start()` direkt eine
  frische Selektion. Klickt er auf Footer, Scrollbar oder Modal,
  bleibt `selecting=false` zurück — kein eingefrorener Highlight mehr.

## Version 0.86.2 (29.04.2026)

### Fixed
- **Rechtsklick im Terminal-Pane unter XRDP/Kitty** — Bisher schluckte
  `EnableMouseCapture` alle Mausevents, sodass Kittys eigenes
  `mouse_map right press ungrabbed paste_from_clipboard` nicht griff
  (Bedingung `ungrabbed` ist nur erfüllt wenn die Maus *nicht* von der
  App gegrabbed ist). Folge: Rechtsklick fiel auf den `_ => {}`-Default-Arm
  in `handle_mouse_event`, machte nichts und ließ aktive Alt+Drag-
  Selektionen visuell stehen ("Bildschirm blockiert, Markierung lässt
  sich nicht entfernen").

### Added
- **Rechtsklick = Paste in der App** — Neuer Match-Arm in `src/app/mouse.rs`
  für `MouseEventKind::Down(MouseButton::Right)`:
  - Wenn eine Mouse-Selection (Alt+Drag) aktiv ist → Selection wird
    geclearet, kein Paste. Behebt das visuelle Hängenbleiben.
  - Sonst: Pane unter dem Cursor wird fokussiert (Claude / LazyGit /
    Terminal / Preview) und `paste_from_clipboard_to_active_pane()`
    wird aufgerufen — exakt derselbe Pfad wie F11 (arboard → xclip
    → xsel → wl-paste). Konsistent mit Kittys eigenem Verhalten,
    funktioniert auch wenn Mouse Capture aktiv ist.

## Version 0.86.1 (29.04.2026) — Hotfix

### Fixed
- **Boot-Hänger auf macOS** — v0.86.0 rief beim App-Start `DependencyReport::check()`
  auf, der für die vier neuen Clipboard-Helper (`xclip`, `xsel`, `wl-copy`,
  `wl-paste`) `check_command()` mit interactive-shell-Fallback (`$SHELL -i -c "..."`)
  ausführte, wenn direct-exec scheiterte. Auf macOS sind diese Helper
  typischerweise nicht installiert → 4× Fish-Init mit `-i`-Flag und
  Job-Control-Aktivierung → Terminal-State korrumpiert. Symptom: App friert
  beim Start, mehrfaches `Ctrl+C` nötig, danach nur Cursor oben links
  (Terminal nicht sauber wiederhergestellt). Linux blieb unbetroffen, weil
  dort `xclip` meist installiert ist und direct-exec greift.

### Changed
- **Pure-Rust Helper-Detection** — `ClipboardHelpers::check()` nutzt jetzt
  `crate::clipboard::which()` (PATH-Lookup ohne Subprocess) statt
  `check_command()`. Helper-Binaries sind keine Shell-Aliases, sondern
  einfache Executables — ein interactive-shell-Fallback ist hier nicht
  sinnvoll. Neue Hilfsfunktion `check_binary(name)` in
  `src/setup/dependency_checker.rs`. Versionsabfrage entfällt — die
  Strategy-Wahl in `src/clipboard.rs` braucht sie nicht.
- **Startup-Latenz** auf macOS reduziert: vorher mehrere Sekunden für die
  4 Helper-Probes, jetzt <1 ms (nur PATH-Splits in Rust).

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
