//! Shared CSS fragment generator for HTML templates.
//!
//! Provides a `TemplateContext` that generates CSS fragments from `DocumentConfig`,
//! ensuring consistent styling across all HTML-based exports (markdown preview,
//! syntax highlighting, PDF intermediate HTML).

use crate::config::DocumentConfig;

/// Context for generating CSS fragments from document configuration.
pub struct TemplateContext<'a> {
    pub doc: &'a DocumentConfig,
}

impl<'a> TemplateContext<'a> {
    pub fn new(doc: &'a DocumentConfig) -> Self {
        Self { doc }
    }

    /// Base body CSS: font-family, font-size, color, line-height, max-width
    pub fn base_body_css(&self) -> String {
        format!(
            r#"body {{
            font-family: {body_font};
            font-size: {body_size};
            line-height: 1.6;
            color: #333;
            max-width: 800px;
            margin: 0 auto;
            padding: 2rem;
            background: #fff;
        }}"#,
            body_font = self.doc.fonts.body,
            body_size = self.doc.sizes.body,
        )
    }

    /// Typography CSS: heading sizes with border-bottom separators
    pub fn typography_css(&self) -> String {
        format!(
            r#"h1 {{
            font-size: {title};
            font-weight: bold;
            border-bottom: 2px solid #eee;
            padding-bottom: 0.3em;
            margin-top: 1.5em;
            margin-bottom: 0.5em;
        }}
        h2 {{
            font-size: {h1};
            font-weight: bold;
            border-bottom: 1px solid #eee;
            padding-bottom: 0.3em;
            margin-top: 1.3em;
            margin-bottom: 0.5em;
        }}
        h3 {{
            font-size: {h2};
            font-weight: bold;
            margin-top: 1.2em;
            margin-bottom: 0.5em;
        }}
        h4 {{
            font-size: {h3};
            font-weight: bold;
            margin-top: 1.1em;
            margin-bottom: 0.5em;
        }}"#,
            title = self.doc.sizes.title,
            h1 = self.doc.sizes.h1,
            h2 = self.doc.sizes.h2,
            h3 = self.doc.sizes.h3,
        )
    }

    /// Table CSS: header background, borders, padding, font-size
    pub fn table_css(&self) -> String {
        format!(
            r#"table {{
            border-collapse: collapse;
            width: 100%;
            margin: 1em 0;
            font-size: {table_size};
        }}
        th, td {{
            border: 1px solid {table_border};
            padding: 6px 12px;
            text-align: left;
        }}
        th {{
            background-color: {table_header_bg};
            font-weight: bold;
            color: #1a1a1a;
        }}
        tr:nth-child(even) td {{
            background-color: #fafafa;
        }}"#,
            table_size = self.doc.sizes.table,
            table_border = self.doc.colors.table_border,
            table_header_bg = self.doc.colors.table_header_bg,
        )
    }

    /// Code/pre CSS: font-family, background, border
    pub fn code_css(&self) -> String {
        format!(
            r#"code {{
            font-family: {code_font};
            background-color: #f4f4f4;
            padding: 2px 6px;
            border-radius: 3px;
            font-size: 0.9em;
        }}
        pre {{
            background-color: #f4f4f4;
            padding: 1em;
            border-radius: 5px;
            overflow-x: auto;
            line-height: 1.4;
        }}
        pre code {{
            background: none;
            padding: 0;
            font-size: {table_size};
        }}"#,
            code_font = self.doc.fonts.code,
            table_size = self.doc.sizes.table,
        )
    }

    /// Blockquote CSS: left border, padding
    pub fn blockquote_css(&self) -> String {
        r#"blockquote {
            border-left: 4px solid #ddd;
            margin: 1em 0;
            padding: 0.5em 1em;
            color: #666;
            background-color: #f9f9f9;
        }"#
        .to_string()
    }

    /// Link CSS: color from accent config
    pub fn link_css(&self) -> String {
        format!(
            r#"a {{
            color: {link};
            text-decoration: none;
        }}
        a:hover {{
            text-decoration: underline;
        }}"#,
            link = self.doc.colors.link,
        )
    }

    /// Footer CSS: separator line, color, font-size
    pub fn footer_css(&self) -> String {
        format!(
            r#".footer, .document-footer {{
            margin-top: 3rem;
            padding-top: 0.5em;
            border-top: 1px solid {header_border};
            font-size: {footer_size};
            color: {footer_color};
            text-align: center;
        }}"#,
            header_border = self.doc.colors.header_border,
            footer_size = self.doc.sizes.footer,
            footer_color = self.doc.colors.footer,
        )
    }

    /// Dark mode CSS media query overrides
    pub fn dark_mode_css(&self) -> String {
        format!(
            r#"@media (prefers-color-scheme: dark) {{
            body {{
                background-color: #1e1e1e;
                color: #d4d4d4;
            }}
            code {{
                background-color: #2d2d2d;
                color: #e8e8e8;
            }}
            pre {{
                background-color: #2d2d2d;
                color: #e8e8e8;
            }}
            blockquote {{
                border-left-color: #444;
                color: #aaa;
                background-color: #2a2a2a;
            }}
            th {{
                background-color: #3a3a3a;
                color: #ffffff;
            }}
            td {{
                border-color: #555;
            }}
            tr:nth-child(even) td {{
                background-color: #2a2a2a;
            }}
            a {{
                color: #6db3f2;
            }}
            h1, h2 {{
                border-bottom-color: #444;
            }}
            .footer, .document-footer {{
                border-top-color: #444;
                color: {footer_color};
            }}
        }}"#,
            footer_color = self.doc.colors.footer,
        )
    }

    /// Resolved footer text with company name substitution
    pub fn footer_text(&self) -> String {
        self.doc.resolved_footer_text()
    }

    /// Resolved footer text with date appended
    pub fn footer_text_with_date(&self, date: &str) -> String {
        self.doc.resolved_footer_with_date(date)
    }

    /// Resolved author string
    pub fn author(&self) -> String {
        self.doc.resolved_author()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DocumentConfig;

    #[test]
    fn test_default_footer_text() {
        let doc = DocumentConfig::default();
        let ctx = TemplateContext::new(&doc);
        assert_eq!(ctx.footer_text(), "Generated by Claude Workbench");
    }

    #[test]
    fn test_custom_company_footer() {
        let mut doc = DocumentConfig::default();
        doc.company.name = "Equitania Software GmbH".to_string();
        let ctx = TemplateContext::new(&doc);
        assert_eq!(ctx.footer_text(), "Generated by Equitania Software GmbH");
    }

    #[test]
    fn test_footer_with_date() {
        let doc = DocumentConfig::default();
        let ctx = TemplateContext::new(&doc);
        assert_eq!(
            ctx.footer_text_with_date("26.03.2026"),
            "Generated by Claude Workbench \u{2014} 26.03.2026"
        );
    }

    #[test]
    fn test_resolved_author() {
        let mut doc = DocumentConfig::default();
        doc.company.name = "TestCo".to_string();
        let ctx = TemplateContext::new(&doc);
        assert_eq!(ctx.author(), "TestCo");
    }

    #[test]
    fn test_css_contains_config_values() {
        let mut doc = DocumentConfig::default();
        doc.colors.table_header_bg = "#AABBCC".to_string();
        doc.fonts.body = "Arial, sans-serif".to_string();
        let ctx = TemplateContext::new(&doc);

        assert!(ctx.table_css().contains("#AABBCC"));
        assert!(ctx.base_body_css().contains("Arial, sans-serif"));
    }
}
