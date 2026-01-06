# Walkthrough - Phase 3 (File Browser, Preview, Config)

I have implemented the core navigation and configuration features.

## Accomplishments

- **File Browser**:
  - Lists current directory contents (directories first).
  - Navigation with `Up`, `Down`, `Enter` (enter dir), `Left`/`Backspace` (go up).
  - Updates the **Preview Pane** automatically on selection.
- **Preview Pane**:
  - Loads text files and displays them with scrolling (`j`/`k`).
  - Shows metadata for directories or binary files.
- **Configuration (`config.yaml`)**:
  - Implemented logic to load config from `./config.yaml` OR `~/.config/claude-workbench/config.yaml`.
  - Created a default `config.yaml` configured for **Fish Shell** (`/opt/homebrew/bin/fish`).
- **Quit Logic**:
  - Added `Ctrl+Q` global shortcut to quit.
  - Added single `q` shortcut (when File Browser is focused).

## Running the App

```bash
cargo run
```

- Navigate files with `F1` (Browser) or arrow keys.
- Open files.
- Use `F6` to verify Fish shell is running in the Terminal pane.
- Quit with `Ctrl+Q`.

## Important Note regarding Project Location

The project is currently located in the temporary workspace.
Please **move/copy** the entire folder to your desired location:

```bash
cp -R . ~/gitbase/workbench/
```

(The previous automatic move attempt might contain outdated code).
