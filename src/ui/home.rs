//! Homepage with video recommendations in a grid layout with cover images

use super::icons;
use super::{Component, SearchPage, Theme, shortcut_footer};
use crate::api::client::ApiClient;
use crate::api::recommend::HomeFeed;
use crate::api::recommend::VideoItem;
use crate::application::AppAction;
use crate::storage::Keybindings;
use ratatui::{
    crossterm::event::{KeyCode, MouseButton, MouseEvent, MouseEventKind},
    prelude::*,
    widgets::*,
};
use ratatui_image::{Resize, StatefulImage, picker::Picker, protocol::StatefulProtocol};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;

/// Video card with cached cover image
pub struct VideoCard {
    pub video: VideoItem,
    pub cover: Option<StatefulProtocol>,
}

/// Message for completed cover download
pub struct CoverResult {
    pub index: usize,
    pub protocol: StatefulProtocol,
}

pub struct HomePage {
    videos: Vec<VideoCard>,
    selected_index: usize,
    loading: bool,
    error_message: Option<String>,
    scroll_row: usize,
    picker: Arc<Picker>,
    columns: usize,
    card_height: u16,
    // Async cover loading
    cover_tx: mpsc::Sender<CoverResult>,
    cover_rx: mpsc::Receiver<CoverResult>,
    pending_downloads: HashSet<usize>,
    fresh_idx: i32,
    loading_more: bool,
    cached_visible_rows: usize,
    footer_notice: Option<String>,
    // Double-click detection
    last_click_time: Option<Instant>,
    last_click_index: Option<usize>,
    feed: HomeFeed,
    pub search: SearchPage,
    pub focus_sources: bool,
    pub selected_source: usize,
}

impl HomePage {
    fn draw_sources(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        // Tabs column: light search-box style entry pinned on top, tab
        // items below. No logo here (the sidebar carries the branding).
        let block = Block::default()
            .style(Style::default().bg(theme.bg_secondary))
            .borders(Borders::LEFT)
            .border_type(BorderType::Plain)
            .border_style(Style::default().fg(theme.border_subtle));
        let outer = block.inner(area);
        frame.render_widget(block, area);

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // search box (light surface)
                Constraint::Length(1), // gap
                Constraint::Min(5),    // tabs
            ])
            .split(outer);

        // Search box: lighter surface + border, search glyph + label.
        let search_selected = self.selected_source == 0;
        let search_bg = if search_selected {
            theme.bg_highlight
        } else {
            theme.bg_modal
        };
        let search_block = Block::default()
            .style(Style::default().bg(search_bg))
            .borders(Borders::ALL)
            .border_type(BorderType::Plain)
            .border_style(Style::default().fg(if search_selected {
                theme.border_focused
            } else {
                search_bg
            }));
        let box_area = Rect {
            x: outer.x + 1,
            y: rows[0].y,
            width: outer.width.saturating_sub(2),
            height: 3,
        };
        frame.render_widget(search_block, box_area);
        let search_text = Paragraph::new(Line::from(vec![
            Span::styled(
                format!("{} ", icons::SEARCH),
                Style::default().fg(if search_selected {
                    theme.bilibili_pink
                } else {
                    theme.fg_muted
                }),
            ),
            Span::styled(
                "搜索",
                Style::default()
                    .fg(theme.fg_primary)
                    .add_modifier(if search_selected {
                        Modifier::BOLD
                    } else {
                        Modifier::empty()
                    }),
            ),
        ]));
        frame.render_widget(
            search_text,
            Rect {
                x: box_area.x + 1,
                y: box_area.y + 1,
                width: box_area.width.saturating_sub(2),
                height: 1,
            },
        );

        // Tabs (source 0 = the search box above).
        let tabs_area = rows[2];
        let count = self.source_count();
        let mut constraints: Vec<Constraint> = Vec::new();
        for _ in 1..count {
            constraints.push(Constraint::Length(3)); // taller selection block
        }
        constraints.push(Constraint::Min(0));
        let tabs = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(tabs_area);

        for (ti, tab_area) in tabs.iter().enumerate() {
            let index = ti + 1;
            if index >= count {
                break;
            }
            let is_selected = index == self.selected_source;
            // Selected tab fills the full 3-row block with its surface.
            if is_selected {
                frame.render_widget(
                    Block::default().style(Style::default().bg(theme.bg_card)),
                    Rect {
                        height: 3.min(tab_area.height),
                        ..*tab_area
                    },
                );
            }
            let label = self.source_label(index);
            let mut spans = Vec::new();
            if is_selected {
                spans.push(Span::styled(
                    "▌ ",
                    Style::default()
                        .fg(theme.bilibili_pink)
                        .add_modifier(Modifier::BOLD),
                ));
            } else {
                spans.push(Span::raw("   "));
            }
            spans.push(Span::styled(
                label,
                if is_selected {
                    Style::default()
                        .fg(theme.fg_primary)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.fg_secondary)
                },
            ));
            frame.render_widget(
                Paragraph::new(Line::from(spans)),
                Rect {
                    y: tab_area.y + 1, // middle row of the 3-row block
                    height: 1,
                    ..*tab_area
                },
            );
        }
    }

    fn draw_feed(&mut self, frame: &mut Frame, area: Rect, theme: &Theme, _keys: &Keybindings) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(10)])
            .split(area);
        if self.loading_more {
            let hint = Paragraph::new(Span::styled(" 加载中…", Style::default().fg(theme.warning)));
            frame.render_widget(hint, chunks[0]);
        }

        if self.loading {
            frame.render_widget(
                Paragraph::new("⏳ 加载中…")
                    .style(Style::default().fg(theme.warning))
                    .alignment(Alignment::Center),
                chunks[0],
            );
        } else if let Some(error) = &self.error_message {
            frame.render_widget(
                Paragraph::new(format!("{} {error}", icons::ERROR))
                    .style(Style::default().fg(theme.error))
                    .alignment(Alignment::Center),
                chunks[0],
            );
        } else if self.videos.is_empty() {
            frame.render_widget(
                Paragraph::new(format!("{} 暂无推荐视频", icons::INBOX))
                    .style(Style::default().fg(theme.fg_secondary))
                    .alignment(Alignment::Center),
                chunks[0],
            );
        } else {
            self.render_grid(frame, chunks[0], theme);
        }
    }
}

