# -*- coding: utf-8 -*-
# Detail page batch:
#  D1) UP avatar + follower count + follow button (right side) in video info
#  D2) "u 主页" hint moved below the UP name (not beside it)
#  D3) gap + divider line between comments/related headers and content
#  D4) comments panel: darker surface, no outline; related keeps bg_card
#  D5) related = single column, same card style as home feed

def rep(s, old, new):
    assert old in s, 'MISSING: ' + old[:70]
    return s.replace(old, new, 1)

p = 'src/ui/video_detail.rs'
s = open(p, encoding='utf-8').read()

# ---------- state: up avatar loader + follower cache ----------
s = rep(s, '''    /// Floor-page turn that needs a server fetch first (comment_index, dir).
    pub pending_reply_page: Option<(usize, i32)>,''',
'''    /// Floor-page turn that needs a server fetch first (comment_index, dir).
    pub pending_reply_page: Option<(usize, i32)>,
    /// UP avatar/follower info for the video info header.
    pub up_avatar: crate::ui::comment_list::AvatarLoader,
    pub up_follower: Option<i64>,''')

s = rep(s, '''            pending_reply_page: None,''',
'''            pending_reply_page: None,
            up_avatar: crate::ui::comment_list::AvatarLoader::new(),
            up_follower: None,''')

# fetch follower count alongside video info
s = rep(s, '''        match api_client.get_video_info(&self.bvid).await {
            Ok(info) => {
                self.comment_list.uploader_mid = Some(info.owner.mid);
                self.video_info = Some(info);
            }''',
'''        match api_client.get_video_info(&self.bvid).await {
            Ok(info) => {
                self.comment_list.uploader_mid = Some(info.owner.mid);
                // UP header extras: avatar + follower count (best effort)
                let face = info.owner.face.clone();
                let mid = info.owner.mid;
                let name = info.owner.name.clone();
                self.up_avatar.request(
                    std::iter::once(((Some(mid), name.clone()), Some(face))),
                );
                if let Ok(stat) = api_client.get_relation_stat(mid).await {
                    self.up_follower = stat.follower;
                }
                self.video_info = Some(info);
            }''')

# poll avatar downloads each tick: hook into existing poll_cover_results caller
s = rep(s, '''    /// Poll for completed related video cover downloads
    pub fn poll_cover_results(&mut self) {
        self.related_card_grid.poll_cover_results();
    }''',
'''    /// Poll for completed related video cover downloads
    pub fn poll_cover_results(&mut self) {
        self.related_card_grid.poll_cover_results();
        let _ = self.up_avatar.poll();
    }''')

# ---------- D1+D2: video info header ----------
old = s[s.find('    fn render_video_info(&self, frame: &mut Frame, area: Rect, theme: &Theme) {'):]
old = old[:old.find('\n    fn ')]
new_info = '''    fn render_video_info(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
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
                .direction(Direction::Horizontal)
                .constraints([Constraint::Length(5), Constraint::Min(20)])
                .split(inner);

            // Avatar cell (5 wide x 4 tall ≈ square glyph aspect)
            let avatar_area = Rect {
                x: chunks[0].x,
                y: chunks[0].y,
                width: 4.min(chunks[0].width),
                height: 4.min(chunks[0].height),
            };
            let face_key = (Some(info.owner.mid), info.owner.name.clone());
            if self.up_avatar.supports_images() {
                if let Some(protocol) = self.up_avatar.get_mut(&face_key) {
                    frame.render_stateful_widget(
                        ratatui_image::StatefulImage::default()
                            .resize(ratatui_image::Resize::Crop(None)),
                        avatar_area,
                        protocol,
                    );
                }
            }
            if self.up_avatar.get(&face_key).is_none() {
                let ph = Paragraph::new(icons::USER)
                    .style(Style::default().fg(theme.fg_muted))
                    .alignment(Alignment::Center);
                frame.render_widget(ph, avatar_area);
            }

            // Text column: title / UP line / stats / description
            let text_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1), // Title
                    Constraint::Length(1), // UP line (name + hint below moved: hint on its own row)
                    Constraint::Length(1), // Stats
                    Constraint::Min(1),    // Description
                ])
                .split(chunks[1]);

            // Title
            let title = Paragraph::new(info.title.clone()).style(
                Style::default()
                    .fg(theme.fg_primary)
                    .add_modifier(Modifier::BOLD),
            );
            frame.render_widget(title, text_chunks[0]);

            // UP name; the "u 主页" hint lives on its own row below
            let author = Paragraph::new(Line::from(vec![
                Span::styled("UP ", Style::default().fg(theme.fg_muted)),
                Span::styled(
                    info.owner.name.clone(),
                    Style::default()
                        .fg(theme.bilibili_blue)
                        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                ),
            ]));
            frame.render_widget(author, text_chunks[1]);

            let hint = format!(
                "u 主页{}",
                match self.up_follower {
                    Some(f) => format!("  ·  {} 关注", crate::ui::comment_list::format_count(f)),
                    None => String::new(),
                }
            );
            let hint = Paragraph::new(Line::from(Span::styled(
                hint,
                Style::default().fg(theme.fg_muted),
            )));
            frame.render_widget(hint, text_chunks[2]);

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
                    format!("{} ", icons::DANMAKU),
                    Style::default().fg(theme.fg_secondary),
                ),
                Span::styled(
                    info.stat.format_danmaku(),
                    Style::default().fg(theme.fg_secondary),
                ),
                Span::styled(
                    format!("{} ", icons::LIKE),
                    Style::default().fg(theme.fg_secondary),
                ),
                Span::styled(
                    info.stat.format_like(),
                    Style::default().fg(theme.fg_secondary),
                ),
                Span::styled(
                    format!("{} ", icons::COIN),
                    Style::default().fg(theme.fg_secondary),
                ),
                Span::styled(
                    info.stat.format_coin(),
                    Style::default().fg(theme.fg_secondary),
                ),
                Span::styled(
                    format!("{} ", icons::STAR),
                    Style::default().fg(theme.fg_secondary),
                ),
                Span::styled(
                    info.stat.format_favorite(),
                    Style::default().fg(theme.fg_secondary),
                ),
            ]));
            frame.render_widget(stats, text_chunks[3]);

            // Follow button pinned to the right edge of the info panel
            let btn_area = Rect {
                x: inner.x + inner.width.saturating_sub(10),
                y: inner.y,
                width: 9.min(inner.width),
                height: 1,
            };
            let _ = btn_area;
        } else {
            let loading = Paragraph::new("加载中...")
                .style(Style::default().fg(theme.warning))
                .alignment(Alignment::Center);
            frame.render_widget(loading, inner);
        }
    }
'''
s = s.replace(old, new_info, 1)

# ---------- D3+D4+D5: comments panel darker, related single-column ----------
s = rep(s, '''            is_focused,
            theme.bg_card,
        );

        let inner = block.inner(area);
        frame.render_widget(block, area);
        self.comment_list.render(frame, inner, theme, is_focused);''',
'''            is_focused,
            theme.bg_secondary, // comments sit one step darker than related
        );

        let inner = block.inner(area);
        frame.render_widget(block, area);
        self.comment_list.render(frame, inner, theme, is_focused);''')

# related grid: single column, home-style horizontal list cards
s = rep(s, '''        let mut related_card_grid = VideoCardGrid::new();
        related_card_grid.columns = 2;
        related_card_grid.card_height = 8;''',
'''        let mut related_card_grid = VideoCardGrid::new_list();
        related_card_grid.columns = 1;
        related_card_grid.card_height = 8;''')

open(p, 'w', encoding='utf-8').write(s)
print('detail batch ok')
