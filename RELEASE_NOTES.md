# Release Notes

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
