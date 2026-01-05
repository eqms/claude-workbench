//! Wizard UI rendering

use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::setup::wizard::{WizardState, WizardStep, WizardField};

/// Render the installation wizard
pub fn render(frame: &mut Frame, area: Rect, state: &WizardState) {
    // Calculate centered popup area (70% width, 80% height)
    let popup_width = (area.width as f32 * 0.7) as u16;
    let popup_height = (area.height as f32 * 0.8) as u16;

    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;

    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    // Clear background
    frame.render_widget(Clear, popup_area);

    // Build title with step indicator
    let title = format!(
        " {} - Step {}/{} ",
        state.step.title(),
        state.step.step_number(),
        WizardStep::total_steps()
    );

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    // Render step-specific content
    match state.step {
        WizardStep::Welcome => render_welcome(frame, inner),
        WizardStep::Dependencies => render_dependencies(frame, inner, state),
        WizardStep::ShellSelection => render_shell_selection(frame, inner, state),
        WizardStep::ClaudeConfig => render_claude_config(frame, inner, state),
        WizardStep::TemplateSelection => render_template_selection(frame, inner, state),
        WizardStep::Confirmation => render_confirmation(frame, inner, state),
        WizardStep::Complete => render_complete(frame, inner),
    }

    // Render navigation footer
    render_footer(frame, popup_area, state);
}

fn render_welcome(frame: &mut Frame, area: Rect) {
    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(2),
        Constraint::Min(1),
        Constraint::Length(3),
    ])
    .split(area);

    // Title banner
    let banner = Paragraph::new(vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("Welcome to ", Style::default()),
            Span::styled("Claude Workbench", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]),
    ])
    .alignment(ratatui::layout::Alignment::Center);
    frame.render_widget(banner, chunks[0]);

    // Description
    let desc = Paragraph::new(vec![
        Line::from(""),
        Line::from("This wizard will help you configure your development environment."),
        Line::from(""),
        Line::from("We'll check for the following tools:"),
        Line::from(""),
        Line::from(vec![
            Span::styled("  • ", Style::default().fg(Color::Cyan)),
            Span::raw("Git (required)"),
        ]),
        Line::from(vec![
            Span::styled("  • ", Style::default().fg(Color::Cyan)),
            Span::raw("Claude CLI (recommended)"),
        ]),
        Line::from(vec![
            Span::styled("  • ", Style::default().fg(Color::Cyan)),
            Span::raw("LazyGit (optional)"),
        ]),
        Line::from(vec![
            Span::styled("  • ", Style::default().fg(Color::Cyan)),
            Span::raw("Available shells (bash, zsh, fish)"),
        ]),
    ])
    .wrap(Wrap { trim: false });
    frame.render_widget(desc, chunks[2]);

    // Hint
    let hint = Paragraph::new(Line::from(vec![
        Span::styled("Press ", Style::default().fg(Color::DarkGray)),
        Span::styled("Enter", Style::default().fg(Color::Yellow)),
        Span::styled(" to continue or ", Style::default().fg(Color::DarkGray)),
        Span::styled("Esc", Style::default().fg(Color::Yellow)),
        Span::styled(" to skip wizard", Style::default().fg(Color::DarkGray)),
    ]))
    .alignment(ratatui::layout::Alignment::Center);
    frame.render_widget(hint, chunks[3]);
}

