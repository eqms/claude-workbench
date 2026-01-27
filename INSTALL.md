# Installation Guide / Installationsanleitung

**[English](#english) | [Deutsch](#deutsch)**

---

<a name="english"></a>
## English

### Quick Install (Recommended)

**Linux / macOS — One-Liner:**
```bash
curl -fsSL https://raw.githubusercontent.com/eqms/claude-workbench/main/scripts/install.sh | bash
```

**Windows — PowerShell One-Liner:**
```powershell
irm https://raw.githubusercontent.com/eqms/claude-workbench/main/scripts/install.ps1 | iex
```

**Or download manually** from the [Releases](https://github.com/eqms/claude-workbench/releases) page.

### Installer Options

The install scripts support several options:

**Linux / macOS (`install.sh`):**
```bash
bash scripts/install.sh --help           # Show help
bash scripts/install.sh --check          # Check dependencies only
bash scripts/install.sh --local          # Build from source with cargo
bash scripts/install.sh --install-dir /usr/local/bin  # Custom install directory
```

**Windows (`install.ps1`):**
```powershell
.\scripts\install.ps1 -Help             # Show help
.\scripts\install.ps1 -Check            # Check dependencies only
.\scripts\install.ps1 -Local            # Build from source with cargo
.\scripts\install.ps1 -InstallDir C:\Tools  # Custom install directory
```

### Platform-Specific Instructions

#### Linux (x64 / ARM64)

```bash
# Recommended: Use the installer
curl -fsSL https://raw.githubusercontent.com/eqms/claude-workbench/main/scripts/install.sh | bash

# Or manual download:
# For x64:
curl -LO https://github.com/eqms/claude-workbench/releases/latest/download/claude-workbench-linux-x64
# For ARM64:
curl -LO https://github.com/eqms/claude-workbench/releases/latest/download/claude-workbench-linux-arm64

# Make it executable
chmod +x claude-workbench-linux-*

# Move to a directory in your PATH (optional)
sudo mv claude-workbench-linux-* /usr/local/bin/claude-workbench

# Run
claude-workbench
```

**Dependencies:**
- A terminal emulator with 256-color support
- Git (for git status integration)
- Optional: [Claude CLI](https://claude.ai/code) for AI-assisted development
- Optional: [LazyGit](https://github.com/jesseduffield/lazygit) for the Git pane

#### macOS (Apple Silicon / Intel)

```bash
# Recommended: Use the installer (handles quarantine automatically)
curl -fsSL https://raw.githubusercontent.com/eqms/claude-workbench/main/scripts/install.sh | bash

# Or manual download:
# For Apple Silicon (M1/M2/M3/M4):
curl -LO https://github.com/eqms/claude-workbench/releases/latest/download/claude-workbench-macos-arm64
# For Intel:
curl -LO https://github.com/eqms/claude-workbench/releases/latest/download/claude-workbench-macos-x64

# Make it executable
chmod +x claude-workbench-macos-*

# Remove quarantine attribute (first run only)
xattr -d com.apple.quarantine claude-workbench-macos-*

# Move to a directory in your PATH (optional)
sudo mv claude-workbench-macos-* /usr/local/bin/claude-workbench

# Run
claude-workbench
```

**Note:** On first run, macOS may block the application. Go to System Settings > Privacy & Security and click "Open Anyway". The installer script handles quarantine removal automatically.

**Dependencies:**
- Terminal.app, iTerm2, or another terminal emulator
- Git (pre-installed on macOS or via `xcode-select --install`)
- Optional: [Claude CLI](https://claude.ai/code) for AI-assisted development
- Optional: [LazyGit](https://github.com/jesseduffield/lazygit) (`brew install lazygit`)

#### Windows (x64 / ARM64)

**Option 1: PowerShell Installer (Recommended)**
```powershell
# One-liner install
irm https://raw.githubusercontent.com/eqms/claude-workbench/main/scripts/install.ps1 | iex

# Or with options
.\scripts\install.ps1 -Check    # Check dependencies first
.\scripts\install.ps1            # Install latest release
```

**Option 2: Direct Download**
1. Download the appropriate binary from the [Releases](https://github.com/eqms/claude-workbench/releases) page:
   - For x64: `claude-workbench-windows-x64.exe`
   - For ARM64 (Surface Pro X, etc.): `claude-workbench-windows-arm64.exe`
2. Move to a convenient location (e.g., `C:\Tools\`)
3. Add the location to your PATH (optional)
4. Run from PowerShell or Windows Terminal

**Option 2: PowerShell Installation**
```powershell
# Download (x64)
Invoke-WebRequest -Uri "https://github.com/eqms/claude-workbench/releases/latest/download/claude-workbench-windows-x64.exe" -OutFile "claude-workbench.exe"

# Download (ARM64)
# Invoke-WebRequest -Uri "https://github.com/eqms/claude-workbench/releases/latest/download/claude-workbench-windows-arm64.exe" -OutFile "claude-workbench.exe"

# Run
.\claude-workbench.exe
```

**Recommended Terminal:**
- [Windows Terminal](https://aka.ms/terminal) for best experience (256-color support, proper Unicode rendering)
- PowerShell or cmd.exe work but may have limited color support

**Dependencies:**
- Git for Windows: https://git-scm.com/download/win
- Optional: [Claude CLI](https://claude.ai/code) for AI-assisted development
- Optional: [LazyGit](https://github.com/jesseduffield/lazygit)

### Build from Source

If you prefer to build from source:

```bash
# Clone the repository
git clone https://github.com/eqms/claude-workbench.git
cd claude-workbench

# Build release version
cargo build --release

# The binary will be at target/release/claude-workbench
```

**Build Dependencies:**
- Rust toolchain (rustc, cargo): https://rustup.rs/
- C compiler (gcc/clang on Unix, MSVC on Windows)

### Configuration

On first run, Claude Workbench creates a configuration directory:
- Linux/macOS: `~/.config/claude-workbench/`
- Windows: `%APPDATA%\claude-workbench\`

Copy the example configuration:
```bash
cp config.yaml.example ~/.config/claude-workbench/config.yaml
```

### Troubleshooting

**"Permission denied" on Linux/macOS:**
```bash
chmod +x claude-workbench-*
```

**"App is damaged" on macOS:**
```bash
xattr -d com.apple.quarantine claude-workbench-macos-*
```

**Colors not displaying correctly:**
- Ensure your terminal supports 256 colors
- Set `TERM=xterm-256color` in your shell profile

**Claude CLI not found:**
- Install Claude CLI: https://claude.ai/code
- Ensure `claude` is in your PATH

---

<a name="deutsch"></a>
## Deutsch

### Schnellinstallation (Empfohlen)

**Linux / macOS — Ein Befehl:**
```bash
curl -fsSL https://raw.githubusercontent.com/eqms/claude-workbench/main/scripts/install.sh | bash
```

**Windows — PowerShell Ein Befehl:**
```powershell
irm https://raw.githubusercontent.com/eqms/claude-workbench/main/scripts/install.ps1 | iex
```

**Oder manuell herunterladen** von der [Releases](https://github.com/eqms/claude-workbench/releases)-Seite.

### Installer-Optionen

**Linux / macOS (`install.sh`):**
```bash
bash scripts/install.sh --help           # Hilfe anzeigen
bash scripts/install.sh --check          # Nur Abhängigkeiten prüfen
bash scripts/install.sh --local          # Aus Quellcode mit cargo bauen
bash scripts/install.sh --install-dir /usr/local/bin  # Eigenes Installationsverzeichnis
```

**Windows (`install.ps1`):**
```powershell
.\scripts\install.ps1 -Help             # Hilfe anzeigen
.\scripts\install.ps1 -Check            # Nur Abhängigkeiten prüfen
.\scripts\install.ps1 -Local            # Aus Quellcode mit cargo bauen
.\scripts\install.ps1 -InstallDir C:\Tools  # Eigenes Installationsverzeichnis
```

### Plattform-spezifische Anleitungen

#### Linux (x64 / ARM64)

```bash
# Empfohlen: Installer verwenden
curl -fsSL https://raw.githubusercontent.com/eqms/claude-workbench/main/scripts/install.sh | bash

# Oder manueller Download:
# Für x64:
curl -LO https://github.com/eqms/claude-workbench/releases/latest/download/claude-workbench-linux-x64
# Für ARM64:
curl -LO https://github.com/eqms/claude-workbench/releases/latest/download/claude-workbench-linux-arm64

# Ausführbar machen
chmod +x claude-workbench-linux-*

# In PATH-Verzeichnis verschieben (optional)
sudo mv claude-workbench-linux-* /usr/local/bin/claude-workbench

# Ausführen
claude-workbench
```

**Abhängigkeiten:**
- Terminal-Emulator mit 256-Farben-Unterstützung
- Git (für Git-Status-Integration)
- Optional: [Claude CLI](https://claude.ai/code) für KI-gestützte Entwicklung
- Optional: [LazyGit](https://github.com/jesseduffield/lazygit) für das Git-Panel

#### macOS (Apple Silicon / Intel)

```bash
# Empfohlen: Installer verwenden (entfernt Quarantäne automatisch)
curl -fsSL https://raw.githubusercontent.com/eqms/claude-workbench/main/scripts/install.sh | bash

# Oder manueller Download:
# Für Apple Silicon (M1/M2/M3/M4):
curl -LO https://github.com/eqms/claude-workbench/releases/latest/download/claude-workbench-macos-arm64
# Für Intel:
curl -LO https://github.com/eqms/claude-workbench/releases/latest/download/claude-workbench-macos-x64

# Ausführbar machen
chmod +x claude-workbench-macos-*

# Quarantäne-Attribut entfernen (nur beim ersten Start)
xattr -d com.apple.quarantine claude-workbench-macos-*

# In PATH-Verzeichnis verschieben (optional)
sudo mv claude-workbench-macos-* /usr/local/bin/claude-workbench

# Ausführen
claude-workbench
```

**Hinweis:** Beim ersten Start kann macOS die Anwendung blockieren. Gehen Sie zu Systemeinstellungen > Datenschutz & Sicherheit und klicken Sie auf "Trotzdem öffnen". Das Installer-Skript entfernt das Quarantäne-Attribut automatisch.

**Abhängigkeiten:**
- Terminal.app, iTerm2 oder ein anderer Terminal-Emulator
- Git (vorinstalliert auf macOS oder via `xcode-select --install`)
- Optional: [Claude CLI](https://claude.ai/code) für KI-gestützte Entwicklung
- Optional: [LazyGit](https://github.com/jesseduffield/lazygit) (`brew install lazygit`)

#### Windows (x64 / ARM64)

**Option 1: PowerShell-Installer (Empfohlen)**
```powershell
# Ein-Befehl-Installation
irm https://raw.githubusercontent.com/eqms/claude-workbench/main/scripts/install.ps1 | iex

# Oder mit Optionen
.\scripts\install.ps1 -Check    # Erst Abhängigkeiten prüfen
.\scripts\install.ps1            # Neueste Version installieren
```

**Option 2: Direkter Download**
1. Laden Sie das passende Binary von der [Releases](https://github.com/eqms/claude-workbench/releases)-Seite herunter:
   - Für x64: `claude-workbench-windows-x64.exe`
   - Für ARM64 (Surface Pro X, etc.): `claude-workbench-windows-arm64.exe`
2. Verschieben Sie die Datei an einen geeigneten Ort (z.B. `C:\Tools\`)
3. Fügen Sie den Speicherort zu PATH hinzu (optional)
4. Starten Sie aus PowerShell oder Windows Terminal

**Option 2: PowerShell-Installation**
```powershell
# Herunterladen (x64)
Invoke-WebRequest -Uri "https://github.com/eqms/claude-workbench/releases/latest/download/claude-workbench-windows-x64.exe" -OutFile "claude-workbench.exe"

# Herunterladen (ARM64)
# Invoke-WebRequest -Uri "https://github.com/eqms/claude-workbench/releases/latest/download/claude-workbench-windows-arm64.exe" -OutFile "claude-workbench.exe"

# Ausführen
.\claude-workbench.exe
```

**Empfohlenes Terminal:**
- [Windows Terminal](https://aka.ms/terminal) für beste Erfahrung (256-Farben, korrekte Unicode-Darstellung)
- PowerShell oder cmd.exe funktionieren, haben aber eingeschränkte Farbunterstützung

**Abhängigkeiten:**
- Git für Windows: https://git-scm.com/download/win
- Optional: [Claude CLI](https://claude.ai/code) für KI-gestützte Entwicklung
- Optional: [LazyGit](https://github.com/jesseduffield/lazygit)

### Aus Quellcode kompilieren

Falls Sie lieber aus dem Quellcode kompilieren möchten:

```bash
# Repository klonen
git clone https://github.com/eqms/claude-workbench.git
cd claude-workbench

# Release-Version kompilieren
cargo build --release

# Das Binary befindet sich unter target/release/claude-workbench
```

**Build-Abhängigkeiten:**
- Rust-Toolchain (rustc, cargo): https://rustup.rs/
- C-Compiler (gcc/clang auf Unix, MSVC auf Windows)

### Konfiguration

Beim ersten Start erstellt Claude Workbench ein Konfigurationsverzeichnis:
- Linux/macOS: `~/.config/claude-workbench/`
- Windows: `%APPDATA%\claude-workbench\`

Kopieren Sie die Beispielkonfiguration:
```bash
cp config.yaml.example ~/.config/claude-workbench/config.yaml
```

### Fehlerbehebung

**"Permission denied" auf Linux/macOS:**
```bash
chmod +x claude-workbench-*
```

**"App ist beschädigt" auf macOS:**
```bash
xattr -d com.apple.quarantine claude-workbench-macos-*
```

**Farben werden nicht korrekt angezeigt:**
- Stellen Sie sicher, dass Ihr Terminal 256 Farben unterstützt
- Setzen Sie `TERM=xterm-256color` in Ihrem Shell-Profil

**Claude CLI nicht gefunden:**
- Installieren Sie Claude CLI: https://claude.ai/code
- Stellen Sie sicher, dass `claude` in Ihrem PATH ist
