use crate::config::LayoutConfig;
use ratatui::layout::{Constraint, Direction, Layout, Rect};

pub fn compute_layout(
    area: Rect,
    show_file_browser: bool,
    show_terminal: bool,
    show_lazygit: bool,
    show_preview: bool,
    layout_config: &LayoutConfig,
) -> (Rect, Rect, Rect, Rect, Rect, Rect) {
    // Use config values for layout percentages
    let claude_pct = layout_config.claude_height_percent;
    let file_pct = layout_config.file_browser_width_percent;
    let preview_pct = layout_config.preview_width_percent;
    let right_pct = layout_config.right_panel_width_percent;

    // Check if any top pane is visible
    let any_top_visible = show_file_browser || show_preview || show_terminal || show_lazygit;

    // 1. Vertical Split: Top (Work Area), Bottom (Claude Code), Footer
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints(if any_top_visible {
            vec![
                Constraint::Min(1),                 // Top Area (dynamic)
                Constraint::Percentage(claude_pct), // Claude Code (Bottom) - from config
                Constraint::Length(1),              // Footer
            ]
        } else {
            vec![
                Constraint::Length(0), // Top Area - no space
                Constraint::Min(1),    // Claude Code - full space
                Constraint::Length(1), // Footer
            ]
        })
        .split(area);

    let top_area = vertical[0];
    let claude_area = vertical[1];
    let footer_area = vertical[2];

    // Determine if right panel is needed
    let show_right_panel = show_terminal || show_lazygit;

    // 2. Horizontal Split of Top Area (FileBrowser | Preview | Right Panel)
    let (file_area, preview_area, right_stack_area) =
        match (show_file_browser, show_preview, show_right_panel) {
            // File browser hidden: distribute space among preview and right panel
            (false, true, true) => {
                let top_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Percentage(preview_pct + file_pct / 2),
                        Constraint::Percentage(right_pct + file_pct / 2),
                    ])
                    .split(top_area);
                (Rect::default(), top_chunks[0], top_chunks[1])
            }
            (false, true, false) => (Rect::default(), top_area, Rect::default()),
            (false, false, true) => (Rect::default(), Rect::default(), top_area),
            (false, false, false) => {
                // Nothing visible in top area
                (Rect::default(), Rect::default(), Rect::default())
            }
            // File browser visible: original logic
            (true, true, true) => {
                let top_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Percentage(file_pct),    // File Browser - from config
                        Constraint::Percentage(preview_pct), // Preview/Editor - from config
                        Constraint::Percentage(right_pct),   // Right Panel - from config
                    ])
                    .split(top_area);
                (top_chunks[0], top_chunks[1], top_chunks[2])
            }
            (true, true, false) => {
                let preview_expanded = 100 - file_pct;
                let top_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Percentage(file_pct),
                        Constraint::Percentage(preview_expanded),
                    ])
                    .split(top_area);
                (top_chunks[0], top_chunks[1], Rect::default())
            }
            (true, false, true) => {
                let right_expanded = 100 - file_pct;
                let top_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Percentage(file_pct),
                        Constraint::Percentage(right_expanded),
                    ])
                    .split(top_area);
                (top_chunks[0], Rect::default(), top_chunks[1])
            }
            (true, false, false) => (top_area, Rect::default(), Rect::default()),
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