impl HomePage {
    /// Render the full-width, vertically centered footer with its own surface.
    fn draw_global_footer(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        theme: &Theme,
        keys: &crate::storage::Keybindings,
    ) {
        let notice = self.footer_notice.take();
        let mut help = shortcut_footer(
            theme,
            [
                ("↑/↓".into(), "选择视频".into(), theme.fg_accent),
                (
                    format!("{} / {}", keys.page_up, keys.page_down),
                    "翻页".into(),
                    theme.fg_accent,
                ),
                ("←/→".into(), "切换面板".into(), theme.fg_accent),
                ("[ ]".into(), "列数".into(), theme.fg_accent),
                ("u".into(), "UP主页".into(), theme.bilibili_blue),
                (keys.confirm.clone(), "播放".into(), theme.success),
                (keys.search_focus.clone(), "搜索".into(), theme.info),
                (keys.refresh.clone(), "刷新".into(), theme.info),
            ],
        );
        if let Some(notice) = notice {
            help.spans.push(Span::styled(
                format!("  {notice}"),
                Style::default().fg(theme.fg_secondary),
            ));
        }
        let footer = Paragraph::new(help).alignment(Alignment::Center).block(
            Block::default()
                .style(Style::default().bg(theme.bg_secondary))
                .padding(ratatui::widgets::Padding::new(0, 0, 1, 0)),
        );
        frame.render_widget(footer, area);
    }
}

impl HomePage {
    /// 默认列数
    const DEFAULT_COLUMNS: usize = 1;
    const COLUMN_CHOICES: [usize; 4] = [1, 2, 3, 4];
    /// 卡片高度
    const CARD_HEIGHT: u16 = 8;
    /// Non-cover rows of a vertical grid card: gap + title(2) + duration
    /// + up + stats + borders.
    const GRID_CARD_EXTRA: u16 = 8;
    /// 预取缓冲行数（可见区域之外额外下载）
    const PREFETCH_BUFFER_ROWS: usize = 2;
    /// 初始可见行数回退值（首次渲染前使用）
    const INITIAL_VISIBLE_ROWS: usize = 3;

    pub fn new() -> Self {
        // Try to detect terminal graphics protocol (Kitty/Sixel/iTerm2)
        // Fall back to halfblocks if detection fails
        let picker = super::image_picker::shared_picker();

        // Create channel for background image downloads
        let (cover_tx, cover_rx) = mpsc::channel(32);

        Self {
            videos: Vec::new(),
            selected_index: 0,
            loading: true,
            error_message: None,
            scroll_row: 0,
            picker,
            columns: Self::DEFAULT_COLUMNS,
            card_height: Self::CARD_HEIGHT,
            cover_tx,
            cover_rx,
            pending_downloads: HashSet::new(),
            fresh_idx: 1,
            loading_more: false,
            cached_visible_rows: Self::INITIAL_VISIBLE_ROWS,
            footer_notice: None,
            last_click_time: None,
            last_click_index: None,
            feed: HomeFeed::Recommended,
            search: SearchPage::new(),
            focus_sources: true,
            selected_source: 1,
        }
    }

