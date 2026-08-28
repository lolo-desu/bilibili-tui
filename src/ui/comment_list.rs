//! Web-style comment list shared by video/dynamic/article detail pages.
//!
//! Layout mimics the bilibili web GUI:
//!
//! - avatar on the left; user name + level + relative time on the header row
//! - multi-line wrapped message
//! - action row with like button + count, reply count, and a fold toggle when
//!   a comment has replies
//!
//! Selection moves per-entry (comment / reply / toggle row, not per-line); the
//! list auto-scrolls to keep the selection visible. Avatars are downloaded in
//! the background and rendered as terminal images.

use super::Theme;
use super::icons;
use super::image_picker::{picker_supports_images, shared_picker};
use crate::api::comment::CommentItem;
use image::DynamicImage;
use ratatui::prelude::*;
use ratatui::widgets::*;
use ratatui_image::{picker::Picker, protocol::StatefulProtocol};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::mpsc;
use unicode_width::UnicodeWidthStr;

/// One flattened, selectable entry in the comment list.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EntryKind {
    /// Top-level comment card.
    Comment,
    /// Reply row inside an expanded comment.
    Reply,
    /// "展开/收起回复" or "加载更多回复" toggle row.
    Toggle,
}

#[derive(Debug, Clone, Copy)]
pub struct Entry {
    pub kind: EntryKind,
    /// Index into `comments`.
    pub comment_index: usize,
    /// Index into the expanded comment's reply list (Reply entries only).
    pub reply_index: usize,
    /// First render line of this entry (absolute, including scrolled-out).
    pub start_line: usize,
    /// Rendered height in rows, including the trailing blank separator.
    pub height: u16,
}

impl Entry {
    pub fn end_line(&self) -> usize {
        self.start_line + self.height as usize
    }
}

/// User intent produced by interacting with the list; pages map these to
/// `AppAction`s (they own oid / comment_type context).
#[derive(Debug, Clone, Copy)]
pub enum CommentIntent {
    /// Expand or collapse replies of the selected top-level comment.
    ToggleReplies { comment_index: usize },
    /// Fetch the next page of replies for the expanded comment.
    LoadMoreReplies { comment_index: usize },
    /// Like/unlike the selected comment or reply.
    Like {
        comment_index: usize,
        reply_index: Option<usize>,
    },
    /// Selection is close to the end and more top-level comments exist.
    LoadMoreComments,
}

/// Avatar download result message.
pub struct AvatarResult {
    pub index: usize,
    pub protocol: StatefulProtocol,
}

/// Async avatar loader: downloads avatar images in the background and keeps
/// one rendered protocol per comment (index-aligned with `comments`).
///
/// The terminal picker is created lazily on first use — `Picker::from_query_stdio`
/// performs terminal capability queries that must never run at page-construction
/// time (it blocks non-TTY test environments).
pub struct AvatarLoader {
    pub protocols: Vec<Option<StatefulProtocol>>,
    pending: HashSet<usize>,
    tx: mpsc::Sender<AvatarResult>,
    rx: mpsc::Receiver<AvatarResult>,
    picker: Option<Arc<Picker>>,
    supports_images: bool,
}

