use std::collections::{BTreeMap, HashSet};
use std::path::Path;

use crossterm::event::{self, Event, KeyCode, KeyModifiers, MouseButton, MouseEventKind};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame, Terminal,
};

use crate::extract::{DocItem, DocKind, DocLanguage, TagKind};

use super::common::{
    enter_tui, highlight_style, leave_tui, render_hint_bar, render_search_bar, truncate, word_wrap,
    COLOR_BORDER, COLOR_CONTENT, COLOR_HINT, COLOR_SECTION, COLOR_TITLE,
};

// ── App state ─────────────────────────────────────────────────────────────────

#[derive(Clone)]
struct TreeRow {
    depth: usize,
    label: String,
    count: usize,
    kind: TreeRowKind,
}

#[derive(Clone)]
struct DetailLink {
    line: usize,
    start_col: usize,
    end_col: usize,
    target_idx: usize,
}

#[derive(Clone)]
enum TreeRowKind {
    Group {
        key: String,
        expanded: bool,
        item_idx: Option<usize>,
        source_idx: Option<usize>,
    },
    Item(usize),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FocusPane {
    Tree,
    Detail,
}

#[derive(Default)]
struct TreeNode {
    key: String,
    label: String,
    count: usize,
    item_idx: Option<usize>,
    source_idx: Option<usize>,
    children: BTreeMap<String, TreeNode>,
    items: Vec<usize>,
}

impl TreeNode {
    fn insert(&mut self, path: &[String], item_idx: usize, group_item: bool, source_item: bool) {
        self.count += 1;
        let Some((label, rest)) = path.split_first() else {
            self.items.push(item_idx);
            return;
        };
        let key = if self.key.is_empty() {
            label.clone()
        } else {
            format!("{}\u{1f}{label}", self.key)
        };
        self.children
            .entry(label.to_ascii_lowercase())
            .or_insert_with(|| TreeNode {
                key,
                label: label.clone(),
                ..TreeNode::default()
            })
            .insert(rest, item_idx, group_item, source_item);

        if rest.is_empty() && group_item {
            if let Some(node) = self.children.get_mut(&label.to_ascii_lowercase()) {
                node.item_idx = Some(item_idx);
                node.items.retain(|&idx| idx != item_idx);
            }
        }
        if rest.is_empty() && source_item {
            if let Some(node) = self.children.get_mut(&label.to_ascii_lowercase()) {
                node.source_idx.get_or_insert(item_idx);
            }
        }
    }
}

struct App {
    all: Vec<DocItem>,

    query: String,
    cursor: usize,

    matching: Vec<usize>,
    rows: Vec<TreeRow>,
    expanded: HashSet<String>,
    list_state: ListState,
    list_scroll: usize,

    scroll: u16,
    show_source: bool,
    source_cache: Option<(usize, String)>,
    overloads_open: bool,
    focus: FocusPane,

    list_area: Rect,
    detail_area: Rect,
    detail_inner_area: Rect,
    detail_links: Vec<DetailLink>,
}

impl App {
    fn new(items: Vec<DocItem>) -> Self {
        let n = items.len();
        let mut app = Self {
            all: items,
            query: String::new(),
            cursor: 0,
            matching: (0..n).collect(),
            rows: Vec::new(),
            expanded: HashSet::new(),
            list_state: ListState::default(),
            list_scroll: 0,
            scroll: 0,
            show_source: false,
            source_cache: None,
            overloads_open: false,
            focus: FocusPane::Tree,
            list_area: Rect::default(),
            detail_area: Rect::default(),
            detail_inner_area: Rect::default(),
            detail_links: Vec::new(),
        };
        app.sort_matching();
        app.rebuild_rows();
        if !app.rows.is_empty() {
            app.list_state.select(Some(0));
        }
        app
    }

    fn refilter(&mut self) {
        let q = self.query.to_ascii_lowercase();
        if q.is_empty() {
            self.matching = (0..self.all.len()).collect();
        } else {
            self.matching = (0..self.all.len())
                .filter(|&i| {
                    let item = &self.all[i];
                    item.name.to_ascii_lowercase().contains(&q)
                        || item.brief.to_ascii_lowercase().contains(&q)
                })
                .collect();
        }
        self.sort_matching();
        self.rebuild_rows();
        let sel = self
            .list_state
            .selected()
            .unwrap_or(0)
            .min(self.rows.len().saturating_sub(1));
        if self.rows.is_empty() {
            self.list_state.select(None);
        } else {
            self.list_state.select(Some(sel));
        }
        self.clamp_list_scroll();
        self.ensure_selection_visible();
        self.scroll = 0;
        self.show_source = false;
        self.source_cache = None;
        self.overloads_open = false;
    }

    fn sort_matching(&mut self) {
        self.matching.sort_by(|&a, &b| {
            let left = &self.all[a];
            let right = &self.all[b];
            item_sort_key(left).cmp(&item_sort_key(right))
        });
    }

    fn rebuild_rows(&mut self) {
        let mut root = TreeNode::default();
        for &idx in &self.matching {
            root.insert(
                &tree_path(&self.all[idx]),
                idx,
                is_group_doc_item(&self.all[idx]),
                true,
            );
        }

        let mut rows = Vec::new();
        let auto_expand = !self.query.is_empty();
        for node in root.children.values() {
            push_tree_rows(node, 0, auto_expand, &self.expanded, &mut rows);
        }
        self.rows = rows;
    }

    fn selected_item(&self) -> Option<&DocItem> {
        let sel = self.list_state.selected()?;
        match self.rows.get(sel)?.kind {
            TreeRowKind::Item(idx) => self.all.get(idx),
            TreeRowKind::Group { item_idx, .. } => item_idx.and_then(|idx| self.all.get(idx)),
        }
    }

    fn selected_idx(&self) -> Option<usize> {
        let sel = self.list_state.selected()?;
        match self.rows.get(sel)?.kind {
            TreeRowKind::Item(idx) => Some(idx),
            TreeRowKind::Group { item_idx, .. } => item_idx,
        }
    }

    fn selected_source_idx(&self) -> Option<usize> {
        let sel = self.list_state.selected()?;
        match self.rows.get(sel)?.kind {
            TreeRowKind::Item(idx) => Some(idx),
            TreeRowKind::Group {
                item_idx,
                source_idx,
                ..
            } => item_idx.or(source_idx),
        }
    }

    fn select(&mut self, pos: usize) {
        if self.rows.is_empty() {
            return;
        }
        let pos = pos.min(self.rows.len() - 1);
        self.list_state.select(Some(pos));
        self.ensure_selection_visible();
        self.source_cache = None;
        self.overloads_open = false;
        self.scroll = if self.show_source {
            self.source_initial_scroll()
        } else {
            0
        };
    }

    fn select_item_idx(&mut self, idx: usize) {
        if !self.matching.contains(&idx) {
            self.query.clear();
            self.matching = (0..self.all.len()).collect();
            self.sort_matching();
        }

        let mut key = String::new();
        for part in tree_path(&self.all[idx]) {
            if key.is_empty() {
                key = part;
            } else {
                key = format!("{}\u{1f}{part}", key);
            }
            self.expanded.insert(key.clone());
        }
        self.rebuild_rows();

        if let Some(pos) = self.rows.iter().position(|row| match row.kind {
            TreeRowKind::Item(item_idx) => item_idx == idx,
            TreeRowKind::Group { item_idx, .. } => item_idx == Some(idx),
        }) {
            self.show_source = false;
            self.select(pos);
        }
    }

    fn toggle_source(&mut self) {
        self.show_source = !self.show_source;
        self.scroll = if self.show_source {
            self.source_initial_scroll()
        } else {
            0
        };
    }

