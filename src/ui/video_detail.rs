//! Video detail page showing video info, comments, and related videos

use super::comment_list::CommentList;
use super::comment_list::{CommentIntent, EntryKind};
use super::icons;
use super::video_card::{VideoCard, VideoCardGrid};
use super::{Component, Theme, panel_block, panel_block_bg, shortcut_footer};
use crate::api::client::ApiClient;
use crate::api::video::{RelatedVideoItem, VideoInfo};
use crate::application::AppAction;
use crate::storage::Keybindings;
use ratatui::{
    crossterm::event::{KeyCode, MouseButton, MouseEvent, MouseEventKind},
    prelude::*,
    widgets::*,
};
use std::collections::HashSet;
use std::time::Instant;

#[derive(Clone, Copy, PartialEq)]
pub enum DetailFocus {
    Comments,
    Episodes,
    Related,
}

pub struct VideoDetailPage {
    pub bvid: String,
    pub aid: i64,
    pub video_info: Option<VideoInfo>,
    /// Web-style comment list state (selection, layout, avatars).
    pub comment_list: CommentList,
    pub related_videos: Vec<RelatedVideoItem>,
    pub related_card_grid: VideoCardGrid,
    pub loading: bool,
    pub error_message: Option<String>,
    pub comment_page: i32,
    pub related_scroll: usize,
    pub focus: DetailFocus,
    pub loading_more_comments: bool,
    /// Floor-page turn that needs a server fetch first (comment_index, dir).
    pub pending_reply_page: Option<(usize, i32)>,
    /// UP avatar/follower info for the video info header.
    pub up_avatar: crate::ui::comment_list::AvatarLoader,
    pub up_follower: Option<i64>,
    /// Whether the logged-in user follows this UP (None = unknown).
    pub following: Option<bool>,
    pub follow_in_flight: bool,
    pub liked_comments: HashSet<i64>,
    pub input_mode: bool,
    pub input_buffer: String,
    last_click_time: Option<Instant>,
    last_click_index: Option<usize>,
    /// Line of last click (comment double-click detection).
    last_click_line: Option<usize>,
    /// Current episode index for multi-part videos (0-based)
    pub current_page_index: usize,
    /// Scroll position in episode list
    pub episode_scroll: usize,
    pub auto_play_pending: bool,
}

impl VideoDetailPage {
    pub fn new(bvid: String, aid: i64) -> Self {
        let mut related_card_grid = VideoCardGrid::new_list();
        related_card_grid.columns = 1;
        related_card_grid.card_height = 8;

        Self {
            bvid,
            aid,
            video_info: None,
            comment_list: CommentList::new(),
            related_videos: Vec::new(),
            related_card_grid,
            loading: true,
            error_message: None,
            comment_page: 1,
            related_scroll: 0,
            focus: DetailFocus::Comments,
            loading_more_comments: false,
            pending_reply_page: None,
            up_avatar: crate::ui::comment_list::AvatarLoader::new(),
            up_follower: None,
            following: None,
            follow_in_flight: false,
            liked_comments: HashSet::new(),
            input_mode: false,
            input_buffer: String::new(),
            last_click_time: None,
            last_click_index: None,
            last_click_line: None,
            current_page_index: 0,
            episode_scroll: 0,
            auto_play_pending: true,
        }
    }

    pub fn play_action(&self) -> AppAction {
        if let Some(pages) = self.get_pages() {
            if pages.len() > 1 {
                return AppAction::PlayVideoWithPages {
                    bvid: self.bvid.clone(),
                    aid: self.aid,
                    pages: pages.clone(),
                    current_index: self.current_page_index,
                };
            }
            if let Some(page) = pages.first() {
                return AppAction::PlayVideo {
                    bvid: self.bvid.clone(),
                    aid: self.aid,
                    cid: page.cid,
                    duration: page.duration,
                };
            }
        }
        let (cid, duration) = self
            .video_info
            .as_ref()
            .map_or((0, 0), |info| (info.cid, info.duration.unwrap_or(0)));
        AppAction::PlayVideo {
            bvid: self.bvid.clone(),
            aid: self.aid,
            cid,
            duration,
        }
    }

    pub async fn load_data(&mut self, api_client: &ApiClient) {
        self.loading = true;
        self.error_message = None;

        // Load video info
        match api_client.get_video_info(&self.bvid).await {
            Ok(info) => {
                self.comment_list.uploader_mid = Some(info.owner.mid);
                // UP header extras: avatar + follower count (best effort)
                let face = info.owner.face.clone();
                let mid = info.owner.mid;
                let name = info.owner.name.clone();
                self.up_avatar
                    .request(std::iter::once(((Some(mid), name.clone()), Some(face))));
                if let Ok(stat) = api_client.get_relation_stat(mid).await {
                    self.up_follower = stat.follower;
                }
                if let Ok(following) = api_client.is_following(mid).await {
                    self.following = Some(following);
                }
                self.video_info = Some(info);
            }
            Err(e) => {
                self.error_message = Some(format!("加载视频信息失败: {}", e));
            }
        }

        // Load comments
        match api_client.get_comments(self.aid, 1).await {
            Ok(data) => {
                let total = data
                    .page
                    .as_ref()
                    .and_then(|p| p.acount.or(p.count))
                    .unwrap_or(0) as i64;
                self.comment_list
                    .set_comments(data.replies.unwrap_or_default(), total);
                self.comment_page = 1;
            }
            Err(e) => {
                if self.error_message.is_none() {
                    self.error_message = Some(format!("加载评论失败: {}", e));
                }
            }
        }

        // Load related videos
        match api_client.get_related_videos(&self.bvid).await {
            Ok(videos) => {
                self.related_videos = videos.clone();
                // Populate video card grid
                self.related_card_grid.clear();
                for video in &videos {
                    let card = VideoCard::new(
                        video.bvid.clone(),
                        video.aid,
                        video.title.clone().unwrap_or_else(|| "无标题".to_string()),
                        video.author_name().to_string(),
                        video.format_views(),
                        video.format_duration(),
                        video.cover_url(),
                    )
                    .with_uploader_mid(video.owner.as_ref().and_then(|owner| owner.mid));
                    self.related_card_grid.add_card(card);
                }
            }
            Err(e) => {
                if self.error_message.is_none() {
                    self.error_message = Some(format!("加载相关视频失败: {}", e));
                }
            }
        }

        self.loading = false;
    }

