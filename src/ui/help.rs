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
        "Claude Workbench Help",
        "======================",
        "",
        "Global Shortcuts:",
        "  Ctrl+Q     : Quit Application",
        "  ?          : Toggle this Help",
        "  Esc        : Close Help / Unfocus",
        "",
        "Navigation / Panes:",
        "  F1         : File Browser",
        "  F2         : Preview & Info",
        "  F4         : Claude Code (PTY)",
        "  F5         : LazyGit (PTY)",
        "  F6         : User Terminal (PTY)",
        "",
        "File Browser:",
        "  j/k, Up/Down : Navigate",
        "  Enter        : Open / Go to Dir",
        "  Back/Left    : Go to Parent",
        "",
        "Terminal Use:",
        "  Arrow keys, Ctrl keys map to PTY.",
    ].join("\n");

    let block = Block::bordered().title(" Help ");
    let paragraph = Paragraph::new(text)
        .block(block)
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, rect);
}
