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
    /// Reply row inside an expanded comment (floor view).
    Reply,
    /// Reply-to-reply row inside the conversation view.
    SubReply,
    /// "展开/收起回复" or page/load-more toggle row.
    Toggle,
    /// Horizontal rule between top-level cards (not selectable).
    Separator,
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
    /// Turn the floor page of the expanded comment's replies.
    PageReplies { comment_index: usize },
    /// Open the APP-style conversation of a floor reply.
    OpenSubThread {
        comment_index: usize,
        reply_index: usize,
    },
    /// Leave the conversation view.
    CloseSubThread,
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
    pub key: (Option<i64>, String),
    pub protocol: StatefulProtocol,
}

/// Async avatar loader: downloads avatar images in the background and keeps
/// one rendered protocol per author, keyed by `(mid, uname)` instead of list
/// index — comment refreshes reorder/re-paginate the list, and index-keyed
/// caches showed the previous holder's avatar under a new name.
///
/// The terminal picker is created lazily on first use — `Picker::from_query_stdio`
/// performs terminal capability queries that must never run at page-construction
/// time (it blocks non-TTY test environments).
pub struct AvatarLoader {
    /// Rendered image protocols keyed by author identity (mid, uname).
    pub protocols: HashMap<(Option<i64>, String), StatefulProtocol>,
    pending: HashSet<(Option<i64>, String)>,
    tx: mpsc::Sender<AvatarResult>,
    rx: mpsc::Receiver<AvatarResult>,
    picker: Option<Arc<Picker>>,
    supports_images: bool,
}

/// Stable author identity used to key avatar cache entries.
fn author_key(
    member: Option<&crate::api::comment::CommentMember>,
) -> Option<(Option<i64>, String)> {
    let member = member?;
    let mid = member.mid.clone().and_then(|m| m.parse::<i64>().ok());
    let name = member.uname.clone().or_else(|| member.avatar.clone())?;
    Some((mid, name))
}

impl AvatarLoader {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(64);
        Self {
            protocols: HashMap::new(),
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
        if !self.protocols.is_empty() {
            return true;
        }
        self.ensure_picker().is_some()
    }

    pub fn get(&self, key: &(Option<i64>, String)) -> Option<&StatefulProtocol> {
        self.protocols.get(key)
    }

    pub fn get_mut(&mut self, key: &(Option<i64>, String)) -> Option<&mut StatefulProtocol> {
        self.protocols.get_mut(key)
    }

    fn is_loaded_or_pending(&self, key: &(Option<i64>, String)) -> bool {
        self.pending.contains(key) || self.protocols.contains_key(key)
    }

    /// Request downloads for the given authors (identity + avatar url).
    pub fn request(
        &mut self,
        authors: impl IntoIterator<Item = ((Option<i64>, String), Option<String>)>,
    ) {
        let Some(picker) = self.ensure_picker() else {
            return;
        };
        for (key, url) in authors {
            if self.is_loaded_or_pending(&key) {
                continue;
            }
            let Some(url) = url else {
                continue;
            };
            self.pending.insert(key.clone());
            let tx = self.tx.clone();
            let picker = Arc::clone(&picker);
            let url = normalize_avatar_url(&url);
            tokio::spawn(async move {
                if let Some(img) = download_image(&url).await {
                    let protocol = picker.new_resize_protocol(img);
                    let _ = tx.send(AvatarResult { key, protocol }).await;
                }
            });
        }
    }

