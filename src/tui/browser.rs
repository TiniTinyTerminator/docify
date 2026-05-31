use std::collections::{BTreeMap, HashSet};
use std::path::Path;

use crossterm::event::{self, Event, KeyCode, KeyModifiers, MouseButton, MouseEventKind};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph, Wrap},
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
            .entry(label.clone())
            .or_insert_with(|| TreeNode {
                key,
                label: label.clone(),
                ..TreeNode::default()
            })
            .insert(rest, item_idx, group_item, source_item);

        if rest.is_empty() && group_item {
            if let Some(node) = self.children.get_mut(label) {
                node.item_idx = Some(item_idx);
                node.items.retain(|&idx| idx != item_idx);
            }
        }
        if source_item {
            if let Some(node) = self.children.get_mut(label) {
                node.source_idx.get_or_insert(item_idx);
            }
        }
    }

    fn attach_source(&mut self, path: &[String], item_idx: usize) -> bool {
        let Some((label, rest)) = path.split_first() else {
            return false;
        };
        let Some(child) = self.children.get_mut(label) else {
            return false;
        };
        if rest.is_empty() {
            child.source_idx = Some(item_idx);
            true
        } else {
            child.attach_source(rest, item_idx)
        }
    }
}

struct App {
    all: Vec<DocItem>,
    visible: Vec<usize>,
    external_selected_idx: Option<usize>,

    query: String,
    cursor: usize,

    matching: Vec<usize>,
    rows: Vec<TreeRow>,
    expanded: HashSet<String>,
    list_state: ListState,
    list_scroll: usize,

    scroll: u16,
    show_source: bool,
    source_cache: Option<(String, String)>,
    source_variant: usize,
    source_variants_cache: Option<(usize, Vec<DocItem>)>,
    overloads_open: bool,
    focus: FocusPane,

    list_area: Rect,
    detail_area: Rect,
    detail_inner_area: Rect,
    detail_links: Vec<DetailLink>,
}

impl App {
    fn new_with_hidden(mut visible_items: Vec<DocItem>, hidden_items: Vec<DocItem>) -> Self {
        let visible_len = visible_items.len();
        visible_items.extend(hidden_items);
        Self::new_with_visible(visible_items, (0..visible_len).collect())
    }

    fn new_with_visible(items: Vec<DocItem>, visible: Vec<usize>) -> Self {
        let mut app = Self {
            all: items,
            visible,
            external_selected_idx: None,
            query: String::new(),
            cursor: 0,
            matching: Vec::new(),
            rows: Vec::new(),
            expanded: HashSet::new(),
            list_state: ListState::default(),
            list_scroll: 0,
            scroll: 0,
            show_source: false,
            source_cache: None,
            source_variant: 0,
            source_variants_cache: None,
            overloads_open: false,
            focus: FocusPane::Tree,
            list_area: Rect::default(),
            detail_area: Rect::default(),
            detail_inner_area: Rect::default(),
            detail_links: Vec::new(),
        };
        app.matching = app.visible.clone();
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
            self.matching = self.visible.clone();
        } else {
            self.matching = self
                .visible
                .iter()
                .copied()
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
        self.reset_source_state();
        self.overloads_open = false;
        self.external_selected_idx = None;
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
            if !is_index_item(&self.all[idx]) {
                continue;
            }
            root.insert(
                &tree_path(&self.all[idx]),
                idx,
                is_group_doc_item(&self.all[idx]),
                true,
            );
        }
        for &idx in &self.matching {
            if is_index_item(&self.all[idx]) {
                continue;
            }
            root.attach_source(&tree_path(&self.all[idx]), idx);
        }