    pub async fn load_more_comments(&mut self, api_client: &ApiClient) {
        if !self.comment_list.has_more || self.loading_more_comments {
            return;
        }

        self.loading_more_comments = true;
        self.comment_page += 1;
        match api_client.get_comments(self.aid, self.comment_page).await {
            Ok(data) => {
                let replies = data.replies.unwrap_or_default();
                if replies.is_empty() {
                    self.comment_list.has_more = false;
                } else {
                    self.comment_list.append_comments(replies);
                }
            }
            Err(_) => {
                self.comment_page -= 1;
                self.comment_list.has_more = false;
            }
        }
        self.loading_more_comments = false;
    }

    /// Open the APP-style conversation of the floor reply `reply_index`
    /// under comment `comment_index`: fetch its children and switch view.
    pub async fn open_sub_thread(
        &mut self,
        comment_index: usize,
        reply_index: usize,
        api_client: &ApiClient,
    ) {
        let Some(comment) = self.comment_list.comments.get(comment_index) else {
            return;
        };
        let root_rpid = comment.rpid;
        let Some(reply) = self
            .comment_list
            .replies
            .get(&root_rpid)
            .and_then(|rs| rs.get(reply_index))
            .cloned()
        else {
            return;
        };
        let focus_rpid = reply.rpid;
        if !self.comment_list.sub_replies.contains_key(&focus_rpid) {
            // children live one level deeper via the same reply/reply API
            if let Ok(data) = api_client
                .get_comment_replies(self.aid, focus_rpid, 1)
                .await
            {
                let children = data.replies.unwrap_or_default();
                self.comment_list.set_sub_replies(focus_rpid, children);
            }
        }
        self.comment_list.enter_sub_thread(root_rpid, focus_rpid);
    }

    pub async fn toggle_comment_replies(&mut self, api_client: &ApiClient) {
        let Some(comment) = self.comment_list.selected_comment() else {
            return;
        };
        let comment_rpid = comment.rpid;
        let reply_count = comment.reply_count();

        // Already expanded -> collapse
        if self.comment_list.expanded.contains(&comment_rpid) {
            self.comment_list.collapse(comment_rpid);
            return;
        }

        // Cached replies -> expand instantly
        if self.comment_list.replies.contains_key(&comment_rpid) {
            let cached = self
                .comment_list
                .replies
                .get(&comment_rpid)
                .cloned()
                .unwrap_or_default();
            self.comment_list.set_replies(comment_rpid, cached);
            return;
        }

        if reply_count == 0 {
            return;
        }

        // Expand and load replies
        self.comment_list.set_loading_replies(comment_rpid);

        match api_client
            .get_comment_replies(self.aid, comment_rpid, 1)
            .await
        {
            Ok(data) => {
                self.comment_list
                    .set_replies(comment_rpid, data.replies.unwrap_or_default());
            }
            Err(_) => {
                self.comment_list.reply_failed(comment_rpid);
            }
        }
    }

    /// Turn the floor page of replies for comment `comment_index`.
    /// Paging past fetched replies schedules a server fetch first.
    pub fn page_comment_replies(&mut self, comment_index: usize, dir: i32) {
        let Some(comment) = self.comment_list.comments.get(comment_index) else {
            return;
        };
        let root_rpid = comment.rpid;
        let total_fetched = self
            .comment_list
            .replies
            .get(&root_rpid)
            .map_or(0, |r| r.len());
        let pages_fetched = total_fetched
            .div_ceil(crate::ui::comment_list::REPLIES_PER_PAGE)
            .max(1);
        let Some((page, _pages)) = self.comment_list.reply_page_info(root_rpid) else {
            return;
        };
        if dir > 0 && page + 1 > pages_fetched {
            // need to fetch the next server page first; keep as pending
            self.pending_reply_page = Some((comment_index, dir));
        } else {
            self.comment_list.page_replies(root_rpid, dir);
        }
    }

