# -*- coding: utf-8 -*-
# up.rs: web-style header with avatar, name+level+sign, follower counts

p = 'src/ui/up.rs'
s = open(p, encoding='utf-8').read()

# imports: add image picker + AvatarLoader pieces + icons
old = '''use super::{Component, Theme, VideoCard, VideoCardGrid, shortcut_footer};'''
new = '''use super::icons;
use super::image_picker::shared_picker;
use super::{
    Component, Theme, VideoCard, VideoCardGrid, shortcut_footer,
};'''
assert old in s, 'imports'
s = s.replace(old, new, 1)

# struct: add avatar state
old = '''pub struct UpPage {
    pub mid: i64,
    pub profile: Option<SpaceInfo>,
    pub relation: Option<RelationStat>,'''
new = '''pub struct UpPage {
    pub mid: i64,
    pub profile: Option<SpaceInfo>,
    pub relation: Option<RelationStat>,
    /// Rendered avatar protocol (downloaded once, web-style header).
    pub avatar: Option<ratatui_image::protocol::StatefulProtocol>,
    avatar_pending: bool,'''
assert old in s, 'struct'
s = s.replace(old, new, 1)

old = '''            mid,
            profile: None,
            relation: None,
            tab: UpTab::Videos,'''
new = '''            mid,
            profile: None,
            relation: None,
            avatar: None,
            avatar_pending: false,
            tab: UpTab::Videos,'''
assert old in s, 'init'
s = s.replace(old, new, 1)

# poll_avatar: download + redraw protocol (call from draw)
old = '''    fn selected_grid(&mut self) -> &mut VideoCardGrid {'''
new = '''    /// Kick off a one-shot avatar download and poll the result channel.
    fn poll_avatar(&mut self) {
        use ratatui_image::resolver::DefaultResolver;
        if self.avatar.is_some() || self.avatar_pending {
            return;
        }
        let Some(url) = self.profile.as_ref().and_then(|p| p.face.clone()) else {
            return;
        };
        self.avatar_pending = true;
        let mid = self.mid;
        let url = url.replacen("http://", "https://", 1);
        let picker = shared_picker();
        tokio::spawn(async move {
            let img = reqwest::get(&url)
                .await
                .ok()
                .and_then(|r| r.bytes().ok())
                .and_then(|b| tokio::task::block_in_place(|| image::load_from_memory(&b).ok()));
            if let Some(img) = img {
                poll_avatar_tx(img, picker, mid);
            }
        });

        fn poll_avatar_tx(
            img: image::DynamicImage,
            picker: std::sync::Arc<ratatui_image::Picker>,
            _mid: i64,
        ) {
            let protocol = picker.new_resize_protocol(img);
            UP_AVATAR_TX.with(|cell| {
                *cell.borrow_mut() = Some(protocol);
            });
        }
        let _ = DefaultResolver;
    }

    /// Take a finished avatar protocol if the background task delivered one.
    fn take_avatar(&mut self) {
        UP_AVATAR_TX.with(|cell| {
            if let Some(protocol) = cell.borrow_mut().take() {
                self.avatar = Some(protocol);
                self.avatar_pending = false;
            }
        });
    }

    fn selected_grid(&mut self) -> &mut VideoCardGrid {'''
assert old in s, 'selected_grid'
s = s.replace(old, new, 1)

# thread-local handoff channel (spawned task -> UI thread)
old = '''#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpTab {
    Videos,
    Favorites,
}'''
new = '''#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpTab {
    Videos,
    Favorites,
}

thread_local! {
    static UP_AVATAR_TX: std::cell::RefCell<Option<ratatui_image::protocol::StatefulProtocol>> =
        const { std::cell::RefCell::new(None) };
}'''
assert old in s, 'UpTab'
s = s.replace(old, new, 1)

