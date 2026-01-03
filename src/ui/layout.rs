use ratatui::layout::{Constraint, Direction, Layout, Rect};

pub fn compute_layout(area: Rect, show_terminal: bool, show_lazygit: bool) -> (Rect, Rect, Rect, Rect, Rect, Rect) {
    // 1. Vertical Split: Top (Work Area), Bottom (Claude Code), Footer
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),             // Top Area (dynamic)
            Constraint::Percentage(40),     // Claude Code (Bottom)
            Constraint::Length(1),          // Footer
        ])
        .split(area);

    let top_area = vertical[0];
    let claude_area = vertical[1];
    let footer_area = vertical[2];

    // Determine if right panel is needed
    let show_right_panel = show_terminal || show_lazygit;

    // 2. Horizontal Split of Top Area
    let (file_area, preview_area, right_stack_area) = if show_right_panel {
        let top_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(20), // File Browser
                Constraint::Percentage(50), // Preview/Editor
                Constraint::Percentage(30), // Right Panel
            ])
            .split(top_area);
        (top_chunks[0], top_chunks[1], top_chunks[2])
    } else {
        // No right panel -> Preview takes more space
        let top_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25), // File Browser
                Constraint::Percentage(75), // Preview/Editor (larger)
            ])
            .split(top_area);
        (top_chunks[0], top_chunks[1], Rect::default())
    };

    // 3. Vertical Split of Right Stack (LazyGit + Terminal)
    let (lazygit_area, terminal_area) = match (show_lazygit, show_terminal) {
        (true, true) => {
            let right_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(50), // LazyGit
                    Constraint::Percentage(50), // Terminal
                ])
                .split(right_stack_area);
            (right_chunks[0], right_chunks[1])
        }
        (true, false) => (right_stack_area, Rect::default()),
        (false, true) => (Rect::default(), right_stack_area),
        (false, false) => (Rect::default(), Rect::default()),
    };

    (
        file_area,
        preview_area,
        claude_area,   
        lazygit_area,  
        terminal_area, 
        footer_area,
    )
}

