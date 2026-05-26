use std::path::Path;

use crossterm::event::{
    self, Event, KeyCode, KeyModifiers, MouseButton, MouseEventKind,
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame, Terminal,
};

use crate::agent::extract_source;
use crate::extract::{DocItem, DocKind, TagKind};

use super::common::{
    enter_tui, highlight_style, leave_tui, render_hint_bar, render_search_bar, truncate, word_wrap,
    COLOR_BORDER, COLOR_CONTENT, COLOR_HINT, COLOR_SECTION, COLOR_TITLE,
};

// ── App state ─────────────────────────────────────────────────────────────────

struct App {
    all: Vec<DocItem>,

    query: String,
    cursor: usize,

    filtered: Vec<usize>,
    list_state: ListState,

    scroll: u16,
    show_source: bool,
    source_cache: Option<(usize, String)>,

    list_area: Rect,
}

impl App {
    fn new(items: Vec<DocItem>) -> Self {
        let n = items.len();
        let mut app = Self {
            all: items,
            query: String::new(),
            cursor: 0,
            filtered: (0..n).collect(),
            list_state: ListState::default(),
            scroll: 0,
            show_source: false,
            source_cache: None,
            list_area: Rect::default(),
        };
        if n > 0 {
            app.list_state.select(Some(0));
        }
        app
    }

    fn refilter(&mut self) {
        let q = self.query.to_ascii_lowercase();
        if q.is_empty() {
            self.filtered = (0..self.all.len()).collect();
        } else {
            self.filtered = (0..self.all.len())
                .filter(|&i| {
                    let item = &self.all[i];
                    item.name.to_ascii_lowercase().contains(&q)
                        || item.brief.to_ascii_lowercase().contains(&q)
                })
                .collect();
        }
        let sel = self
            .list_state
            .selected()
            .unwrap_or(0)
            .min(self.filtered.len().saturating_sub(1));
        if self.filtered.is_empty() {
            self.list_state.select(None);
        } else {
            self.list_state.select(Some(sel));
        }
        self.scroll = 0;
        self.show_source = false;
        self.source_cache = None;
    }

    fn selected_item(&self) -> Option<&DocItem> {
        let sel = self.list_state.selected()?;
        let idx = *self.filtered.get(sel)?;
        self.all.get(idx)
    }

    fn selected_idx(&self) -> Option<usize> {
        let sel = self.list_state.selected()?;
        self.filtered.get(sel).copied()
    }

    fn select(&mut self, pos: usize) {
        if self.filtered.is_empty() {
            return;
        }
        let pos = pos.min(self.filtered.len() - 1);
        self.list_state.select(Some(pos));
        self.scroll = 0;
        self.source_cache = None;
    }

    fn move_up(&mut self) {
        let cur = self.list_state.selected().unwrap_or(0);
        if cur > 0 {
            self.select(cur - 1);
        }
    }

    fn move_down(&mut self) {
        let cur = self.list_state.selected().unwrap_or(0);
        self.select(cur + 1);
    }

    fn source_text(&mut self) -> &str {
        let Some(idx) = self.selected_idx() else {
            return "";
        };
        if self.source_cache.as_ref().map(|(i, _)| *i) == Some(idx) {
            return &self.source_cache.as_ref().unwrap().1;
        }
        let src = extract_source(&self.all[idx], 200);
        self.source_cache = Some((idx, src));
        &self.source_cache.as_ref().unwrap().1
    }
}

// ── Public entry point ────────────────────────────────────────────────────────

pub fn run_doc_browser(dirs: &[&Path]) -> anyhow::Result<()> {
    let mut all_items = Vec::new();
    for dir in dirs {
        all_items.extend(crate::extract::extract_dir(dir).items);
    }

    let mut terminal = enter_tui()?;
    let result = run_loop(&mut terminal, all_items);
    leave_tui(&mut terminal)?;

    result
}