    fn source_initial_scroll(&self) -> u16 {
        let Some(item) = self.selected_source_idx().and_then(|idx| self.all.get(idx)) else {
            return 0;
        };
        declaration_line_idx(item)
            .saturating_sub(4)
            .min(u16::MAX as usize) as u16
    }

    fn toggle_overloads(&mut self) {
        if self
            .selected_idx()
            .is_some_and(|idx| overload_indices(&self.all, idx).len() > 1)
        {
            self.overloads_open = !self.overloads_open;
            self.show_source = false;
            self.scroll = 0;
        }
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

    fn toggle_selected(&mut self) {
        let Some(sel) = self.list_state.selected() else {
            return;
        };
        let Some(row) = self.rows.get(sel) else {
            return;
        };
        let TreeRowKind::Group { key, .. } = &row.kind else {
            return;
        };
        if !self.expanded.insert(key.clone()) {
            self.expanded.remove(key);
        }
        self.rebuild_rows();
        if self.rows.is_empty() {
            self.list_state.select(None);
        } else {
            self.list_state.select(Some(sel.min(self.rows.len() - 1)));
        }
        self.clamp_list_scroll();
        self.ensure_selection_visible();
        self.source_cache = None;
        self.scroll = if self.show_source {
            self.source_initial_scroll()
        } else {
            0
        };
    }

    fn visible_tree_rows(&self) -> usize {
        self.list_area.height.saturating_sub(2).max(1) as usize
    }

    fn clamp_list_scroll(&mut self) {
        let visible = self.visible_tree_rows();
        let max_scroll = self.rows.len().saturating_sub(visible);
        self.list_scroll = self.list_scroll.min(max_scroll);
    }

    fn ensure_selection_visible(&mut self) {
        let Some(sel) = self.list_state.selected() else {
            return;
        };
        let visible = self.visible_tree_rows();
        if sel < self.list_scroll {
            self.list_scroll = sel;
        } else if sel >= self.list_scroll + visible {
            self.list_scroll = sel.saturating_sub(visible.saturating_sub(1));
        }
        self.clamp_list_scroll();
    }

    fn scroll_tree_up(&mut self) {
        self.list_scroll = self.list_scroll.saturating_sub(3);
    }

    fn scroll_tree_down(&mut self) {
        self.list_scroll = self.list_scroll.saturating_add(3);
        self.clamp_list_scroll();
    }

    fn scroll_focused_up(&mut self) {
        match self.focus {
            FocusPane::Tree => self.scroll_tree_up(),
            FocusPane::Detail => self.scroll = self.scroll.saturating_sub(3),
        }
    }

    fn scroll_focused_down(&mut self) {
        match self.focus {
            FocusPane::Tree => self.scroll_tree_down(),
            FocusPane::Detail => self.scroll = self.scroll.saturating_add(3),
        }
    }

    fn page_focused_up(&mut self) {
        match self.focus {
            FocusPane::Tree => {
                let amount = self.visible_tree_rows();
                self.list_scroll = self.list_scroll.saturating_sub(amount);
            }
            FocusPane::Detail => self.scroll = self.scroll.saturating_sub(5),
        }
    }

    fn page_focused_down(&mut self) {
        match self.focus {
            FocusPane::Tree => {
                self.list_scroll = self.list_scroll.saturating_add(self.visible_tree_rows());
                self.clamp_list_scroll();
            }
            FocusPane::Detail => self.scroll = self.scroll.saturating_add(5),
        }
    }

    fn focus_at(&mut self, x: u16, y: u16) {
        if contains(self.list_area, x, y) {
            self.focus = FocusPane::Tree;
        } else if contains(self.detail_area, x, y) {
            self.focus = FocusPane::Detail;
        }
    }

    fn activate_detail_link(&mut self, x: u16, y: u16) -> bool {
        if self.show_source || !contains(self.detail_inner_area, x, y) {
            return false;
        }
        let line = self.scroll as usize + (y - self.detail_inner_area.y) as usize;
        let col = (x - self.detail_inner_area.x) as usize;
        let Some(target_idx) = self
            .detail_links
            .iter()
            .find(|link| link.line == line && col >= link.start_col && col < link.end_col)
            .map(|link| link.target_idx)
        else {
            return false;
        };
        self.select_item_idx(target_idx);
        true
    }

    fn source_text(&mut self) -> &str {
        let Some(idx) = self.selected_source_idx() else {
            return "";
        };
        if self.source_cache.as_ref().map(|(i, _)| *i) == Some(idx) {
            return &self.source_cache.as_ref().unwrap().1;
        }
        let src = std::fs::read_to_string(&self.all[idx].file).unwrap_or_default();
        self.source_cache = Some((idx, src));
        &self.source_cache.as_ref().unwrap().1
    }
}

// ── Public entry point ────────────────────────────────────────────────────────

pub fn run_doc_browser(dirs: &[&Path]) -> anyhow::Result<()> {
    let mut all_items = Vec::new();
    let scan_dirs = crate::project::expand_scan_dirs(dirs);
    for dir in &scan_dirs {
        all_items.extend(crate::extract::extract_dir(dir).items);
    }

    // Dedup across multiple scan dirs: same symbol from a parent dir and a
    // subdirectory that was also added (e.g. via project manifest path-deps).
    // Key on (name, file, line); keep whichever copy has more tags/content.
    let all_items = dedup_items(all_items);

    let mut terminal = enter_tui()?;
    let result = run_loop(&mut terminal, all_items);
    leave_tui(&mut terminal)?;

    result
}

fn dedup_items(items: Vec<crate::extract::DocItem>) -> Vec<crate::extract::DocItem> {
    use std::collections::HashMap;
    let mut seen: HashMap<(String, std::path::PathBuf, usize), usize> = HashMap::new();
    let mut out: Vec<crate::extract::DocItem> = Vec::new();
    for item in items {
        let key = (item.name.clone(), item.file.clone(), item.line);
        let score = item.tags.len() * 10 + item.brief.len() + item.body.len();
        match seen.get(&key).copied() {
            Some(idx) => {
                let prev = out[idx].tags.len() * 10 + out[idx].brief.len() + out[idx].body.len();
                if score > prev {
                    out[idx] = item;
                }
            }
            None => {
                seen.insert(key, out.len());
                out.push(item);
            }
        }
    }
    out
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
                    (KeyCode::Enter, _) | (KeyCode::Char(' '), KeyModifiers::NONE) => {
                        app.toggle_selected()
                    }
                    (KeyCode::Tab, _) => app.toggle_source(),
                    (KeyCode::Char('o'), KeyModifiers::NONE) => app.toggle_overloads(),
                    (KeyCode::PageUp, _) => app.page_focused_up(),
                    (KeyCode::PageDown, _) => app.page_focused_down(),

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
                    MouseEventKind::ScrollUp => {
                        app.focus_at(mouse.column, mouse.row);
                        app.scroll_focused_up();
                    }
                    MouseEventKind::ScrollDown => {
                        app.focus_at(mouse.column, mouse.row);
                        app.scroll_focused_down();
                    }
                    MouseEventKind::Moved => app.focus_at(mouse.column, mouse.row),
                    MouseEventKind::Down(MouseButton::Left) => {
                        app.focus_at(mouse.column, mouse.row);
                        if app.activate_detail_link(mouse.column, mouse.row) {
                            continue;
                        }
                        let area = app.list_area;
                        let (x, y) = (mouse.column, mouse.row);
                        if x >= area.x
                            && x < area.x + area.width
                            && y > area.y
                            && y < area.y + area.height - 1
                        {
                            let row = app.list_scroll + (y - area.y - 1) as usize;
                            if row < app.rows.len() {
                                app.select(row);
                                app.toggle_selected();
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
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
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
    app.clamp_list_scroll();
    let row_width = area.width.saturating_sub(4).max(1) as usize;
    let items: Vec<ListItem> = app
        .rows
        .iter()
        .skip(app.list_scroll)
        .take(app.visible_tree_rows())
        .map(|row| {
            let indent = "  ".repeat(row.depth);
            match row.kind {
                TreeRowKind::Group {
                    expanded, item_idx, ..
                } => {
                    let marker = if expanded { "▾" } else { "▸" };
                    let label_style = group_label_style(item_idx.and_then(|idx| app.all.get(idx)));
                    if let Some(idx) = item_idx {
                        let kind = tui_kind_label(&app.all[idx].kind);
                        let indent_width = indent.chars().count() + 2;
                        let count = format!("  {}", row.count);
                        let available = row_width.saturating_sub(indent_width);
                        let kind_width = kind.chars().count();
                        let label_width = available
                            .saturating_sub(count.chars().count() + kind_width + 1)
                            .max(1);
                        let label = truncate(&row.label, label_width);
                        let used = label.chars().count() + count.chars().count() + kind_width;
                        let gap = available.saturating_sub(used).max(1);
                        ListItem::new(Line::from(vec![
                            Span::raw(indent),
                            Span::styled(marker, Style::default().fg(COLOR_HINT)),
                            Span::raw(" "),
                            Span::styled(label, label_style),
                            Span::styled(count, Style::default().fg(COLOR_HINT)),
                            Span::raw(" ".repeat(gap)),
                            Span::styled(kind, Style::default().fg(kind_color(&app.all[idx].kind))),
                        ]))
                    } else {
                        ListItem::new(Line::from(vec![
                            Span::raw(indent),
                            Span::styled(marker, Style::default().fg(COLOR_HINT)),
                            Span::raw(" "),
                            Span::styled(truncate(&row.label, 34), label_style),
                            Span::styled(
                                format!("  {}", row.count),
                                Style::default().fg(COLOR_HINT),
                            ),
                        ]))
                    }
                }
                TreeRowKind::Item(idx) => {
                    let item = &app.all[idx];
                    let kind = tui_kind_label(&item.kind);
                    let indent_width = indent.chars().count() + 2;
                    let available = row_width.saturating_sub(indent_width);
                    let kind_width = kind.chars().count();
                    let min_gap = 1usize;
                    let name_width = available.saturating_sub(kind_width + min_gap).max(1);
                    let name = truncate(simple_name(&item.name), name_width);
                    let used = name.chars().count() + kind_width;
                    let gap = available.saturating_sub(used).max(min_gap);
                    let name_span = Span::styled(name, tree_name_style(&item.kind));
                    ListItem::new(Line::from(vec![
                        Span::raw(indent),
                        Span::raw("  "),
                        name_span,
                        Span::raw(" ".repeat(gap)),
                        Span::styled(
                            kind,
                            Style::default()
                                .fg(kind_color(&item.kind))
                                .add_modifier(Modifier::BOLD),
                        ),
                    ]))
                }
            }
        })
        .collect();

    let title = format!(" {} / {} ", app.matching.len(), app.all.len());
    let selected = app
        .list_state
        .selected()
        .and_then(|selected| selected.checked_sub(app.list_scroll))
        .filter(|&selected| selected < items.len());
    let mut visible_state = ListState::default();
    visible_state.select(selected);

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(panel_border_style(app.focus == FocusPane::Tree))
                .title(Span::styled(title, Style::default().fg(COLOR_TITLE))),
        )
        .highlight_style(highlight_style())
        .highlight_symbol("▶ ");

    f.render_stateful_widget(list, area, &mut visible_state);
}

fn render_detail(f: &mut Frame, app: &mut App, area: Rect) {
    app.detail_area = area;
    let show_source = app.show_source;
    let title = if show_source {
        " Source [Tab: doc] "
    } else {
        " Documentation [Tab: source] "
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(panel_border_style(app.focus == FocusPane::Detail))
        .title(Span::styled(title, Style::default().fg(COLOR_TITLE)));

    let inner = block.inner(area);
    app.detail_inner_area = inner;
    app.detail_links.clear();
    f.render_widget(block, area);

    if app.rows.is_empty() {
        let hint = Paragraph::new("No symbols match.").style(Style::default().fg(COLOR_CONTENT));
        f.render_widget(hint, inner);
        return;
    }

    let selected_idx = app.selected_idx();
    let selected_source_idx = app.selected_source_idx();

    if show_source {
        let text = app.source_text().to_owned();
        if text.is_empty() {
            let p =
                Paragraph::new("Source not available.").style(Style::default().fg(COLOR_CONTENT));
            f.render_widget(p, inner);
        } else {
            let lang = selected_language(app);
            let decl_line = selected_source_idx
                .and_then(|idx| app.all.get(idx))
                .map(declaration_line_idx);
            let lines: Vec<Line> = text
                .lines()
                .enumerate()
                .map(|(idx, line)| highlight_source_line(idx, line, &lang, decl_line))
                .collect();
            app.scroll = clamped_scroll(app.scroll, lines.len(), inner.height);
            f.render_widget(Paragraph::new(lines).scroll((app.scroll, 0)), inner);
        }
        return;
    }

    let Some(item) = app.selected_item() else {
        let hint = Paragraph::new("No documentation for this item. Press Tab to view source.")
            .style(Style::default().fg(COLOR_CONTENT));
        f.render_widget(hint, inner);
        return;
    };
    let item = item.clone();
    let width = inner.width as usize;

    let mut lines: Vec<Line<'static>> = Vec::new();

    // Header: kind chip + name + language
    lines.push(Line::from(vec![
        Span::styled(
            format!("[{}] ", tui_kind_label(&item.kind)),
            Style::default()
                .fg(kind_color(&item.kind))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            item.name.clone(),
            Style::default()
                .fg(kind_color(&item.kind))
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

    if let Some(idx) = selected_idx {
        push_overload_dropdown(
            &mut lines,
            &app.all,
            idx,
            app.overloads_open,
            &mut app.detail_links,
        );
    }

    // Brief
    if !item.brief.is_empty() {
        for l in word_wrap(&doc_text(&item.brief), width) {
            let line_no = lines.len();
            lines.push(linkable_text_line(
                &l,
                &app.all,
                selected_idx,
                &item.lang,
                line_no,
                &mut app.detail_links,
                Style::default().fg(Color::White),
            ));
        }
        lines.push(Line::raw(""));
    }

    // Signature
    if !item.signature.is_empty() {
        for l in word_wrap(&item.display_signature(), width) {
            let line_no = lines.len();
            lines.push(highlight_code_line_with_links(
                &l,
                &item.lang,
                &app.all,
                selected_idx,
                line_no,
                &mut app.detail_links,
            ));
        }
        lines.push(Line::raw(""));
    }

    // Body (extended description)
    if !item.body.is_empty() {
        for l in word_wrap(&doc_text(&item.body), width) {
            let line_no = lines.len();
            lines.push(linkable_text_line(
                &l,
                &app.all,
                selected_idx,
                &item.lang,
                line_no,
                &mut app.detail_links,
                Style::default().fg(COLOR_CONTENT),
            ));
        }
        lines.push(Line::raw(""));
    }

    if matches!(
        item.kind,
        DocKind::Class | DocKind::Struct | DocKind::Interface
    ) {
        push_type_overview(
            &mut lines,
            &app.all,
            selected_idx.unwrap_or(usize::MAX),
            width,
            selected_idx,
            &item.lang,
            &mut app.detail_links,
        );
    }

    push_param_return_table(&mut lines, &item, width);

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
            lines.push(Line::styled(
                text,
                Style::default().fg(Color::Rgb(200, 120, 80)),
            ));
        }
        lines.push(Line::raw(""));
    }

    // Notes
    for tag in item.tags.iter().filter(|t| t.kind == TagKind::Note) {
        lines.push(section_header("Note"));
        for l in word_wrap(&doc_text(&tag.text), width) {
            let line_no = lines.len();
            lines.push(linkable_text_line(
                &format!("  {l}"),
                &app.all,
                selected_idx,
                &item.lang,
                line_no,
                &mut app.detail_links,
                Style::default().fg(COLOR_CONTENT),
            ));
        }
        lines.push(Line::raw(""));
    }

    // See also
    for tag in item.tags.iter().filter(|t| t.kind == TagKind::See) {
        if !tag.text.is_empty() {
            let line_no = lines.len();
            lines.push(see_also_line(
                &doc_text(&tag.text),
                &app.all,
                selected_idx,
                &item.lang,
                line_no,
                &mut app.detail_links,
            ));
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
                for l in word_wrap(&doc_text(&tag.text), width) {
                    let line_no = lines.len();
                    lines.push(linkable_text_line(
                        &format!("  {l}"),
                        &app.all,
                        selected_idx,
                        &item.lang,
                        line_no,
                        &mut app.detail_links,
                        Style::default().fg(COLOR_CONTENT),
                    ));
                }
                lines.push(Line::raw(""));
            }
        }
    }

    app.scroll = clamped_scroll(app.scroll, lines.len(), inner.height);
    f.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .scroll((app.scroll, 0)),
        inner,
    );
}

fn render_status(f: &mut Frame, app: &App, area: Rect) {
    if app.all.is_empty() {
        render_hint_bar(f, "No documented symbols found.", area);
    } else {
        render_hint_bar(
            f,
            " ↑↓/jk navigate   Enter/click open   o overloads   click refs/types jump   Tab source   PgUp/PgDn scroll   q quit",
            area,
        );
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn selected_language(app: &App) -> DocLanguage {
    app.selected_source_idx()
        .and_then(|idx| app.all.get(idx))
        .or_else(|| app.selected_item())
        .map(|item| item.lang.clone())
        .unwrap_or(DocLanguage::Unknown)
}

fn tui_kind_label(kind: &DocKind) -> &'static str {
    match kind {
        DocKind::Function => "func",
        _ => kind.label(),
    }
}

fn group_label_style(item: Option<&DocItem>) -> Style {
    let color = match item {
        Some(item)
            if matches!(
                item.kind,
                DocKind::Class
                    | DocKind::Struct
                    | DocKind::Interface
                    | DocKind::Enum
                    | DocKind::Typedef
                    | DocKind::Variable
                    | DocKind::Macro
            ) =>
        {
            kind_color(&item.kind)
        }
        _ => COLOR_TITLE,
    };
    Style::default().fg(color).add_modifier(Modifier::BOLD)
}

fn tree_name_style(kind: &DocKind) -> Style {
    Style::default().fg(kind_color(kind))
}

fn linked_symbol_style(kind: &DocKind) -> Style {
    Style::default()
        .fg(kind_color(kind))
        .add_modifier(Modifier::UNDERLINED)
}

fn doc_text(text: &str) -> String {
    #[cfg(feature = "rich-math")]
    {
        crate::util::latex::render_math_lines(text)
    }
    #[cfg(not(feature = "rich-math"))]
    {
        text.to_string()
    }
}

fn see_also_line(
    text: &str,
    all: &[DocItem],
    current_idx: Option<usize>,
    lang: &DocLanguage,
    line_no: usize,
    links: &mut Vec<DetailLink>,
) -> Line<'static> {
    let prefix = "See also  ";
    let mut spans = vec![Span::styled(
        prefix,
        Style::default()
            .fg(COLOR_SECTION)
            .add_modifier(Modifier::BOLD),
    )];
    push_reference_spans(
        &mut spans,
        text,
        prefix.chars().count(),
        all,
        current_idx,
        lang,
        line_no,
        links,
    );
    Line::from(spans)
}

fn push_type_overview(
    lines: &mut Vec<Line<'static>>,
    all: &[DocItem],
    type_idx: usize,
    width: usize,
    current_idx: Option<usize>,
    lang: &DocLanguage,
    links: &mut Vec<DetailLink>,
) {
    let Some(type_item) = all.get(type_idx) else {
        return;
    };
    let members = type_members(all, type_item);
    let public_functions: Vec<_> = members
        .iter()
        .copied()
        .filter(|member| {
            matches!(member.kind, DocKind::Function | DocKind::Subroutine)
                && !matches!(
                    member.meta.access,
                    Some(crate::extract::Access::Private | crate::extract::Access::Protected)
                )
        })
        .collect();
    let protected_functions: Vec<_> = members
        .iter()
        .copied()
        .filter(|member| {
            matches!(member.kind, DocKind::Function | DocKind::Subroutine)
                && matches!(member.meta.access, Some(crate::extract::Access::Protected))
        })
        .collect();
    let private_functions: Vec<_> = members
        .iter()
        .copied()
        .filter(|member| {
            matches!(member.kind, DocKind::Function | DocKind::Subroutine)
                && matches!(member.meta.access, Some(crate::extract::Access::Private))
        })
        .collect();
    let variables: Vec<_> = members
        .iter()
        .copied()
        .filter(|member| matches!(member.kind, DocKind::Variable))
        .collect();
    let types: Vec<_> = members
        .iter()
        .copied()
        .filter(|member| {
            matches!(
                member.kind,
                DocKind::Class
                    | DocKind::Struct
                    | DocKind::Interface
                    | DocKind::Enum
                    | DocKind::Typedef
            )
        })
        .collect();
    let others: Vec<_> = members
        .iter()
        .copied()
        .filter(|member| {
            !matches!(
                member.kind,
                DocKind::Function
                    | DocKind::Subroutine
                    | DocKind::Variable
                    | DocKind::Class
                    | DocKind::Struct
                    | DocKind::Interface
                    | DocKind::Enum
                    | DocKind::Typedef
            )
        })
        .collect();
    let bases = inheritance_list(type_item);

    if public_functions.is_empty()
        && protected_functions.is_empty()
        && private_functions.is_empty()
        && variables.is_empty()
        && types.is_empty()
        && others.is_empty()
        && bases.is_empty()
    {
        return;
    }

    if !bases.is_empty() {
        let text = format!("  Inherits {}", bases.join(", "));
        for line in word_wrap(&text, width) {
            lines.push(Line::styled(line, Style::default().fg(COLOR_CONTENT)));
        }
    }

    push_member_section(
        lines,
        "Public functions",
        &public_functions,
        width,
        false,
        all,
        current_idx,
        lang,
        links,
    );
    push_member_section(
        lines,
        "Protected functions",
        &protected_functions,
        width,
        false,
        all,
        current_idx,
        lang,
        links,
    );
    push_member_section(
        lines,
        "Private functions",
        &private_functions,
        width,
        false,
        all,
        current_idx,
        lang,
        links,
    );
    push_member_section(
        lines,
        "Variables",
        &variables,
        width,
        true,
        all,
        current_idx,
        lang,
        links,
    );
    push_member_section(
        lines,
        "Types",
        &types,
        width,
        true,
        all,
        current_idx,
        lang,
        links,
    );
    push_member_section(
        lines,
        "Other members",
        &others,
        width,
        true,
        all,
        current_idx,
        lang,
        links,
    );

    lines.push(Line::raw(""));
}

fn push_member_section(
    lines: &mut Vec<Line<'static>>,
    title: &str,
    members: &[&DocItem],
    width: usize,
    show_access: bool,
    all: &[DocItem],
    current_idx: Option<usize>,
    lang: &DocLanguage,
    links: &mut Vec<DetailLink>,
) {
    if members.is_empty() {
        return;
    }

    lines.push(section_header(title));
    for member in members {
        let mut text = member_display_text(member);
        if show_access {
            let access = access_label(member);
            if !access.is_empty() {
                text = format!("{access} {text}");
            }
        }
        for line in word_wrap(&format!("  {text}"), width) {
            let line_no = lines.len();
            lines.push(highlight_code_line_with_links(
                &line,
                lang,
                all,
                current_idx,
                line_no,
                links,
            ));
        }
    }
}

fn member_display_text(member: &DocItem) -> String {
    let text = if member.signature.is_empty() {
        simple_name(&member.name).to_string()
    } else {
        member.display_signature()
    };
    if matches!(member.kind, DocKind::Macro | DocKind::Unknown) {
        format!("{} {text}", tui_kind_label(&member.kind))
    } else {
        text
    }
}

fn type_members<'a>(all: &'a [DocItem], type_item: &DocItem) -> Vec<&'a DocItem> {
    let type_name = type_item.name.as_str();
    let simple = simple_name(type_name);
    let prefix = format!("{type_name}::");
    let mut members: Vec<_> = all
        .iter()
        .filter(|item| {
            item.name.starts_with(&prefix)
                || item.meta.parent.as_deref() == Some(simple)
                || item.meta.parent.as_deref() == Some(type_name)
        })
        .collect();
    members.sort_by(|a, b| {
        kind_order(&a.kind)
            .cmp(&kind_order(&b.kind))
            .then(a.name.cmp(&b.name))
    });
    members
}

fn access_label(item: &DocItem) -> &'static str {
    match item.meta.access {
        Some(crate::extract::Access::Public) => "public",
        Some(crate::extract::Access::Protected) => "protected",
        Some(crate::extract::Access::Private) => "private",
        None => "",
    }
}

fn inheritance_list(item: &DocItem) -> Vec<String> {
    item.meta
        .attrs
        .iter()
        .filter_map(|attr| attr.strip_prefix("base:").map(str::to_string))
        .chain(parse_inheritance_from_signature(&item.signature))
        .collect()
}

fn parse_inheritance_from_signature(signature: &str) -> Vec<String> {
    let Some((_, rest)) = signature.split_once(':') else {
        return Vec::new();
    };
    rest.trim()
        .trim_end_matches('{')
        .split(',')
        .filter_map(|base| {
            let name = base
                .split_whitespace()
                .filter(|part| !matches!(*part, "public" | "protected" | "private" | "virtual"))
                .next_back()
                .unwrap_or("")
                .trim();
            (!name.is_empty()).then(|| name.to_string())
        })
        .collect()
}

fn push_overload_dropdown(
    lines: &mut Vec<Line<'static>>,
    all: &[DocItem],
    current_idx: usize,
    open: bool,
    links: &mut Vec<DetailLink>,
) {
    let overloads = overload_indices(all, current_idx);
    if overloads.len() <= 1 {
        return;
    }

    let current_pos = overloads
        .iter()
        .position(|&idx| idx == current_idx)
        .map(|idx| idx + 1)
        .unwrap_or(1);
    let marker = if open { "▾" } else { "▸" };
    lines.push(Line::from(vec![
        Span::styled(
            format!("{marker} Overloads {current_pos}/{} ", overloads.len()),
            Style::default()
                .fg(COLOR_SECTION)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("[o]", Style::default().fg(COLOR_HINT)),
    ]));

    if open {
        for (pos, idx) in overloads.iter().copied().enumerate() {
            let line_no = lines.len();
            let item = &all[idx];
            let prefix = if idx == current_idx { "  * " } else { "    " };
            let sig = item.display_signature();
            let text = if sig.is_empty() {
                item.name.clone()
            } else {
                sig
            };
            let ordinal = format!("{}.", pos + 1);
            let start_col = prefix.chars().count() + ordinal.chars().count() + 1;
            links.push(DetailLink {
                line: line_no,
                start_col,
                end_col: start_col + text.chars().count(),
                target_idx: idx,
            });
            let mut spans = vec![
                Span::styled(prefix, Style::default().fg(COLOR_HINT)),
                Span::styled(ordinal, Style::default().fg(COLOR_HINT)),
                Span::raw(" "),
            ];
            let code = highlight_code_line_with_links(
                &text,
                &item.lang,
                &[],
                None,
                line_no,
                &mut Vec::new(),
            );
            spans.extend(code.spans.into_iter().map(|span| {
                if idx == current_idx {
                    span.patch_style(Style::default().add_modifier(Modifier::BOLD))
                } else {
                    span.patch_style(Style::default().add_modifier(Modifier::UNDERLINED))
                }
            }));
            lines.push(Line::from(spans));
        }
    }
    lines.push(Line::raw(""));
}

fn overload_indices(all: &[DocItem], current_idx: usize) -> Vec<usize> {
    let Some(current) = all.get(current_idx) else {
        return Vec::new();
    };
    if !matches!(current.kind, DocKind::Function | DocKind::Subroutine) {
        return Vec::new();
    }
    let key = overload_key(current);
    let mut indices: Vec<usize> = all
        .iter()
        .enumerate()
        .filter(|(_, item)| {
            matches!(item.kind, DocKind::Function | DocKind::Subroutine)
                && overload_key(item) == key
        })
        .map(|(idx, _)| idx)
        .collect();
    indices.sort_by(|&a, &b| all[a].signature.cmp(&all[b].signature).then(a.cmp(&b)));
    indices
}

fn overload_key(item: &DocItem) -> (DocLanguage, String, String) {
    (
        item.lang.clone(),
        item.name
            .rsplit_once("::")
            .map(|(scope, _)| scope.to_string())
            .or_else(|| {
                item.name
                    .rsplit_once('.')
                    .map(|(scope, _)| scope.to_string())
            })
            .unwrap_or_default(),
        simple_name(&item.name).to_string(),
    )
}

fn push_param_return_table(lines: &mut Vec<Line<'static>>, item: &DocItem, width: usize) {
    let params: Vec<_> = item
        .tags
        .iter()
        .filter(|tag| tag.kind == TagKind::Param)
        .collect();
    let returns = item
        .tags
        .iter()
        .find(|tag| tag.kind == TagKind::Return && !tag.text.is_empty());
    if params.is_empty() && returns.is_none() {
        return;
    }

    lines.push(section_header("Parameters / Returns"));
    let name_width = params
        .iter()
        .filter_map(|tag| tag.name.as_deref())
        .map(str::len)
        .chain(std::iter::once("Returns".len()))
        .max()
        .unwrap_or("Returns".len())
        .clamp(10, 20);
    let available_desc = width.saturating_sub(name_width + 8).max(16);
    let content_desc = params
        .iter()
        .map(|param| doc_text(&param.text).chars().count())
        .chain(
            returns
                .iter()
                .map(|ret| doc_text(&ret.text).chars().count()),
        )
        .max()
        .unwrap_or(16);
    let desc_width = content_desc.clamp(16, available_desc.min(56));

    lines.push(table_rule(name_width, desc_width, "┌", "┬", "┐"));
    lines.push(table_row(
        "Name",
        "Description",
        name_width,
        desc_width,
        true,
        true,
    ));
    lines.push(table_rule(name_width, desc_width, "├", "┼", "┤"));

    for param in &params {
        let name = param.name.as_deref().unwrap_or("?");
        push_wrapped_table_row(lines, name, &doc_text(&param.text), name_width, desc_width);
    }

    if let Some(ret) = returns {
        if !params.is_empty() {
            lines.push(table_rule(name_width, desc_width, "├", "┼", "┤"));
        }
        push_wrapped_table_row(
            lines,
            "Returns",
            &doc_text(&ret.text),
            name_width,
            desc_width,
        );
    }

    lines.push(table_rule(name_width, desc_width, "└", "┴", "┘"));
    lines.push(Line::raw(""));
}

fn push_wrapped_table_row(
    lines: &mut Vec<Line<'static>>,
    name: &str,
    text: &str,
    name_width: usize,
    desc_width: usize,
) {
    let wrapped = word_wrap(text, desc_width);
    if wrapped.is_empty() {
        lines.push(table_row(name, "", name_width, desc_width, false, true));
        return;
    }
    for (idx, desc) in wrapped.iter().enumerate() {
        let label = if idx == 0 { name } else { "" };
        lines.push(table_row(
            label,
            desc,
            name_width,
            desc_width,
            false,
            idx == 0,
        ));
    }
}

fn table_row(
    name: &str,
    text: &str,
    name_width: usize,
    desc_width: usize,
    header: bool,
    show_label: bool,
) -> Line<'static> {
    let name_style = if header {
        Style::default()
            .fg(COLOR_SECTION)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Cyan)
    };
    let text_style = if header {
        Style::default()
            .fg(COLOR_SECTION)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(COLOR_CONTENT)
    };
    let label_style = if show_label {
        name_style
    } else {
        Style::default().fg(Color::DarkGray)
    };