    /// Drain finished downloads; returns true if anything new arrived.
    pub fn poll(&mut self) -> bool {
        let mut updated = false;
        while let Ok(result) = self.rx.try_recv() {
            self.pending.remove(&result.key);
            self.protocols.insert(result.key, result.protocol);
            updated = true;
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

/// Download an image over https (shared with other pages).
pub(super) async fn download_image(url: &str) -> Option<DynamicImage> {
    let response = reqwest::get(url).await.ok()?;
    let bytes = response.bytes().await.ok()?;
    image::load_from_memory(&bytes).ok()
}

/// Level badge color, mirroring bilibili's web palette.
/// Wrap `segments` into visual lines of at most `width` cells, preserving
/// emote styling across wraps.
fn wrap_segments(
    segments: &[crate::api::comment::Segment<'_>],
    width: usize,
    theme: &Theme,
) -> Vec<Vec<Span<'static>>> {
    let mut lines: Vec<Vec<Span<'static>>> = vec![Vec::new()];
    let mut col = 0usize;
    for seg in segments {
        match seg {
            crate::api::comment::Segment::Text(t) => {
                for line in wrap_lines(t, width) {
                    let line_w = line.chars().count();
                    if col > 0 && !lines.last().is_some_and(|l| l.is_empty()) {
                        // this text continues after an emote on the same row;
                        // if it doesn't fit, move it to a fresh line
                        if col + line_w > width {
                            lines.push(Vec::new());
                            col = 0;
                        }
                    }
                    lines
                        .last_mut()
                        .unwrap()
                        .push(Span::styled(line.to_string(), Style::default()));
                    col += line_w;
                }
            }
            crate::api::comment::Segment::Emote(token) => {
                let styled = format!("{}{} ", icons::SMILE, token);
                let w = styled.chars().count();
                if col + w > width && col > 0 {
                    lines.push(Vec::new());
                    col = 0;
                }
                lines.last_mut().unwrap().push(Span::styled(
                    styled,
                    Style::default().fg(theme.bilibili_cyan),
                ));
                col += w;
            }
        }
    }
    if lines.len() == 1 && lines[0].is_empty() {
        lines[0].push(Span::styled(String::new(), Style::default()));
    }
    lines
}

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

const AVATAR_COLS: u16 = 4; // avatar cell width, in terminal columns
const AVATAR_ROWS: u16 = 2; // avatar cell height (≈square in cells)
const GAP_COLS: u16 = 1; // gap between avatar and text column
const CARD_TRAIL_BLANK: u16 = 2; // blank rows after each card (web-like breathing room)

/// Web-style comment list widget + state.
/// Replies shown per floor page when a comment is expanded.
pub const REPLIES_PER_PAGE: usize = 10;

pub struct CommentList {
    /// Top-level comments (hot + recent, in API order).
    pub comments: Vec<CommentItem>,
    /// Fetched replies keyed by root comment rpid.
    pub replies: HashMap<i64, Vec<CommentItem>>,
    /// APP-style conversation view: (root_rpid, focus reply rpid) while the
    /// user is reading the full conversation of one floor reply.
    pub sub_thread: Option<(i64, i64)>,
    /// The focused reply itself, cached so the view works even when the
    /// floor cache (replies) is empty (dev deep link / fresh entry).
    pub sub_focus: Option<CommentItem>,
    /// Fetched children per focused reply rpid.
    pub sub_replies: HashMap<i64, Vec<CommentItem>>,
    /// Current floor page (0-based) per expanded root rpid.
    pub reply_pages: HashMap<i64, usize>,
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
    /// Comment sort: false = hot (default), true = newest.
    pub sort_newest: bool,
    /// Video uploader mid for the UP badge (None = no badges).
    pub uploader_mid: Option<i64>,
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
            sub_thread: None,
            sub_focus: None,
            sub_replies: HashMap::new(),
            reply_pages: HashMap::new(),
            expanded: HashSet::new(),
            loading_replies_for: None,
            loading_more_replies: false,
            has_more: false,
            loading_more: false,
            liked: HashSet::new(),
            like_deltas: HashMap::new(),
            sort_newest: false,
            uploader_mid: None,
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
        self.reply_pages.clear();
        self.expanded.clear();
        self.loading_replies_for = None;
        self.selected = self.selected.min(self.comments.len().saturating_sub(1));
        self.scroll = 0;
        self.has_more = total_count > self.comments.len() as i64;
        self.entries.clear();
    }

    /// Append a page of comments (pagination).
    pub fn append_comments(&mut self, comments: Vec<CommentItem>) {
        self.comments.extend(comments);
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

    /// Enter the conversation view for `focus_rpid` under root `root_rpid`.
    pub fn enter_sub_thread(&mut self, root_rpid: i64, focus_rpid: i64) {
        self.sub_thread = Some((root_rpid, focus_rpid));
        // Remember the reply being opened so the view renders even before
        // children arrive (or when the floor cache was never populated).
        self.sub_focus = self
            .replies
            .get(&root_rpid)
            .and_then(|rs| rs.iter().find(|r| r.rpid == focus_rpid))
            .cloned();
        self.entries.clear();
        self.selected_entry = 0;
        self.scroll = 0;
    }

    /// Leave the conversation view.
    pub fn leave_sub_thread(&mut self) {
        self.sub_thread = None;
        self.entries.clear();
        self.selected_entry = 0;
    }

    /// Insert fetched children for the focused reply.
    pub fn set_sub_replies(&mut self, focus_rpid: i64, children: Vec<CommentItem>) {
        self.sub_replies.insert(focus_rpid, children);
        self.entries.clear();
    }

    /// Position of the selected entry when it is a floor reply
    /// (comment_index, reply_index); None for top-level cards.
    pub fn selected_reply_pos(&self) -> Option<(usize, usize)> {
        let entry = self.entries.get(self.selected_entry)?;
        match entry.kind {
            EntryKind::Reply if entry.reply_index != usize::MAX => {
                Some((entry.comment_index, entry.reply_index))
            }
            _ => None,
        }
    }

    /// Whether a conversation view is open.
    pub fn in_sub_thread(&self) -> bool {
        self.sub_thread.is_some()
    }

    /// Visible slice (floor page) of replies for an expanded root comment.
    pub fn visible_replies(&self, root_rpid: i64) -> &[CommentItem] {
        let all = self
            .replies
            .get(&root_rpid)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        if !self.expanded.contains(&root_rpid) {
            return &[];
        }
        let page = *self.reply_pages.get(&root_rpid).unwrap_or(&0);
        let start = (page * REPLIES_PER_PAGE).min(all.len());
        let end = (start + REPLIES_PER_PAGE).min(all.len());
        &all[start..end]
    }

    /// Current floor page number (1-based) for an expanded root, if any.
    pub fn reply_page_info(&self, root_rpid: i64) -> Option<(usize, usize)> {
        if !self.expanded.contains(&root_rpid) {
            return None;
        }
        let total = self.replies.get(&root_rpid)?.len();
        if total == 0 {
            return None;
        }
        let page = *self.reply_pages.get(&root_rpid).unwrap_or(&0);
        let pages = total.div_ceil(REPLIES_PER_PAGE);
        Some((page + 1, pages))
    }

    /// Turn to the next/previous floor page; returns true when moved.
    pub fn page_replies(&mut self, root_rpid: i64, dir: i32) -> bool {
        let Some((page, pages)) = self.reply_page_info(root_rpid) else {
            return false;
        };
        let next = if dir > 0 {
            (page).min(pages - 1)
        } else {
            page.saturating_sub(2)
        };
        if next + 1 == page {
            return false;
        }
        self.reply_pages.insert(root_rpid, next);
        self.entries.clear();
        true
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

    /// Sort badge shown next to the comment panel title: 最热 / 最新.
    pub fn sort_label(&self) -> &'static str {
        if self.sort_newest { "最新" } else { "最热" }
    }

    /// Sort glyph: fire for hot, clock for newest.
    pub fn sort_icon(&self) -> &'static str {
        if self.sort_newest {
            icons::CLOCK_O
        } else {
            icons::FIRE_ALT
        }
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

    /// Collect visible authors' identities + avatar urls for prefetch.
    #[allow(clippy::type_complexity)]
    fn visible_authors(&self, indices: &[usize]) -> Vec<((Option<i64>, String), Option<String>)> {
        indices
            .iter()
            .filter_map(|i| {
                let c = self.comments.get(*i)?;
                let key = author_key(c.member.as_ref())?;
                Some((key, c.member.as_ref().and_then(|m| m.avatar.clone())))
            })
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

        // Conversation view: focused reply on top, its children below,
        // a back row at the bottom (APP-style 对话页).
        if let Some((root_rpid, focus_rpid)) = self.sub_thread {
            let ci = self
                .comments
                .iter()
                .position(|c| c.rpid == root_rpid)
                .unwrap_or(0);
            let focus = self.sub_focus.clone().or_else(|| {
                self.replies
                    .get(&root_rpid)
                    .and_then(|rs| rs.iter().find(|r| r.rpid == focus_rpid))
                    .cloned()
            });
            if let Some(focus) = focus {
                let msg = focus.message_line_count(content_width).max(1);
                let h = 1 + msg + 1 + CARD_TRAIL_BLANK as usize + 1;
                entries.push(Entry {
                    kind: EntryKind::Reply,
                    comment_index: ci,
                    reply_index: usize::MAX, // focus marker
                    start_line: line,
                    height: h as u16,
                });
                line += h;
                if let Some(children) = self.sub_replies.get(&focus_rpid) {
                    for (si, child) in children.iter().enumerate() {
                        let msg = child.message_line_count(content_width).max(1);
                        let h = 1 + msg + 1 + CARD_TRAIL_BLANK as usize + 1;
                        entries.push(Entry {
                            kind: EntryKind::SubReply,
                            comment_index: ci,
                            reply_index: si,
                            start_line: line,
                            height: h as u16,
                        });
                        line += h;
                    }
                }
                entries.push(Entry {
                    kind: EntryKind::Toggle,
                    comment_index: ci,
                    reply_index: 3, // back row
                    start_line: line,
                    height: 2,
                });
                line += 2;
            }
            self.entries = entries;
            self.total_lines = line;
            self.last_width = width;
            return;
        }

        for (ci, comment) in self.comments.iter().enumerate() {
            // separator line above each card (except the very first)
            if ci > 0 {
                entries.push(Entry {
                    kind: EntryKind::Separator,
                    comment_index: ci,
                    reply_index: 0,
                    start_line: line,
                    height: 1,
                });
                line += 1;
            }
            let msg_lines = comment.message_line_count(content_width).max(1);
            let card_height = 1 + msg_lines + 1 + CARD_TRAIL_BLANK as usize; // header+msg+actions+blank
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
                if self.replies.contains_key(&comment.rpid) {
                    let floor_replies = self.visible_replies(comment.rpid);
                    let page_base = self.reply_pages.get(&comment.rpid).copied().unwrap_or(0)
                        * REPLIES_PER_PAGE;
                    for (ri, reply) in floor_replies.iter().enumerate() {
                        let reply_msg_lines = reply.message_line_count(content_width).max(1);
                        let height = 1 + reply_msg_lines + 1 + 1; // header+msg+actions+blank
                        entries.push(Entry {
                            kind: EntryKind::Reply,
                            comment_index: ci,
                            reply_index: page_base + ri,
                            start_line: line,
                            height: height as u16,
                        });
                        line += height;
                    }
                    // floor pager: 上一页 / 第x/y页 / 下一页 (+加载更多 when server has more)
                    let pager_height = 2;
                    entries.push(Entry {
                        kind: EntryKind::Toggle,
                        comment_index: ci,
                        reply_index: 2, // pager row
                        start_line: line,
                        height: pager_height as u16,
                    });
                    line += pager_height;
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
        if let Some(entry) = self.entries.get(self.selected_entry) {
            let sel_start = entry.start_line;
            let sel_end = entry.end_line().saturating_sub(1);
            // Anchor: keep the selected entry around the lower third so the
            // user can see what is coming next (web-player behaviour).
            // Only starts scrolling once the selection would leave that zone;
            // at the very top/bottom of the list the viewport clamps naturally.
            let anchor_line = sel_end + 1;
            let max_scroll = self.total_lines.saturating_sub(viewport);
            // desired scroll so that selection bottom sits at ~2/3 viewport
            let prefer_bottom = viewport * 2 / 3;
            if anchor_line > self.scroll + prefer_bottom {
                // moving down: bring selection bottom to the anchor line
                self.scroll = (anchor_line - prefer_bottom).min(max_scroll);
            } else if sel_start < self.scroll {
                // moving up: keep the whole entry visible at the top
                self.scroll = sel_start.min(max_scroll);
            }
            self.scroll = self.scroll.min(max_scroll);
        }
    }

    /// Move selection up; returns false when already at the top.
    pub fn move_up(&mut self) -> bool {
        if self.entries.is_empty() {
            return false;
        }
        let mut idx = self.selected_entry;
        while idx > 0 {
            idx -= 1;
            if self.entries[idx].kind != EntryKind::Separator {
                self.selected_entry = idx;
                self.sync_selected_comment();
                return true;
            }
        }
        false
    }

    /// Move selection down; returns intents (load-more) when nearing bottom.
    pub fn move_down(&mut self) -> Option<CommentIntent> {
        if self.entries.is_empty() {
            return None;
        }
        let mut idx = self.selected_entry;
        while idx + 1 < self.entries.len() {
            idx += 1;
            if self.entries[idx].kind != EntryKind::Separator {
                self.selected_entry = idx;
                self.sync_selected_comment();
                // near bottom: request more comments
                if idx + 2 >= self.entries.len() && self.has_more && !self.loading_more {
                    return Some(CommentIntent::LoadMoreComments);
                }
                return None;
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
    /// Entry kind + comment index at the current selection (for key routing).
    pub fn selected_entry_kind(&self) -> Option<(EntryKind, usize)> {
        let entry = self.entries.get(self.selected_entry)?;
        Some((entry.kind, entry.comment_index))
    }

    pub fn selected_entry_info(&self) -> Option<Entry> {
        self.entries.get(self.selected_entry).copied()
    }

    /// Activate the selected entry (Enter / click on toggle).
    pub fn activate_selected(&self) -> Option<CommentIntent> {
        let entry = self.entries.get(self.selected_entry)?;
        match entry.kind {
            EntryKind::Separator => None,
            EntryKind::Comment => Some(CommentIntent::Like {
                comment_index: entry.comment_index,
                reply_index: None,
            }),
            EntryKind::Reply => {
                // The focus row of a conversation view just likes.
                if entry.reply_index == usize::MAX || self.in_sub_thread() {
                    return Some(CommentIntent::Like {
                        comment_index: entry.comment_index,
                        reply_index: Some(entry.reply_index),
                    });
                }
                // In floor view, Space opens the conversation of a reply
                // that has children ("查看对话").
                let comment = self.comments.get(entry.comment_index)?;
                let has_children = self
                    .replies
                    .get(&comment.rpid)
                    .and_then(|rs| rs.get(entry.reply_index))
                    .map(|r| r.rcount.unwrap_or(0) > 0)
                    .unwrap_or(false);
                if has_children {
                    Some(CommentIntent::OpenSubThread {
                        comment_index: entry.comment_index,
                        reply_index: entry.reply_index,
                    })
                } else {
                    Some(CommentIntent::Like {
                        comment_index: entry.comment_index,
                        reply_index: Some(entry.reply_index),
                    })
                }
            }
            EntryKind::SubReply => Some(CommentIntent::Like {
                comment_index: entry.comment_index,
                reply_index: None,
            }),
            EntryKind::Toggle => {
                let comment = self.comments.get(entry.comment_index)?;
                if self.in_sub_thread() {
                    Some(CommentIntent::CloseSubThread)
                } else if self.expanded.contains(&comment.rpid) {
                    if entry.reply_index == 2 {
                        let total_fetched = self.replies.get(&comment.rpid).map_or(0, |r| r.len());
                        let has_more_server = comment.reply_count() as usize > total_fetched;
                        if self
                            .reply_page_info(comment.rpid)
                            .is_none_or(|(_, p)| p <= 1)
                            && has_more_server
                        {
                            // single page + server has more: fetch next page
                            Some(CommentIntent::LoadMoreReplies {
                                comment_index: entry.comment_index,
                            })
                        } else {
                            // pager row cycles to the next floor page (wraps)
                            Some(CommentIntent::PageReplies {
                                comment_index: entry.comment_index,
                            })
                        }
                    } else {
                        // reply_index 0 = collapse row
                        Some(CommentIntent::ToggleReplies {
                            comment_index: entry.comment_index,
                        })
                    }
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

        // avatar prefetch for visible comments and their floor replies
        let visible_comments = self.visible_comment_indices(viewport);
        let mut authors = self.visible_authors(&visible_comments);
        for ci in &visible_comments {
            if let Some(c) = self.comments.get(*ci) {
                for reply in self.visible_replies(c.rpid) {
                    if let Some(key) = author_key(reply.member.as_ref()) {
                        authors.push((key, reply.member.as_ref().and_then(|m| m.avatar.clone())));
                    }
                }
            }
        }
        // Conversation view: focus reply + its children are not part of the
        // floor cache, so request their avatars explicitly.
        if let Some((_, focus_rpid)) = self.sub_thread {
            if let Some(focus) = self.sub_focus.as_ref()
                && let Some(key) = author_key(focus.member.as_ref())
            {
                authors.push((key, focus.member.as_ref().and_then(|m| m.avatar.clone())));
            }
            if let Some(children) = self.sub_replies.get(&focus_rpid) {
                for child in children {
                    if let Some(key) = author_key(child.member.as_ref()) {
                        authors.push((key, child.member.as_ref().and_then(|m| m.avatar.clone())));
                    }
                }
            }
        }
        self.avatars.request(authors);
        self.avatars.poll();

        // selection = thin highlighted outline around the selected comment
        // block (never a filled highlight block)
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
            // Filled background for the whole card, then a pink accent bar
            // on the left edge (web-style selected comment).
            frame.render_widget(
                Block::default().style(Style::default().bg(theme.bg_highlight)),
                rect,
            );
            frame.render_widget(
                Block::default()
                    .borders(Borders::LEFT)
                    .border_style(Style::default().fg(theme.bilibili_pink)),
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
            // outline is drawn separately; rows keep the panel background
            let sel_style = Style::default();

            // Avatar first (needs &mut self for protocol render state).
            // Replies get one too (floor view = same layout as top comments).
            let avatar_member = match entry.kind {
                EntryKind::Comment => self
                    .comments
                    .get(entry.comment_index)
                    .and_then(|c| c.member.as_ref()),
                // usize::MAX = the focused reply of a conversation view; its
                // member lives in sub_focus (the floor cache may be empty).
                EntryKind::Reply if entry.reply_index == usize::MAX => {
                    self.sub_focus.as_ref().and_then(|r| r.member.as_ref())
                }
                EntryKind::Reply => self
                    .comments
                    .get(entry.comment_index)
                    .and_then(|c| self.visible_replies(c.rpid).get(entry.reply_index))
                    .and_then(|r| r.member.as_ref()),
                // conversation children render smaller indent rows; give
                // them the same avatar treatment
                EntryKind::SubReply => {
                    let focus = self.sub_thread.map(|(_, r)| r);
                    focus
                        .and_then(|f| self.sub_replies.get(&f))
                        .and_then(|cs| cs.get(entry.reply_index))
                        .and_then(|r| r.member.as_ref())
                }
                _ => None,
            };
            if avatars_supported && let Some(member) = avatar_member {
                let avatar_rect = Rect {
                    x: area.x,
                    y: row,
                    width: AVATAR_COLS,
                    height: AVATAR_ROWS.min(area.bottom().saturating_sub(row)),
                };
                let protocol = author_key(Some(member))
                    .and_then(|key| self.avatars.get_mut(&key).map(|p| p as *mut _));
                // SAFETY: protocol points into self.avatars, which we borrow
                // mutably only here; no other aliasing borrow is live.
                if let Some(protocol) = protocol.map(|p| unsafe { &mut *p }) {
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
                    // usize::MAX marks the focused reply atop a conversation.
                    if entry.reply_index == usize::MAX {
                        if let Some(reply) = self.sub_focus.as_ref() {
                            self.draw_reply_row(
                                frame,
                                area,
                                row,
                                reply,
                                theme,
                                is_selected,
                                sel_style,
                            );
                        }
                    } else if let Some(replies) = self.replies.get(&comment.rpid)
                        && let Some(reply) = replies.get(entry.reply_index)
                    {
                        self.draw_reply_row(frame, area, row, reply, theme, is_selected, sel_style);
                    }
                }
                EntryKind::SubReply => {
                    if let Some((_, focus_rpid)) = self.sub_thread
                        && let Some(children) = self.sub_replies.get(&focus_rpid)
                        && let Some(child) = children.get(entry.reply_index)
                    {
                        self.draw_sub_reply_row(
                            frame,
                            area,
                            row,
                            child,
                            theme,
                            is_selected,
                            sel_style,
                        );
                    }
                }
                EntryKind::Toggle => {
                    self.draw_toggle_row(frame, area, row, entry, theme, is_selected, sel_style);
                }
                EntryKind::Separator => {
                    let rule = Line::from(vec![Span::styled(
                        "─".repeat(area.width.saturating_sub(2) as usize),
                        Style::default().fg(theme.border_subtle),
                    )]);
                    frame.render_widget(
                        Paragraph::new(rule),
                        Rect {
                            x: area.x + 1,
                            y: row,
                            width: area.width.saturating_sub(2),
                            height: 1,
                        },
                    );
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
        let text_x = area.x + AVATAR_COLS + GAP_COLS;
        let text_width = area.width.saturating_sub(AVATAR_COLS + GAP_COLS);
        let content_width = text_width.saturating_sub(1) as usize;

        // Header row: 昵称 [UP] (web puts name at top; level badge next)
        let level = comment
            .member
            .as_ref()
            .and_then(|m| m.level_info.as_ref())
            .and_then(|l| l.current_level)
            .unwrap_or(0);
        let is_up = self.uploader_mid.is_some()
            && comment
                .member
                .as_ref()
                .and_then(|m| m.mid.clone())
                .and_then(|mid| mid.parse::<i64>().ok())
                == self.uploader_mid;
        let name = truncate_width(
            comment.author_name(),
            text_width.saturating_sub(12) as usize,
        );
        let mut header_spans = vec![Span::styled(
            name,
            Style::default()
                .fg(theme.bilibili_blue)
                .add_modifier(Modifier::BOLD),
        )];
        if is_up {
            header_spans.push(Span::styled(
                format!(" {} ", icons::STAR),
                Style::default()
                    .fg(theme.bilibili_pink)
                    .add_modifier(Modifier::BOLD),
            ));
        }
        header_spans.push(Span::styled(
            format!(" LV{}", level),
            Style::default().fg(level_color(level, theme)),
        ));
        let header = Line::from(
            header_spans
                .drain(..)
                .map(|s| if is_selected { s.style(sel_style) } else { s })
                .collect::<Vec<_>>(),
        );
        frame.render_widget(
            Paragraph::new(header),
            Rect {
                x: text_x,
                y: row,
                width: text_width,
                height: 1,
            },
        );

        // Avatar is rendered by the caller (needs mutable protocol state).

        // Message lines (wrapped; emote-aware when the API provides emotes)
        let segments = comment.message_segments();
        let has_emotes = segments
            .iter()
            .any(|seg| matches!(seg, crate::api::comment::Segment::Emote(_)));
        let line_count = comment.message_line_count(content_width).max(1);
        let msg_lines: Vec<Vec<Span<'static>>> = if has_emotes {
            wrap_segments(&segments, content_width, theme)
        } else {
            wrap_lines(comment.message(), content_width)
                .into_iter()
                .map(|l| vec![Span::styled(l, Style::default().fg(theme.fg_primary))])
                .collect()
        };
        for (li, spans) in msg_lines.iter().enumerate() {
            let y = row + 1 + li as u16;
            if y >= area.bottom() {
                break;
            }
            let spans: Vec<Span<'static>> = spans
                .iter()
                .map(|sp| {
                    let mut s = sp.clone();
                    if is_selected {
                        s = s.style(sel_style);
                    } else if s.style.fg.is_none() {
                        s = s.style(Style::default().fg(theme.fg_primary));
                    }
                    s
                })
                .collect();
            frame.render_widget(
                Paragraph::new(Line::from(spans)),
                Rect {
                    x: text_x,
                    y,
                    width: text_width,
                    height: 1,
                },
            );
        }

        // Action row: 时间 · IP属地 · 点赞 · 回复数 (web order)
        let action_y = row + 1 + line_count as u16;
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
            let mut action_spans = vec![Span::styled(
                comment.format_time_absolute(),
                Style::default().fg(theme.fg_muted),
            )];
            if let Some(loc) = comment.ip_location() {
                action_spans.push(Span::styled(
                    format!(" · IP{}", loc),
                    Style::default().fg(theme.fg_muted),
                ));
            }
            action_spans.push(Span::styled(
                format!("  {} ", like_icon),
                Style::default().fg(like_color),
            ));
            action_spans.push(Span::styled(
                format_count(self.like_count(comment)),
                Style::default().fg(like_color),
            ));
            if comment.reply_count() > 0 {
                action_spans.push(Span::styled(
                    format!("  {} {} 条回复", icons::COMMENT, comment.reply_count()),
                    Style::default().fg(theme.fg_muted),
                ));
            }
            let action = Line::from(action_spans).style(sel_style);
            frame.render_widget(
                Paragraph::new(action),
                Rect {
                    x: text_x,
                    y: action_y,
                    width: text_width,
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
        // Floor view: replies indent clearly to the right of the parent's
        // avatar column, no vertical hierarchy line.
        const REPLY_INDENT: u16 = 4;
        let text_x = area.x + AVATAR_COLS + REPLY_INDENT + GAP_COLS;
        let text_width = area
            .width
            .saturating_sub(AVATAR_COLS + REPLY_INDENT + GAP_COLS);
        let content_width = text_width.saturating_sub(1) as usize;

        // Header: name LV (time moves to action row like web)
        let level = reply
            .member
            .as_ref()
            .and_then(|m| m.level_info.as_ref())
            .and_then(|l| l.current_level)
            .unwrap_or(0);
        let is_up = self.uploader_mid.is_some()
            && reply
                .member
                .as_ref()
                .and_then(|m| m.mid.clone())
                .and_then(|mid| mid.parse::<i64>().ok())
                == self.uploader_mid;
        let name = truncate_width(reply.author_name(), content_width.saturating_sub(10));
        let mut header_spans = vec![Span::styled(
            name,
            Style::default()
                .fg(theme.bilibili_cyan)
                .add_modifier(Modifier::BOLD),
        )];
        if is_up {
            header_spans.push(Span::styled(
                format!(" {} ", icons::STAR),
                Style::default()
                    .fg(theme.bilibili_pink)
                    .add_modifier(Modifier::BOLD),
            ));
        }
        header_spans.push(Span::styled(
            format!(" LV{}", level),
            Style::default().fg(level_color(level, theme)),
        ));
        let header = Line::from(header_spans).style(if is_selected {
            sel_style
        } else {
            Style::default()
        });
        frame.render_widget(
            Paragraph::new(header),
            Rect {
                x: text_x,
                y: row,
                width: text_width,
                height: 1,
            },
        );

        // Message (emote-aware)
        let segments = reply.message_segments();
        let has_emotes = segments
            .iter()
            .any(|seg| matches!(seg, crate::api::comment::Segment::Emote(_)));
        let line_count = reply.message_line_count(content_width).max(1);
        let msg_lines: Vec<Vec<Span<'static>>> = if has_emotes {
            wrap_segments(&segments, content_width, theme)
        } else {
            wrap_lines(reply.message(), content_width)
                .into_iter()
                .map(|l| vec![Span::styled(l, Style::default().fg(theme.fg_primary))])
                .collect()
        };
        for (li, spans) in msg_lines.iter().enumerate() {
            let y = row + 1 + li as u16;
            if y >= area.bottom() {
                break;
            }
            let spans: Vec<Span<'static>> = spans
                .iter()
                .map(|sp| {
                    let mut s = sp.clone();
                    if is_selected {
                        s = s.style(sel_style);
                    } else if s.style.fg.is_none() {
                        s = s.style(Style::default().fg(theme.fg_primary));
                    }
                    s
                })
                .collect();
            frame.render_widget(
                Paragraph::new(Line::from(spans)),
                Rect {
                    x: text_x,
                    y,
                    width: text_width,
                    height: 1,
                },
            );
        }

        // Action row: 时间 · IP属地 · 点赞
        let action_y = row + 1 + line_count as u16;
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
            let mut action_spans = vec![Span::styled(
                reply.format_time_absolute(),
                Style::default().fg(theme.fg_muted),
            )];
            if let Some(loc) = reply.ip_location() {
                action_spans.push(Span::styled(
                    format!(" · IP{}", loc),
                    Style::default().fg(theme.fg_muted),
                ));
            }
            action_spans.push(Span::styled(
                format!("  {} ", like_icon),
                Style::default().fg(like_color),
            ));
            action_spans.push(Span::styled(
                format_count(self.like_count(reply)),
                Style::default().fg(like_color),
            ));
            // "共n条回复" hint when this reply has children (web wording).
            let child_count = reply.rcount.unwrap_or(0).max(0) as usize;
            if child_count > 0 {
                action_spans.push(Span::styled(
                    format!("  共{}条回复", child_count),
                    Style::default().fg(theme.bilibili_blue),
                ));
            }
            let action = Line::from(action_spans).style(if is_selected {
                sel_style
            } else {
                Style::default()
            });
            frame.render_widget(
                Paragraph::new(action),
                Rect {
                    x: text_x,
                    y: action_y,
                    width: text_width,
                    height: 1,
                },
            );
        }
    }

    /// Child reply inside a conversation view: indented with a faint
    /// vertical hierarchy line on the left.
    #[allow(clippy::too_many_arguments)]
    fn draw_sub_reply_row(
        &self,
        frame: &mut Frame,
        area: Rect,
        row: u16,
        reply: &CommentItem,
        theme: &Theme,
        is_selected: bool,
        sel_style: Style,
    ) {
        const INDENT: u16 = AVATAR_COLS + 6;
        let text_x = area.x + INDENT + GAP_COLS;
        let text_width = area.width.saturating_sub(INDENT + GAP_COLS);
        let content_width = text_width.saturating_sub(1) as usize;

        let level = reply
            .member
            .as_ref()
            .and_then(|m| m.level_info.as_ref())
            .and_then(|l| l.current_level)
            .unwrap_or(0);
        let name = truncate_width(reply.author_name(), content_width.saturating_sub(10));
        let header = Line::from(vec![
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
        ])
        .style(if is_selected {
            sel_style
        } else {
            Style::default()
        });
        frame.render_widget(
            Paragraph::new(header),
            Rect {
                x: text_x,
                y: row,
                width: text_width,
                height: 1,
            },
        );

        let segments = reply.message_segments();
        let has_emotes = segments
            .iter()
            .any(|seg| matches!(seg, crate::api::comment::Segment::Emote(_)));
        let line_count = reply.message_line_count(content_width).max(1);
        let msg_lines: Vec<Vec<Span<'static>>> = if has_emotes {
            wrap_segments(&segments, content_width, theme)
        } else {
            wrap_lines(reply.message(), content_width)
                .into_iter()
                .map(|l| vec![Span::styled(l, Style::default().fg(theme.fg_primary))])
                .collect()
        };
        for (li, spans) in msg_lines.iter().enumerate() {
            let y = row + 1 + li as u16;
            if y >= area.bottom() {
                break;
            }
            let spans: Vec<Span<'static>> = spans
                .iter()
                .map(|sp| {
                    let mut s = sp.clone();
                    if is_selected {
                        s = s.style(sel_style);
                    } else if s.style.fg.is_none() {
                        s = s.style(Style::default().fg(theme.fg_primary));
                    }
                    s
                })
                .collect();
            frame.render_widget(
                Paragraph::new(Line::from(spans)),
                Rect {
                    x: text_x,
                    y,
                    width: text_width,
                    height: 1,
                },
            );
        }

        let action_y = row + 1 + line_count as u16;
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
            let mut spans = vec![Span::styled(
                reply.format_time_absolute(),
                Style::default().fg(theme.fg_muted),
            )];
            if let Some(loc) = reply.ip_location() {
                spans.push(Span::styled(
                    format!(" · IP{}", loc),
                    Style::default().fg(theme.fg_muted),
                ));
            }
            spans.push(Span::styled(
                format!("  {} ", like_icon),
                Style::default().fg(like_color),
            ));
            spans.push(Span::styled(
                format_count(self.like_count(reply)),
                Style::default().fg(like_color),
            ));
            let action = Line::from(spans).style(if is_selected {
                sel_style
            } else {
                Style::default()
            });
            frame.render_widget(
                Paragraph::new(action),
                Rect {
                    x: text_x,
                    y: action_y,
                    width: text_width,
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

        // Floor pager row (reply_index == 2): 上一页 | 第x/y页 | 下一页 (+加载更多)
        if entry.reply_index == 2 {
            let total_fetched = self.replies.get(&comment.rpid).map_or(0, |r| r.len());
            let has_more_server = comment.reply_count() as usize > total_fetched;
            let mut spans = Vec::new();
            match self.reply_page_info(comment.rpid) {
                Some((page, pages)) if pages > 1 => {
                    spans.push(Span::styled(
                        format!("{} 上一页 ", icons::LEFT_ARROW),
                        Style::default().fg(if page > 1 {
                            theme.bilibili_blue
                        } else {
                            theme.fg_muted
                        }),
                    ));
                    spans.push(Span::styled(
                        format!(" {}/{} ", page, pages),
                        Style::default().fg(theme.fg_muted),
                    ));
                    spans.push(Span::styled(
                        format!("下一页 {}", icons::RIGHT_ARROW),
                        Style::default().fg(if page < pages {
                            theme.bilibili_blue
                        } else {
                            theme.fg_muted
                        }),
                    ));
                }
                _ => {
                    // single page: offer server load-more only when the
                    // direct reply count really exceeds what we have
                    if has_more_server {
                        spans.push(Span::styled(
                            format!(
                                "{} 加载更多回复 ",
                                if self.loading_more_replies {
                                    icons::SPINNER
                                } else {
                                    icons::DOWNLOAD
                                }
                            ),
                            Style::default().fg(theme.bilibili_blue),
                        ));
                    } else {
                        spans.push(Span::styled(
                            format!("共 {} 条回复", comment.reply_count()),
                            Style::default().fg(theme.fg_muted),
                        ));
                    }
                }
            }
            if has_more_server {
                spans.push(Span::styled(
                    format!("  {} 服务器还有更多", icons::DOWNLOAD),
                    Style::default().fg(theme.fg_muted),
                ));
            }
            let line = Line::from(spans).style(if is_selected {
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
            return;
        }

        let (label, icon, color) = if entry.reply_index == 3 {
            (
                "‹ 返回评论列表".to_string(),
                icons::LEFT_ARROW,
                theme.bilibili_blue,
            )
        } else if self.expanded.contains(&comment.rpid) {
            ("收起回复".to_string(), icons::FOLD_OPEN, theme.fg_muted)
        } else if self.loading_replies_for == Some(comment.rpid) {
            ("加载回复中...".to_string(), icons::SPINNER, theme.warning)
        } else {
            (
                format!("共{}条回复，点击查看", comment.reply_count()),
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
    fn message_segments_split_known_emotes() {
        let item: crate::api::comment::CommentItem = serde_json::from_value(serde_json::json!({
            "rpid": 1, "oid": 1, "mid": 2, "parent": 0,
            "content": {
                "message": "好活[大哭]当赏[大哭]持续关注",
                "emote": {"[大哭]": {"text": "[大哭]", "url": "https://i0.hdslb.com/bfs/emote/1.png"}}
            }
        }))
        .unwrap();
        let segs = item.message_segments();
        assert_eq!(
            segs,
            vec![
                crate::api::comment::Segment::Text("好活"),
                crate::api::comment::Segment::Emote("[大哭]"),
                crate::api::comment::Segment::Text("当赏"),
                crate::api::comment::Segment::Emote("[大哭]"),
                crate::api::comment::Segment::Text("持续关注"),
            ]
        );
    }

    #[test]
    fn unknown_brackets_stay_plain_text() {
        let item: crate::api::comment::CommentItem = serde_json::from_value(serde_json::json!({
            "rpid": 1, "oid": 1, "mid": 2, "parent": 0,
            "content": {"message": "笑死[不存在的]"}
        }))
        .unwrap();
        assert_eq!(
            item.message_segments(),
            vec![crate::api::comment::Segment::Text("笑死[不存在的]")]
        );
    }

    #[test]
    fn sort_label_follows_state() {
        let mut list = CommentList::new();
        assert_eq!(list.sort_label(), "最热");
        list.sort_newest = true;
        assert_eq!(list.sort_label(), "最新");
    }

    #[test]
    fn avatar_loader_keys_by_author_not_index() {
        use crate::api::comment::CommentMember;
        let member_a = CommentMember {
            mid: Some("100".into()),
            uname: Some("甲".into()),
            avatar: None,
            level_info: None,
        };
        let member_b = CommentMember {
            mid: Some("200".into()),
            uname: Some("乙".into()),
            avatar: None,
            level_info: None,
        };
        let key_a = author_key(Some(&member_a)).unwrap();
        let key_b = author_key(Some(&member_b)).unwrap();
        // Different authors never collide, so a refreshed/reordered list
        // cannot show 甲's avatar under 乙's name.
        assert_ne!(key_a, key_b);
        // Same author keeps the same key across reloads.
        assert_eq!(key_a, author_key(Some(&member_a)).unwrap());
        // Missing identity yields no key (placeholder glyph is used).
        let anon = CommentMember {
            mid: None,
            uname: None,
            avatar: None,
            level_info: None,
        };
        assert_eq!(author_key(Some(&anon)), None);
    }

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

    use crate::api::comment::{CommentContent, CommentMember};

    fn item(rpid: i64, parent: i64, msg: &str, rcount: Option<i32>) -> CommentItem {
        CommentItem {
            rpid,
            oid: 1,
            mid: 100 + rpid,
            parent,
            count: None,
            rcount,
            floor: None,
            ctime: Some(1_700_000_000),
            like: Some(0),
            member: Some(CommentMember {
                mid: Some((100 + rpid).to_string()),
                uname: Some(format!("用户{rpid}")),
                avatar: None,
                level_info: None,
            }),
            content: Some(CommentContent {
                message: Some(msg.into()),
                emote: None,
            }),
            reply_control: None,
            replies: None,
        }
    }

    #[test]
    fn sub_thread_renders_focus_and_children() {
        let mut list = CommentList::new();
        list.set_comments(vec![item(1, 0, "主评论", Some(2))], 1);
        // floor replies under the root, second one has children
        list.replies.insert(
            1,
            vec![
                item(11, 1, "一楼回复", Some(1)),
                item(12, 1, "二楼回复", None),
            ],
        );
        list.sub_replies.insert(
            12,
            vec![
                item(21, 12, "子回复甲", None),
                item(22, 12, "子回复乙", None),
            ],
        );

        list.enter_sub_thread(1, 12);
        assert!(list.in_sub_thread());

        // build at a width where messages fit one line
        list.build_entries(80);
        let kinds: Vec<EntryKind> = list.entries.iter().map(|e| e.kind).collect();
        // focus reply, two children, then the back row
        assert_eq!(
            kinds,
            vec![
                EntryKind::Reply,
                EntryKind::SubReply,
                EntryKind::SubReply,
                EntryKind::Toggle
            ]
        );
        // focus row is the usize::MAX marker
        assert_eq!(list.entries[0].reply_index, usize::MAX);

        list.leave_sub_thread();
        assert!(!list.in_sub_thread());
    }

    #[test]
    fn selected_reply_pos_only_for_floor_replies() {
        let mut list = CommentList::new();
        list.set_comments(vec![item(1, 0, "主评论", Some(1))], 1);
        list.replies.insert(1, vec![item(11, 1, "回复", None)]);
        list.expanded.insert(1);
        list.build_entries(80);

        // walk entries; a Reply entry exposes (comment, reply), a Comment does not
        let mut saw_reply = false;
        for i in 0..list.entries.len() {
            list.selected_entry = i;
            if let Some((ci, ri)) = list.selected_reply_pos() {
                assert_eq!(ci, 0);
                assert_eq!(ri, 0);
                saw_reply = true;
            }
        }
        assert!(saw_reply);
    }
}
