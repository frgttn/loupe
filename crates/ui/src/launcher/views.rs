//! Views opened on top of root search ([`View`]): their state on the
//! launcher's screen stack, and how grids, details and forms are drawn. Lists
//! reuse root search's rows, and every view keeps the action panel and Esc.

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use gpui::{
    Animation, AnimationExt, AnyElement, App, Context, Entity, FocusHandle, Focusable, RenderImage,
    ScrollHandle, SharedString, TextRun, Window, div, img, list, prelude::*, px, rgba,
};
use spotlight_core::item::shortcut;
use spotlight_core::view::{DetailView, FieldKind, FieldValue, FormView, ListView, LiveRows, View};

use super::{Entry, EntryKind, Launcher, ToggleLayout, keycaps};
use crate::{
    markdown,
    smooth_scroll::{Scroller, capture_wheel},
    text_input::TextInput,
    theme,
};

/// Tiles per grid row.
pub(super) const GRID_COLUMNS: usize = 5;
const TILE_HEIGHT: f32 = 112.;
/// The grid's padding above its first line and below its last.
pub(super) const GRID_PADDING: f32 = 4.;
const TILE_ICON: f32 = 44.;
/// A tile's width: the grid row's, less its padding, by the columns.
const TILE_WIDTH: f32 = (super::WINDOW_SIZE.0 - 16.) / GRID_COLUMNS as f32;
/// The room for a tile's title: its width less the tile's padding.
const TILE_TEXT_WIDTH: f32 = TILE_WIDTH - 8. - 16.;
const TILE_TEXT_SIZE: f32 = 12.;
/// A title too long for its tile scrolls this fast while selected…
const MARQUEE_SPEED: f32 = 30.;
/// …after resting this long at its start, and again at its end.
const MARQUEE_REST: f32 = 1.2;
/// How far ↑/↓ scroll a detail view.
const DETAIL_STEP: f32 = 48.;
/// How soon a live list whose rows are still settling asks again.
const UNSETTLED_RETRY: Duration = Duration::from_secs(1);
/// `settings` key: `grid` or `list`, how every list shows (root search too).
pub(super) const LAYOUT_SETTING: &str = "layout";

/// One line of the grid: a section's heading, the calculator's card across
/// the whole width, or up to [`GRID_COLUMNS`] tiles (rows `start..end`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum GridLine {
    Heading(usize),
    Card(usize),
    Tiles(usize, usize),
}

impl GridLine {
    fn holds(self, row: usize) -> bool {
        match self {
            GridLine::Heading(_) => false,
            GridLine::Card(card) => card == row,
            GridLine::Tiles(start, end) => (start..end).contains(&row),
        }
    }
}

/// One level of the screen stack above root search.
pub(super) enum Screen {
    List {
        view: ListView,
        /// The provider and view id it came from, to build it again after
        /// a change (an entry deleted); `None` for views carried whole.
        origin: Option<(String, String)>,
        /// The chosen one of `view.filters`.
        filter: usize,
    },
    Detail {
        view: DetailView,
        /// Eased like the result list while smooth scrolling is on.
        scroll: Scroller,
    },
    Form(FormState),
}

pub(super) struct FormState {
    pub view: FormView,
    /// One per field of `view`, in order.
    pub inputs: Vec<FieldInput>,
    /// Why the last submit was refused.
    pub error: Option<SharedString>,
}

pub(super) enum FieldInput {
    Text(Entity<TextInput>),
    Checkbox { on: bool, focus: FocusHandle },
    Dropdown { selected: usize, focus: FocusHandle },
}

impl FormState {
    /// The key that submits it: Ctrl+Enter when Enter starts a new line in
    /// one of its fields.
    pub(super) fn submit_keys(&self) -> &'static str {
        let has_area = self
            .view
            .fields
            .iter()
            .any(|field| matches!(field.kind, FieldKind::TextArea { .. }));
        if has_area { "Ctrl Enter" } else { "Enter" }
    }
}

impl FieldInput {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        match self {
            FieldInput::Text(input) => input.focus_handle(cx),
            FieldInput::Checkbox { focus, .. } | FieldInput::Dropdown { focus, .. } => {
                focus.clone()
            }
        }
    }

    fn value(&self, cx: &App) -> FieldValue {
        match self {
            FieldInput::Text(input) => FieldValue::Text(input.read(cx).text().to_owned()),
            FieldInput::Checkbox { on, .. } => FieldValue::Checkbox(*on),
            FieldInput::Dropdown { selected, .. } => FieldValue::Dropdown(*selected),
        }
    }
}