    Line::from(vec![
        Span::raw("  "),
        Span::styled("│ ", Style::default().fg(Color::DarkGray)),
        Span::styled(format!("{name:<name_width$}"), label_style),
        Span::styled(" │ ", Style::default().fg(Color::DarkGray)),
        Span::styled(format!("{text:<desc_width$}"), text_style),
        Span::styled(" │", Style::default().fg(Color::DarkGray)),
    ])
}

fn table_rule(
    name_width: usize,
    desc_width: usize,
    left: &'static str,
    middle: &'static str,
    right: &'static str,
) -> Line<'static> {
    Line::styled(
        format!(
            "  {left}{}{}{}{right}",
            "─".repeat(name_width + 2),
            middle,
            "─".repeat(desc_width + 2),
        ),
        Style::default().fg(Color::DarkGray),
    )
}

fn highlight_code_line(line: &str, lang: &DocLanguage) -> Line<'static> {
    highlight_code_line_with_links(line, lang, &[], None, 0, &mut Vec::new())
}

fn highlight_source_line(
    idx: usize,
    line: &str,
    lang: &DocLanguage,
    declaration_line: Option<usize>,
) -> Line<'static> {
    let number = Span::styled(
        format!("{:>5} ", idx + 1),
        Style::default().fg(Color::DarkGray),
    );
    let marker = if Some(idx) == declaration_line {
        Span::styled("▶ ", Style::default().fg(Color::Yellow))
    } else {
        Span::raw("  ")
    };
    let mut spans = vec![number, marker];
    spans.extend(highlight_code_line(line, lang).spans);
    Line::from(spans)
}