// ── Event loop ────────────────────────────────────────────────────────────────

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    items: Vec<DocItem>,
) -> anyhow::Result<()> {
    let mut app = App::new(items);

    loop {
        terminal.draw(|f| render(f, &mut app))?;

        if event::poll(std::time::Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(key) => match (key.code, key.modifiers) {
                    (KeyCode::Esc, _)
                    | (KeyCode::Char('c'), KeyModifiers::CONTROL)
                    | (KeyCode::Char('q'), KeyModifiers::NONE) => return Ok(()),

                    (KeyCode::Up, _) | (KeyCode::Char('k'), KeyModifiers::NONE) => app.move_up(),
                    (KeyCode::Down, _) | (KeyCode::Char('j'), KeyModifiers::NONE) => {
                        app.move_down()
                    }
                    (KeyCode::Tab, _) => {
                        app.show_source = !app.show_source;
                        app.scroll = 0;
                    }
                    (KeyCode::PageUp, _) => app.scroll = app.scroll.saturating_sub(5),
                    (KeyCode::PageDown, _) => app.scroll = app.scroll.saturating_add(5),

                    (KeyCode::Char(c), KeyModifiers::NONE)
                    | (KeyCode::Char(c), KeyModifiers::SHIFT) => {
                        app.query.insert(app.cursor, c);
                        app.cursor += c.len_utf8();
                        app.refilter();
                    }
                    (KeyCode::Backspace, _) => {
                        if app.cursor > 0 {
                            let prev = app.query[..app.cursor]
                                .char_indices()
                                .last()
                                .map(|(i, _)| i)
                                .unwrap_or(0);
                            app.query.drain(prev..app.cursor);
                            app.cursor = prev;
                            app.refilter();
                        }
                    }
                    (KeyCode::Delete, _) => {
                        if app.cursor < app.query.len() {
                            app.query.remove(app.cursor);
                            app.refilter();
                        }
                    }
                    (KeyCode::Home, _) => app.cursor = 0,
                    (KeyCode::End, _) => app.cursor = app.query.len(),
                    _ => {}
                },

                Event::Mouse(mouse) => match mouse.kind {
                    MouseEventKind::ScrollUp => app.move_up(),
                    MouseEventKind::ScrollDown => app.move_down(),
                    MouseEventKind::Down(MouseButton::Left) => {
                        let area = app.list_area;
                        let (x, y) = (mouse.column, mouse.row);
                        if x >= area.x
                            && x < area.x + area.width
                            && y > area.y
                            && y < area.y + area.height - 1
                        {
                            let row = (y - area.y - 1) as usize;
                            if row < app.filtered.len() {
                                app.select(row);
                            }
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    }
}

// ── Rendering ─────────────────────────────────────────────────────────────────

fn render(f: &mut Frame, app: &mut App) {
    let area = f.area();
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0), Constraint::Length(1)])
        .split(area);

    render_search_bar(f, " Filter symbols ", &app.query, outer[0]);
    render_body(f, app, outer[1]);
    render_status(f, app, outer[2]);
}

fn render_body(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(38), Constraint::Percentage(62)])
        .split(area);

    render_list(f, app, chunks[0]);
    render_detail(f, app, chunks[1]);
}

fn render_list(f: &mut Frame, app: &mut App, area: Rect) {
    app.list_area = area;
    let items: Vec<ListItem> = app
        .filtered
        .iter()
        .map(|&i| {
            let item = &app.all[i];
            let kind_chip = Span::styled(
                format!("{:<5}", item.kind.label()),
                Style::default()
                    .fg(kind_color(&item.kind))
                    .add_modifier(Modifier::BOLD),
            );
            let name_span = Span::styled(
                truncate(&item.name, 30),
                Style::default().fg(Color::White),
            );
            let lang_span = Span::styled(
                format!("  {}", item.lang.label()),
                Style::default().fg(COLOR_HINT),
            );
            ListItem::new(Line::from(vec![kind_chip, name_span, lang_span]))
        })
        .collect();

    let title = format!(" {} / {} ", app.filtered.len(), app.all.len());
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(COLOR_BORDER))
                .title(Span::styled(title, Style::default().fg(COLOR_TITLE))),
        )
        .highlight_style(highlight_style())
        .highlight_symbol("▶ ");

    f.render_stateful_widget(list, area, &mut app.list_state);
}