    pub async fn load_recommendations(&mut self, api_client: &ApiClient) {
        self.loading = true;
        self.error_message = None;
        self.pending_downloads.clear();
        self.fresh_idx = 1;

        match api_client.get_recommendations().await {
            Ok(videos) => {
                self.videos = videos
                    .into_iter()
                    .map(|video| VideoCard { video, cover: None })
                    .collect();
                self.loading = false;
                self.selected_index = 0;
                self.scroll_row = 0;
            }
            Err(e) => {
                self.error_message = Some(format!("加载推荐视频失败: {}", e));
                self.loading = false;
            }
        }
    }

    pub fn begin_loading(&mut self) {
        self.loading = true;
        self.error_message = None;
        self.pending_downloads.clear();
        self.fresh_idx = 1;
    }

    pub fn apply_recommendations(&mut self, feed: HomeFeed, videos: Vec<VideoItem>) {
        if self.feed != feed {
            return;
        }
        self.videos = videos
            .into_iter()
            .map(|video| VideoCard { video, cover: None })
            .collect();
        self.loading = false;
        self.selected_index = 0;
        self.scroll_row = 0;
        self.error_message = None;
    }

    pub fn apply_recommendations_error(&mut self, msg: String) {
        self.error_message = Some(msg);
        self.loading = false;
    }

    pub fn begin_load_more(&mut self) -> Option<i32> {
        if self.loading_more || !matches!(self.feed, HomeFeed::Recommended | HomeFeed::Popular) {
            return None;
        }
        self.loading_more = true;
        self.fresh_idx += 1;
        Some(self.fresh_idx)
    }

    pub fn apply_load_more(&mut self, feed: HomeFeed, videos: Vec<VideoItem>) {
        if self.feed != feed {
            return;
        }
        for video in videos {
            self.videos.push(VideoCard { video, cover: None });
        }
        self.loading_more = false;
    }

    pub fn apply_load_more_error(&mut self) {
        self.fresh_idx -= 1;
        self.loading_more = false;
    }

    pub async fn load_more(&mut self, api_client: &ApiClient) {
        if self.loading_more {
            return;
        }

        self.loading_more = true;
        self.fresh_idx += 1;

        match api_client.get_recommendations_paged(self.fresh_idx).await {
            Ok(videos) => {
                for video in videos {
                    self.videos.push(VideoCard { video, cover: None });
                }
                self.loading_more = false;
            }
            Err(_) => {
                self.fresh_idx -= 1;
                self.loading_more = false;
            }
        }
    }

    pub fn is_near_bottom(&self, visible_rows: usize) -> bool {
        if self.videos.is_empty() {
            return false;
        }
        let current_row = self.selected_row();
        let total_rows = self.total_rows();
        let last_row = total_rows.saturating_sub(1);

        if total_rows <= visible_rows {
            // When all currently loaded rows fit in viewport, trigger load-more at the real bottom.
            current_row >= last_row
        } else {
            // Keep preloading behavior when content is taller than viewport.
            current_row + 2 >= last_row
        }
    }

    /// Start background downloads for visible covers (non-blocking)
    pub fn start_cover_downloads(&mut self) {
        if !self.videos.is_empty() {
            // Calculate visible range using current viewport rows + small buffer
            let start = self.scroll_row * self.columns;
            let prefetch_rows = self.cached_visible_rows + Self::PREFETCH_BUFFER_ROWS;
            let end = (start + self.columns * prefetch_rows).min(self.videos.len());

            for idx in start..end {
                // Skip if already has cover or is pending
                if self.videos[idx].cover.is_some() || self.pending_downloads.contains(&idx) {
                    continue;
                }

                if let Some(pic_url) = self.videos[idx].video.pic.clone() {
                    self.pending_downloads.insert(idx);
                    let tx = self.cover_tx.clone();
                    let picker = Arc::clone(&self.picker);

                    // Spawn background task
                    tokio::spawn(async move {
                        if let Some(img) = super::download_cover(&pic_url).await {
                            let protocol = picker.new_resize_protocol(img);
                            let _ = tx
                                .send(CoverResult {
                                    index: idx,
                                    protocol,
                                })
                                .await;
                        }
                    });
                }
            }
        }
        self.search.start_cover_downloads();
    }

    /// Poll for completed cover downloads (non-blocking)
    pub fn poll_cover_results(&mut self) {
        // Try to receive all available results without blocking
        while let Ok(result) = self.cover_rx.try_recv() {
            if result.index < self.videos.len() {
                self.videos[result.index].cover = Some(result.protocol);
                self.pending_downloads.remove(&result.index);
            }
        }
        self.search.poll_cover_results();
    }

