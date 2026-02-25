# Release Notes

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
