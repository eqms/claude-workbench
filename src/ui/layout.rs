use ratatui::layout::{Constraint, Direction, Layout, Rect};

pub fn compute_layout(area: Rect) -> (Rect, Rect, Rect, Rect, Rect, Rect) {
    // 1. Vertical Split: Top (Work Area), Bottom (Claude Code), Footer
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),             // Top Area (dynamic)
            Constraint::Percentage(30),     // Claude Code (Bottom)
            Constraint::Length(1),          // Footer
        ])
        .split(area);

    let top_area = vertical[0];
    let claude_area = vertical[1];
    let footer_area = vertical[2];

    // 2. Horizontal Split of Top Area
    let top_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20), // File Browser
            Constraint::Percentage(50), // Preview/Editor
            Constraint::Percentage(30), // Right Terminals (LazyGit + Shell)
        ])
        .split(top_area);

    let file_area = top_chunks[0];
    let preview_area = top_chunks[1];
    let right_stack_area = top_chunks[2];

    // 3. Vertical Split of Right Stack (LazyGit + Terminal)
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(50), // LazyGit
            Constraint::Percentage(50), // Terminal
        ])
        .split(right_stack_area);
        
    let lazygit_area = right_chunks[0];
    let terminal_area = right_chunks[1];

    (
        file_area,
        preview_area,
        claude_area,   
        lazygit_area,  
        terminal_area, 
        footer_area,
    )
}