impl AvatarLoader {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(64);
        Self {
            protocols: Vec::new(),
            pending: HashSet::new(),
            tx,
            rx,
            picker: None,
            supports_images: false,
        }
    }

    fn ensure_picker(&mut self) -> Option<Arc<Picker>> {
        if self.picker.is_none() {
            if picker_supports_images() {
                self.supports_images = true;
                self.picker = Some(shared_picker());
            } else {
                // No TTY: prefer the USER glyph placeholder over halfblocks.
                self.supports_images = false;
            }
        }
        self.picker.clone()
    }

    /// Whether the terminal supports image rendering (queried lazily once).
    pub fn supports_images(&mut self) -> bool {
        if self.protocols.iter().any(|p| p.is_some()) {
            return true;
        }
        self.ensure_picker().is_some()
    }

    /// Sync list size after comments change (keeps index alignment).
    pub fn sync_len(&mut self, len: usize) {
        while self.protocols.len() > len {
            self.protocols.pop();
        }
        while self.protocols.len() < len {
            self.protocols.push(None);
        }
    }

    pub fn get(&self, index: usize) -> Option<&StatefulProtocol> {
        self.protocols.get(index).and_then(|p| p.as_ref())
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut StatefulProtocol> {
        self.protocols.get_mut(index).and_then(|p| p.as_mut())
    }

    fn is_loaded_or_pending(&self, index: usize) -> bool {
        self.pending.contains(&index)
            || self
                .protocols
                .get(index)
                .map(|p| p.is_some())
                .unwrap_or(true)
    }

    /// Request downloads for the given comment indices.
    pub fn request(&mut self, indices: impl IntoIterator<Item = usize>, urls: &[Option<String>]) {
        let Some(picker) = self.ensure_picker() else {
            return;
        };
        for idx in indices {
            if self.is_loaded_or_pending(idx) {
                continue;
            }
            let Some(url) = urls.get(idx).and_then(|u| u.as_ref()) else {
                continue;
            };
            self.pending.insert(idx);
            let tx = self.tx.clone();
            let picker = Arc::clone(&picker);
            let url = normalize_avatar_url(url);
            tokio::spawn(async move {
                if let Some(img) = download_image(&url).await {
                    let protocol = picker.new_resize_protocol(img);
                    let _ = tx
                        .send(AvatarResult {
                            index: idx,
                            protocol,
                        })
                        .await;
                }
            });
        }
    }

    /// Drain finished downloads; returns true if anything new arrived.
    pub fn poll(&mut self) -> bool {
        let mut updated = false;
        while let Ok(result) = self.rx.try_recv() {
            self.pending.remove(&result.index);
            if let Some(slot) = self.protocols.get_mut(result.index) {
                *slot = Some(result.protocol);
                updated = true;
            }
        }
        updated
    }
}

impl Default for AvatarLoader {
    fn default() -> Self {
        Self::new()
    }
}

fn normalize_avatar_url(url: &str) -> String {
    url.replacen("http://", "https://", 1)
}

async fn download_image(url: &str) -> Option<DynamicImage> {
    let response = reqwest::get(url).await.ok()?;
    let bytes = response.bytes().await.ok()?;
    image::load_from_memory(&bytes).ok()
}

/// Level badge color, mirroring bilibili's web palette.
fn level_color(level: i32, theme: &Theme) -> Color {
    match level {
        0 | 1 => theme.fg_muted,
        2 => Color::Rgb(195, 178, 103), // green-ish
        3 => Color::Rgb(97, 168, 234),  // blue
        4 => Color::Rgb(240, 147, 65),  // orange
        5 => Color::Rgb(255, 183, 42),  // yellow
        _ => Color::Rgb(255, 107, 148), // pink/red (Lv6+)
    }
}

const AVATAR_COLS: u16 = 3; // avatar cell width, in terminal columns
const AVATAR_ROWS: u16 = 2; // avatar cell height (≈square in cells)

/// Web-style comment list widget + state.
pub struct CommentList {
    /// Top-level comments (hot + recent, in API order).
    pub comments: Vec<CommentItem>,
    /// Fetched replies keyed by root comment rpid.
    pub replies: HashMap<i64, Vec<CommentItem>>,
    /// Root rpids currently expanded.
    pub expanded: HashSet<i64>,
    /// Root rpid whose replies are being fetched.
    pub loading_replies_for: Option<i64>,
    /// True when the reply loader should show a spinner row.
    pub loading_more_replies: bool,
    /// More top-level comments available on the server.
    pub has_more: bool,
    /// A page of top-level comments is being fetched.
    pub loading_more: bool,
    /// Locally-liked rpids (toggled until refresh).
    pub liked: HashSet<i64>,
    /// Local like-count deltas applied on top of API counts.
    pub like_deltas: HashMap<i64, i64>,
    /// Comment currently selected (top-level index).
    pub selected: usize,
    /// Selected entry index into `entries` (comment / reply / toggle row).
    pub selected_entry: usize,
    /// First visible render line.
    pub scroll: usize,

    pub avatars: AvatarLoader,

    /// Flattened layout cache from the last draw.
    entries: Vec<Entry>,
    total_lines: usize,
    last_width: u16,
}

