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
| Ctrl+Shift+W | Setup Wizard |
| F1 | Focus File Browser |
| F2 | Toggle Preview Pane |
| F3 | Refresh File Browser |
| F4 | Focus Claude Code |
| F5 | Toggle LazyGit (restarts in current directory) |
| F6 | Toggle User Terminal (syncs to current directory) |
| F7 | Claude Settings (~/.claude) |
| F8 | Settings |
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
| g | Go to path |

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
| Ctrl+N / Ctrl+P | Navigate matches |
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
| Shift+Arrow | Select text |
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
| Ctrl+C | Copy to System Clipboard |
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
| y | Copy absolute path |
| Y | Copy relative path |
| g | Go to path |

### Browser Preview (o key)
Open files in external applications:
- **HTML/HTM**: Direct browser opening
- **Markdown**: Converts to styled HTML with dark mode
- **PDF**: Opens in default PDF viewer
- **Images**: PNG, JPG, GIF, SVG, WebP in system viewer
- **O (Shift+O)**: Open directory in Finder/file manager

### Terminal Selection Mode

#### Keyboard Selection (Ctrl+S)
1. Press Ctrl+S in any terminal pane
2. Use j/k or arrows to adjust selection
3. Shift+Up/Down for 5-line jumps
4. g/G to jump to start/end of buffer
5. Enter or y to copy to Claude, Ctrl+C to copy to System Clipboard
6. Esc to cancel

#### Mouse Selection
1. Alt+Click and drag to select lines
2. Release to enter selection mode
3. Enter or y to copy to Claude, Ctrl+C to copy to System Clipboard

**Note:** Regular click only focuses pane (no selection).

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

---

<a name="deutsch"></a>
## Deutsch

### Tastenkürzel

#### Global (funktionieren überall, auch in Terminals)
| Taste | Aktion |
|-------|--------|
| Ctrl+Q/C | Beenden |
| Ctrl+P | Fuzzy-Finder |
| Ctrl+Shift+W | Setup-Assistent |
| F1 | Dateibrowser fokussieren |
| F2 | Vorschau-Bereich umschalten |
| F3 | Dateibrowser aktualisieren |
| F4 | Claude Code fokussieren |
| F5 | LazyGit umschalten (startet im aktuellen Verzeichnis neu) |
| F6 | Benutzer-Terminal umschalten (wechselt ins aktuelle Verzeichnis) |
| F7 | Claude Einstellungen (~/.claude) |
| F8 | Einstellungen |
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
| g | Zu Pfad springen |

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
| Ctrl+N / Ctrl+P | Treffer navigieren |
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
| Shift+Pfeiltaste | Text auswählen |
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
| Ctrl+C | Ins System-Clipboard kopieren |
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
| y | Absoluten Pfad kopieren |
| Y | Relativen Pfad kopieren |
| g | Zu Pfad springen |

### Browser-Vorschau (o-Taste)
Dateien in externen Anwendungen öffnen:
- **HTML/HTM**: Direkte Browser-Öffnung
- **Markdown**: Konvertierung zu gestyltem HTML mit Dark-Mode
- **PDF**: Öffnet im Standard-PDF-Viewer
- **Bilder**: PNG, JPG, GIF, SVG, WebP im System-Viewer
- **O (Shift+O)**: Verzeichnis im Finder/Dateimanager öffnen

### Terminal-Auswahlmodus

#### Tastatur-Auswahl (Ctrl+S)
1. Ctrl+S in einem Terminal-Bereich drücken
2. j/k oder Pfeiltasten zur Anpassung der Auswahl
3. Shift+Up/Down für 5-Zeilen-Sprünge
4. g/G zum Springen an Anfang/Ende des Buffers
5. Enter oder y zum Kopieren an Claude, Ctrl+C ins System-Clipboard
6. Esc zum Abbrechen

#### Maus-Auswahl
1. Alt+Klicken und Ziehen um Zeilen auszuwählen
2. Loslassen zum Betreten des Auswahlmodus
3. Enter oder y zum Kopieren an Claude, Ctrl+C ins System-Clipboard

**Hinweis:** Normaler Klick fokussiert nur den Bereich (keine Auswahl).

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