    fn visible_rows(&self, height: u16, grid_width: u16) -> usize {
        let available_height = height.saturating_sub(1);
        (available_height / self.effective_card_height(grid_width)).max(1) as usize
    }

    /// Cover rows for a 16:9 image at the given card width (cells are
    /// ~1:2, so rows = width * 9/32), clamped for sanity.
    fn cover_height(&self, card_width: u16) -> u16 {
        ((card_width as u32 * 9 / 32) as u16).clamp(7, 12)
    }

    /// Horizontal list cards (1-2 cols) are short; vertical grid cards
    /// (3-4 cols) carry a 16:9 cover on top sized by the card width.
    fn effective_card_height(&self, grid_width: u16) -> u16 {
        if self.columns <= 2 {
            self.card_height
        } else {
            let card_width = (grid_width / self.columns as u16).max(8);
            self.cover_height(card_width) + Self::GRID_CARD_EXTRA
        }
    }

    fn selected_row(&self) -> usize {
        self.selected_index / self.columns
    }

    fn update_scroll(&mut self, visible_rows: usize) {
        let current_row = self.selected_row();
        if current_row < self.scroll_row {
            self.scroll_row = current_row;
        } else if current_row >= self.scroll_row + visible_rows {
            self.scroll_row = current_row - visible_rows + 1;
        }
    }

    fn move_page(&mut self, down: bool) -> bool {
        let Some(last_index) = self.videos.len().checked_sub(1) else {
            return false;
        };
        let page_size = self
            .cached_visible_rows
            .max(1)
            .saturating_mul(self.columns.max(1));
        let old_index = self.selected_index;
        self.selected_index = if down {
            old_index.saturating_add(page_size).min(last_index)
        } else {
            old_index.saturating_sub(page_size)
        };
        if self.selected_index == old_index {
            return false;
        }
        self.update_scroll(self.cached_visible_rows.max(1));
        true
    }

    fn total_rows(&self) -> usize {
        self.videos.len().div_ceil(self.columns)
    }

    pub fn set_footer_notice(&mut self, notice: String) {
        self.footer_notice = Some(notice);
    }

    pub fn feed(&self) -> HomeFeed {
        self.feed
    }

    pub fn search_mut(&mut self) -> &mut SearchPage {
        &mut self.search
    }

    pub fn search_ref(&self) -> &SearchPage {
        &self.search
    }

    pub fn begin_search(&mut self) {
        self.selected_source = 0;
        self.focus_sources = false;
        self.search.input_mode = true;
        self.search.show_hot_list = true;
    }

    pub fn select_source(&mut self, source: usize) {
        self.selected_source = source.min(HomeFeed::ALL.len());
        if self.selected_source == 0 {
            self.begin_search();
        }
    }

    pub fn source_count(&self) -> usize {
        HomeFeed::ALL.len() + 1
    }

    fn source_label(&self, index: usize) -> String {
        if index == 0 {
            format!("{} 搜索", icons::SEARCH).to_string()
        } else {
            HomeFeed::ALL[index - 1].label().to_string()
        }
    }

    fn source_feed(&self, index: usize) -> Option<HomeFeed> {
        (index > 0).then(|| HomeFeed::ALL[index - 1])
    }

    pub fn begin_feed_load(&mut self, feed: HomeFeed) {
        self.feed = feed;
        self.selected_source = HomeFeed::ALL
            .iter()
            .position(|candidate| *candidate == feed)
            .map(|index| index + 1)
            .unwrap_or(1);
        self.begin_loading();
        self.videos.clear();
        self.selected_index = 0;
        self.scroll_row = 0;
        self.loading_more = false;
    }
}

impl Default for HomePage {
    fn default() -> Self {
        Self::new()
    }
}

impl HomePage {
    /// Current grid column count (public for tests/deep-links).
    pub fn column_count(&self) -> usize {
        self.columns
    }

    /// Switch grid column count (1/2/3/4) in the given direction.
    pub fn cycle_columns(&mut self, direction: i32) {
        let cur = Self::COLUMN_CHOICES
            .iter()
            .position(|c| *c == self.columns)
            .unwrap_or(0);
        let len = Self::COLUMN_CHOICES.len();
        let next = if direction >= 0 {
            (cur + 1) % len
        } else {
            (cur + len - 1) % len
        };
        self.columns = Self::COLUMN_CHOICES[next];
        self.scroll_row = self.selected_index / self.columns.max(1);
        self.update_scroll(self.cached_visible_rows);
    }
}