        let mut rows = Vec::new();
        let auto_expand = !self.query.is_empty();
        for node in root.children.values() {
            push_tree_rows(node, 0, auto_expand, &self.expanded, &mut rows);
        }
        self.rows = rows;
    }

    fn selected_item(&self) -> Option<&DocItem> {
        if let Some(idx) = self.external_selected_idx {
            return self.all.get(idx);
        }
        let sel = self.list_state.selected()?;
        match self.rows.get(sel)?.kind {
            TreeRowKind::Item(idx) => self.all.get(idx),
            TreeRowKind::Group { item_idx, .. } => item_idx.and_then(|idx| self.all.get(idx)),
        }
    }

    fn selected_idx(&self) -> Option<usize> {
        if let Some(idx) = self.external_selected_idx {
            return Some(idx);
        }
        let sel = self.list_state.selected()?;
        match self.rows.get(sel)?.kind {
            TreeRowKind::Item(idx) => Some(idx),
            TreeRowKind::Group { item_idx, .. } => item_idx,
        }
    }

    fn selected_source_idx(&self) -> Option<usize> {
        if let Some(idx) = self.external_selected_idx {
            return Some(idx);
        }
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

    fn selected_source_item(&mut self) -> Option<DocItem> {
        let idx = self.selected_source_idx()?;
        let variants = self.source_variants(idx);
        variants
            .get(self.source_variant.min(variants.len().saturating_sub(1)))
            .cloned()
    }

    fn source_variant_status(&mut self) -> Option<(&'static str, usize, usize)> {
        let idx = self.selected_source_idx()?;
        let variants = self.source_variants(idx);
        let len = variants.len();
        if len == 0 {
            None
        } else {
            let pos = self.source_variant.min(len - 1);
            Some((source_variant_label(&variants[pos]), pos + 1, len))
        }
    }

    fn select(&mut self, pos: usize) {
        if self.rows.is_empty() {
            return;
        }
        let pos = pos.min(self.rows.len() - 1);
        self.list_state.select(Some(pos));
        self.external_selected_idx = None;
        self.ensure_selection_visible();
        self.reset_source_state();
        self.overloads_open = false;
        self.scroll = if self.show_source {
            self.source_initial_scroll()
        } else {
            0
        };
    }

    fn select_item_idx(&mut self, idx: usize) {
        if !self.visible.contains(&idx) {
            self.external_selected_idx = Some(idx);
            self.show_source = false;
            self.reset_source_state();
            self.overloads_open = false;
            self.scroll = 0;
            return;
        }

        if !self.matching.contains(&idx) {
            self.query.clear();
            self.matching = self.visible.clone();
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

    fn focus_left(&mut self) {
        self.focus = FocusPane::Tree;
    }

    fn focus_right(&mut self) {
        self.focus = FocusPane::Detail;
    }

    fn next_focused_tab(&mut self) {
        self.toggle_source();
    }

    fn previous_focused_tab(&mut self) {
        self.toggle_source();
    }

    fn reset_source_state(&mut self) {
        self.source_cache = None;
        self.source_variant = 0;
        self.source_variants_cache = None;
    }

    fn source_initial_scroll(&mut self) -> u16 {
        let Some(item) = self.selected_source_item() else {
            return 0;
        };
        declaration_line_idx(&item)
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
        self.reset_source_state();
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
        if self.overload_count() > 1 {
            self.previous_overload();
        } else if self.show_source && self.source_variant_count() > 1 {
            self.previous_source_variant();
        } else {
            self.scroll = self.scroll.saturating_sub(5);
        }
    }

    fn page_focused_down(&mut self) {
        if self.overload_count() > 1 {
            self.next_overload();
        } else if self.show_source && self.source_variant_count() > 1 {
            self.next_source_variant();
        } else {
            self.scroll = self.scroll.saturating_add(5);
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
        let Some(item) = self.selected_source_item() else {
            return "";
        };
        let key = source_cache_key(&item);
        if self.source_cache.as_ref().map(|(i, _)| i.as_str()) == Some(key.as_str()) {
            return &self.source_cache.as_ref().unwrap().1;
        }
        let src = extract_source_text(&item);
        self.source_cache = Some((key, src));
        &self.source_cache.as_ref().unwrap().1
    }

    fn source_variant_count(&mut self) -> usize {
        let Some(idx) = self.selected_source_idx() else {
            return 0;
        };
        self.source_variants(idx).len()
    }

    fn overload_count(&self) -> usize {
        self.selected_idx()
            .map(|idx| overload_indices(&self.all, idx).len())
            .unwrap_or(0)
    }

    fn previous_overload(&mut self) -> bool {
        let Some(idx) = self.selected_idx() else {
            return false;
        };
        let overloads = overload_indices(&self.all, idx);
        if overloads.len() <= 1 {
            return false;
        }
        let pos = overloads
            .iter()
            .position(|&overload| overload == idx)
            .unwrap_or(0);
        let target = overloads
            .get(pos.checked_sub(1).unwrap_or(overloads.len() - 1))
            .copied()
            .unwrap_or(idx);
        let keep_source = self.show_source;
        self.select_item_idx(target);
        if keep_source {
            self.show_source = true;
            self.scroll = self.source_initial_scroll();
        }
        true
    }

    fn next_overload(&mut self) -> bool {
        let Some(idx) = self.selected_idx() else {
            return false;
        };
        let overloads = overload_indices(&self.all, idx);
        if overloads.len() <= 1 {
            return false;
        }
        let pos = overloads
            .iter()
            .position(|&overload| overload == idx)
            .unwrap_or(0);
        let target = overloads
            .get((pos + 1) % overloads.len())
            .copied()
            .unwrap_or(idx);
        let keep_source = self.show_source;
        self.select_item_idx(target);
        if keep_source {
            self.show_source = true;
            self.scroll = self.source_initial_scroll();
        }
        true
    }

    fn previous_source_variant(&mut self) {
        let count = self.source_variant_count();
        if count <= 1 {
            return;
        }
        self.source_variant = if self.source_variant == 0 {
            count - 1
        } else {
            self.source_variant - 1
        };
        self.source_cache = None;
        self.scroll = self.source_initial_scroll();
    }

    fn next_source_variant(&mut self) {
        let count = self.source_variant_count();
        if count <= 1 {
            return;
        }
        self.source_variant = (self.source_variant + 1) % count;
        self.source_cache = None;
        self.scroll = self.source_initial_scroll();
    }

    fn source_variants(&mut self, idx: usize) -> Vec<DocItem> {
        if self
            .source_variants_cache
            .as_ref()
            .map(|(cached_idx, _)| *cached_idx)
            == Some(idx)
        {
            return self.source_variants_cache.as_ref().unwrap().1.clone();
        }
        let variants = self
            .all
            .get(idx)
            .map(source_variants_for_item)
            .unwrap_or_default();
        self.source_variant = self.source_variant.min(variants.len().saturating_sub(1));
        self.source_variants_cache = Some((idx, variants.clone()));
        variants
    }
}

// ── Public entry point ────────────────────────────────────────────────────────

pub fn run_doc_browser(dirs: &[&Path]) -> anyhow::Result<()> {
    let mut all_items = Vec::new();
    let scan_dirs = crate::project::expand_scan_dirs(dirs);
    for dir in &scan_dirs {
        all_items.extend(crate::extract::extract_dir(dir).items);
    }
    run_doc_browser_with_items(all_items)
}

pub fn run_doc_browser_with_items(all_items: Vec<DocItem>) -> anyhow::Result<()> {
    run_doc_browser_with_hidden(all_items, Vec::new())
}

pub fn run_doc_browser_with_hidden(
    visible_items: Vec<DocItem>,
    hidden_items: Vec<DocItem>,
) -> anyhow::Result<()> {
    // Dedup across multiple scan dirs: same symbol from a parent dir and a
    // subdirectory that was also added (e.g. via project manifest path-deps).
    // Key on (name, file, line); keep whichever copy has more tags/content.
    let visible_items = dedup_items(visible_items);
    let hidden_items = dedup_items(hidden_items);

    let mut terminal = enter_tui()?;
    let result = run_loop(&mut terminal, visible_items, hidden_items);
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
    visible_items: Vec<DocItem>,
    hidden_items: Vec<DocItem>,
) -> anyhow::Result<()> {
    let mut app = App::new_with_hidden(visible_items, hidden_items);

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
                    (KeyCode::Left, _) => app.focus_left(),
                    (KeyCode::Right, _) => app.focus_right(),
                    (KeyCode::Tab, _) => app.next_focused_tab(),
                    (KeyCode::BackTab, _) => app.previous_focused_tab(),
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

    let hidden = app.all.len().saturating_sub(app.visible.len());
    let title = if hidden == 0 {
        format!(" {} / {} ", app.matching.len(), app.visible.len())
    } else {
        format!(
            " {} / {}  +{} refs ",
            app.matching.len(),
            app.visible.len(),
            hidden
        )
    };
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
                .border_type(BorderType::Rounded)
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
        let source_file = app
            .selected_source_item()
            .map(|item| source_file_title(&item))
            .unwrap_or_else(|| "source".to_string());
        if let Some((label, pos, total)) = app
            .source_variant_status()
            .filter(|(_, _, total)| *total > 1)
        {
            format!(" Source: {source_file} [Tab: doc] [{label} {pos}/{total}] [PgUp/PgDn] ")
        } else {
            format!(" Source: {source_file} [Tab: doc] ")
        }
    } else {
        " Documentation [Tab: source] ".to_string()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
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

    if show_source {
        let selected_source_item = app.selected_source_item();
        let text = app.source_text().to_owned();
        if text.is_empty() {
            let p =
                Paragraph::new("Source not available.").style(Style::default().fg(COLOR_CONTENT));
            f.render_widget(p, inner);
        } else {
            let lang = selected_language(app);
            let decl_line = selected_source_item.as_ref().map(declaration_line_idx);
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

    push_item_header_lines(&mut lines, &item, width);

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

    let has_docs = !item.brief.is_empty() || !item.body.is_empty() || !item.tags.is_empty();

    // Brief
    if !item.brief.is_empty() {
        push_doc_text_lines(
            &mut lines,
            &doc_text(&item.brief),
            width,
            &app.all,
            selected_idx,
            &item.lang,
            &mut app.detail_links,
            Style::default().fg(COLOR_CONTENT),
        );
        lines.push(Line::raw(""));
    }

    if !has_docs {
        lines.push(Line::styled(
            "No documentation.",
            Style::default().fg(COLOR_HINT),
        ));
        lines.push(Line::raw(""));
    }

    // Signature
    if !item.signature.is_empty() {
        push_signature_lines(
            &mut lines,
            &item,
            width,
            &app.all,
            selected_idx,
            &mut app.detail_links,
        );
        lines.push(Line::raw(""));
    }

    // Body (extended description)
    if !item.body.is_empty() {
        push_doc_text_lines(
            &mut lines,
            &doc_text(&item.body),
            width,
            &app.all,
            selected_idx,
            &item.lang,
            &mut app.detail_links,
            Style::default().fg(COLOR_CONTENT),
        );
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
            " ←/→ focus pane   ↑↓/jk navigate   Enter/click open   Tab pane tab   o overloads   PgUp/PgDn scroll   q quit",
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

fn push_item_header_lines(lines: &mut Vec<Line<'static>>, item: &DocItem, width: usize) {
    let kind_style = Style::default()
        .fg(kind_color(&item.kind))
        .add_modifier(Modifier::BOLD);
    let mut spans = vec![Span::styled(
        format!("[{}] ", tui_kind_label(&item.kind)),
        kind_style,
    )];
    spans.extend(qualified_name_spans(item));
    spans.push(Span::styled(
        format!("  {}", item.lang.label()),
        Style::default().fg(Color::Yellow),
    ));
    push_wrapped_spans(lines, spans, width.max(1));
}

fn qualified_name_spans(item: &DocItem) -> Vec<Span<'static>> {
    let name_style = Style::default()
        .fg(kind_color(&item.kind))
        .add_modifier(Modifier::BOLD);
    let scope_style = Style::default().fg(COLOR_TITLE);
    let sep_style = Style::default().fg(Color::DarkGray);

    let mut spans = Vec::new();
    let parts: Vec<&str> = if item.name.contains("::") {
        item.name.split("::").collect()
    } else if item.name.contains('.') {
        item.name.split('.').collect()
    } else {
        Vec::new()
    };
    if parts.is_empty() {
        spans.push(Span::styled(item.name.clone(), name_style));
        return spans;
    }

    let sep = if item.name.contains("::") { "::" } else { "." };
    for (idx, part) in parts.iter().enumerate() {
        if idx > 0 {
            spans.push(Span::styled(sep.to_string(), sep_style));
        }
        let style = if idx + 1 == parts.len() {
            name_style
        } else {
            scope_style
        };
        spans.push(Span::styled((*part).to_string(), style));
    }
    spans
}

fn push_wrapped_spans(lines: &mut Vec<Line<'static>>, spans: Vec<Span<'static>>, width: usize) {
    let mut current = Vec::new();
    let mut col = 0usize;
    for span in spans {
        let mut text = span.content.as_ref();
        while !text.is_empty() {
            let remaining = width.saturating_sub(col).max(1);
            let take = text
                .char_indices()
                .map(|(idx, _)| idx)
                .nth(remaining)
                .unwrap_or(text.len());
            if col > 0 && text.chars().count() > remaining {
                lines.push(Line::from(current));
                current = Vec::new();
                col = 0;
                continue;
            }
            let (chunk, rest) = text.split_at(take);
            if !chunk.is_empty() {
                current.push(Span::styled(chunk.to_string(), span.style));
                col += chunk.chars().count();
            }
            text = rest;
            if !text.is_empty() {
                lines.push(Line::from(current));
                current = Vec::new();
                col = 0;
            }
        }
    }
    if !current.is_empty() {
        lines.push(Line::from(current));
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

fn push_signature_lines(
    lines: &mut Vec<Line<'static>>,
    item: &DocItem,
    width: usize,
    all: &[DocItem],
    selected_idx: Option<usize>,
    links: &mut Vec<DetailLink>,
) {
    for raw_line in item.display_signature().lines() {
        for wrapped in word_wrap(raw_line, width) {
            let line_no = lines.len();
            lines.push(highlight_code_line_with_links(
                &wrapped,
                &item.lang,
                all,
                selected_idx,
                line_no,
                links,
            ));
        }
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
            .then(normalized_signature(a).cmp(&normalized_signature(b)))
    });
    let mut seen = HashSet::new();
    members.retain(|member| {
        seen.insert(format!(
            "{:?}\0{}\0{}",
            member.kind,
            member.name,
            normalized_signature(member)
        ))
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
    if matches!(current.lang, DocLanguage::C) {
        return vec![current_idx];
    }
    let key = overload_key(current);
    let mut candidates: Vec<usize> = all
        .iter()
        .enumerate()
        .filter(|(_, item)| {
            matches!(item.kind, DocKind::Function | DocKind::Subroutine)
                && overload_key(item) == key
        })
        .map(|(idx, _)| idx)
        .collect();
    candidates.sort_by(|&a, &b| {
        normalized_signature(&all[a])
            .cmp(&normalized_signature(&all[b]))
            .then(a.cmp(&b))
    });

    let mut seen = HashSet::new();
    let mut indices = Vec::new();
    for idx in candidates {
        if seen.insert(normalized_signature(&all[idx])) {
            indices.push(idx);
        }
    }
    indices
}

fn normalized_signature(item: &DocItem) -> String {
    item.display_signature()
        .trim()
        .trim_end_matches(';')
        .trim_end_matches('{')
        .trim()
        .to_string()
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

fn push_doc_text_lines(
    lines: &mut Vec<Line<'static>>,
    text: &str,
    width: usize,
    all: &[DocItem],
    selected_idx: Option<usize>,
    lang: &DocLanguage,
    links: &mut Vec<DetailLink>,
    base_style: Style,
) {
    let raw_lines: Vec<&str> = text.lines().collect();
    let mut i = 0usize;
    while i < raw_lines.len() {
        if let Some((table, consumed)) = parse_markdown_table(&raw_lines, i) {
            push_markdown_table(lines, &table, width);
            i += consumed;
            continue;
        }

        for wrapped in word_wrap(raw_lines[i], width) {
            let line_no = lines.len();
            lines.push(linkable_text_line(
                &wrapped,
                all,
                selected_idx,
                lang,
                line_no,
                links,
                base_style,
            ));
        }
        i += 1;
    }
}

#[derive(Debug, Clone)]
struct MarkdownTable {
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
}

fn parse_markdown_table(lines: &[&str], start: usize) -> Option<(MarkdownTable, usize)> {
    let header = split_markdown_table_row(*lines.get(start)?)?;
    let separator = split_markdown_table_row(*lines.get(start + 1)?)?;
    if header.is_empty()
        || !separator
            .iter()
            .all(|cell| is_markdown_separator_cell(cell))
    {
        return None;
    }

    let mut rows = Vec::new();
    let mut i = start + 2;
    while let Some(line) = lines.get(i) {
        let Some(cells) = split_markdown_table_row(line) else {
            break;
        };
        if cells.is_empty() {
            break;
        }
        rows.push(cells);
        i += 1;
    }

    Some((
        MarkdownTable {
            headers: header,
            rows,
        },
        i - start,
    ))
}

fn split_markdown_table_row(line: &str) -> Option<Vec<String>> {
    let trimmed = line.trim();
    if !trimmed.starts_with('|') || !trimmed.ends_with('|') || trimmed.matches('|').count() < 2 {
        return None;
    }
    Some(
        trimmed
            .trim_matches('|')
            .split('|')
            .map(|cell| clean_markdown_table_cell(cell.trim()))
            .collect(),
    )
}

fn is_markdown_separator_cell(cell: &str) -> bool {
    let trimmed = cell.trim_matches(':').trim();
    !trimmed.is_empty() && trimmed.chars().all(|ch| ch == '-')
}

fn clean_markdown_table_cell(cell: &str) -> String {
    cell.replace('`', "")
        .replace("**", "")
        .replace('*', "")
        .trim()
        .to_string()
}

fn push_markdown_table(lines: &mut Vec<Line<'static>>, table: &MarkdownTable, width: usize) {
    if table.headers.len() != 2 {
        push_markdown_table_fallback(lines, table, width);
        return;
    }

    let name_width = table
        .rows
        .iter()
        .filter_map(|row| row.first())
        .map(|cell| cell.chars().count())
        .chain(table.headers.first().map(|cell| cell.chars().count()))
        .max()
        .unwrap_or(8)
        .clamp(4, 20);
    let available_desc = width.saturating_sub(name_width + 8).max(16);
    let content_desc = table
        .rows
        .iter()
        .filter_map(|row| row.get(1))
        .map(|cell| cell.chars().count())
        .chain(table.headers.get(1).map(|cell| cell.chars().count()))
        .max()
        .unwrap_or(16);
    let desc_width = content_desc.clamp(16, available_desc.min(56));

    lines.push(table_rule(name_width, desc_width, "┌", "┬", "┐"));
    lines.push(table_row(
        table.headers.first().map(String::as_str).unwrap_or(""),
        table.headers.get(1).map(String::as_str).unwrap_or(""),
        name_width,
        desc_width,
        true,
        true,
    ));
    lines.push(table_rule(name_width, desc_width, "├", "┼", "┤"));
    for row in &table.rows {
        push_wrapped_table_row(
            lines,
            row.first().map(String::as_str).unwrap_or(""),
            row.get(1).map(String::as_str).unwrap_or(""),
            name_width,
            desc_width,
        );
    }
    lines.push(table_rule(name_width, desc_width, "└", "┴", "┘"));
}

fn push_markdown_table_fallback(
    lines: &mut Vec<Line<'static>>,
    table: &MarkdownTable,
    width: usize,
) {
    let header = table.headers.join("  ");
    for wrapped in word_wrap(&header, width) {
        lines.push(Line::styled(
            wrapped,
            Style::default()
                .fg(COLOR_SECTION)
                .add_modifier(Modifier::BOLD),
        ));
    }
    for row in &table.rows {
        for wrapped in word_wrap(&row.join("  "), width) {
            lines.push(Line::styled(wrapped, Style::default().fg(COLOR_CONTENT)));
        }
    }
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
    let mut col = 0usize;
    push_markdown_linkable_spans(
        &mut spans,
        text,
        all,
        current_idx,
        lang,
        line_no,
        links,
        base_style,
        &mut col,
    );
    Line::from(spans)
}

fn push_markdown_linkable_spans(
    spans: &mut Vec<Span<'static>>,
    mut text: &str,
    all: &[DocItem],
    current_idx: Option<usize>,
    lang: &DocLanguage,
    line_no: usize,
    links: &mut Vec<DetailLink>,
    base_style: Style,
    col: &mut usize,
) {
    while !text.is_empty() {
        if let Some(after) = text.strip_prefix('`') {
            if let Some(end) = after.find('`') {
                let code = &after[..end];
                spans.push(Span::styled(
                    code.to_string(),
                    Style::default().fg(Color::Rgb(210, 170, 110)),
                ));
                *col += code.chars().count();
                text = &after[end + 1..];
                continue;
            }
        }

        if let Some(after) = text.strip_prefix("**") {
            if let Some(end) = after.find("**") {
                push_plain_linkable_segment(
                    spans,
                    &after[..end],
                    all,
                    current_idx,
                    lang,
                    line_no,
                    links,
                    base_style.add_modifier(Modifier::BOLD),
                    col,
                );
                text = &after[end + 2..];
                continue;
            }
        }

        if text.starts_with('*') && !text.starts_with("**") {
            let after = &text[1..];
            if let Some(end) = after.find('*') {
                push_plain_linkable_segment(
                    spans,
                    &after[..end],
                    all,
                    current_idx,
                    lang,
                    line_no,
                    links,
                    base_style.add_modifier(Modifier::ITALIC),
                    col,
                );
                text = &after[end + 1..];
                continue;
            }
        }

        let next_marker = ["`", "**", "*"]
            .iter()
            .filter_map(|marker| text.find(marker))
            .filter(|&idx| idx > 0)
            .min()
            .unwrap_or(text.len());
        let (plain, rest) = text.split_at(next_marker);
        if plain.is_empty() {
            let Some(ch) = text.chars().next() else {
                break;
            };
            spans.push(Span::styled(ch.to_string(), base_style));
            *col += 1;
            text = &text[ch.len_utf8()..];
        } else {
            push_plain_linkable_segment(
                spans,
                plain,
                all,
                current_idx,
                lang,
                line_no,
                links,
                base_style,
                col,
            );
            text = rest;
        }
    }
}

fn push_plain_linkable_segment(
    spans: &mut Vec<Span<'static>>,
    text: &str,
    all: &[DocItem],
    current_idx: Option<usize>,
    lang: &DocLanguage,
    line_no: usize,
    links: &mut Vec<DetailLink>,
    base_style: Style,
    col: &mut usize,
) {
    let mut token = String::new();
    let mut token_start = *col;

    for ch in text.chars() {
        if ch.is_alphanumeric() || ch == '_' || ch == ':' || ch == '.' {
            if token.is_empty() {
                token_start = *col;
            }
            token.push(ch);
        } else {
            push_linkable_text_token(
                spans,
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
        *col += 1;
    }
    push_linkable_text_token(
        spans,
        &token,
        token_start,
        all,
        current_idx,
        lang,
        line_no,
        links,
        base_style,
    );
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
    let clean = clean_reference_name(token);
    let target = if is_prose_reference_candidate(clean) {
        find_doc_target(all, clean, current_idx, lang)
    } else {
        None
    };
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
    let target = if is_prose_reference_candidate(trimmed) {
        find_doc_target(all, trimmed, current_idx, lang)
    } else {
        None
    };
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
        .filter(|(_, item)| {
            &item.lang == lang
                && !is_constructor(item)
                && !is_current_symbol_reference(all, current_idx, item, needle)
        })
        .find(|(_, item)| item_matches_ref(item, needle))
        .map(|(idx, _)| idx)
}

fn item_matches_ref(item: &DocItem, needle: &str) -> bool {
    item.name == needle || simple_name(&item.name) == needle
}

fn is_current_symbol_reference(
    all: &[DocItem],
    current_idx: Option<usize>,
    candidate: &DocItem,
    needle: &str,
) -> bool {
    let Some(current) = current_idx.and_then(|idx| all.get(idx)) else {
        return false;
    };
    if !item_matches_ref(candidate, needle) {
        return false;
    }
    if matches!(current.kind, DocKind::Function | DocKind::Subroutine)
        && matches!(candidate.kind, DocKind::Function | DocKind::Subroutine)
    {
        return overload_key(current) == overload_key(candidate);
    }
    current.kind == candidate.kind
        && scoped_name_key(&current.name) == scoped_name_key(&candidate.name)
}

fn scoped_name_key(name: &str) -> (String, String) {
    let scope = name
        .rsplit_once("::")
        .map(|(scope, _)| scope)
        .or_else(|| name.rsplit_once('.').map(|(scope, _)| scope))
        .unwrap_or("");
    (scope.to_string(), simple_name(name).to_string())
}

fn is_prose_reference_candidate(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    name.contains("::")
        || name.contains('.')
        || name.contains('_')
        || name
            .chars()
            .next()
            .map(|ch| ch.is_uppercase())
            .unwrap_or(false)
}

/// Returns true for C/C++ constructors that should not be used as inline
/// reference targets when a type with the same name is present.
fn is_constructor(item: &DocItem) -> bool {
    if matches!(item.lang, DocLanguage::C | DocLanguage::Cpp) && item.kind == DocKind::Function {
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
    if matches!(item.kind, DocKind::Module) && item.name.is_empty() {
        return item.line.saturating_sub(1);
    }
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

fn source_cache_key(item: &DocItem) -> String {
    item.file.display().to_string()
}

fn extract_source_text(item: &DocItem) -> String {
    std::fs::read_to_string(&item.file).unwrap_or_default()
}

fn source_file_title(item: &DocItem) -> String {
    item.file
        .strip_prefix(std::env::current_dir().unwrap_or_default())
        .unwrap_or(&item.file)
        .display()
        .to_string()
}

fn source_variants_for_item(item: &DocItem) -> Vec<DocItem> {
    if !matches!(item.lang, DocLanguage::C | DocLanguage::Cpp)
        || !matches!(item.kind, DocKind::Function | DocKind::Subroutine)
    {
        return vec![item.clone()];
    }

    let Some(root) = item.file.parent() else {
        return vec![item.clone()];
    };
    let name = simple_name(&item.name);
    let mut variants = Vec::new();
    let mut seen = HashSet::new();

    for entry in walkdir::WalkDir::new(root)
        .follow_links(false)
        .max_depth(3)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
    {
        let path = entry.path();
        if !is_c_family_source(path) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        for (line_idx, line) in text.lines().enumerate() {
            let trimmed = line.trim_start();
            if !is_function_source_candidate(trimmed, name) {
                continue;
            }
            let key = (path.to_path_buf(), line_idx + 1);
            if !seen.insert(key) {
                continue;
            }
            let mut variant = item.clone();
            variant.file = path.to_path_buf();
            variant.line = line_idx + 1;
            variant.signature = trimmed.to_string();
            variants.push(variant);
        }
    }

    if variants.is_empty() {
        return vec![item.clone()];
    }
    variants.sort_by(|a, b| {
        source_variant_rank(a)
            .cmp(&source_variant_rank(b))
            .then(a.file.cmp(&b.file))
            .then(a.line.cmp(&b.line))
    });
    variants
}

fn source_variant_rank(item: &DocItem) -> u8 {
    match item
        .file
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
    {
        "h" | "hh" | "hpp" | "hxx" | "cuh" => 0,
        _ => 1,
    }
}

fn source_variant_label(item: &DocItem) -> &'static str {
    match item
        .file
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
    {
        "h" | "hh" | "hpp" | "hxx" | "cuh" => "prototype",
        _ => "definition",
    }
}

fn is_c_family_source(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()).unwrap_or(""),
        "c" | "h" | "cc" | "cpp" | "cxx" | "c++" | "hh" | "hpp" | "hxx" | "cu" | "cuh"
    )
}

fn is_function_source_candidate(line: &str, name: &str) -> bool {
    if line.is_empty()
        || line.starts_with("//")
        || line.starts_with("/*")
        || line.starts_with('*')
        || line.starts_with('#')
    {
        return false;
    }
    let Some(pos) = line.find(name) else {
        return false;
    };
    let before = line[..pos].chars().last();
    if before.is_some_and(|ch| ch.is_alphanumeric() || ch == '_') {
        return false;
    }
    let after = &line[pos + name.len()..];
    after.trim_start().starts_with('(')
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
        DocLanguage::D | DocLanguage::Zig => false,
        DocLanguage::Python => matches!(
            token,
            "def" | "class" | "return" | "import" | "from" | "as" | "with"
                | "if" | "elif" | "else" | "for" | "while" | "try" | "except"
                | "lambda" | "yield" | "async" | "await" | "pass" | "raise"
        ),
        DocLanguage::TypeScript | DocLanguage::JavaScript => matches!(
            token,
            "function" | "class" | "interface" | "type" | "enum" | "const"
                | "let" | "var" | "return" | "export" | "import" | "default"
                | "extends" | "implements" | "new" | "this" | "async" | "await"
                | "typeof" | "instanceof" | "in" | "of" | "for" | "while"
        ),
        DocLanguage::CSharp => matches!(
            token,
            "public" | "private" | "protected" | "internal" | "static"
                | "class" | "struct" | "interface" | "enum" | "namespace"
                | "void" | "return" | "new" | "this" | "base" | "override"
                | "virtual" | "abstract" | "sealed" | "readonly" | "const"
                | "using" | "async" | "await" | "delegate" | "event"
        ),
        DocLanguage::Php => matches!(
            token,
            "public" | "private" | "protected" | "static" | "function"
                | "class" | "interface" | "trait" | "abstract" | "final"
                | "return" | "new" | "extends" | "implements" | "echo"
                | "namespace" | "use" | "require" | "include"
        ),
        DocLanguage::Ruby => matches!(
            token,
            "def" | "class" | "module" | "end" | "return" | "do"
                | "if" | "elsif" | "else" | "unless" | "while" | "for"
                | "begin" | "rescue" | "ensure" | "raise" | "yield"
                | "self" | "super" | "attr_accessor" | "attr_reader" | "attr_writer"
        ),
        DocLanguage::Lua => matches!(
            token,
            "function" | "local" | "return" | "end" | "if" | "then"
                | "else" | "elseif" | "while" | "do" | "for" | "repeat"
                | "until" | "and" | "or" | "not" | "nil" | "true" | "false"
        ),
        DocLanguage::R => matches!(
            token,
            "function" | "return" | "if" | "else" | "for" | "while"
                | "repeat" | "break" | "next" | "TRUE" | "FALSE" | "NULL"
                | "NA" | "Inf" | "NaN" | "library" | "require"
        ),
        DocLanguage::Haskell => matches!(
            token,
            "module" | "where" | "import" | "data" | "newtype" | "type"
                | "class" | "instance" | "let" | "in" | "do" | "case"
                | "of" | "if" | "then" | "else" | "deriving" | "forall"
        ),
        DocLanguage::Unknown => false,
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
        DocLanguage::Python => 3,
        DocLanguage::TypeScript => 4,
        DocLanguage::JavaScript => 5,
        DocLanguage::CSharp => 6,
        DocLanguage::Java => 7,
        DocLanguage::Kotlin => 8,
        DocLanguage::Go => 9,
        DocLanguage::Swift => 10,
        DocLanguage::Ruby => 11,
        DocLanguage::Php => 12,
        DocLanguage::Lua => 13,
        DocLanguage::R => 14,
        DocLanguage::Haskell => 15,
        DocLanguage::Fortran => 16,
        DocLanguage::D => 17,
        DocLanguage::Ada => 18,
        DocLanguage::Zig => 19,
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

fn is_index_item(item: &DocItem) -> bool {
    has_doc_content(item) || !matches!(item.kind, DocKind::Module)
}

fn has_doc_content(item: &DocItem) -> bool {
    !item.brief.is_empty() || !item.body.is_empty() || !item.tags.is_empty()
}

fn is_type_group_doc_item(item: &DocItem) -> bool {
    matches!(
        item.kind,
        DocKind::Class | DocKind::Struct | DocKind::Interface
    ) && !item.name.is_empty()
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
            if is_type_group_doc_item(item) {
                return split_scope(&item.name);
            }
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
            if is_type_group_doc_item(item) {
                return split_scope(&item.name);
            }
            let parts = split_scope_prefix(&item.name, ".");
            if parts.is_empty() {
                vec!["(root)".to_string()]
            } else {
                parts
            }
        }
        DocLanguage::Java
        | DocLanguage::Kotlin
        | DocLanguage::Swift
        | DocLanguage::Zig
        | DocLanguage::TypeScript
        | DocLanguage::JavaScript
        | DocLanguage::CSharp
        | DocLanguage::Php
        | DocLanguage::Ruby
        | DocLanguage::Lua
        | DocLanguage::R
        | DocLanguage::Haskell
        | DocLanguage::Python => {
            if is_type_group_doc_item(item) {
                return split_scope(&item.name);
            }
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use ratatui::style::{Color, Modifier, Style};

    use super::{
        find_doc_target, item_group_parts, linkable_text_line, overload_indices,
        push_doc_text_lines, push_item_header_lines, push_signature_lines,
        source_variants_for_item, type_members, App, FocusPane, TreeNode, TreeRowKind,
    };
    use crate::extract::{DocItem, DocKind, DocLanguage, DocMeta};

    #[test]
    fn tree_nodes_do_not_merge_case_distinct_labels() {
        let mut root = TreeNode::default();
        root.insert(&["Rust".into(), "Foo".into()], 0, false, true);
        root.insert(&["Rust".into(), "foo".into()], 1, false, true);

        let rust = root.children.get("Rust").expect("language group exists");
        assert!(rust.children.contains_key("Foo"));
        assert!(rust.children.contains_key("foo"));
        assert_eq!(rust.children.len(), 2);
    }

    #[test]
    fn tree_nodes_keep_source_targets_on_ancestor_groups() {
        let mut root = TreeNode::default();
        root.insert(
            &[
                "C/C++".into(),
                "stats.h".into(),
                "stats".into(),
                "OrderStatistics".into(),
            ],
            7,
            false,
            true,
        );

        let lang = root.children.get("C/C++").expect("language group exists");
        let file = lang.children.get("stats.h").expect("file group exists");
        let namespace = file.children.get("stats").expect("namespace group exists");
        let class = namespace
            .children
            .get("OrderStatistics")
            .expect("class group exists");

        assert_eq!(lang.source_idx, Some(7));
        assert_eq!(file.source_idx, Some(7));
        assert_eq!(namespace.source_idx, Some(7));
        assert_eq!(class.source_idx, Some(7));
    }

    #[test]
    fn source_only_namespace_does_not_create_index_node() {
        let namespace = item("stats", DocKind::Module, DocLanguage::Cpp);
        let app = App::new_with_visible(vec![namespace], vec![0]);

        assert!(app.rows.is_empty());
    }

    #[test]
    fn source_only_namespace_attaches_to_documented_children() {
        let mut namespace = item("stats", DocKind::Module, DocLanguage::Cpp);
        namespace.file = "stats.h".into();
        let mut mean = item("stats::mean", DocKind::Function, DocLanguage::Cpp);
        mean.file = "stats.h".into();
        mean.brief = "Arithmetic mean.".into();
        let mut app = App::new_with_visible(vec![namespace, mean], vec![0, 1]);
        app.expanded.insert("C/C++".into());
        app.expanded.insert("C/C++\u{1f}stats.h".into());
        app.rebuild_rows();

        let namespace = app
            .rows
            .iter()
            .find(|row| row.label == "stats")
            .expect("namespace group should exist");
        assert!(!app.rows.iter().any(|row| match row.kind {
            TreeRowKind::Item(idx) => idx == 0,
            TreeRowKind::Group { item_idx, .. } => item_idx == Some(0),
        }));
        assert!(matches!(
            namespace.kind,
            TreeRowKind::Group {
                source_idx: Some(0),
                ..
            }
        ));
    }

    #[test]
    fn horizontal_keys_change_focus_and_tab_switches_detail_tabs() {
        let mut item = item("stats::mean", DocKind::Function, DocLanguage::Cpp);
        item.brief = "Arithmetic mean.".into();
        let mut app = App::new_with_visible(vec![item], vec![0]);

        app.focus_right();
        assert_eq!(app.focus, FocusPane::Detail);
        assert!(!app.show_source);
        app.next_focused_tab();
        assert!(app.show_source);
        app.previous_focused_tab();
        assert!(!app.show_source);

        // Tab now toggles the detail pane even when the tree is focused.
        app.focus_left();
        assert_eq!(app.focus, FocusPane::Tree);
        app.next_focused_tab();
        assert!(app.show_source);
        app.previous_focused_tab();
        assert!(!app.show_source);
    }

    #[test]
    fn rust_struct_doc_routes_to_own_type_group() {
        let item = item("Builder", DocKind::Struct, DocLanguage::Rust);
        assert_eq!(item_group_parts(&item), vec!["Builder".to_string()]);
    }

    #[test]
    fn java_class_doc_routes_to_own_type_group() {
        let item = item("collections.IntStack", DocKind::Class, DocLanguage::Java);
        assert_eq!(
            item_group_parts(&item),
            vec!["collections".to_string(), "IntStack".to_string()]
        );
    }

    #[test]
    fn markdown_inline_styles_are_rendered_without_markers() {
        let mut links = Vec::new();
        let line = linkable_text_line(
            "use **bold** and *italic* plus `code`",
            &[],
            None,
            &DocLanguage::Rust,
            0,
            &mut links,
            Style::default().fg(Color::White),
        );
        let text: String = line
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect();

        assert_eq!(text, "use bold and italic plus code");
        assert!(line.spans.iter().any(|span| span.content.as_ref() == "bold"
            && span.style.add_modifier.contains(Modifier::BOLD)));
        assert!(line
            .spans
            .iter()
            .any(|span| span.content.as_ref() == "italic"
                && span.style.add_modifier.contains(Modifier::ITALIC)));
    }

    #[test]
    fn prose_links_can_target_type_definitions() {
        let all = vec![item("std::vector", DocKind::Class, DocLanguage::Cpp)];
        let mut links = Vec::new();
        let line = linkable_text_line(
            "returns std::vector values",
            &all,
            None,
            &DocLanguage::Cpp,
            0,
            &mut links,
            Style::default().fg(Color::White),
        );
        let text: String = line
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect();

        assert_eq!(text, "returns std::vector values");
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target_idx, 0);
    }

    #[test]
    fn prose_does_not_link_current_function_family() {
        let all = vec![
            item("stats::mean", DocKind::Function, DocLanguage::Cpp),
            item("stats::mean", DocKind::Function, DocLanguage::Cpp),
            item("stats::variance", DocKind::Function, DocLanguage::Cpp),
        ];
        let mut links = Vec::new();
        let line = linkable_text_line(
            "stats::mean calls stats::variance",
            &all,
            Some(0),
            &DocLanguage::Cpp,
            0,
            &mut links,
            Style::default().fg(Color::White),
        );
        let text: String = line
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect();

        assert_eq!(text, "stats::mean calls stats::variance");
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target_idx, 2);
    }

    #[test]
    fn prose_does_not_link_plain_lowercase_words() {
        let all = vec![item("mean", DocKind::Function, DocLanguage::Rust)];
        let mut links = Vec::new();
        let line = linkable_text_line(
            "the mean of values",
            &all,
            None,
            &DocLanguage::Rust,
            0,
            &mut links,
            Style::default().fg(Color::White),
        );
        let text: String = line
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect();

        assert_eq!(text, "the mean of values");
        assert!(links.is_empty());
    }

    #[test]
    fn class_overview_dedupes_identical_members_but_keeps_overloads() {
        let class = item("stats::OrderStatistics", DocKind::Class, DocLanguage::Cpp);
        let mut median_decl = item(
            "stats::OrderStatistics::median",
            DocKind::Function,
            DocLanguage::Cpp,
        );
        median_decl.signature = "double median() const;".into();
        let mut median_duplicate = median_decl.clone();
        median_duplicate.line = 42;
        let mut percentile = item(
            "stats::OrderStatistics::percentile",
            DocKind::Function,
            DocLanguage::Cpp,
        );
        percentile.signature = "double percentile(double p) const;".into();
        let mut percentile_overload = percentile.clone();
        percentile_overload.signature = "double percentile(int rank) const;".into();

        let all = vec![
            class.clone(),
            median_decl,
            median_duplicate,
            percentile,
            percentile_overload,
        ];
        let members = type_members(&all, &class);
        let names_and_sigs: Vec<_> = members
            .iter()
            .map(|item| (item.name.as_str(), item.signature.as_str()))
            .collect();

        assert_eq!(members.len(), 3);
        assert_eq!(
            names_and_sigs
                .iter()
                .filter(|(name, _)| *name == "stats::OrderStatistics::median")
                .count(),
            1
        );
        assert_eq!(
            names_and_sigs
                .iter()
                .filter(|(name, _)| *name == "stats::OrderStatistics::percentile")
                .count(),
            2
        );
    }

    #[test]
    fn documentation_signature_keeps_multiline_declarations() {
        let mut item = item("integrate", DocKind::Function, DocLanguage::C);
        item.signature =
            "double integrate(\n    double (*f)(double),\n    double a,\n    double b\n);".into();
        let mut lines = Vec::new();
        let mut links = Vec::new();

        push_signature_lines(&mut lines, &item, 120, &[], None, &mut links);
        let rendered: Vec<_> = lines
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>()
            })
            .collect();

        assert!(rendered
            .iter()
            .any(|line| line.contains("double integrate(")));
        assert!(rendered
            .iter()
            .any(|line| line.contains("double (*f)(double),")));
        assert!(rendered.iter().any(|line| line.trim() == ")"));
    }

    #[test]
    fn documentation_header_wraps_long_cpp_type_names() {
        let item = item(
            "__gnu_pbds::detail::container_base_dispatch::_Alloc",
            DocKind::Typedef,
            DocLanguage::Cpp,
        );
        let mut lines = Vec::new();

        push_item_header_lines(&mut lines, &item, 32);
        let rendered = lines
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>();
        let joined = rendered.join("");

        assert!(rendered.len() > 1, "{rendered:?}");
        assert!(joined.contains("[type] "));
        assert!(joined.contains("__gnu_pbds::detail::container_base_dispatch::_Alloc"));
        assert!(joined.contains("C++"));
    }

    #[test]
    fn markdown_table_is_rendered_without_separator_row() {
        let mut lines = Vec::new();
        let mut links = Vec::new();
        push_doc_text_lines(
            &mut lines,
            "| Parameter | Meaning |\n|-----------|---------|\n| `eps` | Absolute error tolerance |",
            80,
            &[],
            None,
            &DocLanguage::C,
            &mut links,
            Style::default().fg(Color::White),
        );
        let text = lines
            .iter()
            .flat_map(|line| line.spans.iter())
            .map(|span| span.content.as_ref())
            .collect::<Vec<_>>()
            .join("\n");

        assert!(text.contains("Parameter"));
        assert!(text.contains("Absolute error tolerance"));
        assert!(!text.contains("-----------"));
        assert!(lines
            .iter()
            .any(|line| line.spans.iter().any(|span| span.content.contains('┌'))));
    }

    #[test]
    fn hidden_cached_items_are_linkable_but_not_listed() {
        let visible = vec![item(
            "local::uses_vector",
            DocKind::Function,
            DocLanguage::Cpp,
        )];
        let hidden = vec![item("std::vector", DocKind::Class, DocLanguage::Cpp)];
        let mut app = App::new_with_hidden(visible, hidden);

        assert!(!app.rows.is_empty());
        assert!(app
            .rows
            .iter()
            .all(|row| row.label != "std" && row.label != "vector"));
        assert_eq!(
            find_doc_target(&app.all, "std::vector", None, &DocLanguage::Cpp),
            Some(1)
        );

        app.select_item_idx(1);
        assert_eq!(
            app.selected_item().map(|item| item.name.as_str()),
            Some("std::vector")
        );
        assert!(app
            .rows
            .iter()
            .all(|row| row.label != "std" && row.label != "vector"));
    }

    #[test]
    fn c_function_prototype_and_definition_are_source_variants_not_overloads() {
        let root = temp_root("source-variants");
        let header = root.join("mathlib.h");
        let source = root.join("mathlib.c");
        std::fs::write(
            &header,
            "/** Clamp. */\ndouble clamp(double v, double lo, double hi);\n",
        )
        .unwrap();
        std::fs::write(
            &source,
            "#include \"mathlib.h\"\ndouble clamp(double v, double lo, double hi) { return v; }\n",
        )
        .unwrap();

        let mut doc = item("clamp", DocKind::Function, DocLanguage::C);
        doc.file = header;
        doc.line = 1;
        doc.signature = "double clamp(double v, double lo, double hi);".into();

        let variants = source_variants_for_item(&doc);
        assert_eq!(variants.len(), 2);
        assert!(variants.iter().any(|item| item.file.ends_with("mathlib.h")));
        assert!(variants.iter().any(|item| item.file.ends_with("mathlib.c")));
        assert_eq!(super::source_variant_label(&variants[0]), "prototype");
        assert_eq!(super::source_variant_label(&variants[1]), "definition");

        let items = vec![doc.clone(), {
            let mut definition = doc;
            definition.file = source;
            definition.line = 2;
            definition.signature =
                "double clamp(double v, double lo, double hi) { return v; }".into();
            definition
        }];
        assert_eq!(overload_indices(&items, 0), vec![0]);

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn source_view_reads_full_file_not_snippet() {
        let root = temp_root("full-source");
        let source = root.join("mathlib.c");
        std::fs::write(
            &source,
            "double before(void) { return 0.0; }\n\n/** Clamp. */\ndouble clamp(double v) { return v; }\n\ndouble after(void) { return 1.0; }\n",
        )
        .unwrap();

        let mut doc = item("clamp", DocKind::Function, DocLanguage::C);
        doc.file = source;
        doc.line = 3;

        let text = super::extract_source_text(&doc);
        assert!(text.contains("double before"));
        assert!(text.contains("double clamp"));
        assert!(text.contains("double after"));

        std::fs::remove_dir_all(root).unwrap();
    }

    fn item(name: &str, kind: DocKind, lang: DocLanguage) -> DocItem {
        DocItem {
            name: name.to_string(),
            kind,
            brief: String::new(),
            body: String::new(),
            tags: Vec::new(),
            file: PathBuf::from("src/lib.rs"),
            line: 1,
            lang,
            signature: String::new(),
            meta: DocMeta::default(),
        }
    }

    fn temp_root(label: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("docify-tui-{label}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        root
    }
}
