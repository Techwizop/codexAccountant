use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::text::Span;

use crate::ui_consts::PREVIEW_WRAP_WIDTH;
use crate::wrapping::RtOptions;
use crate::wrapping::word_wrap_lines;

/// Wrap content with explicit prefixes for the first and subsequent lines.
pub(crate) fn wrap_prefixed_line(
    content: Vec<Span<'static>>,
    initial_prefix: Line<'static>,
    subsequent_prefix: Line<'static>,
) -> Vec<Line<'static>> {
    let base = Line::from(content);
    let lines = vec![base];
    word_wrap_lines(
        &lines,
        RtOptions::new(PREVIEW_WRAP_WIDTH)
            .initial_indent(initial_prefix)
            .subsequent_indent(subsequent_prefix),
    )
}

/// Wrap a bullet line with a standard bullet prefix.
pub(crate) fn wrap_bullet_line(content: Vec<Span<'static>>) -> Vec<Line<'static>> {
    wrap_prefixed_line(
        content,
        Line::from(vec!["  • ".dim()]),
        Line::from(vec!["    ".into()]),
    )
}

/// Wrap a subdetail line (tree connector) beneath a bullet.
pub(crate) fn wrap_subdetail_line(content: Vec<Span<'static>>) -> Vec<Line<'static>> {
    wrap_prefixed_line(
        content,
        Line::from(vec!["    └ ".dim()]),
        Line::from(vec!["      ".into()]),
    )
}

/// Wrap a plain line without any explicit prefix.
pub(crate) fn wrap_plain_line(content: Vec<Span<'static>>) -> Vec<Line<'static>> {
    let line = Line::from(content);
    let lines = [line];
    word_wrap_lines(&lines, RtOptions::new(PREVIEW_WRAP_WIDTH))
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_accounting_api::preview_copy::DUPLICATE_GUIDANCE_PREFIX;

    use ratatui::style::Stylize;
    use unicode_width::UnicodeWidthStr;

    fn flatten(line: &Line<'_>) -> String {
        line.spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<String>()
    }

    #[test]
    fn wrap_plain_line_limits_width_for_long_content() {
        let long_text = "wrap me ".repeat(12);
        let lines = wrap_plain_line(vec![long_text.into()]);
        assert!(lines.len() > 1);
        for line in &lines {
            let width = UnicodeWidthStr::width(flatten(line).as_str());
            assert!(
                width <= PREVIEW_WRAP_WIDTH,
                "line width {width} exceeded wrap width {PREVIEW_WRAP_WIDTH}"
            );
        }
    }

    #[test]
    fn wrap_bullet_line_handles_multibyte_and_styles() {
        let long_key = "重复键-デモ-very-long-key-1234567890".repeat(4);
        let lines = wrap_bullet_line(vec!["Duplicate".dim(), " ".into(), long_key.cyan().bold()]);
        assert!(lines.len() >= 2);
        assert!(flatten(&lines[0]).starts_with("  • "));
        assert!(flatten(&lines[1]).starts_with("    "));
        for line in &lines {
            let width = UnicodeWidthStr::width(flatten(line).as_str());
            assert!(
                width <= PREVIEW_WRAP_WIDTH,
                "bullet line width {width} exceeded wrap width {PREVIEW_WRAP_WIDTH}"
            );
        }
    }

    #[test]
    fn wrap_subdetail_line_preserves_duplicate_guidance_alignment() {
        let key = "REF-0000000000000000001";
        let lines =
            wrap_subdetail_line(vec![DUPLICATE_GUIDANCE_PREFIX.dim(), key.magenta().bold()]);
        assert!(!lines.is_empty());
        assert!(flatten(&lines[0]).starts_with("    └ "));
        let widths: Vec<usize> = lines
            .iter()
            .map(|line| UnicodeWidthStr::width(flatten(line).as_str()))
            .collect();
        assert!(
            widths.iter().all(|w| *w <= PREVIEW_WRAP_WIDTH),
            "subdetail widths {widths:?} exceed wrap width {PREVIEW_WRAP_WIDTH}"
        );
    }
}