impl CommentList {
    pub fn new() -> Self {
        Self {
            comments: Vec::new(),
            replies: HashMap::new(),
            expanded: HashSet::new(),
            loading_replies_for: None,
            loading_more_replies: false,
            has_more: false,
            loading_more: false,
            liked: HashSet::new(),
            like_deltas: HashMap::new(),
            selected: 0,
            selected_entry: 0,
            scroll: 0,
            avatars: AvatarLoader::new(),
            entries: Vec::new(),
            total_lines: 0,
            last_width: 0,
        }
    }

    /// Replace all comments (initial load / refresh).
    pub fn set_comments(&mut self, comments: Vec<CommentItem>, total_count: i64) {
        self.comments = comments;
        self.replies.clear();
        self.expanded.clear();
        self.loading_replies_for = None;
        self.selected = self.selected.min(self.comments.len().saturating_sub(1));
        self.scroll = 0;
        self.has_more = total_count > self.comments.len() as i64;
        self.avatars.sync_len(self.comments.len());
        self.entries.clear();
    }

    /// Append a page of comments (pagination).
    pub fn append_comments(&mut self, comments: Vec<CommentItem>) {
        self.comments.extend(comments);
        self.avatars.sync_len(self.comments.len());
        self.entries.clear();
    }

    /// Store fetched replies for a root comment and expand it.
    pub fn set_replies(&mut self, root_rpid: i64, replies: Vec<CommentItem>) {
        self.replies.insert(root_rpid, replies);
        self.expanded.insert(root_rpid);
        self.loading_replies_for = None;
        self.loading_more_replies = false;
        self.entries.clear();
    }

    /// Collapse (or mark loading) replies for a root comment.
    pub fn collapse(&mut self, root_rpid: i64) {
        self.expanded.remove(&root_rpid);
        self.loading_replies_for = None;
        self.entries.clear();
    }

    pub fn set_loading_replies(&mut self, root_rpid: i64) {
        self.expanded.insert(root_rpid);
        self.loading_replies_for = Some(root_rpid);
        self.entries.clear();
    }

    pub fn reply_failed(&mut self, root_rpid: i64) {
        if self.loading_replies_for == Some(root_rpid) {
            self.loading_replies_for = None;
        }
        self.loading_more_replies = false;
        self.entries.clear();
    }

    /// Apply a like toggle locally (count +1 / -1).
    pub fn apply_like(&mut self, rpid: i64) {
        if self.liked.remove(&rpid) {
            *self.like_deltas.entry(rpid).or_insert(0) -= 1;
        } else {
            self.liked.insert(rpid);
            *self.like_deltas.entry(rpid).or_insert(0) += 1;
        }
    }

    /// Optimistically set like state without toggling (e.g. after API call).
    pub fn set_liked(&mut self, rpid: i64, liked: bool) {
        if liked && self.liked.insert(rpid) {
            *self.like_deltas.entry(rpid).or_insert(0) += 1;
        } else if !liked && self.liked.remove(&rpid) {
            *self.like_deltas.entry(rpid).or_insert(0) -= 1;
        }
    }

    fn like_count(&self, comment: &CommentItem) -> i64 {
        comment.like.unwrap_or(0) as i64 + self.like_deltas.get(&comment.rpid).copied().unwrap_or(0)
    }

    fn is_liked(&self, rpid: i64) -> bool {
        self.liked.contains(&rpid)
    }

    /// Reset to the first comment (e.g. after refresh).
    pub fn reset_selection(&mut self) {
        self.selected_entry = 0;
        self.selected = 0;
        self.scroll = 0;
    }

    /// The selected top-level comment.
    pub fn selected_comment(&self) -> Option<&CommentItem> {
        self.comments.get(self.selected)
    }

    fn avatar_urls(&self) -> Vec<Option<String>> {
        self.comments
            .iter()
            .map(|c| c.member.as_ref().and_then(|m| m.avatar.clone()))
            .collect()
    }

    // ------------------------------------------------------------------
    // Layout
    // ------------------------------------------------------------------

