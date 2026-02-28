# Release Notes

## Version 0.60.0 (28.02.2026)

### Added
- **Remote Control Toggle** im Permission Mode Dialog: Neue Checkbox unterhalb der
  5 Permission-Modi erlaubt es, Claude Code im Remote Control Modus zu starten
  (`claude remote-control`). Session kann dann von anderen Geräten (Browser, Handy)
  weitergenutzt werden.
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