    pub async fn load_more_replies_at(&mut self, comment_index: usize, api_client: &ApiClient) {
        let Some(comment) = self.comment_list.comments.get(comment_index) else {
            return;
        };
        let root_rpid = comment.rpid;
        let next_page = self
            .comment_list
            .replies
            .get(&root_rpid)
            .map(|r| r.len() / 20 + 1)
            .unwrap_or(1);
        self.comment_list.loading_more_replies = true;
        match api_client
            .get_comment_replies(self.aid, root_rpid, next_page as i32)
            .await
        {
            Ok(data) => {
                let mut existing = self
                    .comment_list
                    .replies
                    .remove(&root_rpid)
                    .unwrap_or_default();
                existing.extend(data.replies.unwrap_or_default());
                self.comment_list.set_replies(root_rpid, existing);
                // resume a floor-page turn that was waiting for this fetch
                if let Some((pending_idx, dir)) = self.pending_reply_page.take()
                    && pending_idx == comment_index
                {
                    self.comment_list.page_replies(root_rpid, dir);
                }
            }
            Err(_) => {
                self.pending_reply_page = None;
                self.comment_list.reply_failed(root_rpid);
            }
        }
    }

    /// Poll for completed related video cover downloads
    pub fn poll_cover_results(&mut self) {
        self.related_card_grid.poll_cover_results();
        let _ = self.up_avatar.poll();
    }

    /// Start background downloads for visible related video covers
    pub fn start_cover_downloads(&mut self) {
        self.related_card_grid.start_cover_downloads();
    }

