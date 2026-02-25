//! Template system for predefined configurations

use crate::config::{Config, LayoutConfig, PtyConfig};
use serde::{Deserialize, Serialize};

/// Template categories
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateCategory {
    Layout,
    Pty,
    Workflow,
}

impl TemplateCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            TemplateCategory::Layout => "layout",
            TemplateCategory::Pty => "pty",
            TemplateCategory::Workflow => "workflow",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "layout" => Some(TemplateCategory::Layout),
            "pty" => Some(TemplateCategory::Pty),
            "workflow" => Some(TemplateCategory::Workflow),
            _ => None,
        }
    }
}

/// Template definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub layout: Option<LayoutConfig>,
    pub pty: Option<PtyConfig>,
}

impl Template {
    pub fn category_enum(&self) -> Option<TemplateCategory> {
        TemplateCategory::parse(&self.category)
    }
}

/// Get all built-in templates
pub fn get_builtin_templates() -> Vec<Template> {
    vec![
        // Layout Templates
        Template {
            id: "layout_balanced".to_string(),
            name: "Balanced".to_string(),
            description: "Equal space for files, preview, and terminals".to_string(),
            category: "layout".to_string(),
            layout: Some(LayoutConfig {
                file_browser_width_percent: 20,
                preview_width_percent: 50,
                right_panel_width_percent: 30,
                claude_height_percent: 40,
            }),
            pty: None,
        },
        Template {
            id: "layout_code_focused".to_string(),
            name: "Code Focused".to_string(),
            description: "Maximized preview for coding".to_string(),
            category: "layout".to_string(),
            layout: Some(LayoutConfig {
                file_browser_width_percent: 15,
                preview_width_percent: 65,
                right_panel_width_percent: 20,
                claude_height_percent: 35,
            }),
            pty: None,
        },
        Template {
            id: "layout_claude_focused".to_string(),
            name: "Claude Focused".to_string(),
            description: "Larger Claude pane for AI interaction".to_string(),
            category: "layout".to_string(),
            layout: Some(LayoutConfig {
                file_browser_width_percent: 20,
                preview_width_percent: 45,
                right_panel_width_percent: 35,
                claude_height_percent: 55,
            }),
            pty: None,
        },
        // Workflow Templates (Combined Layout + PTY)
        Template {
            id: "workflow_full_dev".to_string(),
            name: "Full Development".to_string(),
            description: "Optimized for full-stack development with Claude".to_string(),
            category: "workflow".to_string(),
            layout: Some(LayoutConfig {
                file_browser_width_percent: 18,
                preview_width_percent: 52,
                right_panel_width_percent: 30,
                claude_height_percent: 45,
            }),
            pty: Some(PtyConfig {
                claude_command: vec!["claude".to_string()],
                lazygit_command: vec!["lazygit".to_string()],
                scrollback_lines: 2000,
                auto_restart: true,
                copy_lines_count: 50,
            }),
        },
        Template {
            id: "workflow_minimal".to_string(),
            name: "Minimal".to_string(),
            description: "Lightweight setup - shell fallbacks for missing tools".to_string(),
            category: "workflow".to_string(),
            layout: Some(LayoutConfig {
                file_browser_width_percent: 25,
                preview_width_percent: 75,
                right_panel_width_percent: 0,
                claude_height_percent: 40,
            }),
            pty: Some(PtyConfig {
                claude_command: vec![
                    "/bin/bash".to_string(),
                    "-c".to_string(),
                    "echo 'Claude CLI not installed - using shell'; exec bash".to_string(),
                ],
                lazygit_command: vec![
                    "/bin/bash".to_string(),
                    "-c".to_string(),
                    "echo 'lazygit not installed - using shell'; exec bash".to_string(),
                ],
                scrollback_lines: 1000,
                auto_restart: true,
                copy_lines_count: 50,
            }),
        },
    ]
}

/// Apply template to config
pub fn apply_template(config: &mut Config, template: &Template) {
    if let Some(layout) = &template.layout {
        config.layout = layout.clone();
    }
    if let Some(pty) = &template.pty {
        config.pty = pty.clone();
    }
    config.setup.active_template = template.id.clone();
}

/// Get template by ID
pub fn get_template_by_id(id: &str) -> Option<Template> {
    get_builtin_templates().into_iter().find(|t| t.id == id)
}

/// Get templates by category
pub fn get_templates_by_category(category: TemplateCategory) -> Vec<Template> {
    get_builtin_templates()
        .into_iter()
        .filter(|t| t.category == category.as_str())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_templates() {
        let templates = get_builtin_templates();
        assert!(!templates.is_empty());
        assert!(templates.len() >= 4);
    }

    #[test]
    fn test_get_template_by_id() {
        let template = get_template_by_id("workflow_full_dev");
        assert!(template.is_some());
        assert_eq!(template.unwrap().name, "Full Development");
    }

    #[test]
    fn test_apply_template() {
        let mut config = Config::default();
        let template = get_template_by_id("layout_code_focused").unwrap();
        apply_template(&mut config, &template);
        assert_eq!(config.layout.preview_width_percent, 65);
        assert_eq!(config.setup.active_template, "layout_code_focused");
    }
}