fn highlight_code_line_with_links(
    line: &str,
    lang: &DocLanguage,
    all: &[DocItem],
    current_idx: Option<usize>,
    line_no: usize,
    links: &mut Vec<DetailLink>,
) -> Line<'static> {
    let trimmed = line.trim_start();
    if trimmed.starts_with("//")
        || trimmed.starts_with("///")
        || trimmed.starts_with("/*")
        || trimmed.starts_with('*')
        || trimmed.starts_with("--")
        || trimmed.starts_with('!')
        || trimmed.starts_with('#')
    {
        return Line::styled(line.to_string(), Style::default().fg(Color::DarkGray));
    }

    let mut spans = Vec::new();
    let mut cur = String::new();
    let mut col = 0usize;
    let mut chars = line.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '"' || ch == '\'' {
            push_code_token_with_links(
                &mut spans,
                &cur,
                lang,
                all,
                current_idx,
                line_no,
                &mut col,
                links,
            );
            cur.clear();
            let quote = ch;
            let mut lit = String::from(ch);
            let mut escaped = false;
            for next in chars.by_ref() {
                lit.push(next);
                if escaped {
                    escaped = false;
                    continue;
                }
                if next == '\\' {
                    escaped = true;
                } else if next == quote {
                    break;
                }
            }
            col += lit.chars().count();
            spans.push(Span::styled(
                lit,
                Style::default().fg(Color::Rgb(210, 170, 110)),
            ));
        } else if ch.is_alphanumeric() || ch == '_' {
            cur.push(ch);
        } else {
            push_code_token_with_links(
                &mut spans,
                &cur,
                lang,
                all,
                current_idx,
                line_no,
                &mut col,
                links,
            );
            cur.clear();
            col += 1;
            spans.push(Span::styled(ch.to_string(), punctuation_style(ch)));
        }
    }
    push_code_token_with_links(
        &mut spans,
        &cur,
        lang,
        all,
        current_idx,
        line_no,
        &mut col,
        links,
    );

    Line::from(spans)
}