fn render_detail(f: &mut Frame, app: &mut App, area: Rect) {
    let show_source = app.show_source;
    let title = if show_source {
        " Source [Tab: doc] "
    } else {
        " Documentation [Tab: source] "
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(COLOR_BORDER))
        .title(Span::styled(title, Style::default().fg(COLOR_TITLE)));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if app.filtered.is_empty() {
        let hint = Paragraph::new("No symbols match.")
            .style(Style::default().fg(COLOR_CONTENT));
        f.render_widget(hint, inner);
        return;
    }

    let scroll = app.scroll;

    if show_source {
        let text = app.source_text().to_owned();
        if text.is_empty() {
            let p = Paragraph::new("Source not available.")
                .style(Style::default().fg(COLOR_CONTENT));
            f.render_widget(p, inner);
        } else {
            let lines: Vec<Line> = text
                .lines()
                .map(|l| Line::styled(l.to_owned(), Style::default().fg(Color::Gray)))
                .collect();
            f.render_widget(Paragraph::new(lines).scroll((scroll, 0)), inner);
        }
        return;
    }

    let Some(item) = app.selected_item() else { return };
    let item = item.clone();
    let width = inner.width as usize;

    let mut lines: Vec<Line> = Vec::new();

    // Header: kind chip + name + language
    lines.push(Line::from(vec![
        Span::styled(
            format!("[{}] ", item.kind.label()),
            Style::default()
                .fg(kind_color(&item.kind))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            item.name.clone(),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("  {}", item.lang.label()),
            Style::default().fg(Color::Yellow),
        ),
    ]));

    // File + line
    lines.push(Line::styled(
        format!("{}:{}", item.file.display(), item.line),
        Style::default().fg(COLOR_HINT),
    ));
    lines.push(Line::raw(""));

    // Brief
    if !item.brief.is_empty() {
        for l in word_wrap(&item.brief, width) {
            lines.push(Line::styled(l, Style::default().fg(Color::White)));
        }
        lines.push(Line::raw(""));
    }

    // Signature
    if !item.signature.is_empty() {
        for l in word_wrap(&item.display_signature(), width) {
            lines.push(Line::styled(l, Style::default().fg(Color::Rgb(150, 200, 150))));
        }
        lines.push(Line::raw(""));
    }

    // Body (extended description)
    if !item.body.is_empty() {
        for l in word_wrap(&item.body, width) {
            lines.push(Line::styled(l, Style::default().fg(COLOR_CONTENT)));
        }
        lines.push(Line::raw(""));
    }

    // Parameters
    let params: Vec<_> = item.tags.iter().filter(|t| t.kind == TagKind::Param).collect();
    if !params.is_empty() {
        lines.push(section_header("Parameters"));
        for p in &params {
            let name_part = p.name.as_deref().map(|n| format!("{n}: ")).unwrap_or_default();
            for l in word_wrap(&format!("  {}{}", name_part, p.text), width) {
                lines.push(Line::styled(l, Style::default().fg(COLOR_CONTENT)));
            }
        }
        lines.push(Line::raw(""));
    }

    // Returns
    if let Some(ret) = item.tags.iter().find(|t| t.kind == TagKind::Return) {
        if !ret.text.is_empty() {
            lines.push(section_header("Returns"));
            for l in word_wrap(&ret.text, width) {
                lines.push(Line::styled(format!("  {l}"), Style::default().fg(COLOR_CONTENT)));
            }
            lines.push(Line::raw(""));
        }
    }

    // Throws
    let throws: Vec<_> = item
        .tags
        .iter()
        .filter(|t| matches!(&t.kind, TagKind::Other(s) if s.starts_with("throws")))
        .collect();
    if !throws.is_empty() {
        lines.push(section_header("Throws"));
        for t in &throws {
            let exc = match &t.kind {
                TagKind::Other(s) => s.trim_start_matches("throws").trim().to_owned(),
                _ => String::new(),
            };
            let text = if exc.is_empty() {
                format!("  {}", t.text)
            } else {
                format!("  {exc}: {}", t.text)
            };
            lines.push(Line::styled(text, Style::default().fg(Color::Rgb(200, 120, 80))));
        }
        lines.push(Line::raw(""));
    }

    // Notes
    for tag in item.tags.iter().filter(|t| t.kind == TagKind::Note) {
        lines.push(section_header("Note"));
        for l in word_wrap(&tag.text, width) {
            lines.push(Line::styled(format!("  {l}"), Style::default().fg(COLOR_CONTENT)));
        }
        lines.push(Line::raw(""));
    }

    // See also
    for tag in item.tags.iter().filter(|t| t.kind == TagKind::See) {
        if !tag.text.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("See also  ", Style::default().fg(COLOR_SECTION).add_modifier(Modifier::BOLD)),
                Span::styled(tag.text.clone(), Style::default().fg(Color::Cyan)),
            ]));
            lines.push(Line::raw(""));
        }
    }

    // Other tags (Warning, Deprecated, Since, Example, …)
    for tag in &item.tags {
        match &tag.kind {
            TagKind::Param | TagKind::Return | TagKind::Note | TagKind::See | TagKind::Brief => {}
            TagKind::Other(s) if s.starts_with("throws") => {}
            TagKind::Other(_)
            | TagKind::Warning
            | TagKind::Deprecated
            | TagKind::Since
            | TagKind::Example => {
                lines.push(section_header(tag.kind.label()));
                for l in word_wrap(&tag.text, width) {
                    lines.push(Line::styled(format!("  {l}"), Style::default().fg(COLOR_CONTENT)));
                }
                lines.push(Line::raw(""));
            }
        }
    }

    f.render_widget(
        Paragraph::new(lines).wrap(Wrap { trim: false }).scroll((scroll, 0)),
        inner,
    );
}

fn render_status(f: &mut Frame, app: &App, area: Rect) {
    if app.all.is_empty() {
        render_hint_bar(f, "No documented symbols found.", area);
    } else {
        render_hint_bar(
            f,
            " ↑↓/jk navigate   Tab source   PgUp/PgDn scroll   type to filter   q quit",
            area,
        );
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Colour coding by symbol kind.
fn kind_color(kind: &DocKind) -> Color {
    match kind {
        DocKind::Function | DocKind::Subroutine => Color::Rgb(100, 180, 255),
        DocKind::Struct | DocKind::Class        => Color::Rgb(180, 140, 255),
        DocKind::Enum                           => Color::Rgb(255, 180,  80),
        DocKind::Typedef                        => Color::Rgb(130, 220, 180),
        DocKind::Variable                       => Color::Rgb(200, 200, 100),
        DocKind::Macro                          => Color::Rgb(255, 120, 120),
        DocKind::Module | DocKind::Interface    => Color::Rgb(120, 220, 200),
        DocKind::Unknown                        => Color::DarkGray,
    }
}

/// Bold yellow section header (Parameters, Returns, …).
fn section_header(label: &str) -> Line<'static> {
    Line::styled(
        label.to_owned(),
        Style::default()
            .fg(COLOR_SECTION)
            .add_modifier(Modifier::BOLD),
    )
}