    fn render_video_info(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let block = panel_block_bg(
            theme,
            Some(Line::from(Span::styled(
                format!(" {} 视频信息 ", icons::PLAY),
                Style::default().fg(theme.bilibili_pink),
            ))),
            false,
            theme.bg_secondary,
        );

        let inner = block.inner(area);
        frame.render_widget(block, area);

        if let Some(ref info) = self.video_info {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1), // Title
                    Constraint::Length(1), // UP line
                    Constraint::Length(1), // Stats
                    Constraint::Min(1),    // Description
                ])
                .split(inner);

            // Title
            let title = Paragraph::new(info.title.clone()).style(
                Style::default()
                    .fg(theme.fg_primary)
                    .add_modifier(Modifier::BOLD),
            );
            frame.render_widget(title, chunks[0]);

            // UP name + u 主页 hint on one row (web shows plain text here)
            let author = Paragraph::new(Line::from(vec![
                Span::styled("UP ", Style::default().fg(theme.fg_muted)),
                Span::styled(
                    info.owner.name.clone(),
                    Style::default()
                        .fg(theme.bilibili_blue)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!(
                        "  ·  u 主页{}",
                        match self.up_follower {
                            Some(f) =>
                                format!("  ·  {} 粉丝", crate::ui::comment_list::format_count(f)),
                            None => String::new(),
                        }
                    ),
                    Style::default().fg(theme.fg_muted),
                ),
            ]));
            frame.render_widget(author, chunks[1]);

            // Stats
            let stats = Paragraph::new(Line::from(vec![
                Span::styled(
                    format!("{} ", icons::PLAY),
                    Style::default().fg(theme.fg_secondary),
                ),
                Span::styled(
                    info.stat.format_views(),
                    Style::default().fg(theme.fg_secondary),
                ),
                Span::styled(
                    format!("  {} ", icons::DANMAKU),
                    Style::default().fg(theme.fg_secondary),
                ),
                Span::styled(
                    info.stat.format_danmaku(),
                    Style::default().fg(theme.fg_secondary),
                ),
                Span::styled(
                    format!("  {} ", icons::LIKE),
                    Style::default().fg(theme.fg_secondary),
                ),
                Span::styled(
                    info.stat.format_like(),
                    Style::default().fg(theme.fg_secondary),
                ),
                Span::styled(
                    format!("  {} ", icons::COIN),
                    Style::default().fg(theme.fg_secondary),
                ),
                Span::styled(
                    info.stat.format_coin(),
                    Style::default().fg(theme.fg_secondary),
                ),
                Span::styled(
                    format!("  {} ", icons::STAR),
                    Style::default().fg(theme.fg_secondary),
                ),
                Span::styled(
                    info.stat.format_favorite(),
                    Style::default().fg(theme.fg_secondary),
                ),
            ]));
            frame.render_widget(stats, chunks[2]);

            // Description (dimmed, single line)
            let desc = info
                .desc
                .as_deref()
                .unwrap_or("")
                .lines()
                .next()
                .unwrap_or("")
                .to_string();
            let desc = Paragraph::new(crate::ui::truncate_chars(
                desc.trim(),
                (chunks[3].width as usize).saturating_sub(1),
            ))
            .style(Style::default().fg(theme.fg_muted));
            frame.render_widget(desc, chunks[3]);
        }
    }

    fn render_comments(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let is_focused = self.focus == DetailFocus::Comments;
        let total = self.comment_list.comments.len();
        let more_hint = if self.comment_list.has_more { "+" } else { "" };
        let sort_label = self.comment_list.sort_label();
        let block = panel_block_bg(
            theme,
            Some(Line::from(vec![
                Span::styled(
                    format!(" 评论 {}{} ", total, more_hint),
                    Style::default()
                        .fg(theme.fg_primary)
                        .add_modifier(Modifier::BOLD),
                ),
                // text sort tabs: unselected first, selected marked bold+pink
                Span::styled(
                    if sort_label == "最热" {
                        " 最热"
                    } else {
                        "  最热"
                    },
                    Style::default().fg(if sort_label == "最热" {
                        theme.bilibili_pink
                    } else {
                        theme.fg_muted
                    }),
                ),
                Span::styled(" | ", Style::default().fg(theme.border_subtle)),
                Span::styled(
                    if sort_label == "最新" {
                        "最新"
                    } else {
                        " 最新"
                    },
                    Style::default().fg(if sort_label == "最新" {
                        theme.bilibili_pink
                    } else {
                        theme.fg_muted
                    }),
                ),
                Span::styled("  (t切换)", Style::default().fg(theme.fg_muted)),
            ])),
            is_focused,
            theme.bg_secondary, // comments sit one step darker than related
        );

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Gap + faint divider under the panel title, then the list below it.
        let head = Rect { height: 1, ..inner };
        let rule = Rect {
            y: inner.y + 1,
            height: 1,
            ..inner
        };
        let list_area = Rect {
            y: inner.y + 2,
            width: inner.width.saturating_sub(1),
            height: inner.height.saturating_sub(2),
            ..inner
        };
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "─".repeat(head.width as usize),
                Style::default().fg(theme.border_subtle),
            ))),
            rule,
        );
        self.comment_list
            .render(frame, list_area, theme, is_focused);
    }

    /// Right rail: UP card on top (web order), then episodes + related.
    fn render_right_column(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(7),      // UP card
                Constraint::Percentage(40), // episodes (if any)
                Constraint::Percentage(60), // related fills the rest
            ])
            .split(area);

        self.render_up_card(frame, rows[0], theme);

        if self.has_multiple_pages() {
            self.render_episodes(frame, rows[1], theme);
            // related stretches from below episodes to the window bottom
            let rest = Rect {
                y: rows[2].y,
                height: area.bottom().saturating_sub(rows[2].y),
                ..rows[2]
            };
            self.render_related(frame, rest, theme);
        } else {
            // no episodes: related takes everything below the UP card
            let rest = Rect {
                y: rows[1].y,
                height: area.bottom().saturating_sub(rows[1].y),
                ..rows[1]
            };
            self.render_related(frame, rest, theme);
        }
    }

    /// UP card: avatar left, name + follower + follow button right (web).
    fn render_up_card(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let block = panel_block_bg(theme, None, false, theme.bg_card);
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let Some(info) = self.video_info.as_ref() else {
            return;
        };
        let mid = info.owner.mid;
        let name = info.owner.name.clone();
        let face = info.owner.face.clone();

        let face_key = (Some(mid), name.clone());
        if self.up_avatar.get(&face_key).is_none() {
            // re-request from the render loop until the picker is ready
            self.up_avatar
                .request(std::iter::once((face_key.clone(), Some(face))));
        }
        let avatar_ready = self.up_avatar.get(&face_key).is_some();

        // avatar column (6 wide) + text column; cells are ~1:2 so a square
        // face needs cols = 2 x rows.
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(7), Constraint::Min(10)])
            .split(inner);
        if avatar_ready && let Some(protocol) = self.up_avatar.get_mut(&face_key) {
            let avatar_area = Rect {
                width: 6.min(cols[0].width),
                height: 3.min(cols[0].height),
                ..cols[0]
            };
            frame.render_stateful_widget(
                ratatui_image::StatefulImage::default().resize(ratatui_image::Resize::Scale(None)),
                avatar_area,
                protocol,
            );
        } else {
            let ph = Paragraph::new(icons::USER)
                .style(Style::default().fg(theme.fg_muted))
                .alignment(Alignment::Center);
            frame.render_widget(ph, cols[0]);
        }

        let text_rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // name
                Constraint::Length(1), // follower
                Constraint::Length(1), // gap
                Constraint::Length(1), // follow button
            ])
            .split(cols[1]);

        let name_p = Paragraph::new(Span::styled(
            super::truncate_chars(&name, (cols[1].width as usize).saturating_sub(2)),
            Style::default()
                .fg(theme.bilibili_blue)
                .add_modifier(Modifier::BOLD),
        ));
        frame.render_widget(name_p, text_rows[0]);

        let fans = match self.up_follower {
            Some(f) => format!("{} 粉丝", crate::ui::comment_list::format_count(f)),
            None => String::new(),
        };
        let fans_p = Paragraph::new(Span::styled(fans, Style::default().fg(theme.fg_muted)));
        frame.render_widget(fans_p, text_rows[1]);

        let (label, fg, bg) = match self.following {
            Some(true) => (
                " 已关注 ".to_string(),
                theme.fg_secondary,
                theme.bg_secondary,
            ),
            _ => (
                " + 关注 ".to_string(),
                theme.fg_primary,
                theme.bilibili_pink,
            ),
        };
        let btn_w = 10.min(cols[1].width);
        let btn = Paragraph::new(Line::from(Span::styled(
            label,
            Style::default().fg(fg).bg(bg).add_modifier(Modifier::BOLD),
        )));
        let btn_area = Rect {
            x: cols[1].x,
            y: text_rows[3].y,
            width: btn_w,
            height: 1,
        };
        frame.render_widget(btn, btn_area);
    }

    fn render_related(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let is_focused = self.focus == DetailFocus::Related;
        let block = panel_block_bg(
            theme,
            Some(Line::from(Span::styled(
                format!(" {} 相关推荐 ", icons::TV),
                Style::default().fg(if is_focused {
                    theme.bilibili_pink
                } else {
                    theme.fg_muted
                }),
            ))),
            is_focused,
            theme.bg_card,
        );

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Breathing room below the title, then list.
        let list_area = Rect {
            y: inner.y + 1,
            height: inner.height.saturating_sub(1),
            ..inner
        };
        self.render_related_list(frame, list_area, theme);
    }

    fn render_related_list(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let list_area = area;
        if self.related_card_grid.cards.is_empty() {
            let empty = Paragraph::new("暂无相关视频")
                .style(Style::default().fg(theme.fg_secondary))
                .alignment(Alignment::Center);
            frame.render_widget(empty, list_area);
            return;
        }

        // Sync scroll position with grid
        self.related_card_grid.selected_index = self.related_scroll;

        // Render the video card grid
        self.related_card_grid.render(frame, list_area, theme);
    }

    /// Check if video has multiple parts
    fn has_multiple_pages(&self) -> bool {
        self.video_info
            .as_ref()
            .and_then(|info| info.pages.as_ref())
            .map(|pages| pages.len() > 1)
            .unwrap_or(false)
    }

    /// Get the video pages
    fn get_pages(&self) -> Option<&Vec<crate::api::video::VideoPage>> {
        self.video_info
            .as_ref()
            .and_then(|info| info.pages.as_ref())
    }

    fn render_episodes(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let is_focused = self.focus == DetailFocus::Episodes;
        let pages = match self.get_pages() {
            Some(p) => p,
            None => return,
        };

        let block = panel_block_bg(
            theme,
            Some(Line::from(Span::styled(
                format!(" {} 选集 ({}) ", icons::LIST, pages.len()),
                Style::default().fg(if is_focused {
                    theme.bilibili_pink
                } else {
                    theme.fg_muted
                }),
            ))),
            is_focused,
            theme.bg_modal,
        );

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let visible_count = inner.height as usize;
        let scroll_offset = if self.episode_scroll >= visible_count {
            self.episode_scroll - visible_count + 1
        } else {
            0
        };

        let items: Vec<ListItem> = pages
            .iter()
            .enumerate()
            .skip(scroll_offset)
            .take(visible_count)
            .map(|(idx, page)| {
                let is_current = idx == self.current_page_index;
                let is_selected = idx == self.episode_scroll;

                // Format duration as mm:ss
                let duration = {
                    let mins = page.duration / 60;
                    let secs = page.duration % 60;
                    format!("{:02}:{:02}", mins, secs)
                };

                let prefix = if is_current { "▶ " } else { "  " };
                let title = truncate_str(&page.part, 30);

                let style = if is_selected && is_focused {
                    Style::default()
                        .fg(theme.bilibili_pink)
                        .add_modifier(Modifier::BOLD)
                } else if is_current {
                    Style::default().fg(theme.fg_accent)
                } else {
                    Style::default().fg(theme.fg_primary)
                };

                ListItem::new(Line::from(vec![
                    Span::styled(prefix, style),
                    Span::styled(
                        format!("P{} ", page.page),
                        Style::default().fg(theme.fg_secondary),
                    ),
                    Span::styled(title, style),
                    Span::styled(
                        format!("  {}", duration),
                        Style::default().fg(theme.fg_muted),
                    ),
                ]))
            })
            .collect();

        let list = List::new(items);
        frame.render_widget(list, inner);
    }
}