impl Component for HomePage {
    fn draw(&mut self, frame: &mut Frame, area: Rect, theme: &Theme, keys: &Keybindings) {
        // Global footer row across the whole page width; panes live above it.
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(10), Constraint::Length(3)])
            .split(area);
        let panes = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(28), Constraint::Min(30)])
            .split(rows[0]);
        self.draw_sources(frame, panes[0], theme);

        if self.selected_source == 0 {
            self.search.draw(frame, panes[1], theme, keys);
        } else {
            self.draw_feed(frame, panes[1], theme, keys);
        }
        self.draw_global_footer(frame, rows[1], theme, keys);
    }

    fn handle_input(
        &mut self,
        key: KeyCode,
        keys: &crate::storage::Keybindings,
    ) -> Option<AppAction> {
        // Search input has priority: when the search box is focused, every
        // character (including `i` / `/` which are also `search_focus` shortcuts)
        // must be inserted as text, not as a shortcut.
        if self.selected_source == 0 && self.search.input_mode {
            return self.search.handle_input(key, keys);
        }
        if keys.matches_quit(key) {
            return Some(AppAction::Quit);
        }
        if keys.matches_search_focus(key) {
            self.begin_search();
            return Some(AppAction::None);
        }
        // Tab navigation always wins, including while either pane is loading.
        if keys.matches_nav_next(key) {
            return Some(AppAction::NavNext);
        }
        if keys.matches_nav_prev(key) {
            return Some(AppAction::NavPrev);
        }
        if self.focus_sources {
            if keys.matches_down(key) {
                self.selected_source = (self.selected_source + 1) % self.source_count();
            } else if keys.matches_up(key) {
                self.selected_source = if self.selected_source == 0 {
                    self.source_count() - 1
                } else {
                    self.selected_source - 1
                };
            } else if keys.matches_right(key) || keys.matches_confirm(key) {
                if self.selected_source == 0 {
                    self.begin_search();
                } else if let Some(feed) = self.source_feed(self.selected_source)
                    && feed != self.feed
                {
                    self.focus_sources = false;
                    return Some(AppAction::SwitchHomeFeed(feed));
                } else {
                    self.focus_sources = false;
                }
            }
            return Some(AppAction::None);
        }

        if keys.matches_left(key) {
            // Multi-column grids move the cursor one column first; only the
            // leftmost column hands focus back to the source list.
            let col = self.selected_index % self.columns;
            if col > 0 {
                self.selected_index -= 1;
                return Some(AppAction::None);
            }
            self.focus_sources = true;
            if self.selected_source == 0 || self.loading || self.loading_more {
                self.loading = false;
                self.loading_more = false;
                return Some(AppAction::CancelPendingLoads);
            }
            return Some(AppAction::None);
        }
        if keys.matches_right(key) {
            // moving right inside the grid; at the last column do nothing
            let col = self.selected_index % self.columns;
            let new_idx = self.selected_index + 1;
            if col + 1 < self.columns && new_idx < self.videos.len() {
                self.selected_index = new_idx;
            }
            return Some(AppAction::None);
        }
        if self.selected_source == 0 {
            return self.search.handle_input(key, keys);
        }
        if key == KeyCode::Char('[') {
            self.cycle_columns(-1);
            return Some(AppAction::None);
        }
        if key == KeyCode::Char(']') {
            self.cycle_columns(1);
            return Some(AppAction::None);
        }
        if self.loading {
            return Some(AppAction::None);
        }
        if keys.matches_page_down(key) {
            self.move_page(true);
            if self.is_near_bottom(self.cached_visible_rows) && !self.loading_more {
                return Some(AppAction::LoadMoreRecommendations);
            }
            return Some(AppAction::None);
        }
        if keys.matches_page_up(key) {
            self.move_page(false);
            return Some(AppAction::None);
        }
        if keys.matches_down(key) {
            if !self.videos.is_empty() {
                let new_idx = self.selected_index + self.columns;
                if new_idx < self.videos.len() {
                    self.selected_index = new_idx;
                }
                self.update_scroll(self.cached_visible_rows);
                if self.is_near_bottom(self.cached_visible_rows) && !self.loading_more {
                    return Some(AppAction::LoadMoreRecommendations);
                }
            }
            return Some(AppAction::None);
        }
        if keys.matches_up(key) {
            if !self.videos.is_empty() && self.selected_index >= self.columns {
                self.selected_index -= self.columns;
                self.update_scroll(self.cached_visible_rows);
            }
            return Some(AppAction::None);
        }
        if key == KeyCode::Char('u')
            && let Some(mid) = self
                .videos
                .get(self.selected_index)
                .and_then(|card| card.video.owner.as_ref())
                .map(|owner| owner.mid)
        {
            return Some(AppAction::OpenUpPage(mid));
        }
        if keys.matches_confirm(key) || keys.matches_play(key) {
            if let Some(card) = self.videos.get(self.selected_index)
                && let Some(bvid) = &card.video.bvid
            {
                let aid = card.video.id;
                return Some(AppAction::OpenVideoDetail(bvid.clone(), aid));
            }
            return Some(AppAction::None);
        }
        if keys.matches_refresh(key) {
            self.loading = true;
            self.videos.clear();
            self.pending_downloads.clear();
            return Some(AppAction::RefreshHome);
        }
        if keys.matches_next_theme(key) {
            return Some(AppAction::NextTheme);
        }
        if keys.matches_open_settings(key) {
            return Some(AppAction::SwitchToSettings);
        }
        Some(AppAction::None)
    }

    fn handle_mouse(&mut self, event: MouseEvent, area: Rect) -> Option<AppAction> {
        let panes = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(28), Constraint::Min(30)])
            .split(area);
        let position = ratatui::layout::Position::new(event.column, event.row);

        if panes[0].contains(position) {
            self.focus_sources = true;
            match event.kind {
                MouseEventKind::ScrollDown => {
                    self.selected_source = (self.selected_source + 1) % self.source_count();
                }
                MouseEventKind::ScrollUp => {
                    self.selected_source = if self.selected_source == 0 {
                        self.source_count() - 1
                    } else {
                        self.selected_source - 1
                    };
                }
                MouseEventKind::Down(MouseButton::Left) => {
                    let row = event.row.saturating_sub(panes[0].y + 1) as usize;
                    if row < self.source_count() {
                        self.selected_source = row;
                        if row == 0 {
                            self.begin_search();
                        } else if let Some(feed) = self.source_feed(row)
                            && feed != self.feed
                        {
                            return Some(AppAction::SwitchHomeFeed(feed));
                        }
                    }
                }
                _ => {}
            }
            return Some(AppAction::None);
        }

        if !panes[1].contains(position) {
            return None;
        }
        self.focus_sources = false;
        if self.selected_source == 0 {
            return self.search.handle_mouse(event, panes[1]);
        }

        match event.kind {
            MouseEventKind::ScrollDown => {
                // Scroll down by one row
                if !self.videos.is_empty() {
                    let new_idx = self.selected_index + self.columns;
                    if new_idx < self.videos.len() {
                        self.selected_index = new_idx;
                        self.update_scroll(self.cached_visible_rows);
                        // Check for pagination only when actually moved
                        if self.is_near_bottom(self.cached_visible_rows) && !self.loading_more {
                            return Some(AppAction::LoadMoreRecommendations);
                        }
                    }
                }
                None
            }
            MouseEventKind::ScrollUp => {
                // Scroll up by one row
                if !self.videos.is_empty() && self.selected_index >= self.columns {
                    self.selected_index -= self.columns;
                    self.update_scroll(self.cached_visible_rows);
                }
                None
            }
            MouseEventKind::Down(MouseButton::Left) => {
                let content_top = panes[1].y + 3;
                let content_bottom = panes[1].bottom().saturating_sub(2);

                if event.row >= content_top && event.row < content_bottom {
                    // Calculate which card was clicked
                    let relative_y = event.row - content_top;
                    let click_row =
                        (relative_y / self.effective_card_height(panes[1].width)) as usize;
                    let actual_row = self.scroll_row + click_row;

                    let card_width = panes[1].width / self.columns as u16;
                    let click_col = (event.column.saturating_sub(panes[1].x) / card_width) as usize;

                    let click_idx = actual_row * self.columns + click_col.min(self.columns - 1);

                    if click_idx < self.videos.len() {
                        // Check for double-click (same card within 500ms)
                        let now = Instant::now();
                        let is_double_click = self.last_click_index == Some(click_idx)
                            && self
                                .last_click_time
                                .is_some_and(|t| now.duration_since(t).as_millis() < 500);

                        if is_double_click {
                            // Double-click: open video detail
                            self.last_click_time = None;
                            self.last_click_index = None;
                            if let Some(card) = self.videos.get(click_idx)
                                && let Some(bvid) = &card.video.bvid
                            {
                                let aid = card.video.id;
                                return Some(AppAction::OpenVideoDetail(bvid.clone(), aid));
                            }
                        } else {
                            // Single click: select card and record for potential double-click
                            self.selected_index = click_idx;
                            self.update_scroll(self.cached_visible_rows);
                            self.last_click_time = Some(now);
                            self.last_click_index = Some(click_idx);
                        }
                    }
                }
                None
            }
            MouseEventKind::Down(MouseButton::Middle) => {
                // Middle click opens video detail
                if let Some(card) = self.videos.get(self.selected_index)
                    && let Some(bvid) = &card.video.bvid
                {
                    let aid = card.video.id;
                    return Some(AppAction::OpenVideoDetail(bvid.clone(), aid));
                }
                None
            }
            _ => None,
        }
    }
}