    /// Flatten comments/replies into renderable entries for `width` columns.
    fn build_entries(&mut self, width: u16) {
        let mut entries = Vec::new();
        let mut line = 0usize;
        let content_width = width.saturating_sub(AVATAR_COLS + 1).max(8) as usize;

        for (ci, comment) in self.comments.iter().enumerate() {
            let msg_lines = wrap_lines(comment.message(), content_width).len().max(1);
            let card_height = 1 /* header */ + msg_lines + 1 /* actions */ + 1 /* blank */;
            entries.push(Entry {
                kind: EntryKind::Comment,
                comment_index: ci,
                reply_index: 0,
                start_line: line,
                height: card_height as u16,
            });
            line += card_height;

            let is_expanded = self.expanded.contains(&comment.rpid);
            if is_expanded {
                if let Some(replies) = self.replies.get(&comment.rpid) {
                    for (ri, reply) in replies.iter().enumerate() {
                        let reply_msg_lines =
                            wrap_lines(reply.message(), content_width).len().max(1);
                        let height = 1 + reply_msg_lines + 1 + 1; // header+msg+actions+blank
                        entries.push(Entry {
                            kind: EntryKind::Reply,
                            comment_index: ci,
                            reply_index: ri,
                            start_line: line,
                            height: height as u16,
                        });
                        line += height;
                    }
                    // "load more replies" toggle when more exist on server
                    if comment.reply_count() as usize > replies.len() {
                        entries.push(Entry {
                            kind: EntryKind::Toggle,
                            comment_index: ci,
                            reply_index: 1,
                            start_line: line,
                            height: 2,
                        });
                        line += 2;
                    }
                } else if self.loading_replies_for == Some(comment.rpid) {
                    entries.push(Entry {
                        kind: EntryKind::Toggle,
                        comment_index: ci,
                        reply_index: 0,
                        start_line: line,
                        height: 1,
                    });
                    line += 1;
                }
                // collapse row
                entries.push(Entry {
                    kind: EntryKind::Toggle,
                    comment_index: ci,
                    reply_index: 0,
                    start_line: line,
                    height: 2,
                });
                line += 2;
            } else if comment.reply_count() > 0 {
                // single-row preview toggle
                entries.push(Entry {
                    kind: EntryKind::Toggle,
                    comment_index: ci,
                    reply_index: 0,
                    start_line: line,
                    height: 1,
                });
                line += 1;
            }
        }

        self.entries = entries;
        self.total_lines = line;
        self.last_width = width;
    }

    fn clamp_scroll(&mut self, viewport: usize) {
        let viewport = viewport.max(1);
        if let Some(entry) = self.entries.get(self.visible_entry_index()) {
            // keep selected entry fully visible
            let sel_start = entry.start_line;
            let sel_end = entry.end_line().saturating_sub(1);
            if sel_start < self.scroll {
                self.scroll = sel_start;
            } else if sel_end >= self.scroll + viewport {
                self.scroll = sel_end + 1 - viewport;
            }
        }
        let max_scroll = self.total_lines.saturating_sub(viewport);
        self.scroll = self.scroll.min(max_scroll);
    }

    /// Index into `entries` of the currently selected entry.
    fn visible_entry_index(&self) -> usize {
        self.selected_entry
    }

    /// Move selection up; returns false when already at the top.
    pub fn move_up(&mut self) -> bool {
        if self.selected_entry == 0 || self.entries.is_empty() {
            return false;
        }
        self.selected_entry -= 1;
        self.sync_selected_comment();
        true
    }

    /// Move selection down; returns intents (load-more) when nearing bottom.
    pub fn move_down(&mut self) -> Option<CommentIntent> {
        if self.entries.is_empty() {
            return None;
        }
        if self.selected_entry + 1 < self.entries.len() {
            self.selected_entry += 1;
            self.sync_selected_comment();
            // near bottom: request more comments
            if self.selected_entry + 2 >= self.entries.len() && self.has_more && !self.loading_more
            {
                return Some(CommentIntent::LoadMoreComments);
            }
        }
        None
    }

    /// Keep `selected` (comment index) in sync with entry navigation.
    fn sync_selected_comment(&mut self) {
        if let Some(entry) = self.entries.get(self.selected_entry) {
            self.selected = entry.comment_index;
        }
    }

    /// Selected entry info.
    pub fn selected_entry_info(&self) -> Option<Entry> {
        self.entries.get(self.selected_entry).copied()
    }

