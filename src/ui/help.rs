//! Help screen with scrolling support

use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};

use crate::types::HelpState;

/// All help content lines
fn help_content() -> Vec<Line<'static>> {
    vec![
        // Title
        Line::from(Span::styled(
            "Claude Workbench v0.18.0 Help",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )),
        Line::from("═".repeat(40)),
        Line::from(""),

        // Global Shortcuts
        Line::from(Span::styled(
            "Global Shortcuts (work everywhere)",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Ctrl+Q       ", Style::default().fg(Color::Cyan)),
            Span::raw("Quit Application"),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+P       ", Style::default().fg(Color::Cyan)),
            Span::raw("Fuzzy Find Files"),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+,       ", Style::default().fg(Color::Cyan)),
            Span::raw("Open Settings Menu"),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+Shift+W ", Style::default().fg(Color::Cyan)),
            Span::raw("Re-run Setup Wizard"),
        ]),
        Line::from(vec![
            Span::styled("  F3           ", Style::default().fg(Color::Cyan)),
            Span::raw("Refresh File Browser"),
        ]),
        Line::from(vec![
            Span::styled("  F10          ", Style::default().fg(Color::Cyan)),
            Span::raw("About / License Info"),
        ]),
        Line::from(vec![
            Span::styled("  F12          ", Style::default().fg(Color::Cyan)),
            Span::raw("Toggle this Help"),
        ]),
        Line::from(vec![
            Span::styled("  Esc          ", Style::default().fg(Color::Cyan)),
            Span::raw("Close Dialogs / Help"),
        ]),
        Line::from(""),

        // Context Shortcuts
        Line::from(Span::styled(
            "Context Shortcuts (FileBrowser/Preview only)",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ?            ", Style::default().fg(Color::Cyan)),
            Span::raw("Toggle Help (not in terminals)"),
        ]),
        Line::from(vec![
            Span::styled("  i            ", Style::default().fg(Color::Cyan)),
            Span::raw("About (FileBrowser only)"),
        ]),
        Line::from(""),

        // Navigation/Panes
        Line::from(Span::styled(
            "Navigation / Panes",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  F1           ", Style::default().fg(Color::Cyan)),
            Span::raw("File Browser"),
        ]),
        Line::from(vec![
            Span::styled("  F2           ", Style::default().fg(Color::Cyan)),
            Span::raw("Preview Pane (syntax highlighting)"),
        ]),
        Line::from(vec![
            Span::styled("  F4           ", Style::default().fg(Color::Cyan)),
            Span::raw("Claude Code (PTY)"),
        ]),
        Line::from(vec![
            Span::styled("  F5           ", Style::default().fg(Color::Cyan)),
            Span::raw("LazyGit (PTY)"),
        ]),
        Line::from(vec![
            Span::styled("  F6           ", Style::default().fg(Color::Cyan)),
            Span::raw("User Terminal (PTY)"),
        ]),
        Line::from(""),

        // File Browser
        Line::from(Span::styled(
            "File Browser (F1)",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  j/k, ↑/↓     ", Style::default().fg(Color::Cyan)),
            Span::raw("Navigate files"),
        ]),
        Line::from(vec![
            Span::styled("  Enter        ", Style::default().fg(Color::Cyan)),
            Span::raw("Open file / Enter directory"),
        ]),
        Line::from(vec![
            Span::styled("  Double-Click ", Style::default().fg(Color::Cyan)),
            Span::raw("Open file / Enter directory"),
        ]),
        Line::from(vec![
            Span::styled("  Back/←       ", Style::default().fg(Color::Cyan)),
            Span::raw("Go to parent directory"),
        ]),
        Line::from(vec![
            Span::styled("  o            ", Style::default().fg(Color::Cyan)),
            Span::raw("Open in Browser/Viewer"),
        ]),
        Line::from(vec![
            Span::styled("  O (Shift+O)  ", Style::default().fg(Color::Cyan)),
            Span::raw("Open directory in Finder"),
        ]),
        Line::from(vec![
            Span::styled("  .            ", Style::default().fg(Color::Cyan)),
            Span::raw("Toggle hidden files"),
        ]),
        Line::from(""),
        Line::from(Span::styled("  Status bar shows:", Style::default().fg(Color::DarkGray))),
        Line::from("  - File size and modification date"),
        Line::from("  - Git branch, modified/untracked/staged counts"),
        Line::from(""),
        Line::from(Span::styled("  Git Status Colors:", Style::default().fg(Color::DarkGray))),
        Line::from(vec![
            Span::raw("  - "),
            Span::styled("Yellow", Style::default().fg(Color::Yellow)),
            Span::raw(": Untracked (?)"),
        ]),
        Line::from(vec![
            Span::raw("  - "),
            Span::styled("Orange", Style::default().fg(Color::Rgb(255, 165, 0))),
            Span::raw(": Modified (M)"),
        ]),
        Line::from(vec![
            Span::raw("  - "),
            Span::styled("Green", Style::default().fg(Color::Green)),
            Span::raw(": Staged (+)"),
        ]),
        Line::from(vec![
            Span::raw("  - "),
            Span::styled("Gray", Style::default().fg(Color::DarkGray)),
            Span::raw(": Ignored (·)"),
        ]),
        Line::from(vec![
            Span::raw("  - "),
            Span::styled("Red", Style::default().fg(Color::Red)),
            Span::raw(": Conflict (!)"),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Git Remote (new in v0.18):",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from("  - Auto-checks for remote changes on repo switch"),
        Line::from("  - Prompts to pull if remote is ahead"),
        Line::from(""),

        // Browser Preview
        Line::from(Span::styled(
            "Browser Preview (o key)",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("  - HTML/HTM:  Direct browser opening"),
        Line::from("  - Markdown:  Convert to styled HTML, dark mode"),
        Line::from("  - PDF:       Open in default PDF viewer"),
        Line::from("  - Images:    PNG/JPG/GIF/SVG/WebP in viewer"),
        Line::from(""),

        // Fuzzy Finder
        Line::from(Span::styled(
            "Fuzzy Finder (Ctrl+P)",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Type         ", Style::default().fg(Color::Cyan)),
            Span::raw("Filter files by name"),
        ]),
        Line::from(vec![
            Span::styled("  ↑/↓          ", Style::default().fg(Color::Cyan)),
            Span::raw("Navigate results"),
        ]),
        Line::from(vec![
            Span::styled("  Enter        ", Style::default().fg(Color::Cyan)),
            Span::raw("Open selected file"),
        ]),
        Line::from(vec![
            Span::styled("  Esc          ", Style::default().fg(Color::Cyan)),
            Span::raw("Close finder"),
        ]),
        Line::from(""),

        // Preview Pane
        Line::from(Span::styled(
            "Preview Pane (F2)",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("  - Syntax highlighting for 500+ languages"),
        Line::from("  - Markdown rendering with formatting"),
        Line::from(vec![
            Span::styled("  j/k, ↑/↓     ", Style::default().fg(Color::Cyan)),
            Span::raw("Scroll 1 line"),
        ]),
        Line::from(vec![
            Span::styled("  PgUp/PgDn    ", Style::default().fg(Color::Cyan)),
            Span::raw("Scroll 10 lines"),
        ]),
        Line::from(vec![
            Span::styled("  Home/End     ", Style::default().fg(Color::Cyan)),
            Span::raw("Jump to start/end"),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Search & Replace (MC Edit style):",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(vec![
            Span::styled("  / or Ctrl+F  ", Style::default().fg(Color::Green)),
            Span::raw("Start search"),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+H       ", Style::default().fg(Color::Green)),
            Span::raw("Search & Replace (Edit mode) / Toggle mode"),
        ]),
        Line::from(vec![
            Span::styled("  Tab          ", Style::default().fg(Color::Green)),
            Span::raw("Switch Find/Replace fields"),
        ]),
        Line::from(vec![
            Span::styled("  n / N        ", Style::default().fg(Color::Green)),
            Span::raw("Next / Previous match"),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+N/P     ", Style::default().fg(Color::Green)),
            Span::raw("Navigate while typing"),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+I       ", Style::default().fg(Color::Green)),
            Span::raw("Toggle case sensitivity"),
        ]),
        Line::from(vec![
            Span::styled("  Enter        ", Style::default().fg(Color::Green)),
            Span::raw("Confirm / Replace current"),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+R       ", Style::default().fg(Color::Green)),
            Span::raw("Replace all matches"),
        ]),
        Line::from(vec![
            Span::styled("  Esc          ", Style::default().fg(Color::Green)),
            Span::raw("Cancel search"),
        ]),
        Line::from(""),

        // Editor Mode
        Line::from(Span::styled(
            "Editor Mode (in Preview Pane)",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  E            ", Style::default().fg(Color::Cyan)),
            Span::raw("Enter Edit Mode"),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+S       ", Style::default().fg(Color::Cyan)),
            Span::raw("Save File"),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+Z       ", Style::default().fg(Color::Cyan)),
            Span::raw("Undo"),
        ]),
        Line::from(vec![
            Span::styled("  Esc          ", Style::default().fg(Color::Cyan)),
            Span::raw("Exit (confirm if modified)"),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  MC Edit Style Selection:",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(vec![
            Span::styled("  Shift+↑/↓/←/→", Style::default().fg(Color::Green)),
            Span::raw("Select text with cursor"),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+F3      ", Style::default().fg(Color::Green)),
            Span::raw("Toggle block marking mode"),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+F5      ", Style::default().fg(Color::Green)),
            Span::raw("Copy selected block"),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+F6      ", Style::default().fg(Color::Green)),
            Span::raw("Move (cut) selected block"),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+F8      ", Style::default().fg(Color::Green)),
            Span::raw("Delete selected block"),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+Y       ", Style::default().fg(Color::Green)),
            Span::raw("Delete current line"),
        ]),
        Line::from(""),

        // Terminal Panes
        Line::from(Span::styled(
            "Terminal Panes (F4/F5/F6)",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("  All keys map to PTY (shell input)."),
        Line::from(vec![
            Span::styled("  \\ + Enter   ", Style::default().fg(Color::Cyan)),
            Span::raw("Insert newline in Claude Code (F4)"),
        ]),
        Line::from(vec![
            Span::styled("  Shift+PgUp   ", Style::default().fg(Color::Cyan)),
            Span::raw("Scroll 10 lines up"),
        ]),
        Line::from(vec![
            Span::styled("  Shift+PgDn   ", Style::default().fg(Color::Cyan)),
            Span::raw("Scroll 10 lines down"),
        ]),
        Line::from(vec![
            Span::styled("  Shift+↑/↓    ", Style::default().fg(Color::Cyan)),
            Span::raw("Scroll 1 line"),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+S       ", Style::default().fg(Color::Cyan)),
            Span::raw("Start Selection Mode"),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  CLI Navigation (new in v0.18):",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(vec![
            Span::styled("  Alt+← / Alt+→", Style::default().fg(Color::Green)),
            Span::raw("Word navigation (back/forward)"),
        ]),
        Line::from(vec![
            Span::styled("  PageUp       ", Style::default().fg(Color::Green)),
            Span::raw("Jump to line start (Home)"),
        ]),
        Line::from(vec![
            Span::styled("  PageDown     ", Style::default().fg(Color::Green)),
            Span::raw("Jump to line end (End)"),
        ]),
        Line::from(""),

        // IMPORTANT: Selection Mode (highlighted section)
        Line::from("━".repeat(40)),
        Line::from(Span::styled(
            "★ Selection Mode (Ctrl+S / Alt+Click) ★",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )),
        Line::from("━".repeat(40)),
        Line::from(""),
        Line::from(Span::styled(
            "Select text from Terminal or Preview and copy to Claude.",
            Style::default().fg(Color::White).add_modifier(Modifier::ITALIC),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  j/↓          ", Style::default().fg(Color::Green)),
            Span::raw("Extend selection down"),
        ]),
        Line::from(vec![
            Span::styled("  k/↑          ", Style::default().fg(Color::Green)),
            Span::raw("Shrink selection up"),
        ]),
        Line::from(vec![
            Span::styled("  Shift+↓      ", Style::default().fg(Color::Green)),
            Span::raw("Extend by 5 lines"),
        ]),
        Line::from(vec![
            Span::styled("  Shift+↑      ", Style::default().fg(Color::Green)),
            Span::raw("Shrink by 5 lines"),
        ]),
        Line::from(vec![
            Span::styled("  g            ", Style::default().fg(Color::Green)),
            Span::raw("Jump to buffer start"),
        ]),
        Line::from(vec![
            Span::styled("  G            ", Style::default().fg(Color::Green)),
            Span::raw("Jump to buffer end"),
        ]),
        Line::from(vec![
            Span::styled("  Enter / y    ", Style::default().fg(Color::Green)),
            Span::raw("Copy selection to Claude"),
        ]),
        Line::from(vec![
            Span::styled("  Esc          ", Style::default().fg(Color::Green)),
            Span::raw("Cancel selection"),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Intelligent filtering removes shell prompts and",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            "  preserves error messages and stack traces.",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),

        // Mouse Selection
        Line::from(Span::styled(
            "Mouse Selection (Alt+Click)",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("  Alt+Click and drag in Terminal or Preview panes"),
        Line::from("  to select text. Release to enter Selection Mode,"),
        Line::from("  then press Enter/y to copy to Claude."),
        Line::from(""),
        Line::from("  Note: Regular click only focuses pane (no selection)."),
        Line::from(""),
        Line::from("━".repeat(40)),
        Line::from(""),

        // Drag & Drop
        Line::from(Span::styled(
            "Drag & Drop",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("  Drag files from File Browser to"),
        Line::from("  Claude/Terminal panes to insert path."),
        Line::from("  Paths with spaces are auto-quoted."),
        Line::from(""),

        // Configuration
        Line::from(Span::styled(
            "Configuration (config.yaml)",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("  - Shell path and arguments"),
        Line::from("  - Layout percentages"),
        Line::from("  - File browser settings (hidden files, date)"),
        Line::from("  - Claude startup prefixes (optional)"),
        Line::from(""),

        // Footer
        Line::from(Span::styled(
            "Footer: Shows shortcuts, date/time, version.",
            Style::default().fg(Color::DarkGray),
        )),
    ]
}

/// Render the help screen
pub fn render(frame: &mut Frame, state: &mut HelpState) {
    let area = frame.area();

    // Calculate centered popup area (70% width, 80% height)
    let popup_width = (area.width as f32 * 0.7).max(60.0).min(area.width as f32) as u16;
    let popup_height = (area.height as f32 * 0.8).max(20.0).min(area.height as f32) as u16;

    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;

    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    // Store area for mouse events
    state.popup_area = Some(popup_area);

    // Clear background
    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" Help ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    // Store content area
    state.content_area = Some(inner);

    // Layout: content area + footer
    let chunks = Layout::vertical([
        Constraint::Min(1),     // Content
        Constraint::Length(1),  // Footer
    ])
    .split(inner);

    let content_area = chunks[0];
    let footer_area = chunks[1];

    // Get help content
    let content_lines = help_content();
    let total_lines = content_lines.len();
    state.total_lines = total_lines;

    // Calculate visible lines
    let visible_height = content_area.height as usize;
    let max_scroll = total_lines.saturating_sub(visible_height);
    let scroll = state.scroll.min(max_scroll);

    // Render content with scroll offset
    let visible_lines: Vec<Line> = content_lines
        .into_iter()
        .skip(scroll)
        .take(visible_height)
        .collect();

    let paragraph = Paragraph::new(visible_lines);
    frame.render_widget(paragraph, content_area);

    // Render scrollbar if needed
    if total_lines > visible_height {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .symbols(ratatui::symbols::scrollbar::VERTICAL)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"));

        let mut scrollbar_state = ScrollbarState::new(max_scroll)
            .position(scroll);

        // Render scrollbar in the popup area (right border)
        let scrollbar_area = Rect::new(
            popup_area.x + popup_area.width - 1,
            popup_area.y + 1,
            1,
            popup_area.height - 3, // Leave space for footer
        );

        frame.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);

        // Scroll position indicator
        let indicator = format!(" [{}/{}] ", scroll + 1, max_scroll + 1);
        let indicator_area = Rect::new(
            popup_area.x + popup_area.width - indicator.len() as u16 - 2,
            popup_area.y,
            indicator.len() as u16,
            1,
        );
        frame.render_widget(
            Paragraph::new(indicator).style(Style::default().fg(Color::DarkGray)),
            indicator_area,
        );
    }

    // Footer with navigation hints
    let footer = Paragraph::new(Line::from(vec![
        Span::styled("↑/↓ j/k", Style::default().fg(Color::Yellow)),
        Span::styled(" Scroll  ", Style::default().fg(Color::DarkGray)),
        Span::styled("PgUp/PgDn", Style::default().fg(Color::Yellow)),
        Span::styled(" Page  ", Style::default().fg(Color::DarkGray)),
        Span::styled("g/G", Style::default().fg(Color::Yellow)),
        Span::styled(" Top/Bottom  ", Style::default().fg(Color::DarkGray)),
        Span::styled("Esc", Style::default().fg(Color::Yellow)),
        Span::styled(" Close", Style::default().fg(Color::DarkGray)),
    ]))
    .alignment(ratatui::layout::Alignment::Center);

    frame.render_widget(footer, footer_area);
}