# draw_header: web-style banner: avatar left, name+level+sign right, stats below
old = '''    fn draw_header(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let (name, sign) = self
            .profile
            .as_ref()
            .map(|p| (p.name.as_str(), p.sign.as_deref().unwrap_or("暂无签名")))
            .unwrap_or(("加载中…", ""));
        let stats = self
            .relation
            .as_ref()
            .map(|r| {
                format!(
                    "关注 {}  ·  粉丝 {}",
                    format_count(r.following.unwrap_or_default()),
                    format_count(r.follower.unwrap_or_default())
                )
            })
            .unwrap_or_default();
        let text = vec![
            Line::from(Span::styled(
                name,
                Style::default()
                    .fg(theme.bilibili_pink)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(sign),
            Line::from(Span::styled(stats, Style::default().fg(theme.fg_muted))),
        ];
        frame.render_widget(
            Paragraph::new(text)
                .wrap(Wrap { trim: true })
                .block(Block::default().borders(Borders::ALL).title(" UP主空间 ")),
            area,
        );
    }'''
new = '''    fn draw_header(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        self.poll_avatar();
        self.take_avatar();

        let (name, sign, level) = self
            .profile
            .as_ref()
            .map(|p| {
                (
                    p.name.as_str(),
                    p.sign.as_deref().unwrap_or("暂无签名"),
                    p.level.unwrap_or(0),
                )
            })
            .unwrap_or(("加载中…", "", 0));
        let (follower, following) = self
            .relation
            .as_ref()
            .map(|r| {
                (
                    format_count(r.follower.unwrap_or_default()),
                    format_count(r.following.unwrap_or_default()),
                )
            })
            .unwrap_or_else(|| ("-".to_string(), "-".to_string()));

        // Banner block with soft panel background (web user-space style)
        let banner = Block::default()
            .borders(Borders::ROUNDED)
            .border_style(Style::default().fg(theme.border_subtle))
            .style(Style::default().bg(theme.bg_card))
            .title(Span::styled(
                format!(" {} UP主空间 ", icons::USER),
                Style::default().fg(theme.bilibili_pink),
            ));
        let inner = banner.inner(area);
        frame.render_widget(banner, area);

        // Avatar square on the left (2 rows tall ≈ 4 cols wide)
        let avatar_rect = Rect {
            x: inner.x + 1,
            y: inner.y + 1,
            width: 4,
            height: (inner.height.saturating_sub(2)).min(4),
        };
        if let Some(protocol) = self.avatar.as_mut() {
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

        // Identity column right of the avatar
        let text_x = avatar_rect.x + avatar_rect.width + 2;
        let text_w = inner.right().saturating_sub(text_x + 1);
        let name_spans = vec![
            Span::styled(
                name.to_string(),
                Style::default()
                    .fg(theme.bilibili_pink)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" LV{}", level),
                Style::default().fg(theme.bilibili_cyan),
            ),
        ];
        let stat_line = Line::from(vec![
            Span::styled(format!("{} 粉丝", follower), Style::default().fg(theme.fg_secondary)),
            Span::styled("  ·  ", Style::default().fg(theme.fg_muted)),
            Span::styled(format!("{} 关注", following), Style::default().fg(theme.fg_secondary)),
        ]);
        let column = vec![
            Line::from(name_spans),
            Line::from(Span::styled(sign.to_string(), Style::default().fg(theme.fg_muted))),
            stat_line,
        ];
        frame.render_widget(
            Paragraph::new(column).wrap(Wrap { trim: true }),
            Rect {
                x: text_x,
                y: inner.y + 1,
                width: text_w,
                height: inner.height.saturating_sub(2),
            },
        );
    }'''
assert old in s, 'draw_header'
s = s.replace(old, new, 1)

# draw(): pass mut self to draw_header
s = s.replace('self.draw_header(frame, chunks[0], theme);', 'self.draw_header(frame, chunks[0], theme);')

# footer: add UP home hint
old = '''                ("1/2".into(), "投稿/收藏夹".into(), theme.info),'''
new = '''                ("u".into(), "用户主页".into(), theme.info),
                ("1/2".into(), "投稿/收藏夹".into(), theme.info),'''
assert old in s
s = s.replace(old, new, 1)

open(p, 'w', encoding='utf-8').write(s)
print('up.rs header done')