impl HomePage {
    fn render_grid(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let visible_rows = self.visible_rows(area.height, area.width);
        self.cached_visible_rows = visible_rows;

        // Fixed-height rows plus a spacer so leftover space never
        // stretches the last card row (Flex::Legacy quirk).
        let mut row_constraints: Vec<Constraint> = (0..visible_rows)
            .map(|_| Constraint::Length(self.effective_card_height(area.width)))
            .collect();
        row_constraints.push(Constraint::Min(0));

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints(row_constraints)
            .split(area);

        // Collect all card areas first
        let mut card_areas: Vec<(usize, Rect)> = Vec::new();

        for (row_offset, row_area) in rows.iter().enumerate() {
            let actual_row = self.scroll_row + row_offset;
            let start_idx = actual_row * self.columns;

            if start_idx >= self.videos.len() {
                break;
            }

            let col_constraints: Vec<Constraint> = (0..self.columns)
                .map(|_| Constraint::Ratio(1, self.columns as u32))
                .collect();

            let cols = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(col_constraints)
                .split(*row_area);

            for (col_idx, col_area) in cols.iter().enumerate() {
                let video_idx = start_idx + col_idx;
                if video_idx >= self.videos.len() {
                    break;
                }
                card_areas.push((video_idx, *col_area));
            }
        }

        // Now render each card with mutable access
        for (video_idx, col_area) in card_areas {
            let is_selected = video_idx == self.selected_index;
            self.render_video_card(frame, col_area, video_idx, is_selected, theme);
        }
    }

