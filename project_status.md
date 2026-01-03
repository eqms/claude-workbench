# Project Status & Roadmap

## Current Status (Phase 4 Completed)

### Implemented Features
1.  **UI Layout**: Fixed 3-column layout (File Browser, Preview, Terminal Stack).
2.  **Terminal Integration**:
    -   Uses `portable-pty` & `vt100`.
    -   **Input Mapping**: Arrow keys, Ctrl-sequences working (`src/input`).
    -   **Colors & Cursor**: Full ANSI color support and cursor rendering.
    -   **LazyGit**: Integrated in dedicated pane.
    -   **Configurable Shell**: Fish/Bash via config.
3.  **File Browser**:
    -   Navigation & Selection.
    -   **Dynamic Title**: Shows current path.
4.  **UI Enhancements**:
    -   **Footer**: Shortcut visualization (MC-style).
    -   **Help System**: '?' Popup for shortcuts.
5.  **Preview Pane**: Text content display.
6.  **Configuration**: YAML based.

### Known Issues
-   **Terminal Resize**: PTY geometry doesn't yet auto-update on window resize.
-   **Deployment**: Setup is manual.

## Next Steps (Phase 5: Search & Interaction)
-   [ ] **Fuzzy Finder**: Implement file search modal.
-   [ ] **Drag & Drop**: Implement visual mockup for D&D.
-   [ ] **Mouse Support**: Click to focus panes.