    /// Activate the selected entry (Enter / click on toggle).
    pub fn activate_selected(&self) -> Option<CommentIntent> {
        let entry = self.entries.get(self.selected_entry)?;
        match entry.kind {
            EntryKind::Comment => Some(CommentIntent::Like {
                comment_index: entry.comment_index,
                reply_index: None,
            }),
            EntryKind::Reply => Some(CommentIntent::Like {
                comment_index: entry.comment_index,
                reply_index: Some(entry.reply_index),
            }),
            EntryKind::Toggle => {
                let comment = self.comments.get(entry.comment_index)?;
                if self.expanded.contains(&comment.rpid) && entry.reply_index == 0 {
                    Some(CommentIntent::ToggleReplies {
                        comment_index: entry.comment_index,
                    })
                } else if entry.reply_index == 1 {
                    Some(CommentIntent::LoadMoreReplies {
                        comment_index: entry.comment_index,
                    })
                } else {
                    Some(CommentIntent::ToggleReplies {
                        comment_index: entry.comment_index,
                    })
                }
            }
        }
    }

    /// Map a click at absolute `row` to entry selection; returns the entry.
    pub fn click_at(&mut self, row_in_list: usize) -> Option<Entry> {
        let entry = *self
            .entries
            .iter()
            .find(|e| row_in_list >= e.start_line && row_in_list < e.end_line())?;
        self.selected_entry = self
            .entries
            .iter()
            .position(|e| e.start_line == entry.start_line && e.kind == entry.kind)
            .unwrap_or(self.selected_entry);
        self.sync_selected_comment();
        Some(entry)
    }

    // ------------------------------------------------------------------
    // Rendering
    // ------------------------------------------------------------------