    fn render_video_card(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        video_idx: usize,
        is_selected: bool,
        theme: &Theme,
    ) {
        let card_bg = if is_selected {
            theme.bg_highlight
        } else {
            theme.bg_card
        };
        let block = Block::default()
            .style(Style::default().bg(card_bg))
            .borders(Borders::ALL)
            .border_type(BorderType::Plain)
            .border_style(Style::default().fg(if is_selected {
                theme.border_focused
            } else {
                theme.bg_card
            }));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // 3-4 columns: vertical web-style card (cover on top);
        // 1-2 columns: horizontal list card (cover left, info right)
        let vertical = self.columns >= 3;
        let cover_h = if vertical {
            self.cover_height(inner.width)
        } else {
            7
        };
        let card_chunks = if vertical {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(cover_h), // ~16:9 cover (full width)
                    Constraint::Length(1),       // breathing room between cover and text
                    Constraint::Min(5),          // text block
                ])
                .split(inner)
        } else {
            Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Length(24), Constraint::Min(20)])
                .split(inner)
        };

        // Cover fills its container via center-crop (uniform aspect).
        let cover_area = card_chunks[0];
        if let Some(cover) = &mut self.videos[video_idx].cover {
            let image_widget = StatefulImage::default().resize(Resize::Crop(None));
            frame.render_stateful_widget(image_widget, cover_area, cover);
        } else {
            let is_pending = self.pending_downloads.contains(&video_idx);
            let placeholder_text = if is_pending {
                format!("{} 加载中...", icons::TV)
            } else {
                icons::TV.to_string()
            };
            let placeholder = Paragraph::new(placeholder_text)
                .style(Style::default().fg(theme.fg_secondary))
                .alignment(Alignment::Center);
            frame.render_widget(placeholder, cover_area);
        }

        let info_area = if vertical {
            card_chunks[2]
        } else {
            card_chunks[1]
        };
        let card = &self.videos[video_idx];

        let title = card.video.title.as_deref().unwrap_or("无标题");
        let author = card.video.author_name();
        let views = card.video.format_views();
        let duration = card.video.format_duration();

        let title_width = (info_area.width as usize).saturating_sub(1);
        let title_lines: Vec<String> = Self::wrap_title(title, title_width.max(8), 2);

        let meta_style = Style::default().fg(theme.fg_secondary);

        let danmaku = card
            .video
            .stat
            .as_ref()
            .and_then(|stat| stat.danmaku)
            .map(format_count)
            .unwrap_or_else(|| "-".to_string());
        // The recommendation API omits reply counts; hide that segment then.
        let replies = card
            .video
            .stat
            .as_ref()
            .and_then(|stat| stat.reply)
            .map(format_count);

        // Stats row: view / danmaku / reply, each with an icon.
        let mut stats_spans = vec![Span::styled(
            format!("{} {}", icons::PLAY, views),
            meta_style,
        )];
        stats_spans.push(Span::styled(
            format!("  {} {}", icons::DANMAKU, danmaku),
            meta_style,
        ));
        if let Some(replies) = replies {
            stats_spans.push(Span::styled(
                format!("  {} {}", icons::COMMENT, replies),
                meta_style,
            ));
        }
        let up_row = Line::from(vec![
            Span::styled("UP ", meta_style),
            Span::styled(author, Style::default().fg(theme.bilibili_cyan)),
        ]);
        let stats_row = Line::from(stats_spans);
        let duration_row = Line::from(Span::styled(duration, Style::default().fg(theme.success)));

        // Top block: title then duration right below it; bottom rows: UP
        // line above the stats line, glued to the card bottom.
        let rows = vec![up_row, stats_row];
        let title_block: Vec<Line> = title_lines
            .iter()
            .take(2)
            .map(|t| {
                Line::from(Span::styled(
                    t,
                    if is_selected {
                        Style::default()
                            .fg(theme.fg_primary)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(theme.fg_secondary)
                    },
                ))
            })
            .collect();

        let info_lines = info_area.height as usize;
        let mut lines: Vec<Line> = Vec::new();
        let bottom_count = rows.len();
        let top_cap = info_lines.saturating_sub(bottom_count);
        let title_count = title_block.len().min(top_cap);
        lines.extend(title_block.into_iter().take(title_count));
        if title_count < top_cap {
            lines.push(duration_row);
        }
        for _ in 0..info_lines
            .saturating_sub(title_count + bottom_count)
            .saturating_sub(1)
        {
            lines.push(Line::raw(""));
        }
        lines.extend(rows);

        let info = Paragraph::new(lines);
        frame.render_widget(info, info_area);
    }
    fn wrap_title(text: &str, width: usize, max_lines: usize) -> Vec<String> {
        wrap_text_chars(text, width, max_lines)
    }
}