fn push_code_token_with_links(
    spans: &mut Vec<Span<'static>>,
    token: &str,
    lang: &DocLanguage,
    all: &[DocItem],
    current_idx: Option<usize>,
    line_no: usize,
    col: &mut usize,
    links: &mut Vec<DetailLink>,
) {
    if token.is_empty() {
        return;
    }
    let target = if is_keyword(token, lang) || is_builtin_type(token, lang) {
        None
    } else {
        find_doc_target(all, token, current_idx, lang)
    };
    let style = if let Some(target_idx) = target {
        linked_symbol_style(&all[target_idx].kind)
    } else if is_keyword(token, lang) {
        Style::default()
            .fg(Color::Rgb(120, 170, 255))
            .add_modifier(Modifier::BOLD)
    } else if token.chars().all(|ch| ch.is_ascii_digit()) {
        Style::default().fg(Color::Rgb(210, 170, 255))
    } else if is_builtin_type(token, lang) {
        Style::default().fg(Color::Rgb(130, 220, 180))
    } else {
        Style::default().fg(Color::Gray)
    };
    let start_col = *col;
    *col += token.chars().count();
    if let Some(target_idx) = target {
        links.push(DetailLink {
            line: line_no,
            start_col,
            end_col: *col,
            target_idx,
        });
    }
    spans.push(Span::styled(token.to_string(), style));
}