    /// Render the comment list inside `area` (excluding block borders).
    pub fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme, _focused: bool) {
        if area.width < 10 || area.height == 0 {
            return;
        }
        if self.comments.is_empty() {
            let msg = if self.loading_more {
                "加载评论中..."
            } else {
                "暂无评论，快来抢沙发"
            };
            frame.render_widget(
                Paragraph::new(msg)
                    .style(Style::default().fg(theme.fg_muted))
                    .alignment(Alignment::Center),
                area,
            );
            return;
        }

        // Rebuild layout when stale, clamping selection into range.
        self.build_entries(area.width);
        if self.selected_entry >= self.entries.len() {
            self.selected_entry = self.entries.len().saturating_sub(1);
        }
        self.selected = self.selected.min(self.comments.len().saturating_sub(1));

        let viewport = area.height as usize;
        self.clamp_scroll(viewport);

        // avatar prefetch for visible comments
        let visible_comments = self.visible_comment_indices(viewport);
        let urls = self.avatar_urls();
        self.avatars
            .request(visible_comments.iter().copied(), &urls);
        let avatars_updated = self.avatars.poll();
        let _ = avatars_updated;

        // background layer per line: selection highlight
        for (i, entry) in self.entries.iter().enumerate() {
            let rel_start = entry.start_line.saturating_sub(self.scroll);
            if rel_start >= viewport {
                break;
            }
            let rel_end = (entry.end_line() - self.scroll).min(viewport);
            if entry.end_line() <= self.scroll {
                continue;
            }
            let is_selected = i == self.selected_entry;
            if !is_selected {
                continue;
            }
            let height = (rel_end - rel_start) as u16;
            let rect = Rect {
                x: area.x,
                y: area.y + rel_start as u16,
                width: area.width,
                height,
            };
            frame.render_widget(
                Block::default().style(Style::default().bg(theme.bg_highlight)),
                rect,
            );
        }

        // draw entries
        let avatars_supported = self.supports_avatars();
        for (i, entry) in self.entries.iter().enumerate() {
            if entry.end_line() <= self.scroll {
                continue;
            }
            let rel = entry.start_line.saturating_sub(self.scroll);
            if rel >= viewport {
                break;
            }
            let row = area.y + rel as u16;
            let is_selected = i == self.selected_entry;
            let sel_style = Style::default().bg(theme.bg_highlight);

            // Avatar first (needs &mut self for protocol render state)
            if entry.kind == EntryKind::Comment && avatars_supported {
                let avatar_rect = Rect {
                    x: area.x,
                    y: row,
                    width: AVATAR_COLS,
                    height: AVATAR_ROWS.min(area.bottom().saturating_sub(row)),
                };
                let comment_idx = entry.comment_index;
                if let Some(protocol) = self.avatars.get_mut(comment_idx) {
                    use ratatui_image::StatefulImage;
                    frame.render_stateful_widget(StatefulImage::new(), avatar_rect, protocol);
                } else {
                    frame.render_widget(
                        Paragraph::new(icons::USER)
                            .style(Style::default().fg(theme.fg_muted))
                            .alignment(Alignment::Center),
                        avatar_rect,
                    );
                }
            }

            match entry.kind {
                EntryKind::Comment => {
                    if let Some(comment) = self.comments.get(entry.comment_index) {
                        self.draw_comment_card(
                            frame,
                            area,
                            row,
                            comment,
                            theme,
                            is_selected,
                            sel_style,
                        );
                    }
                }
                EntryKind::Reply => {
                    let comment = &self.comments[entry.comment_index];
                    if let Some(replies) = self.replies.get(&comment.rpid)
                        && let Some(reply) = replies.get(entry.reply_index)
                    {
                        self.draw_reply_row(frame, area, row, reply, theme, is_selected, sel_style);
                    }
                }
                EntryKind::Toggle => {
                    self.draw_toggle_row(frame, area, row, entry, theme, is_selected, sel_style);
                }
            }
        }

        // scrollbar
        if self.total_lines > viewport {
            draw_scrollbar(frame, area, self.scroll, viewport, self.total_lines, theme);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_comment_card(
        &self,
        frame: &mut Frame,
        area: Rect,
        row: u16,
        comment: &CommentItem,
        theme: &Theme,
        is_selected: bool,
        sel_style: Style,
    ) {
        let content_width = area.width.saturating_sub(AVATAR_COLS + 1).max(8) as usize;
        let x_text = area.x + AVATAR_COLS + 1;
        let text_width = area.width.saturating_sub(AVATAR_COLS + 1);

        // Header: username + level + time
        let level = comment
            .member
            .as_ref()
            .and_then(|m| m.level_info.as_ref())
            .and_then(|l| l.current_level)
            .unwrap_or(0);
        let name = truncate_width(comment.author_name(), content_width.saturating_sub(12));
        let mut header_spans = vec![
            Span::styled(
                name,
                Style::default()
                    .fg(theme.bilibili_blue)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" LV{}", level),
                Style::default().fg(level_color(level, theme)),
            ),
            Span::styled(
                format!("  {}", comment.format_time()),
                Style::default().fg(theme.fg_muted),
            ),
        ];
        // UP badge: comment author == video uploader (mid match) is not
        // tracked here; badge hooks remain for future use.
        let header = Line::from(
            header_spans
                .drain(..)
                .map(|s| if is_selected { s.style(sel_style) } else { s })
                .collect::<Vec<_>>(),
        );
        frame.render_widget(
            Paragraph::new(header),
            Rect {
                x: x_text,
                y: row,
                width: text_width,
                height: 1,
            },
        );

        // Avatar is rendered by the caller (needs mutable protocol state).

        // Message lines (wrapped)
        let lines = wrap_lines(comment.message(), content_width);
        for (li, line_text) in lines.iter().enumerate() {
            let y = row + 1 + li as u16;
            if y >= area.bottom() {
                break;
            }
            let mut span = Span::styled(line_text.clone(), Style::default().fg(theme.fg_primary));
            if is_selected {
                span = span.style(sel_style);
            }
            frame.render_widget(
                Paragraph::new(Line::from(vec![Span::raw(" "), span])),
                Rect {
                    x: x_text.saturating_sub(1),
                    y,
                    width: text_width + 1,
                    height: 1,
                },
            );
        }

        // Action row: like · reply count · location placeholder
        let action_y = row + 1 + lines.len() as u16;
        if action_y < area.bottom() {
            let liked = self.is_liked(comment.rpid);
            let like_icon = if liked {
                icons::LIKE_FILLED
            } else {
                icons::LIKE
            };
            let like_color = if liked {
                theme.bilibili_pink
            } else {
                theme.fg_muted
            };
            let reply_info = if comment.reply_count() > 0 {
                format!("  {} {} 条回复", icons::COMMENT, comment.reply_count())
            } else {
                String::new()
            };
            let action = Line::from(vec![
                Span::raw(" "),
                Span::styled(like_icon, Style::default().fg(like_color)),
                Span::styled(
                    format!(" {} ", format_count(self.like_count(comment))),
                    Style::default().fg(like_color),
                ),
                Span::styled(reply_info, Style::default().fg(theme.fg_muted)),
            ])
            .style(sel_style);
            frame.render_widget(
                Paragraph::new(action),
                Rect {
                    x: x_text.saturating_sub(1),
                    y: action_y,
                    width: text_width + 1,
                    height: 1,
                },
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_reply_row(
        &self,
        frame: &mut Frame,
        area: Rect,
        row: u16,
        reply: &CommentItem,
        theme: &Theme,
        is_selected: bool,
        sel_style: Style,
    ) {
        let content_width = area.width.saturating_sub(AVATAR_COLS + 2).max(8) as usize;
        let indent = AVATAR_COLS + 1;
        let x_text = area.x + indent + 1;
        let text_width = area.width.saturating_sub(indent + 1);

        // Header
        let level = reply
            .member
            .as_ref()
            .and_then(|m| m.level_info.as_ref())
            .and_then(|l| l.current_level)
            .unwrap_or(0);
        let name = truncate_width(reply.author_name(), content_width.saturating_sub(12));
        let header = Line::from(vec![
            Span::styled(icons::REPLY_ARROW, Style::default().fg(theme.fg_muted)),
            Span::raw(" "),
            Span::styled(
                name,
                Style::default()
                    .fg(theme.bilibili_cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" LV{}", level),
                Style::default().fg(level_color(level, theme)),
            ),
            Span::styled(
                format!("  {}", reply.format_time()),
                Style::default().fg(theme.fg_muted),
            ),
        ])
        .style(if is_selected {
            sel_style
        } else {
            Style::default()
        });
        frame.render_widget(
            Paragraph::new(header),
            Rect {
                x: x_text - 2,
                y: row,
                width: text_width + 2,
                height: 1,
            },
        );

        // Message
        let lines = wrap_lines(reply.message(), content_width);
        for (li, line_text) in lines.iter().enumerate() {
            let y = row + 1 + li as u16;
            if y >= area.bottom() {
                break;
            }
            let span = Span::styled(line_text.clone(), Style::default().fg(theme.fg_primary));
            let line = Line::from(vec![Span::raw("  "), span]).style(if is_selected {
                sel_style
            } else {
                Style::default()
            });
            frame.render_widget(
                Paragraph::new(line),
                Rect {
                    x: x_text - 2,
                    y,
                    width: text_width + 2,
                    height: 1,
                },
            );
        }

        // Action row: like only
        let action_y = row + 1 + lines.len() as u16;
        if action_y < area.bottom() {
            let liked = self.is_liked(reply.rpid);
            let like_icon = if liked {
                icons::LIKE_FILLED
            } else {
                icons::LIKE
            };
            let like_color = if liked {
                theme.bilibili_pink
            } else {
                theme.fg_muted
            };
            let action = Line::from(vec![
                Span::raw("  "),
                Span::styled(like_icon, Style::default().fg(like_color)),
                Span::styled(
                    format!(" {}", format_count(self.like_count(reply))),
                    Style::default().fg(like_color),
                ),
            ])
            .style(if is_selected {
                sel_style
            } else {
                Style::default()
            });
            frame.render_widget(
                Paragraph::new(action),
                Rect {
                    x: x_text - 2,
                    y: action_y,
                    width: text_width + 2,
                    height: 1,
                },
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_toggle_row(
        &self,
        frame: &mut Frame,
        area: Rect,
        row: u16,
        entry: &Entry,
        theme: &Theme,
        is_selected: bool,
        sel_style: Style,
    ) {
        let comment = &self.comments[entry.comment_index];
        let indent = AVATAR_COLS + 1;
        let x = area.x + indent;
        let width = area.width.saturating_sub(indent);

        let (label, icon, color) = if entry.reply_index == 1 {
            (
                "加载更多回复".to_string(),
                icons::DOWNLOAD,
                theme.bilibili_blue,
            )
        } else if self.expanded.contains(&comment.rpid) {
            ("收起回复".to_string(), icons::FOLD_OPEN, theme.fg_muted)
        } else if self.loading_replies_for == Some(comment.rpid) {
            ("加载回复中...".to_string(), icons::SPINNER, theme.warning)
        } else {
            (
                format!("展开 {} 条回复", comment.reply_count()),
                icons::FOLD_CLOSED,
                theme.bilibili_blue,
            )
        };

        let line = Line::from(vec![
            Span::styled(icon, Style::default().fg(color)),
            Span::styled(format!(" {}", label), Style::default().fg(color)),
        ])
        .style(if is_selected {
            sel_style
        } else {
            Style::default()
        });
        frame.render_widget(
            Paragraph::new(line),
            Rect {
                x,
                y: row,
                width,
                height: 1,
            },
        );
    }

    fn supports_avatars(&mut self) -> bool {
        self.avatars.supports_images()
    }

    /// Comment indices visible in the current viewport (for avatar prefetch).
    fn visible_comment_indices(&self, viewport: usize) -> Vec<usize> {
        self.entries
            .iter()
            .filter(|e| e.end_line() > self.scroll && e.start_line < self.scroll + viewport)
            .map(|e| e.comment_index)
            .collect::<HashSet<_>>()
            .into_iter()
            .collect()
    }
}

impl Default for CommentList {
    fn default() -> Self {
        Self::new()
    }
}

/// Format count with 万 abbreviation (web style).
pub fn format_count(n: i64) -> String {
    if n >= 10_000 {
        format!("{:.1}万", n as f64 / 10_000.0)
    } else {
        format!("{n}")
    }
}

/// Draw a thin scrollbar on the right edge of the area.
fn draw_scrollbar(
    frame: &mut Frame,
    area: Rect,
    scroll: usize,
    viewport: usize,
    total: usize,
    theme: &Theme,
) {
    let track_height = area.height as usize;
    if track_height == 0 || total <= viewport {
        return;
    }
    let thumb_len = ((viewport * track_height) / total).clamp(1, track_height);
    let max_scroll = total - viewport;
    let pos = (scroll * (track_height - thumb_len)).div_ceil(max_scroll.max(1));
    for y in 0..track_height {
        let in_thumb = y >= pos && y < pos + thumb_len;
        frame.render_widget(
            Paragraph::new(" ").style(Style::default().bg(if in_thumb {
                theme.fg_muted
            } else {
                theme.bg_secondary
            })),
            Rect {
                x: area.x + area.width - 1,
                y: area.y + y as u16,
                width: 1,
                height: 1,
            },
        );
    }
}

/// Wrap text to `width` display columns (CJK-aware).
pub fn wrap_lines(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut cur_width = 0usize;

    for ch in text.chars() {
        let w = UnicodeWidthStr::width(ch.to_string().as_str());
        if cur_width + w > width && !current.is_empty() {
            lines.push(std::mem::take(&mut current));
            cur_width = 0;
        }
        current.push(ch);
        cur_width += w;
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

/// Truncate to `width` display columns with ellipsis.
pub fn truncate_width(text: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    if UnicodeWidthStr::width(text) <= width {
        return text.to_string();
    }
    let mut out = String::new();
    let mut w = 0usize;
    for ch in text.chars() {
        let cw = UnicodeWidthStr::width(ch.to_string().as_str());
        // reserve 1 column for the ellipsis
        if w + cw > width - 1 {
            break;
        }
        out.push(ch);
        w += cw;
    }
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_respects_cjk_width() {
        // 8 columns: 4 CJK chars fill one line
        let lines = wrap_lines("一二三四五六七八", 8);
        assert_eq!(lines, vec!["一二三四", "五六七八"]);
    }

    #[test]
    fn wrap_keeps_ascii_words() {
        let lines = wrap_lines("abcdefgh ij", 4);
        assert_eq!(lines, vec!["abcd", "efgh", " ij"]);
    }

    #[test]
    fn truncate_appends_ellipsis() {
        // 6 columns: 2 CJK chars (4 cols) + ellipsis (1 col) = 5 used
        let out = truncate_width("一二三四五", 6);
        assert_eq!(out, "一二…");
        assert!(UnicodeWidthStr::width(out.as_str()) <= 6);
        // ascii passthrough when it fits
        assert_eq!(truncate_width("abc", 5), "abc");
    }

    #[test]
    fn format_count_uses_wan() {
        assert_eq!(format_count(9_999), "9999");
        assert_eq!(format_count(23_456), "2.3万");
    }
}