/// Character-based greedy wrap with a hard line cap (CJK-friendly).
fn wrap_text_chars(text: &str, width: usize, max_lines: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut current_len = 0usize;
    for ch in text.chars() {
        if current_len + 1 > width {
            lines.push(std::mem::take(&mut current));
            current_len = 0;
            if lines.len() == max_lines {
                let mut last = lines.pop().unwrap_or_default();
                if last.chars().count() + 1 > width {
                    last.pop();
                }
                last.push('…');
                lines.push(last);
                return lines;
            }
        }
        current.push(ch);
        current_len += 1;
    }
    if !current.is_empty() || lines.is_empty() {
        lines.push(current);
    }
    lines
}

fn format_count(value: i64) -> String {
    if value >= 10_000 {
        format!("{:.1}万", value as f64 / 10_000.0)
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycle_columns_steps_through_layouts() {
        let mut page = HomePage::new();
        assert_eq!(page.columns, 1);
        page.cycle_columns(1);
        assert_eq!(page.columns, 2);
        page.cycle_columns(1);
        assert_eq!(page.columns, 3);
        page.cycle_columns(1);
        assert_eq!(page.columns, 4);
        page.cycle_columns(1);
        assert_eq!(page.columns, 1, "wraps around to single column");
        page.cycle_columns(-1);
        assert_eq!(page.columns, 4, "wraps backwards");
    }

    #[test]
    fn bracket_keys_cycle_columns_via_input() {
        let mut page = HomePage::new();
        page.focus_sources = false; // grid must have focus
        page.selected_source = 1; // leave the search pane (source 0)
        let keys = Keybindings::default();
        page.handle_input(KeyCode::Char(']'), &keys);
        assert_eq!(page.columns, 2);
        page.handle_input(KeyCode::Char('['), &keys);
        assert_eq!(page.columns, 1);
    }

    #[test]
    fn home_pane_switch_preempts_loading() {
        let mut page = HomePage::new();
        page.focus_sources = false;
        page.loading = true;
        let keys = Keybindings::default();
        assert!(matches!(
            page.handle_input(KeyCode::Left, &keys),
            Some(AppAction::CancelPendingLoads)
        ));
        assert!(page.focus_sources);
    }

    #[test]
    fn home_is_a_single_column_list() {
        let page = HomePage::new();
        assert_eq!(page.columns, 1);
    }

    #[test]
    fn home_contains_search_source() {
        let mut page = HomePage::new();
        assert_eq!(page.source_count(), HomeFeed::ALL.len() + 1);
        assert_eq!(page.feed(), HomeFeed::Recommended);
        page.begin_search();
        assert_eq!(page.selected_source, 0);
        assert!(page.search.input_mode);
    }
}