impl Component for VideoDetailPage {
    fn draw(&mut self, frame: &mut Frame, area: Rect, theme: &Theme, keys: &Keybindings) {
        // Paint the whole page in the panel surface tone first so any gap
        // between panels shows the page background, never the terminal bg.
        frame.render_widget(
            Block::default().style(Style::default().bg(theme.bg_secondary)),
            area,
        );
        // Web layout: LEFT = info + comments; RIGHT = UP card + episodes +
        // related. The comment panel stretches to the window bottom and the
        // shortcut row overlays its lower padding.
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(40), Constraint::Length(56)])
            .split(area);

        if self.input_mode {
            // Input mode: info top, editor + hints below (left column only).
            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(6), // video info
                    Constraint::Length(3), // editor
                    Constraint::Length(3), // hints
                    Constraint::Min(0),
                ])
                .split(columns[0]);
            self.render_video_info(frame, rows[0], theme);
            let input_block = Block::default()
                .style(Style::default().bg(theme.bg_secondary))
                .title(Span::styled(
                    format!(" {} 发表评论 ", icons::EDIT),
                    Style::default()
                        .fg(theme.bilibili_pink)
                        .add_modifier(Modifier::BOLD),
                ));
            let input_text = format!("{}_", self.input_buffer);
            let input = Paragraph::new(input_text)
                .style(Style::default().fg(theme.fg_primary))
                .block(input_block);
            frame.render_widget(input, rows[1]);
            let help = shortcut_footer(
                theme,
                [
                    (keys.confirm.clone(), "发送评论".into(), theme.success),
                    (keys.back.clone(), "取消".into(), theme.info),
                ],
            );
            let help = Paragraph::new(help)
                .alignment(Alignment::Center)
                .block(Block::default().padding(ratatui::widgets::Padding::new(0, 0, 1, 0)));
            frame.render_widget(help, rows[2]);
            // right column keeps its content
            self.render_right_column(frame, columns[1], theme);
            return;
        }

        let left_rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(6), // video info
                Constraint::Min(10),   // comments (to the window bottom)
            ])
            .split(columns[0]);

        self.render_video_info(frame, left_rows[0], theme);

        if self.loading {
            let loading = Paragraph::new("⏳ 加载中...")
                .style(Style::default().fg(theme.warning))
                .alignment(Alignment::Center)
                .block(panel_block(theme, None, false));
            frame.render_widget(loading, left_rows[1]);
        } else if let Some(error) = &self.error_message {
            let error_widget = Paragraph::new(format!("{} {}", icons::ERROR, error))
                .style(Style::default().fg(theme.error))
                .alignment(Alignment::Center)
                .block(panel_block(theme, None, false));
            frame.render_widget(error_widget, left_rows[1]);
        } else {
            // Reserve the bottom 3 rows for the shortcut bar: the comment
            // list shortens so content never slides underneath the bar.
            let panel_area = left_rows[1];
            let list_area = Rect {
                height: panel_area.height.saturating_sub(3),
                ..panel_area
            };
            self.render_comments(frame, list_area, theme);

            let footer_area = Rect {
                x: panel_area.x + 1,
                y: area.bottom().saturating_sub(3),
                width: panel_area.width.saturating_sub(2),
                height: 3,
            };
            let in_thread = self.comment_list.sub_thread.is_some();
            let mut items = vec![
                (
                    format!("{}/{}", keys.nav_up, keys.nav_down),
                    "选择".into(),
                    theme.fg_accent,
                ),
                ("Space".into(), "展开回复".into(), theme.success),
                ("r".into(), "点赞".into(), theme.warning),
                ("c".into(), "评论".into(), theme.info),
                ("t".into(), "最热/最新".into(), theme.info),
            ];
            if in_thread {
                items.push(("Esc".into(), "退出对话".into(), theme.bilibili_pink));
            } else {
                items.push(("v".into(), "查看对话".into(), theme.bilibili_blue));
            }
            items.push(("u".into(), "UP主页".into(), theme.bilibili_blue));
            items.push(("f".into(), "关注".into(), theme.bilibili_pink));
            items.push((keys.play.clone(), "播放".into(), theme.success));
            items.push((keys.back.clone(), "返回".into(), theme.info));
            let help = shortcut_footer(theme, items);
            let help = Paragraph::new(help)
                .alignment(Alignment::Center)
                .block(Block::default().style(Style::default().bg(theme.bg_secondary)));
            frame.render_widget(help, footer_area);
        }

        self.render_right_column(frame, columns[1], theme);
    }

    fn handle_input(
        &mut self,
        key: KeyCode,
        keys: &crate::storage::Keybindings,
    ) -> Option<AppAction> {
        // Handle input mode for adding comments
        if self.input_mode {
            match key {
                KeyCode::Esc => {
                    self.input_mode = false;
                    self.input_buffer.clear();
                    return Some(AppAction::None);
                }
                KeyCode::Enter => {
                    if !self.input_buffer.is_empty() {
                        let message = self.input_buffer.clone();
                        self.input_buffer.clear();
                        self.input_mode = false;
                        return Some(AppAction::AddComment {
                            oid: self.aid,
                            comment_type: 1, // Video comment type
                            message,
                            root: None,
                        });
                    }
                    return Some(AppAction::None);
                }
                KeyCode::Backspace => {
                    self.input_buffer.pop();
                    return Some(AppAction::None);
                }
                KeyCode::Char(c) => {
                    self.input_buffer.push(c);
                    return Some(AppAction::None);
                }
                _ => return Some(AppAction::None),
            }
        }

        if keys.matches_quit(key) || keys.matches_back(key) {
            if self.comment_list.in_sub_thread() {
                self.comment_list.leave_sub_thread();
                return Some(AppAction::None);
            }
            return Some(AppAction::BackToList);
        }
        if key == KeyCode::Char('u')
            && let Some(info) = &self.video_info
        {
            return Some(AppAction::OpenUpPage(info.owner.mid));
        }
        if key == KeyCode::Char('f')
            && let Some(info) = &self.video_info
            && !self.follow_in_flight
        {
            self.follow_in_flight = true;
            return Some(AppAction::ToggleFollowUp {
                mid: info.owner.mid,
            });
        }
        if keys.matches_play(key) {
            return Some(self.play_action());
        }
        if keys.matches_comment(key) {
            // Enter comment input mode
            self.input_mode = true;
            self.input_buffer.clear();
            return Some(AppAction::None);
        }
        // 'v': APP-style conversation view of the selected floor reply.
        if key == KeyCode::Char('v')
            && self.focus == DetailFocus::Comments
            && !self.comment_list.in_sub_thread()
        {
            if let Some((ci, ri)) = self.comment_list.selected_reply_pos() {
                return Some(AppAction::OpenSubThread {
                    comment_index: ci,
                    reply_index: ri,
                });
            }
            return Some(AppAction::None);
        }
        if keys.matches_toggle_replies(key) {
            if self.focus == DetailFocus::Comments
                && let Some(intent) = self.comment_list.activate_selected()
            {
                return comment_intent_to_action(intent, self.aid);
            }
            return Some(AppAction::None);
        }
        // Tab switches focus between Comments, Episodes, and Related (page-specific, not nav)
        if key == KeyCode::Tab {
            self.focus = if self.has_multiple_pages() {
                match self.focus {
                    DetailFocus::Comments => DetailFocus::Episodes,
                    DetailFocus::Episodes => DetailFocus::Related,
                    DetailFocus::Related => DetailFocus::Comments,
                }
            } else {
                match self.focus {
                    DetailFocus::Comments => DetailFocus::Related,
                    DetailFocus::Episodes => DetailFocus::Related,
                    DetailFocus::Related => DetailFocus::Comments,
                }
            };
            return Some(AppAction::None);
        }
        if keys.matches_down(key) {
            match self.focus {
                DetailFocus::Comments => {
                    if let Some(CommentIntent::LoadMoreComments) = self.comment_list.move_down() {
                        return Some(AppAction::LoadMoreComments);
                    }
                }
                DetailFocus::Episodes => {
                    if let Some(pages) = self.get_pages()
                        && self.episode_scroll + 1 < pages.len()
                    {
                        self.episode_scroll += 1;
                    }
                }
                DetailFocus::Related => {
                    if self.related_card_grid.move_down() {
                        self.related_scroll = self.related_card_grid.selected_index;
                    }
                }
            }
            return Some(AppAction::None);
        }
        if keys.matches_up(key) {
            match self.focus {
                DetailFocus::Comments => {
                    self.comment_list.move_up();
                }
                DetailFocus::Episodes => {
                    if self.episode_scroll > 0 {
                        self.episode_scroll -= 1;
                    }
                }
                DetailFocus::Related => {
                    if self.related_card_grid.move_up() {
                        self.related_scroll = self.related_card_grid.selected_index;
                    }
                }
            }
            return Some(AppAction::None);
        }
        if keys.matches_left(key) {
            if self.focus == DetailFocus::Related && self.related_card_grid.move_left() {
                self.related_scroll = self.related_card_grid.selected_index;
            }
            return Some(AppAction::None);
        }
        if keys.matches_right(key) {
            if self.focus == DetailFocus::Related && self.related_card_grid.move_right() {
                self.related_scroll = self.related_card_grid.selected_index;
            }
            return Some(AppAction::None);
        }
        if key == KeyCode::Char(' ') && self.focus == DetailFocus::Comments {
            // Space: expand/collapse replies of the selected comment
            if let Some(entry) = self.comment_list.selected_entry_info() {
                match entry.kind {
                    EntryKind::Comment => {
                        return Some(AppAction::ToggleCommentRepliesAt {
                            comment_index: entry.comment_index,
                        });
                    }
                    EntryKind::Toggle => {
                        if let Some(intent) = self.comment_list.activate_selected() {
                            return comment_intent_to_action(intent, self.aid);
                        }
                    }
                    _ => {}
                }
            }
            return Some(AppAction::None);
        }
        if key == KeyCode::Char('t') && self.focus == DetailFocus::Comments {
            // Toggle hot/newest sort and reload comments
            self.comment_list.sort_newest = !self.comment_list.sort_newest;
            self.comment_list.reset_selection();
            self.comment_page = 1;
            self.loading_more_comments = false;
            return Some(AppAction::ReloadComments {
                oid: self.aid,
                sort: if self.comment_list.sort_newest { 0 } else { 1 },
            });
        }
        if keys.matches_confirm(key) {
            match self.focus {
                DetailFocus::Comments => {
                    // Enter: expand/collapse replies on a comment card
                    if let Some(entry) = self.comment_list.selected_entry_info() {
                        match entry.kind {
                            EntryKind::Comment => {
                                return Some(AppAction::ToggleCommentRepliesAt {
                                    comment_index: entry.comment_index,
                                });
                            }
                            _ => {
                                if let Some(intent) = self.comment_list.activate_selected() {
                                    return comment_intent_to_action(intent, self.aid);
                                }
                            }
                        }
                    }
                }
                DetailFocus::Episodes => {
                    // Select and play the episode with auto-advance
                    if let Some(pages) = self.get_pages().cloned()
                        && self.episode_scroll < pages.len()
                    {
                        self.current_page_index = self.episode_scroll;
                        return Some(AppAction::PlayVideoWithPages {
                            bvid: self.bvid.clone(),
                            aid: self.aid,
                            pages,
                            current_index: self.episode_scroll,
                        });
                    }
                }
                DetailFocus::Related => {
                    if let Some(card) = self.related_card_grid.selected_card()
                        && let Some(bvid) = &card.bvid
                    {
                        let aid = card.aid.unwrap_or(0);
                        return Some(AppAction::OpenVideoDetail(bvid.clone(), aid));
                    }
                }
            }
            return Some(AppAction::None);
        }
        Some(AppAction::None)
    }

    fn handle_mouse(&mut self, event: MouseEvent, area: Rect) -> Option<AppAction> {
        if self.input_mode {
            return None;
        }

        match event.kind {
            MouseEventKind::ScrollDown => {
                match self.focus {
                    DetailFocus::Comments => {
                        if let Some(CommentIntent::LoadMoreComments) = self.comment_list.move_down()
                        {
                            return Some(AppAction::LoadMoreComments);
                        }
                    }
                    DetailFocus::Related => {
                        if self.related_card_grid.move_down() {
                            self.related_scroll = self.related_card_grid.selected_index;
                        }
                    }
                    DetailFocus::Episodes => {
                        if let Some(pages) = self.get_pages()
                            && self.episode_scroll + 1 < pages.len()
                        {
                            self.episode_scroll += 1;
                        }
                    }
                }
                None
            }
            MouseEventKind::ScrollUp => {
                match self.focus {
                    DetailFocus::Comments => {
                        self.comment_list.move_up();
                    }
                    DetailFocus::Related => {
                        if self.related_card_grid.move_up() {
                            self.related_scroll = self.related_card_grid.selected_index;
                        }
                    }
                    DetailFocus::Episodes => {
                        if self.episode_scroll > 0 {
                            self.episode_scroll -= 1;
                        }
                    }
                }
                None
            }
            MouseEventKind::Down(MouseButton::Left) => {
                // Comment pane click: select entry, double-click activates.
                {
                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([
                            Constraint::Length(6),
                            Constraint::Min(10),
                            Constraint::Length(2),
                        ])
                        .split(area);
                    let content_chunks = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
                        .split(chunks[1]);
                    let comments_area = content_chunks[0];
                    let inner = Rect {
                        x: comments_area.x + 1,
                        y: comments_area.y + 1,
                        width: comments_area.width.saturating_sub(2),
                        height: comments_area.height.saturating_sub(2),
                    };
                    if self.focus == DetailFocus::Comments
                        && inner.contains(Position::new(event.column, event.row))
                    {
                        let rel_row = (event.row - inner.y) as usize;
                        if let Some(entry) = self
                            .comment_list
                            .click_at(self.comment_list.scroll + rel_row)
                        {
                            let now = Instant::now();
                            let is_double = self.last_click_line == Some(entry.start_line)
                                && self
                                    .last_click_time
                                    .is_some_and(|t| now.duration_since(t).as_millis() < 500);
                            if is_double {
                                self.last_click_time = None;
                                self.last_click_line = None;
                                if let Some(intent) = self.comment_list.activate_selected() {
                                    return comment_intent_to_action(intent, self.aid);
                                }
                            } else {
                                self.last_click_time = Some(now);
                                self.last_click_line = Some(entry.start_line);
                            }
                        }
                        return None;
                    }
                }

                if self.focus != DetailFocus::Related {
                    return None;
                }

                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(6),
                        Constraint::Min(10),
                        Constraint::Length(2),
                    ])
                    .split(area);

                if self.loading {
                    return None;
                }

                if let Some(error) = &self.error_message
                    && (error.contains("视频信息") || error.contains("加载视频"))
                {
                    return None;
                }

                let content_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
                    .split(chunks[1]);

                let related_area = content_chunks[1];

                if !related_area.contains(ratatui::layout::Position::new(event.column, event.row)) {
                    return None;
                }

                let relative_y = event.row - related_area.y;
                let click_row = (relative_y / self.related_card_grid.card_height) as usize;
                let actual_row = self.related_card_grid.scroll_row + click_row;

                let card_width = related_area.width / self.related_card_grid.columns as u16;
                let click_col = (event.column.saturating_sub(related_area.x) / card_width) as usize;

                let click_idx = actual_row * self.related_card_grid.columns + click_col;

                if click_idx < self.related_card_grid.cards.len() {
                    let now = Instant::now();
                    let is_double_click = self.last_click_index == Some(click_idx)
                        && self
                            .last_click_time
                            .is_some_and(|t| now.duration_since(t).as_millis() < 500);

                    if is_double_click {
                        self.last_click_time = None;
                        self.last_click_index = None;
                        if let Some(card) = self.related_card_grid.cards.get(click_idx)
                            && let Some(ref bvid) = card.bvid
                        {
                            let aid = card.aid.unwrap_or(0);
                            return Some(AppAction::OpenVideoDetail(bvid.clone(), aid));
                        }
                    } else {
                        self.related_card_grid.selected_index = click_idx;
                        self.related_card_grid
                            .update_scroll(self.related_card_grid.cached_visible_rows);
                        self.related_scroll = self.related_card_grid.selected_index;
                        self.last_click_time = Some(now);
                        self.last_click_index = Some(click_idx);
                    }
                }
                None
            }
            _ => None,
        }
    }
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.chars().count() > max_len {
        s.chars()
            .take(max_len.saturating_sub(3))
            .collect::<String>()
            + "..."
    } else {
        s.to_string()
    }
}

/// Map a comment-list intent to an AppAction using this page's oid/type.
fn comment_intent_to_action(intent: CommentIntent, aid: i64) -> Option<AppAction> {
    match intent {
        CommentIntent::LoadMoreComments => Some(AppAction::LoadMoreComments),
        CommentIntent::ToggleReplies { comment_index } => {
            Some(AppAction::ToggleCommentRepliesAt { comment_index })
        }
        CommentIntent::LoadMoreReplies { comment_index } => {
            Some(AppAction::LoadMoreReplies { comment_index })
        }
        CommentIntent::PageReplies { comment_index } => {
            Some(AppAction::PageCommentReplies { comment_index })
        }
        CommentIntent::OpenSubThread {
            comment_index,
            reply_index,
        } => Some(AppAction::OpenSubThread {
            comment_index,
            reply_index,
        }),
        CommentIntent::CloseSubThread => Some(AppAction::CloseSubThread),
        CommentIntent::Like {
            comment_index,
            reply_index,
        } => Some(AppAction::LikeCommentAt {
            oid: aid,
            comment_index,
            reply_index,
            comment_type: 1,
        }),
    }
}
