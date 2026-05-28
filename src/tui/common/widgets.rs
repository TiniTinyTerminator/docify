use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use super::theme::{COLOR_BORDER, COLOR_CONTENT, COLOR_HINT, COLOR_SEARCH, COLOR_TITLE};

// ── Text helpers ──────────────────────────────────────────────────────────────

/// Word-wrap `text` to `width` columns, preserving paragraph breaks (`\n`) and
/// leading indentation.
pub fn word_wrap(text: &str, width: usize) -> Vec<String> {
    if width < 4 {
        return vec![text.to_owned()];
    }
    let mut result = Vec::new();
    for para in text.split('\n') {
        if para.is_empty() {
            result.push(String::new());
            continue;
        }
        if is_preformatted_line(para) {
            result.push(para.to_owned());
            continue;
        }
        let indent: String = para.chars().take_while(|c| c.is_whitespace()).collect();
        let effective_width = width.saturating_sub(indent.len());
        let mut cur = String::new();
        for word in para.split_whitespace() {
            if cur.is_empty() {
                cur.push_str(&indent);
                cur.push_str(word);
            } else if cur.len() - indent.len() + 1 + word.len() <= effective_width {
                cur.push(' ');
                cur.push_str(word);
            } else {
                result.push(std::mem::take(&mut cur));
                cur.push_str(&indent);
                cur.push_str(word);
            }
        }
        if !cur.is_empty() {
            result.push(cur);
        }
    }
    result
}

fn is_preformatted_line(line: &str) -> bool {
    is_markdown_table_line(line)
        || contains_box_drawing(line)
        || contains_nonleading_repeated_spaces(line)
}

fn is_markdown_table_line(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with('|') && trimmed.ends_with('|') && trimmed.matches('|').count() >= 2
}

fn contains_box_drawing(line: &str) -> bool {
    line.chars().any(|ch| {
        matches!(
            ch,
            '─' | '━'
                | '│'
                | '┃'
                | '┌'
                | '┐'
                | '└'
                | '┘'
                | '├'
                | '┤'
                | '┬'
                | '┴'
                | '┼'
                | '╭'
                | '╮'
                | '╰'
                | '╯'
                | '⎛'
                | '⎞'
                | '⎝'
                | '⎠'
                | '⎮'
                | '⌠'
                | '⌡'
        )
    })
}

fn contains_nonleading_repeated_spaces(line: &str) -> bool {
    let trimmed = line.trim_start_matches(' ');
    trimmed.contains("  ")
}

/// Truncate `s` to at most `max` bytes, appending `…` if truncated.
pub fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_owned()
    } else {
        format!("{}…", &s[..max.saturating_sub(1)])
    }
}

// ── Shared widgets ────────────────────────────────────────────────────────────

/// Render a single-line search/filter bar with a blinking-cursor indicator.
pub fn render_search_bar(f: &mut Frame, title: &str, query: &str, area: Rect) {
    let display = format!("{query}_");
    let p = Paragraph::new(display)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(COLOR_SEARCH))
                .title(Span::styled(
                    title,
                    Style::default()
                        .fg(COLOR_SEARCH)
                        .add_modifier(Modifier::BOLD),
                )),
        )
        .style(Style::default().fg(COLOR_TITLE));
    f.render_widget(p, area);
}

/// Render a one-line status / hint bar.
pub fn render_hint_bar(f: &mut Frame, text: &str, area: Rect) {
    let span = Span::styled(text, Style::default().fg(COLOR_HINT));
    f.render_widget(Paragraph::new(Line::from(span)), area);
}

/// Render a one-line error bar (red text).
pub fn render_error_bar(f: &mut Frame, text: &str, area: Rect) {
    let span = Span::styled(text, Style::default().fg(ratatui::style::Color::Red));
    f.render_widget(Paragraph::new(Line::from(span)), area);
}

/// Render an inactive bordered panel with a grey "empty" hint inside.
pub fn render_empty_panel(f: &mut Frame, title: &str, hint: &str, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(COLOR_BORDER))
        .title(Span::styled(title, Style::default().fg(COLOR_TITLE)));
    let inner = block.inner(area);
    f.render_widget(block, area);
    let p = Paragraph::new(hint).style(Style::default().fg(COLOR_CONTENT));
    f.render_widget(p, inner);
}

#[cfg(test)]
mod tests {
    use super::word_wrap;

    #[test]
    fn word_wrap_preserves_box_drawing_math_layout() {
        let line = "│f(t)            │ 𝓛[f(t)]=F(s) │";
        assert_eq!(word_wrap(line, 12), vec![line.to_string()]);
    }

    #[test]
    fn word_wrap_preserves_repeated_internal_spaces() {
        let line = "e  f(t)         F(s-a)";
        assert_eq!(word_wrap(line, 10), vec![line.to_string()]);
    }

    #[test]
    fn word_wrap_preserves_markdown_table_rows() {
        let line = "| Parameter | Meaning |";
        assert_eq!(word_wrap(line, 8), vec![line.to_string()]);
    }
}