impl Screen {
    /// `smooth`: whether scrolling it animates (the Smooth Scrolling setting).
    fn new(view: View, smooth: bool, cx: &mut Context<Launcher>) -> Self {
        match view {
            // Whether rows show as tiles is the Layout setting's, for every list.
            View::List(view) | View::Grid(view) => Screen::List {
                view,
                origin: None,
                filter: 0,
            },
            View::Detail(view) => Screen::Detail {
                view,
                scroll: Scroller::new(ScrollHandle::new(), smooth),
            },
            View::Form(view) => {
                let inputs = view
                    .fields
                    .iter()
                    .map(|field| match &field.kind {
                        FieldKind::Text {
                            placeholder,
                            default,
                            ..
                        }
                        | FieldKind::TextArea {
                            placeholder,
                            default,
                            ..
                        } => {
                            let multiline = matches!(field.kind, FieldKind::TextArea { .. });
                            let (placeholder, default) = (placeholder.clone(), default.clone());
                            FieldInput::Text(cx.new(|cx| {
                                let mut input = if multiline {
                                    TextInput::multiline(placeholder, cx)
                                } else {
                                    TextInput::new(placeholder, cx)
                                };
                                input.set_text(default, cx);
                                input
                            }))
                        }
                        FieldKind::Checkbox { default } => FieldInput::Checkbox {
                            on: *default,
                            focus: cx.focus_handle(),
                        },
                        FieldKind::Dropdown { default, options } => FieldInput::Dropdown {
                            selected: (*default).min(options.len().saturating_sub(1)),
                            focus: cx.focus_handle(),
                        },
                    })
                    .collect();
                Screen::Form(FormState {
                    view,
                    inputs,
                    error: None,
                })
            }
        }
    }

    pub(super) fn title(&self) -> &str {
        match self {
            Screen::List { view, .. } => &view.title,
            Screen::Detail { view, .. } => &view.title,
            Screen::Form(form) => &form.view.title,
        }
    }

    /// Lists and grids filter with the search field; details and forms don't
    /// use it.
    pub(super) fn has_search(&self) -> bool {
        matches!(self, Screen::List { .. })
    }
}

impl Launcher {
    /// Opens `view` on top of the current screen.
    pub(super) fn push_view(&mut self, view: View, window: &mut Window, cx: &mut Context<Self>) {
        // Rows of their own (open windows, processes) bring icons the
        // catalog didn't have.
        let icons: Vec<(String, String)> = match &view {
            View::List(list) | View::Grid(list) => list
                .items
                .iter()
                .filter(|item| !self.icons.contains_key(&item.id))
                .filter_map(|item| Some((item.id.clone(), super::icon_source(&item.icon)?)))
                .collect(),
            _ => Vec::new(),
        };
        self.load_icons(icons, window.scale_factor(), cx);
        let screen = Screen::new(view, self.scroll.is_enabled(), cx);
        self.screens.push(screen);
        self.enter_screen(window, cx);
    }

    /// Opens `view` as if a command had pushed it. For the self-test.
    pub fn open_view(&mut self, view: View, window: &mut Window, cx: &mut Context<Self>) {
        self.push_view(view, window, cx);
    }

    /// Back to the screen below (root search at the bottom).
    pub(super) fn pop_screen(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // A typed keyword's form was left: its snippet won't be pasted.
        self.keyword_target = None;
        if let Some(Screen::List {
            view,
            origin: Some((provider, _)),
            ..
        }) = self.screens.pop()
            && view.live
        {
            if let Some(cancel) = self.live_cancel.take() {
                cancel.store(true, Ordering::Relaxed);
            }
            // A live list's rows are only ever shown once (files found while
            // typing), so their icons go with it rather than pile up.
            self.forget_icons(&provider);
        }
        self.enter_screen(window, cx);
    }

    /// In a live list, asks its provider for the rows of what is typed, on a
    /// background thread. A newer search cancels this one, and only the
    /// newest one's rows are shown.
    pub(super) fn search_live(&mut self, cx: &mut Context<Self>) {
        let Some(Screen::List {
            view,
            origin: Some((provider, id)),
            filter,
            ..
        }) = self.screens.last()
        else {
            return;
        };
        if !view.live {
            return;
        }
        let Some(provider) = self
            .search
            .providers()
            .iter()
            .find(|candidate| candidate.id() == provider)
            .cloned()
        else {
            return;
        };
        let keyword = view
            .filters
            .get(*filter)
            .and_then(|choice| choice.keyword.clone());
        let id = id.clone();
        if let Some(older) = self.live_cancel.take() {
            older.store(true, Ordering::Relaxed);
        }
        let cancel = Arc::new(AtomicBool::new(false));
        self.live_cancel = Some(cancel.clone());
        self.live_generation += 1;
        let generation = self.live_generation;
        let query = self.query(cx);
        let rows = cx
            .background_executor()
            .spawn(async move { provider.search_view(&id, &query, keyword.as_deref(), &cancel) });
        cx.spawn(async move |this, cx| {
            let Some(rows) = rows.await else {
                return;
            };
            this.update(cx, |this, cx| this.show_live_rows(generation, rows, cx))
                .ok();
        })
        .detach();
    }

