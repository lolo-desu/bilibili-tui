# -*- coding: utf-8 -*-
# Detail page rework (web layout):
#  - two columns: LEFT = video info (top) + comments (fills to bottom);
#    RIGHT = UP card (avatar/name/fans/follow) + episodes + related
#  - comments panel border wraps its bg block cleanly; panels are edge to
#    edge with no gutter (block borders form the seam)
#  - comments header: "评论 n  最热|最新" text tabs, no icons / (t切换)
#  - related title gets top padding
#  - footer overlays the comments panel bottom edge
#  - APP-style conversation view: button/hint opens a page showing the
#    full reply tree of one floor reply (its children + replies to them)

def rep(s, old, new):
    assert old in s, 'MISSING: ' + old[:80]
    return s.replace(old, new, 1)

p = 'src/ui/video_detail.rs'
s = open(p, encoding='utf-8').read()

# ---------------- draw(): two-column layout ----------------
start = s.find('    fn draw(&mut self, frame: &mut Frame, area: Rect, theme: &Theme, keys: &Keybindings) {')
assert start != -1
end = s.find('\n    fn handle_input(', start)
assert end != -1
new_draw = '''    fn draw(&mut self, frame: &mut Frame, area: Rect, theme: &Theme, keys: &Keybindings) {
        // Web layout: LEFT = info + comments; RIGHT = UP card + episodes +
        // related. The comment panel stretches to the window bottom and the
        // shortcut row overlays its lower padding.
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(62), Constraint::Percentage(38)])
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
            self.render_comments(frame, left_rows[1], theme);

            // Shortcut row overlays the comments panel's bottom padding.
            let footer_area = Rect {
                x: left_rows[1].x + 1,
                y: area.bottom().saturating_sub(3),
                width: left_rows[1].width.saturating_sub(2),
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

'''
s = s[:start] + new_draw + s[end:]

# ---------------- right column: UP card + episodes + related ----------------
s = rep(s, '''    fn render_related(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {''',
'''    /// Right rail: UP card on top (web order), then episodes + related.
    fn render_right_column(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(7), // UP card
                Constraint::Min(8),    // episodes (if any) or related
                Constraint::Min(8),    // related
            ])
            .split(area);

        self.render_up_card(frame, rows[0], theme);

        if self.has_multiple_pages() {
            self.render_episodes(frame, rows[1], theme);
            self.render_related(frame, rows[2], theme);
        } else {
            self.render_related(frame, rows[1], theme);
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

        // avatar column (4 wide) + text column
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(5), Constraint::Min(10)])
            .split(inner);
        if avatar_ready
            && let Some(protocol) = self.up_avatar.get_mut(&face_key)
        {
            frame.render_stateful_widget(
                ratatui_image::StatefulImage::default()
                    .resize(ratatui_image::Resize::Crop(None)),
                cols[0],
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
        let fans_p = Paragraph::new(Span::styled(
            fans,
            Style::default().fg(theme.fg_muted),
        ));
        frame.render_widget(fans_p, text_rows[1]);

        let (label, fg, bg) = match self.following {
            Some(true) => (" 已关注 ".to_string(), theme.fg_secondary, theme.bg_secondary),
            _ => (" + 关注 ".to_string(), theme.fg_primary, theme.bilibili_pink),
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

    fn render_related(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {''')

open(p, 'w', encoding='utf-8').write(s)
print('layout + up card ok')