fn render_dependencies(frame: &mut Frame, area: Rect, state: &WizardState) {
    let chunks = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(1),
        Constraint::Length(3),
    ])
    .split(area);

    // Header
    let header = Paragraph::new("Checking installed tools...")
        .style(Style::default().fg(Color::Yellow));
    frame.render_widget(header, chunks[0]);

    // Dependencies list
    let mut items: Vec<ListItem> = Vec::new();

    // Git
    let git_status = if state.deps.git.found {
        let version = state.deps.git.version.as_deref().unwrap_or("unknown");
        Line::from(vec![
            Span::styled("✓ ", Style::default().fg(Color::Green)),
            Span::styled("git", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!(" - {}", version)),
        ])
    } else {
        Line::from(vec![
            Span::styled("✗ ", Style::default().fg(Color::Red)),
            Span::styled("git", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(" - NOT FOUND (required)", Style::default().fg(Color::Red)),
        ])
    };
    items.push(ListItem::new(git_status));

    // Claude CLI
    let claude_status = if state.deps.claude_cli.found {
        let version = state.deps.claude_cli.version.as_deref().unwrap_or("unknown");
        Line::from(vec![
            Span::styled("✓ ", Style::default().fg(Color::Green)),
            Span::styled("claude", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!(" - {}", version)),
        ])
    } else {
        Line::from(vec![
            Span::styled("○ ", Style::default().fg(Color::Yellow)),
            Span::styled("claude", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(" - not found (optional)", Style::default().fg(Color::DarkGray)),
        ])
    };
    items.push(ListItem::new(claude_status));

    // LazyGit
    let lazygit_status = if state.deps.lazygit.found {
        let version = state.deps.lazygit.version.as_deref().unwrap_or("unknown");
        Line::from(vec![
            Span::styled("✓ ", Style::default().fg(Color::Green)),
            Span::styled("lazygit", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!(" - {}", version)),
        ])
    } else {
        Line::from(vec![
            Span::styled("○ ", Style::default().fg(Color::Yellow)),
            Span::styled("lazygit", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(" - not found (optional)", Style::default().fg(Color::DarkGray)),
        ])
    };
    items.push(ListItem::new(lazygit_status));

    // Shells header
    items.push(ListItem::new(Line::from("")));
    items.push(ListItem::new(Line::from(vec![
        Span::styled("Available Shells:", Style::default().add_modifier(Modifier::BOLD)),
    ])));

    for shell in &state.deps.shells {
        let path = shell.path.as_ref().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
        items.push(ListItem::new(Line::from(vec![
            Span::styled("  ✓ ", Style::default().fg(Color::Green)),
            Span::raw(&shell.name),
            Span::styled(format!(" ({})", path), Style::default().fg(Color::DarkGray)),
        ])));
    }

    let list = List::new(items);
    frame.render_widget(list, chunks[1]);

    // Summary
    let (found, missing_req, missing_opt) = state.deps.summary();
    let summary_style = if missing_req > 0 {
        Style::default().fg(Color::Red)
    } else if missing_opt > 0 {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::Green)
    };

    let summary = Paragraph::new(format!(
        "Found: {} | Missing required: {} | Missing optional: {}",
        found, missing_req, missing_opt
    ))
    .style(summary_style)
    .alignment(ratatui::layout::Alignment::Center);
    frame.render_widget(summary, chunks[2]);
}

fn render_shell_selection(frame: &mut Frame, area: Rect, state: &WizardState) {
    let chunks = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(1),
    ])
    .split(area);

    let header = Paragraph::new("Select your preferred shell for the terminal pane:");
    frame.render_widget(header, chunks[0]);

    let items: Vec<ListItem> = state
        .available_shells
        .iter()
        .enumerate()
        .map(|(i, shell)| {
            let prefix = if i == state.selected_shell_idx { "● " } else { "○ " };
            let style = if i == state.selected_shell_idx {
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(Line::from(vec![
                Span::styled(prefix, style),
                Span::styled(shell.as_str(), style),
            ]))
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, chunks[1]);
}

fn render_claude_config(frame: &mut Frame, area: Rect, state: &WizardState) {
    let chunks = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(4),
        Constraint::Length(1),
        Constraint::Length(4),
        Constraint::Min(1),
    ])
    .split(area);

    let header = Paragraph::new("Configure tool paths:");
    frame.render_widget(header, chunks[0]);

    // Claude path
    let claude_editing = state.editing_field == Some(WizardField::ClaudePath);
    let claude_style = if state.focused_field == 0 {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let claude_value = if claude_editing {
        format!("{}█", state.input_buffer)
    } else {
        state.claude_path.clone()
    };

    let claude_status = if state.deps.claude_cli.found { "✓" } else { "○" };
    let claude_status_style = if state.deps.claude_cli.found {
        Style::default().fg(Color::Green)
    } else {
        Style::default().fg(Color::Yellow)
    };

    let claude_block = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("Claude CLI Path: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(claude_status, claude_status_style),
        ]),
        Line::from(vec![
            Span::styled("▸ ", claude_style),
            Span::styled(&claude_value, claude_style),
        ]),
        Line::from(Span::styled(
            if claude_editing { "  [Enter to confirm, Esc to cancel]" } else { "  [Enter to edit]" },
            Style::default().fg(Color::DarkGray),
        )),
    ]);
    frame.render_widget(claude_block, chunks[1]);

    // LazyGit path
    let lazygit_editing = state.editing_field == Some(WizardField::LazygitPath);
    let lazygit_style = if state.focused_field == 1 {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let lazygit_value = if lazygit_editing {
        format!("{}█", state.input_buffer)
    } else {
        state.lazygit_path.clone()
    };

    let lazygit_status = if state.deps.lazygit.found { "✓" } else { "○" };
    let lazygit_status_style = if state.deps.lazygit.found {
        Style::default().fg(Color::Green)
    } else {
        Style::default().fg(Color::Yellow)
    };

    let lazygit_block = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("LazyGit Path: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(lazygit_status, lazygit_status_style),
        ]),
        Line::from(vec![
            Span::styled("▸ ", lazygit_style),
            Span::styled(&lazygit_value, lazygit_style),
        ]),
        Line::from(Span::styled(
            if lazygit_editing { "  [Enter to confirm, Esc to cancel]" } else { "  [Enter to edit]" },
            Style::default().fg(Color::DarkGray),
        )),
    ]);
    frame.render_widget(lazygit_block, chunks[3]);
}