    /// In root search after the file prefix, asks the files provider for the
    /// rows of what follows it, like a live list. Without the prefix, lets go
    /// of the rows found before.
    pub(super) fn search_prefix(&mut self, cx: &mut Context<Self>) {
        if !self.screens.is_empty() {
            return;
        }
        let query = self.query(cx);
        let Some(rest) = super::file_query(&self.file_prefix, &query).map(str::to_owned) else {
            if !self.prefix_rows.is_empty() {
                self.prefix_rows.clear();
                self.forget_icons(super::FILES_PROVIDER);
            }
            return;
        };
        let Some(provider) = self
            .search
            .providers()
            .iter()
            .find(|candidate| candidate.id() == super::FILES_PROVIDER)
            .cloned()
        else {
            return;
        };
        if let Some(older) = self.live_cancel.take() {
            older.store(true, Ordering::Relaxed);
        }
        let cancel = Arc::new(AtomicBool::new(false));
        self.live_cancel = Some(cancel.clone());
        self.live_generation += 1;
        let generation = self.live_generation;
        let rows = cx
            .background_executor()
            .spawn(async move { provider.search_view(super::FILES_VIEW, &rest, None, &cancel) });
        cx.spawn(async move |this, cx| {
            let Some(rows) = rows.await else {
                return;
            };
            this.update(cx, |this, cx| {
                if generation != this.live_generation || !this.screens.is_empty() {
                    return;
                }
                let icons: Vec<(String, String)> = rows
                    .items
                    .iter()
                    .filter(|item| !this.icons.contains_key(&item.id))
                    .filter_map(|item| Some((item.id.clone(), super::icon_source(&item.icon)?)))
                    .collect();
                this.prefix_rows = rows.items;
                let query = this.query(cx);
                this.refilter(&query);
                this.load_icons(icons, this.window_scale, cx);
                if rows.unsettled {
                    this.search_again(generation, Self::search_prefix, cx);
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// Frees the icons of rows from `provider` that were only ever shown
    /// once (files found while typing), so they don't pile up.
    fn forget_icons(&mut self, provider: &str) {
        let prefix = format!("{provider}:");
        let stale: Vec<String> = self
            .icons
            .keys()
            .filter(|id| id.starts_with(&prefix))
            .cloned()
            .collect();
        for id in stale {
            if let Some(image) = self.icons.remove(&id) {
                self.stale_images.push(image);
            }
        }
    }

    /// Puts a live search's rows in the open list, unless a newer search or
    /// another screen has taken its place.
    fn show_live_rows(&mut self, generation: u64, rows: LiveRows, cx: &mut Context<Self>) {
        if generation != self.live_generation {
            return;
        }
        let icons: Vec<(String, String)> = rows
            .items
            .iter()
            .filter(|item| !self.icons.contains_key(&item.id))
            .filter_map(|item| Some((item.id.clone(), super::icon_source(&item.icon)?)))
            .collect();
        let Some(Screen::List { view, .. }) = self.screens.last_mut() else {
            return;
        };
        if !view.live {
            return;
        }
        view.items = rows.items;
        view.empty = rows.empty;
        let query = self.query(cx);
        self.refilter(&query);
        self.load_icons(icons, self.window_scale, cx);
        if rows.unsettled {
            self.search_again(generation, Self::search_live, cx);
        }
        cx.notify();
    }

    /// While the rows still change by themselves (files being indexed),
    /// runs `search` again in a moment, unless something newer was searched
    /// meanwhile.
    fn search_again(
        &self,
        generation: u64,
        search: fn(&mut Self, &mut Context<Self>),
        cx: &mut Context<Self>,
    ) {
        let timer = cx.background_executor().timer(UNSETTLED_RETRY);
        cx.spawn(async move |this, cx| {
            timer.await;
            this.update(cx, |this, cx| {
                if generation == this.live_generation {
                    search(this, cx);
                }
            })
            .ok();
        })
        .detach();
    }

    /// Sets up the search field, the rows and the keyboard for the top screen.
    pub(super) fn enter_screen(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let placeholder: SharedString = match self.screens.last() {
            None => super::SEARCH_PLACEHOLDER.into(),
            Some(Screen::List { view, .. }) => view
                .placeholder
                .clone()
                .unwrap_or_else(|| format!("Search {}…", view.title.to_lowercase()))
                .into(),
            Some(_) => SharedString::default(),
        };
        self.input.update(cx, |input, cx| {
            input.clear(cx);
            input.set_placeholder(placeholder, cx);
        });
        self.refilter("");
        self.sync_grid_arrows(cx);
        match self.screens.last() {
            None | Some(Screen::List { .. }) => window.focus(&self.input.focus_handle(cx)),
            Some(Screen::Detail { .. }) => window.focus(&self.focus),
            Some(Screen::Form(form)) => match form.inputs.first() {
                Some(input) => window.focus(&input.focus_handle(cx)),
                None => window.focus(&self.focus),
            },
        }
        cx.notify();
    }

    /// Builds the open provider list again, after its items changed, and
    /// keeps the search text. Selects the row `keep` (an item id) if given
    /// and still there, otherwise stays at the same position.
    pub(super) fn refresh_screen(
        &mut self,
        keep: Option<&str>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(Screen::List {
            origin: Some((provider, id)),
            ..
        }) = self.screens.last()
        else {
            return;
        };
        let fresh = self
            .search
            .providers()
            .iter()
            .find(|candidate| candidate.id() == provider)
            .and_then(|provider| provider.view(id));
        match fresh {
            Some(View::List(fresh) | View::Grid(fresh)) => {
                if let Some(Screen::List { view, filter, .. }) = self.screens.last_mut() {
                    if *filter >= fresh.filters.len() {
                        *filter = 0;
                    }
                    *view = fresh;
                }
            }
            _ => return self.pop_screen(window, cx),
        }
        // A live list's rows come from a background search.
        self.search_live(cx);
        let previous = self.selected;
        let query = self.query(cx);
        self.refilter(&query);
        self.selected = keep
            .and_then(|keep| {
                self.results
                    .iter()
                    .position(|row| self.item(row).id == keep)
            })
            .unwrap_or_else(|| previous.min(self.results.len().saturating_sub(1)));
        self.reveal_selected();
        cx.notify();
    }

    /// Tab / Shift+Tab in a list with a filter: the next or previous choice.
    /// Returns `false` when the open view has no filter.
    pub(super) fn step_filter(&mut self, forward: bool, cx: &mut Context<Self>) -> bool {
        let Some(Screen::List { view, filter, .. }) = self.screens.last_mut() else {
            return false;
        };
        let count = view.filters.len();
        if count == 0 {
            return false;
        }
        *filter = if forward {
            (*filter + 1) % count
        } else {
            (*filter + count - 1) % count
        };
        self.search_live(cx);
        let query = self.query(cx);
        self.refilter(&query);
        cx.notify();
        true
    }

    /// The open list's filter choice, if it has a filter.
    pub(super) fn filter_label(&self) -> Option<SharedString> {
        match self.screens.last() {
            Some(Screen::List { view, filter, .. }) => view
                .filters
                .get(*filter)
                .map(|choice| SharedString::from(choice.label.clone())),
            _ => None,
        }
    }

    /// Whether the rows show as tiles: the Layout setting, in root search
    /// and lists, except those with a preview beside the rows (clipboard
    /// history, files), which a tile has no room for.
    pub(super) fn in_grid(&self, cx: &App) -> bool {
        self.grid_layout
            && matches!(self.screens.last(), None | Some(Screen::List { .. }))
            && !self.shows_previews(cx)
    }

    /// Whether the rows show as tiles. For the self-test.
    pub fn in_grid_view(&self, cx: &App) -> bool {
        self.in_grid(cx)
    }

    /// The Layout setting: every list as a grid (`true`) or as rows.
    pub fn grid_layout(&self) -> bool {
        self.grid_layout
    }

    /// Sets the Layout setting (from Settings, or Ctrl+Shift+G) and saves it.
    pub fn set_grid_layout(&mut self, grid: bool, cx: &mut Context<Self>) {
        if grid == self.grid_layout {
            return;
        }
        self.grid_layout = grid;
        // Two quick switches save in either order; each writes the latest.
        self.saved_layout.store(grid, Ordering::Relaxed);
        let latest = self.saved_layout.clone();
        self.persist(cx, move |store| {
            let grid = latest.load(Ordering::Relaxed);
            store.set_setting(LAYOUT_SETTING, if grid { "grid" } else { "list" })
        });
        self.sync_grid_arrows(cx);
        self.reveal_selected();
        cx.notify();
    }

    /// ←/→ move between tiles in a grid, so the search field passes them on;
    /// in rows they move the text cursor.
    pub(super) fn sync_grid_arrows(&mut self, cx: &mut Context<Self>) {
        let grid = self.in_grid(cx);
        self.input
            .update(cx, |input, _| input.set_pass_horizontal_arrows(grid));
    }

    /// The action panel's entry that switches the Layout setting, in lists
    /// opened from root search (not those that always show rows, beside a
    /// preview). Root search's rows keep their own actions; Ctrl+Shift+G
    /// works there too.
    pub(super) fn layout_entry(&self) -> Option<Entry> {
        match self.screens.last() {
            Some(Screen::List { view, .. })
                if !view.items.iter().any(|item| item.preview.is_some()) => {}
            _ => return None,
        }
        let title = if self.grid_layout {
            "Show as List"
        } else {
            "Show as Grid"
        };
        Some(Entry::new(title, EntryKind::ToggleLayout).with_shortcut(shortcut::LAYOUT))
    }

    pub(super) fn toggle_layout_action(
        &mut self,
        _: &ToggleLayout,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.panel = None;
        self.toggle_layout(window, cx);
    }

    /// Switches every list between tiles and rows, keeping the search text
    /// and the selection.
    pub(super) fn toggle_layout(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.collapsed(cx) {
            return;
        }
        let grid = !self.grid_layout;
        self.set_grid_layout(grid, cx);
        if matches!(self.screens.last(), None | Some(Screen::List { .. })) {
            window.focus(&self.input.focus_handle(cx));
        }
        self.show_toast(
            if grid {
                "Shown as a grid"
            } else {
                "Shown as a list"
            }
            .into(),
            cx,
        );
    }

    /// The grid's lines for the rows shown: headings where root search's
    /// sections start, the calculator's card on its own, and tiles between.
    pub(super) fn grid_lines(&self) -> Vec<GridLine> {
        fn flush(lines: &mut Vec<GridLine>, run: &mut Option<usize>, end: usize) {
            let Some(mut start) = run.take() else {
                return;
            };
            while start < end {
                let stop = (start + GRID_COLUMNS).min(end);
                lines.push(GridLine::Tiles(start, stop));
                start = stop;
            }
        }
        let count = self.results.len();
        let card = self.card_row();
        let mut lines = Vec::new();
        let mut run = None;
        let mut section = 0;
        for row in 0..count {
            while section < self.sections.len() && self.sections[section].0 <= row {
                flush(&mut lines, &mut run, row);
                lines.push(GridLine::Heading(section));
                section += 1;
            }
            if card == Some(row) {
                flush(&mut lines, &mut run, row);
                lines.push(GridLine::Card(row));
            } else if run.is_none() {
                run = Some(row);
            }
        }
        flush(&mut lines, &mut run, count);
        lines
    }

    /// Scrolls the grid so the selected tile shows, with the heading above
    /// it when it is in a section's first line; eased when `animate`.
    pub(super) fn reveal_selected_tile(&mut self, animate: bool) {
        let lines = self.grid_lines();
        let Some(line) = lines.iter().position(|line| line.holds(self.selected)) else {
            return;
        };
        if line > 0 && matches!(lines[line - 1], GridLine::Heading(_)) {
            self.grid_scroll.reveal(&[line, line - 1], animate);
        } else {
            self.grid_scroll.reveal(&[line], animate);
        }
    }

    fn select_tile(&mut self, next: usize, cx: &mut Context<Self>) {
        if next != self.selected {
            self.selected = next;
            self.confirming = None;
            self.reveal_selected_tile(true);
            cx.notify();
        }
    }

    /// ←/→ in the grid: the previous or next tile, across lines too.
    pub(super) fn grid_move(&mut self, delta: isize, cx: &mut Context<Self>) {
        let count = self.results.len();
        if count == 0 {
            return;
        }
        let next = (self.selected as isize + delta).clamp(0, count as isize - 1) as usize;
        self.select_tile(next, cx);
    }

    /// ↑/↓ in the grid: the tile in the same column of the line above or
    /// below (its last one when that line is shorter), over headings.
    pub(super) fn grid_step(&mut self, down: bool, cx: &mut Context<Self>) {
        let lines = self.grid_lines();
        let Some(current) = lines.iter().position(|line| line.holds(self.selected)) else {
            return;
        };
        let column = match lines[current] {
            GridLine::Tiles(start, _) => self.selected - start,
            _ => 0,
        };
        let tiles = |line: &GridLine| !matches!(line, GridLine::Heading(_));
        let target = if down {
            lines[current + 1..]
                .iter()
                .position(tiles)
                .map(|ix| current + 1 + ix)
        } else {
            lines[..current].iter().rposition(tiles)
        };
        let next = match target.map(|ix| lines[ix]) {
            Some(GridLine::Tiles(start, end)) => (start + column).min(end - 1),
            Some(GridLine::Card(row)) => row,
            _ => return,
        };
        self.select_tile(next, cx);
    }

    /// The items of the open list or grid, to filter.
    pub(super) fn list_items(&self) -> Option<&[spotlight_core::item::SearchItem]> {
        match self.screens.last() {
            Some(Screen::List { view, .. }) => Some(&view.items),
            _ => None,
        }
    }

    /// ↑/↓ in a detail view scroll it.
    pub(super) fn detail_scroll(&mut self, down: bool, cx: &mut Context<Self>) {
        if let Some(Screen::Detail { scroll, .. }) = self.screens.last_mut() {
            // Positive deltas move the content down, as for the wheel.
            scroll.scroll_by(px(if down { -DETAIL_STEP } else { DETAIL_STEP }));
            cx.notify();
        }
    }

    /// Advances the open detail view's scrolling by a frame; returns whether
    /// another frame is needed.
    pub(super) fn tick_detail_scroll(&mut self) -> bool {
        match self.screens.last_mut() {
            Some(Screen::Detail { scroll, .. }) => scroll.tick(),
            _ => false,
        }
    }

    /// Turns smooth scrolling on or off in every open detail view.
    pub(super) fn set_detail_smooth_scrolling(&mut self, enabled: bool) {
        for screen in &mut self.screens {
            if let Screen::Detail { scroll, .. } = screen {
                scroll.set_enabled(enabled);
            }
        }
    }

    /// Submits the open form: checks it, fills its command and runs that.
    pub(super) fn submit_form(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(Screen::Form(form)) = self.screens.last() else {
            return;
        };
        let values: Vec<FieldValue> = form.inputs.iter().map(|input| input.value(cx)).collect();
        match form.view.check(&values) {
            Ok(()) => {
                let command = form.view.fill(&values);
                self.perform(&command, window, cx);
            }
            Err(why) => {
                if let Some(Screen::Form(form)) = self.screens.last_mut() {
                    form.error = Some(why.into());
                }
                cx.notify();
            }
        }
    }

    /// Tab / Shift+Tab: the next or previous field, wrapping around.
    pub(super) fn focus_field(
        &mut self,
        forward: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(Screen::Form(form)) = self.screens.last() else {
            return;
        };
        let count = form.inputs.len();
        if count == 0 {
            return;
        }
        let current = form
            .inputs
            .iter()
            .position(|input| input.focus_handle(cx).is_focused(window))
            .unwrap_or(count - 1);
        let next = if forward {
            (current + 1) % count
        } else {
            (current + count - 1) % count
        };
        window.focus(&form.inputs[next].focus_handle(cx));
        cx.notify();
    }

    /// Space / ←/→ on the focused checkbox or dropdown.
    pub(super) fn change_choice(
        &mut self,
        delta: isize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(Screen::Form(form)) = self.screens.last_mut() else {
            return;
        };
        for (field, input) in form.view.fields.iter().zip(&mut form.inputs) {
            match input {
                FieldInput::Checkbox { on, focus } if focus.is_focused(window) => *on = !*on,
                FieldInput::Dropdown { selected, focus } if focus.is_focused(window) => {
                    if let FieldKind::Dropdown { options, .. } = &field.kind
                        && !options.is_empty()
                    {
                        let count = options.len() as isize;
                        *selected = (*selected as isize + delta).rem_euclid(count) as usize;
                    }
                }
                _ => continue,
            }
            form.error = None;
            cx.notify();
            return;
        }
    }

    /// The open form's values and its error. For the self-test.
    pub fn form_state(&self, cx: &App) -> Option<(Vec<FieldValue>, Option<String>)> {
        let Some(Screen::Form(form)) = self.screens.last() else {
            return None;
        };
        let values = form.inputs.iter().map(|input| input.value(cx)).collect();
        Some((values, form.error.as_ref().map(|error| error.to_string())))
    }

    /// The open form's text field that has the keyboard.
    pub(super) fn focused_form_input(
        &self,
        window: &Window,
        cx: &App,
    ) -> Option<Entity<TextInput>> {
        let Some(Screen::Form(form)) = self.screens.last() else {
            return None;
        };
        form.inputs.iter().find_map(|input| match input {
            FieldInput::Text(text) if text.focus_handle(cx).is_focused(window) => {
                Some(text.clone())
            }
            _ => None,
        })
    }

    /// The selected row or tile. For the self-test.
    pub fn selected_index(&self) -> usize {
        self.selected
    }

    /// Lists and grids: the rows; details and forms: their own body.
    pub(super) fn render_screen_body(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let qr = match self.screens.last()? {
            Screen::Detail { view, .. } => view.qr.clone(),
            _ => None,
        };
        let qr = qr.and_then(|text| self.qr_image(&text, window.scale_factor()));
        match self.screens.last()? {
            // Rows or tiles: drawn by the launcher, as for root search.
            Screen::List { .. } => None,
            Screen::Detail { view, scroll } => Some(render_detail(view, scroll, qr, cx)),
            Screen::Form(form) => Some(self.render_form(form, window, cx)),
        }
    }

    /// The rows as tiles, under root search's section headings.
    pub(super) fn render_grid(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let lines = self.grid_lines();
        // Lines differ in height, which the list measures once: it starts
        // over when they change.
        if lines != self.grid_shape {
            self.grid_scroll.reset(lines.len());
            self.grid_shape = lines;
            self.reveal_selected_tile(false);
        }
        let grid = list(
            self.grid_scroll.state(),
            cx.processor(|this, line: usize, window, cx| {
                match this.grid_shape.get(line).copied() {
                    Some(GridLine::Heading(section)) => {
                        let title = this.sections[section].1.clone();
                        super::render_heading(line, title)
                    }
                    Some(GridLine::Card(row)) => div()
                        .px_2()
                        .child(this.render_card(line, row, 0, cx))
                        .into_any_element(),
                    Some(GridLine::Tiles(start, end)) => div()
                        .w_full()
                        .h(px(TILE_HEIGHT))
                        .px_2()
                        .flex()
                        .children((start..end).map(|ix| this.render_tile(ix, window, cx)))
                        .into_any_element(),
                    None => div().into_any_element(),
                }
            }),
        )
        .size_full()
        .py(px(GRID_PADDING));
        // Wheel notches are animated instead; everything else falls through to GPUI.
        let wheel = capture_wheel(cx.entity().downgrade(), |this: &mut Self, delta, cx| {
            if !this.grid_scroll.is_enabled() {
                return false;
            }
            this.grid_scroll.scroll_by(delta);
            cx.notify();
            true
        });
        div()
            .size_full()
            .relative()
            .child(grid)
            .child(wheel)
            .into_any_element()
    }

    fn render_tile(&self, ix: usize, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let Some(row) = self.results.get(ix) else {
            return div().into_any_element();
        };
        let item = self.item(row);
        let selected = ix == self.selected;
        div()
            .id(("tile", ix))
            .flex_1()
            .max_w(px(TILE_WIDTH))
            .h_full()
            .p_1()
            .child(
                div()
                    .id("tile-inner")
                    .size_full()
                    .px_2()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .gap_2()
                    .rounded(px(8.))
                    .when(selected, |tile| tile.bg(rgba(theme::p().selected)))
                    .when(!selected, |tile| {
                        tile.hover(|style| style.bg(rgba(theme::p().hover)))
                    })
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.selected = ix;
                        this.confirm(&super::Confirm, window, cx);
                    }))
                    .child(self.render_icon_sized(item, TILE_ICON))
                    .child(tile_title(ix, &item.title, selected, window)),
            )
            .into_any_element()
    }

    fn render_form(
        &self,
        form: &FormState,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let fields =
            form.view
                .fields
                .iter()
                .zip(&form.inputs)
                .enumerate()
                .map(|(ix, (field, input))| {
                    let focused = input.focus_handle(cx).is_focused(window);
                    let area = matches!(field.kind, FieldKind::TextArea { .. });
                    let control: AnyElement = match (input, &field.kind) {
                        (FieldInput::Text(input), _) => div()
                            .flex_1()
                            .min_w_0()
                            .when(area, |field| field.py(px(7.)))
                            .when(!area, |field| field.h(px(34.)).items_center())
                            .px_2()
                            .flex()
                            .rounded(px(6.))
                            .border_1()
                            .border_color(rgba(if focused {
                                theme::p().accent
                            } else {
                                theme::p().surface_border
                            }))
                            .bg(rgba(theme::p().surface))
                            .child(div().flex_1().child(input.clone()))
                            .into_any_element(),
                        (FieldInput::Checkbox { on, focus }, _) => {
                            let on = *on;
                            div()
                                .id(("checkbox", ix))
                                .key_context(CHOICE_CONTEXT)
                                .track_focus(focus)
                                .flex()
                                .items_center()
                                .gap_2()
                                .cursor_pointer()
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.toggle_checkbox(ix, window, cx)
                                }))
                                .child(
                                    div()
                                        .size(px(18.))
                                        .rounded(px(4.))
                                        .border_1()
                                        .border_color(rgba(if focused {
                                            theme::p().accent
                                        } else {
                                            theme::p().surface_border
                                        }))
                                        .when(on, |check| check.bg(rgba(theme::p().accent)))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .text_size(px(12.))
                                        .when(on, |check| check.child("✓")),
                                )
                                .into_any_element()
                        }
                        (
                            FieldInput::Dropdown { selected, focus },
                            FieldKind::Dropdown { options, .. },
                        ) => {
                            let selected = *selected;
                            div()
                                .id(("dropdown", ix))
                                .key_context(CHOICE_CONTEXT)
                                .track_focus(focus)
                                .flex_1()
                                .p_1()
                                .rounded(px(8.))
                                .border_1()
                                .border_color(rgba(if focused {
                                    theme::p().accent
                                } else {
                                    0x00000000
                                }))
                                .flex()
                                .flex_wrap()
                                .gap_1()
                                .children(options.iter().enumerate().map(|(option, (label, _))| {
                                    let chosen = option == selected;
                                    div()
                                        .id(("option", ix * 100 + option))
                                        .px_2()
                                        .h(px(26.))
                                        .flex()
                                        .items_center()
                                        .rounded_full()
                                        .border_1()
                                        .text_size(px(12.))
                                        .cursor_pointer()
                                        .when(chosen, |chip| {
                                            chip.border_color(rgba(theme::p().accent))
                                                .bg(rgba(theme::p().accent_soft))
                                        })
                                        .when(!chosen, |chip| {
                                            chip.border_color(rgba(theme::p().surface_border))
                                                .text_color(rgba(theme::p().text_muted))
                                        })
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            this.choose_option(ix, option, window, cx)
                                        }))
                                        .child(SharedString::from(label.clone()))
                                }))
                                .into_any_element()
                        }
                        (FieldInput::Dropdown { .. }, _) => div().into_any_element(),
                    };
                    div()
                        .flex()
                        // A text area's label stays by its first row.
                        .when(area, |row| row.items_start())
                        .when(!area, |row| row.items_center())
                        .gap_3()
                        .child(
                            div()
                                .flex_none()
                                .when(area, |label| label.pt(px(8.)))
                                .w(px(110.))
                                .text_size(px(13.))
                                .text_color(rgba(theme::p().text_muted))
                                .child(SharedString::from(field.label.clone())),
                        )
                        .child(control)
                });
        div()
            .flex_1()
            .px_6()
            .py_5()
            .flex()
            .flex_col()
            .gap_3()
            .children(fields)
            .when_some(form.error.clone(), |body, error| {
                body.child(
                    div()
                        .pl(px(122.))
                        .text_size(px(12.))
                        .text_color(rgba(theme::p().error))
                        .child(error),
                )
            })
            .child(
                div()
                    .pl(px(122.))
                    .flex()
                    .items_center()
                    .gap_1()
                    .text_size(px(11.))
                    .text_color(rgba(theme::p().text_faint))
                    .child("Tab moves between fields;")
                    .child(keycaps(form.submit_keys()))
                    .child(SharedString::from(form.view.submit_title.to_lowercase()))
                    .child("·")
                    .child(keycaps("Esc"))
                    .child("back"),
            )
            .into_any_element()
    }

    fn toggle_checkbox(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(Screen::Form(form)) = self.screens.last_mut()
            && let Some(FieldInput::Checkbox { on, focus }) = form.inputs.get_mut(ix)
        {
            *on = !*on;
            window.focus(focus);
            form.error = None;
            cx.notify();
        }
    }

    fn choose_option(
        &mut self,
        ix: usize,
        option: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(Screen::Form(form)) = self.screens.last_mut()
            && let Some(FieldInput::Dropdown { selected, focus }) = form.inputs.get_mut(ix)
        {
            *selected = option;
            window.focus(focus);
            form.error = None;
            cx.notify();
        }
    }
}