fn push_reference_spans(
    spans: &mut Vec<Span<'static>>,
    text: &str,
    start_col: usize,
    all: &[DocItem],
    current_idx: Option<usize>,
    lang: &DocLanguage,
    line_no: usize,
    links: &mut Vec<DetailLink>,
) {
    let mut token = String::new();
    let mut token_start = start_col;
    let mut col = start_col;

    for ch in text.chars() {
        if ch.is_alphanumeric() || ch == '_' || ch == ':' || ch == '.' {
            if token.is_empty() {
                token_start = col;
            }
            token.push(ch);
        } else {
            push_reference_token(
                spans,
                &token,
                token_start,
                all,
                current_idx,
                lang,
                line_no,
                links,
            );
            token.clear();
            spans.push(Span::styled(
                ch.to_string(),
                Style::default().fg(Color::Cyan),
            ));
        }
        col += 1;
    }
    push_reference_token(
        spans,
        &token,
        token_start,
        all,
        current_idx,
        lang,
        line_no,
        links,
    );
}

fn linkable_text_line(
    text: &str,
    all: &[DocItem],
    current_idx: Option<usize>,
    lang: &DocLanguage,
    line_no: usize,
    links: &mut Vec<DetailLink>,
    base_style: Style,
) -> Line<'static> {
    let mut spans = Vec::new();
    let mut token = String::new();
    let mut token_start = 0usize;
    let mut col = 0usize;

    for ch in text.chars() {
        if ch.is_alphanumeric() || ch == '_' || ch == ':' || ch == '.' {
            if token.is_empty() {
                token_start = col;
            }
            token.push(ch);
        } else {
            push_linkable_text_token(
                &mut spans,
                &token,
                token_start,
                all,
                current_idx,
                lang,
                line_no,
                links,
                base_style,
            );
            token.clear();
            spans.push(Span::styled(ch.to_string(), base_style));
        }
        col += 1;
    }
    push_linkable_text_token(
        &mut spans,
        &token,
        token_start,
        all,
        current_idx,
        lang,
        line_no,
        links,
        base_style,
    );

    Line::from(spans)
}