fn render_template_selection(frame: &mut Frame, area: Rect, state: &WizardState) {
    let chunks = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(1),
    ])
    .split(area);

    let header = Paragraph::new("Choose a workflow template:");
    frame.render_widget(header, chunks[0]);

    let items: Vec<ListItem> = state
        .available_templates
        .iter()
        .enumerate()
        .map(|(i, template)| {
            let prefix = if i == state.selected_template_idx { "● " } else { "○ " };
            let style = if i == state.selected_template_idx {
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let desc_style = if i == state.selected_template_idx {
                Style::default().fg(Color::White)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            ListItem::new(vec![
                Line::from(vec![
                    Span::styled(prefix, style),
                    Span::styled(&template.name, style),
                ]),
                Line::from(vec![
                    Span::raw("    "),
                    Span::styled(&template.description, desc_style),
                ]),
                Line::from(""),
            ])
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, chunks[1]);
}

fn render_confirmation(frame: &mut Frame, area: Rect, state: &WizardState) {
    let chunks = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(1),
        Constraint::Length(3),
    ])
    .split(area);

    let header = Paragraph::new("Summary of your configuration:")
        .style(Style::default().add_modifier(Modifier::BOLD));
    frame.render_widget(header, chunks[0]);

    let template_name = state
        .selected_template()
        .map(|t| t.name.as_str())
        .unwrap_or("None");

    let summary = Paragraph::new(vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Shell:       ", Style::default().fg(Color::DarkGray)),
            Span::raw(state.selected_shell()),
        ]),
        Line::from(vec![
            Span::styled("  Claude CLI:  ", Style::default().fg(Color::DarkGray)),
            Span::raw(&state.claude_path),
        ]),
        Line::from(vec![
            Span::styled("  LazyGit:     ", Style::default().fg(Color::DarkGray)),
            Span::raw(&state.lazygit_path),
        ]),
        Line::from(vec![
            Span::styled("  Template:    ", Style::default().fg(Color::DarkGray)),
            Span::raw(template_name),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Config file: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                crate::config::get_config_path()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| "~/.config/claude-workbench/config.yaml".to_string()),
                Style::default().fg(Color::Cyan),
            ),
        ]),
    ]);
    frame.render_widget(summary, chunks[1]);

    let hint = Paragraph::new(Line::from(vec![
        Span::styled("Press ", Style::default().fg(Color::DarkGray)),
        Span::styled("Enter", Style::default().fg(Color::Green)),
        Span::styled(" to save configuration", Style::default().fg(Color::DarkGray)),
    ]))
    .alignment(ratatui::layout::Alignment::Center);
    frame.render_widget(hint, chunks[2]);
}

fn render_complete(frame: &mut Frame, area: Rect) {
    let content = Paragraph::new(vec![
        Line::from(""),
        Line::from(""),
        Line::from(vec![
            Span::styled("✓ ", Style::default().fg(Color::Green)),
            Span::styled("Setup Complete!", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from("Your configuration has been saved."),
        Line::from(""),
        Line::from("Press Enter to start using Claude Workbench."),
    ])
    .alignment(ratatui::layout::Alignment::Center);
    frame.render_widget(content, area);
}

fn render_footer(frame: &mut Frame, popup_area: Rect, state: &WizardState) {
    let footer_area = Rect::new(
        popup_area.x + 1,
        popup_area.y + popup_area.height - 2,
        popup_area.width - 2,
        1,
    );

    let nav_text = match state.step {
        WizardStep::Welcome => "[Esc] Skip  [Enter] Continue →",
        WizardStep::Complete => "[Enter] Start",
        _ => {
            if state.editing_field.is_some() {
                "[Esc] Cancel  [Enter] Confirm"
            } else {
                "← [Left] Back  [Enter] Continue →  [Esc] Cancel"
            }
        }
    };

    let footer = Paragraph::new(nav_text)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(ratatui::layout::Alignment::Center);
    frame.render_widget(footer, footer_area);
}