/// Key context of focused checkboxes and dropdowns, where Space and ←/→
/// change the choice (elsewhere Space types a space).
pub(super) const CHOICE_CONTEXT: &str = "FormChoice";

/// `qr`: the image of the view's QR code and its size, drawn above the text.
fn render_detail(
    view: &DetailView,
    scroll: &Scroller,
    qr: Option<(Arc<RenderImage>, f32)>,
    cx: &mut Context<Launcher>,
) -> AnyElement {
    let text = div()
        .id("detail")
        .size_full()
        .overflow_y_scroll()
        .track_scroll(&scroll.handle())
        .px_5()
        .py_4()
        .text_size(px(14.))
        .when_some(qr, |text, (image, size)| {
            text.child(
                div().flex().justify_center().pb_4().child(
                    img(image)
                        .size(px(size))
                        .flex_none()
                        .rounded(px(6.))
                        .overflow_hidden(),
                ),
            )
        })
        .child(markdown::render(&view.markdown));
    // Wheel notches are animated instead; everything else falls through to GPUI.
    let wheel = capture_wheel(cx.entity().downgrade(), |this: &mut Launcher, delta, cx| {
        let Some(Screen::Detail { scroll, .. }) = this.screens.last_mut() else {
            return false;
        };
        if !scroll.is_enabled() {
            return false;
        }
        scroll.scroll_by(delta);
        cx.notify();
        true
    });
    div()
        .flex_1()
        .min_h_0()
        .flex()
        .child(
            div()
                .flex_1()
                .min_w_0()
                .h_full()
                .relative()
                .child(text)
                .child(wheel),
        )
        .when(!view.metadata.is_empty(), |body| {
            body.child(
                div()
                    .id("detail-metadata")
                    .flex_none()
                    .w(px(220.))
                    .h_full()
                    // A contact read from a QR code can have more facts than fit.
                    .overflow_y_scroll()
                    .px_4()
                    .py_4()
                    .border_l_1()
                    .border_color(rgba(theme::p().hairline))
                    .flex()
                    .flex_col()
                    .gap_3()
                    .children(view.metadata.iter().map(|(label, value)| {
                        div()
                            .flex()
                            .flex_col()
                            .gap_0p5()
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(rgba(theme::p().text_faint))
                                    .child(SharedString::from(label.clone())),
                            )
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(rgba(theme::p().text))
                                    .child(SharedString::from(value.clone())),
                            )
                    })),
            )
        })
        .into_any_element()
}