fn push_linkable_text_token(
    spans: &mut Vec<Span<'static>>,
    token: &str,
    start_col: usize,
    all: &[DocItem],
    current_idx: Option<usize>,
    lang: &DocLanguage,
    line_no: usize,
    links: &mut Vec<DetailLink>,
    base_style: Style,
) {
    if token.is_empty() {
        return;
    }
    let target = find_doc_target(all, clean_reference_name(token), current_idx, lang);
    let mut style = base_style;
    if let Some(target_idx) = target {
        style = linked_symbol_style(&all[target_idx].kind);
    }
    if let Some(target_idx) = target {
        links.push(DetailLink {
            line: line_no,
            start_col,
            end_col: start_col + token.chars().count(),
            target_idx,
        });
    }
    spans.push(Span::styled(token.to_string(), style));
}

fn push_reference_token(
    spans: &mut Vec<Span<'static>>,
    token: &str,
    start_col: usize,
    all: &[DocItem],
    current_idx: Option<usize>,
    lang: &DocLanguage,
    line_no: usize,
    links: &mut Vec<DetailLink>,
) {
    if token.is_empty() {
        return;
    }
    let trimmed = token.trim_matches(|ch: char| !ch.is_alphanumeric() && ch != '_' && ch != ':');
    let target = find_doc_target(all, trimmed, current_idx, lang);
    let style = target
        .map(|target_idx| linked_symbol_style(&all[target_idx].kind))
        .unwrap_or_else(|| Style::default().fg(Color::Cyan));
    if let Some(target_idx) = target {
        links.push(DetailLink {
            line: line_no,
            start_col,
            end_col: start_col + token.chars().count(),
            target_idx,
        });
    }
    spans.push(Span::styled(token.to_string(), style));
}

fn find_doc_target(
    all: &[DocItem],
    name: &str,
    current_idx: Option<usize>,
    lang: &DocLanguage,
) -> Option<usize> {
    let needle = clean_reference_name(name);
    if needle.is_empty() {
        return None;
    }
    all.iter()
        .enumerate()
        .filter(|(idx, item)| {
            Some(*idx) != current_idx
                && &item.lang == lang
                && !is_class_or_constructor(item)
        })
        .find(|(_, item)| item_matches_ref(item, needle))
        .map(|(idx, _)| idx)
}

fn item_matches_ref(item: &DocItem, needle: &str) -> bool {
    item.name == needle || simple_name(&item.name) == needle
}

/// Returns true for items that should never be used as inline reference link
/// targets: class/struct/interface type definitions, and C++ constructors
/// (identified by their simple name matching their parent scope name, e.g.
/// `stats::OrderStatistics::OrderStatistics`).
fn is_class_or_constructor(item: &DocItem) -> bool {
    // Type definitions are structural — don't turn mentions of a class name
    // into jump links in prose.
    if matches!(
        item.kind,
        DocKind::Class | DocKind::Struct | DocKind::Interface
    ) {
        return true;
    }
    // C/C++ constructors: Function whose unqualified name equals its parent
    // scope's unqualified name (e.g. `Foo::Foo` or `ns::Foo::Foo`).
    if matches!(item.lang, DocLanguage::C | DocLanguage::Cpp)
        && item.kind == DocKind::Function
    {
        if let Some((parent, ctor)) = item.name.rsplit_once("::") {
            let parent_simple = simple_name(parent);
            if parent_simple == ctor {
                return true;
            }
        }
    }
    false
}

fn clean_reference_name(name: &str) -> &str {
    name.trim_matches(|ch: char| !ch.is_alphanumeric() && ch != '_' && ch != ':' && ch != '.')
}

fn declaration_line_idx(item: &DocItem) -> usize {
    let Ok(text) = std::fs::read_to_string(&item.file) else {
        return item.line.saturating_sub(1);
    };
    let lines: Vec<&str> = text.lines().collect();
    let start = item
        .line
        .saturating_sub(1)
        .min(lines.len().saturating_sub(1));
    let Some(line) = lines.get(start).map(|line| line.trim_start()) else {
        return start;
    };

    if line.starts_with("///") || line.starts_with("//!") {
        let mut idx = start;
        while idx < lines.len()
            && lines[idx]
                .trim_start()
                .starts_with(|ch: char| ch == '/' || ch == '!')
        {
            idx += 1;
        }
        return next_nonblank_line(&lines, idx).unwrap_or(start);
    }

    if line.starts_with("/**") || line.starts_with("/*!") || line.starts_with("/*") {
        let mut idx = start;
        while idx < lines.len() {
            if lines[idx].contains("*/") {
                idx += 1;
                break;
            }
            idx += 1;
        }
        return next_nonblank_line(&lines, idx).unwrap_or(start);
    }

    start
}

fn next_nonblank_line(lines: &[&str], mut idx: usize) -> Option<usize> {
    while idx < lines.len() {
        if !lines[idx].trim().is_empty() {
            return Some(idx);
        }
        idx += 1;
    }
    None
}

fn punctuation_style(ch: char) -> Style {
    match ch {
        '(' | ')' | '[' | ']' | '{' | '}' => Style::default().fg(Color::Rgb(180, 180, 220)),
        '*' | '&' | '=' | '+' | '-' | '/' | '<' | '>' | '!' | ':' | ';' | ',' | '.' => {
            Style::default().fg(Color::Rgb(150, 150, 170))
        }
        _ => Style::default().fg(Color::Gray),
    }
}

fn is_keyword(token: &str, lang: &DocLanguage) -> bool {
    let common_c = matches!(
        token,
        "const"
            | "static"
            | "extern"
            | "inline"
            | "struct"
            | "class"
            | "enum"
            | "typedef"
            | "namespace"
            | "template"
            | "typename"
            | "public"
            | "private"
            | "protected"
            | "virtual"
            | "override"
            | "return"
            | "__global__"
            | "__device__"
            | "__host__"
    );
    match lang {
        DocLanguage::C | DocLanguage::Cpp => common_c,
        DocLanguage::Rust => matches!(
            token,
            "pub"
                | "fn"
                | "struct"
                | "enum"
                | "trait"
                | "impl"
                | "mod"
                | "let"
                | "mut"
                | "const"
                | "async"
                | "unsafe"
                | "return"
                | "where"
                | "for"
        ),
        DocLanguage::Go => matches!(
            token,
            "func" | "type" | "struct" | "interface" | "package" | "var" | "const" | "return"
        ),
        DocLanguage::Java | DocLanguage::Kotlin => matches!(
            token,
            "public"
                | "private"
                | "protected"
                | "class"
                | "interface"
                | "enum"
                | "fun"
                | "static"
                | "final"
                | "override"
                | "return"
                | "throws"
        ),
        DocLanguage::Swift => matches!(
            token,
            "public"
                | "private"
                | "func"
                | "struct"
                | "class"
                | "enum"
                | "protocol"
                | "let"
                | "var"
                | "return"
                | "throws"
        ),
        DocLanguage::Ada => matches!(
            token.to_ascii_lowercase().as_str(),
            "package" | "procedure" | "function" | "type" | "is" | "return" | "begin" | "end"
        ),
        DocLanguage::Fortran => matches!(
            token.to_ascii_lowercase().as_str(),
            "module" | "subroutine" | "function" | "integer" | "real" | "intent" | "end"
        ),
        DocLanguage::D | DocLanguage::Zig | DocLanguage::Unknown => false,
    }
}

