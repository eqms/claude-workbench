use crate::types::DragState;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

/// Renders a small ghost element that follows the cursor during drag operations.
/// Shows the filename being dragged with an icon.
pub fn render(f: &mut Frame, drag_state: &DragState) {
    if !drag_state.dragging {
        return;
    }

    let Some(path) = &drag_state.dragged_path else {
        return;
    };

    // Get filename for display
    let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("file");

    // Determine icon based on whether it's a directory
    let icon = if path.is_dir() { "📁" } else { "📄" };

    // Create ghost text: "📄 filename"
    let ghost_text = format!("{} {}", icon, filename);

    // Calculate ghost dimensions (add padding for border)
    let text_width = ghost_text.chars().count() as u16 + 2; // +2 for borders
    let ghost_width = text_width.min(30); // Max width 30
    let ghost_height: u16 = 3; // Single line + borders

    // Position ghost slightly offset from cursor (bottom-right)
    let x = drag_state.current_x.saturating_add(1);
    let y = drag_state.current_y.saturating_add(1);

    // Ensure ghost stays within terminal bounds
    let frame_area = f.area();
    let x = x.min(frame_area.width.saturating_sub(ghost_width));
    let y = y.min(frame_area.height.saturating_sub(ghost_height));

    let ghost_area = Rect::new(x, y, ghost_width, ghost_height);

    // Render ghost with cyan background to indicate drag operation
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .style(Style::default().bg(Color::Black));

    let paragraph = Paragraph::new(ghost_text)
        .style(Style::default().fg(Color::White).bg(Color::Black))
        .block(block);

    // Render on top of everything
    f.render_widget(paragraph, ghost_area);
}
