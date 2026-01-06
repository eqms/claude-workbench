use ratatui::{
    layout::{Constraint, Layout},
    widgets::{Block, Clear, Paragraph, Wrap},
    Frame,
};

pub fn render(f: &mut Frame) {
    let area = f.area(); 
    
    // Popup Layout: centered box
    let vertical = Layout::vertical([
        Constraint::Percentage(20),
        Constraint::Percentage(60),
        Constraint::Percentage(20),
    ]);
    let [_, center_area, _] = vertical.areas(area);
    
    let horizontal = Layout::horizontal([
        Constraint::Percentage(20),
        Constraint::Percentage(60),
        Constraint::Percentage(20),
    ]);
    let [_, rect, _] = horizontal.areas(center_area);

    // Clear background to avoid artifacts
    f.render_widget(Clear, rect);

    let text = vec![
        "Claude Workbench v0.11 Help",
        "============================",
        "",
        "Global Shortcuts:",
        "  Ctrl+Q       : Quit Application",
        "  Ctrl+P       : Fuzzy Find Files",
        "  Ctrl+,       : Open Settings Menu",
        "  Ctrl+Shift+W : Re-run Setup Wizard",
        "  F3           : Refresh File Browser",
        "  i            : About / License Info",
        "  ?            : Toggle this Help",
        "  Esc          : Close Dialogs / Help",
        "",
        "Navigation / Panes:",
        "  F1         : File Browser",
        "  F2         : Preview Pane (syntax highlighting)",
        "  F4         : Claude Code (PTY)",
        "  F5         : LazyGit (PTY)",
        "  F6         : User Terminal (PTY)",
        "",
        "File Browser (F1):",
        "  j/k, Up/Down : Navigate files",
        "  Enter        : Open file / Enter directory",
        "  Double-Click : Open file / Enter directory",
        "  Back/Left    : Go to parent directory",
        "  o            : Open in Browser/Viewer",
        "                 (HTML/MD→Browser, PDF/Images→Viewer)",
        "  O (Shift+O)  : Open directory in Finder",
        "",
        "  Status bar shows:",
        "  - File size and modification date",
        "  - Git branch, modified/untracked/staged counts",
        "",
        "  Git Status Colors:",
        "  - Yellow: Untracked (?)",
        "  - Orange: Modified (M)",
        "  - Green:  Staged (+)",
        "  - Gray:   Ignored (·)",
        "  - Red:    Conflict (!)",
        "",
        "Browser Preview (o key):",
        "  - HTML/HTM:  Direct browser opening",
        "  - Markdown:  Convert to styled HTML, dark mode",
        "  - PDF:       Open in default PDF viewer",
        "  - Images:    PNG/JPG/GIF/SVG/WebP in viewer",
        "",
        "Fuzzy Finder (Ctrl+P):",
        "  Type       : Filter files by name",
        "  Up/Down    : Navigate results",
        "  Enter      : Open selected file",
        "  Esc        : Close finder",
        "",
        "Preview Pane (F2):",
        "  - Syntax highlighting for 500+ languages",
        "  - Markdown rendering with formatting",
        "  j/k        : Scroll preview",
        "",
        "Editor Mode (in Preview Pane):",
        "  E          : Enter Edit Mode",
        "  Ctrl+S     : Save File",
        "  Ctrl+Z     : Undo",
        "  Ctrl+Y     : Redo",
        "  Esc        : Exit (confirm if modified)",
        "",
        "Terminal Panes (F4/F5/F6):",
        "  All keys map to PTY (shell input).",
        "  Shift+PgUp/PgDn : Scroll 10 lines",
        "  Shift+Up/Down   : Scroll 1 line",
        "",
        "Terminal Selection Mode:",
        "  Ctrl+S     : Start selection at cursor line",
        "  j/Down     : Extend selection down",
        "  k/Up       : Shrink selection up",
        "  Enter / y  : Copy to Claude as code block",
        "  Esc        : Cancel selection",
        "",
        "Drag & Drop:",
        "  Drag files from File Browser to",
        "  Claude/Terminal panes to insert path.",
        "  Paths with spaces are auto-quoted.",
        "",
        "Configuration (config.yaml):",
        "  - Shell path and arguments",
        "  - Layout percentages",
        "  - File browser settings (hidden files, date)",
        "  - Claude startup prefixes (optional)",
        "",
        "Footer: Shows shortcuts, date/time, version.",
    ].join("\n");

    let block = Block::bordered().title(" Help ");
    let paragraph = Paragraph::new(text)
        .block(block)
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, rect);
}