fn is_builtin_type(token: &str, lang: &DocLanguage) -> bool {
    match lang {
        DocLanguage::C | DocLanguage::Cpp => matches!(
            token,
            "void"
                | "bool"
                | "char"
                | "short"
                | "int"
                | "long"
                | "float"
                | "double"
                | "size_t"
                | "uint8_t"
                | "uint16_t"
                | "uint32_t"
                | "uint64_t"
        ),
        DocLanguage::Rust => matches!(
            token,
            "bool"
                | "char"
                | "str"
                | "String"
                | "usize"
                | "isize"
                | "u8"
                | "u16"
                | "u32"
                | "u64"
                | "i8"
                | "i16"
                | "i32"
                | "i64"
                | "f32"
                | "f64"
        ),
        _ => false,
    }
}

fn push_tree_rows(
    node: &TreeNode,
    depth: usize,
    auto_expand: bool,
    expanded: &HashSet<String>,
    out: &mut Vec<TreeRow>,
) {
    let is_expanded = auto_expand || expanded.contains(&node.key);
    out.push(TreeRow {
        depth,
        label: node.label.clone(),
        count: node.count,
        kind: TreeRowKind::Group {
            key: node.key.clone(),
            expanded: is_expanded,
            item_idx: node.item_idx,
            source_idx: node.source_idx,
        },
    });

    if !is_expanded {
        return;
    }

    for child in node.children.values() {
        push_tree_rows(child, depth + 1, auto_expand, expanded, out);
    }
    for &idx in &node.items {
        out.push(TreeRow {
            depth: depth + 1,
            label: String::new(),
            count: 1,
            kind: TreeRowKind::Item(idx),
        });
    }
}

fn item_sort_key(item: &DocItem) -> (u8, String, u8, String) {
    (
        language_order(&item.lang),
        tree_path(item).join("\u{1f}").to_ascii_lowercase(),
        kind_order(&item.kind),
        item.name.to_ascii_lowercase(),
    )
}

fn language_order(lang: &DocLanguage) -> u8 {
    match lang {
        DocLanguage::C => 0,
        DocLanguage::Cpp => 1,
        DocLanguage::Rust => 2,
        DocLanguage::Fortran => 3,
        DocLanguage::D => 4,
        DocLanguage::Ada => 5,
        DocLanguage::Java => 6,
        DocLanguage::Go => 7,
        DocLanguage::Zig => 8,
        DocLanguage::Kotlin => 9,
        DocLanguage::Swift => 10,
        DocLanguage::Unknown => 255,
    }
}

fn kind_order(kind: &DocKind) -> u8 {
    match kind {
        DocKind::Module => 0,
        DocKind::Class | DocKind::Struct | DocKind::Interface => 1,
        DocKind::Enum | DocKind::Typedef => 2,
        DocKind::Function | DocKind::Subroutine => 3,
        DocKind::Variable => 4,
        DocKind::Macro => 5,
        DocKind::Unknown => 255,
    }
}

fn tree_path(item: &DocItem) -> Vec<String> {
    let mut path = vec![language_group_label(&item.lang).to_string()];
    path.extend(item_group_parts(item));
    path
}

fn is_group_doc_item(item: &DocItem) -> bool {
    item.name.is_empty()
        || matches!(
            item.kind,
            DocKind::Class | DocKind::Struct | DocKind::Interface | DocKind::Module
        )
}

fn language_group_label(lang: &DocLanguage) -> &'static str {
    match lang {
        DocLanguage::C | DocLanguage::Cpp => "C/C++",
        _ => lang.label(),
    }
}

fn item_group_parts(item: &DocItem) -> Vec<String> {
    match item.lang {
        DocLanguage::C | DocLanguage::Cpp => {
            let mut parts = vec![item
                .file
                .file_name()
                .and_then(|name| name.to_str())
                .map(str::to_owned)
                .unwrap_or_else(|| item.file.display().to_string())];
            if matches!(
                item.kind,
                DocKind::Class | DocKind::Struct | DocKind::Interface | DocKind::Module
            ) && !item.name.is_empty()
            {
                parts.extend(split_scope(&item.name));
            } else {
                parts.extend(split_scope_prefix(&item.name, "::"));
            }
            parts
        }
        DocLanguage::Rust => {
            if matches!(item.kind, DocKind::Module) && !item.name.is_empty() {
                return split_scope(&item.name);
            }
            let parts = split_scope_prefix(&item.name, "::");
            if parts.is_empty() {
                vec!["(root)".to_string()]
            } else {
                parts
            }
        }
        DocLanguage::Ada | DocLanguage::D | DocLanguage::Go | DocLanguage::Fortran => {
            let parts = split_scope_prefix(&item.name, ".");
            if parts.is_empty() {
                vec!["(root)".to_string()]
            } else {
                parts
            }
        }
        DocLanguage::Java | DocLanguage::Kotlin | DocLanguage::Swift | DocLanguage::Zig => {
            let parent = item
                .meta
                .parent
                .as_deref()
                .filter(|parent| !parent.is_empty())
                .map(str::to_owned)
                .or_else(|| {
                    item.name
                        .rsplit_once('.')
                        .map(|(module, _)| module.to_string())
                })
                .or_else(|| {
                    item.name
                        .rsplit_once("::")
                        .map(|(module, _)| module.to_string())
                });
            match parent {
                Some(parent) => split_scope(&parent),
                None => vec!["(root)".to_string()],
            }
        }
        DocLanguage::Unknown => vec!["(unknown)".to_string()],
    }
}

fn split_scope_prefix(name: &str, sep: &str) -> Vec<String> {
    name.rsplit_once(sep)
        .map(|(prefix, _)| split_scope(prefix))
        .unwrap_or_default()
}

fn split_scope(scope: &str) -> Vec<String> {
    if scope.contains("::") {
        scope
            .split("::")
            .filter(|part| !part.is_empty())
            .map(str::to_owned)
            .collect()
    } else {
        scope
            .split('.')
            .filter(|part| !part.is_empty())
            .map(str::to_owned)
            .collect()
    }
}

fn simple_name(name: &str) -> &str {
    name.rsplit_once("::")
        .map(|(_, simple)| simple)
        .or_else(|| name.rsplit_once('.').map(|(_, simple)| simple))
        .unwrap_or(name)
}

/// Colour coding by symbol kind.
fn kind_color(kind: &DocKind) -> Color {
    match kind {
        DocKind::Function | DocKind::Subroutine => Color::Rgb(100, 180, 255),
        DocKind::Struct | DocKind::Class => Color::Rgb(180, 140, 255),
        DocKind::Enum => Color::Rgb(255, 180, 80),
        DocKind::Typedef => Color::Rgb(130, 220, 180),
        DocKind::Variable => Color::Rgb(200, 200, 100),
        DocKind::Macro => Color::Rgb(255, 120, 120),
        DocKind::Module | DocKind::Interface => Color::Rgb(120, 220, 200),
        DocKind::Unknown => Color::DarkGray,
    }
}

fn panel_border_style(focused: bool) -> Style {
    if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(COLOR_BORDER)
    }
}

fn contains(area: Rect, x: u16, y: u16) -> bool {
    x >= area.x
        && x < area.x.saturating_add(area.width)
        && y >= area.y
        && y < area.y.saturating_add(area.height)
}

fn clamped_scroll(scroll: u16, content_len: usize, viewport_height: u16) -> u16 {
    let max_scroll = content_len.saturating_sub(viewport_height as usize);
    scroll.min(max_scroll.min(u16::MAX as usize) as u16)
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