/// A tile's title: centred when it fits. One too long is cut at the end with
/// "…", and scrolls to its end and back while its tile is selected.
fn tile_title(ix: usize, title: &str, selected: bool, window: &mut Window) -> AnyElement {
    let title = SharedString::from(title.to_owned());
    let overflow = if selected {
        let font_size = px(TILE_TEXT_SIZE);
        let run = TextRun {
            len: title.len(),
            font: window.text_style().font(),
            color: window.text_style().color,
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let width = window
            .text_system()
            .shape_line(title.clone(), font_size, &[run], None)
            .width;
        f32::from(width) - TILE_TEXT_WIDTH
    } else {
        0.
    };
    if overflow <= 1. {
        return div()
            .w_full()
            .flex()
            .justify_center()
            .child(
                div()
                    .max_w_full()
                    .text_size(px(TILE_TEXT_SIZE))
                    .truncate()
                    .child(title),
            )
            .into_any_element();
    }
    let scroll = overflow / MARQUEE_SPEED;
    let total = MARQUEE_REST + scroll + MARQUEE_REST;
    // Restarts whenever the tile is selected anew: the id is only there then.
    div()
        .w_full()
        .overflow_hidden()
        .child(
            div()
                .flex_none()
                .relative()
                .whitespace_nowrap()
                .text_size(px(TILE_TEXT_SIZE))
                .child(title)
                .with_animation(
                    ("marquee", ix),
                    Animation::new(Duration::from_secs_f32(total)).repeat(),
                    move |text, progress| {
                        let at = progress * total;
                        let moved = ((at - MARQUEE_REST) / scroll).clamp(0., 1.);
                        text.left(px(-overflow * moved))
                    },
                ),
        )
        .into_any_element()
}
