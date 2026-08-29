# -*- coding: utf-8 -*-
# 4. related+comments same bg  5. home header removed  9. global footer

def rep(s, old, new, count=1):
    assert old in s, 'MISSING: ' + old[:70]
    return s.replace(old, new, count)

# ---- 4. related panel bg = bg_card (same as comments) ----
p = 'src/ui/video_detail.rs'
s = open(p, encoding='utf-8').read()
if 'is_focused,\n            theme.bg_secondary,\n        );' in s:
    s = rep(s, '''            is_focused,
            theme.bg_secondary,
        );''',
'''            is_focused,
            theme.bg_card,
        );''')
    open(p, 'w', encoding='utf-8').write(s)
    print('4. related bg ok')
else:
    print('4. already applied or pattern changed')

# ---- 5+9. home: global footer ----
p = 'src/ui/home.rs'
s = open(p, encoding='utf-8').read()

s = rep(s, '''    fn draw(&mut self, frame: &mut Frame, area: Rect, theme: &Theme, keys: &Keybindings) {
        let panes = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(28), Constraint::Min(30)])
            .split(area);
        self.draw_sources(frame, panes[0], theme);

        if self.selected_source == 0 {
            self.search.draw(frame, panes[1], theme, keys);
        } else {
            self.draw_feed(frame, panes[1], theme, keys);
        }
    }''',
'''    fn draw(&mut self, frame: &mut Frame, area: Rect, theme: &Theme, keys: &Keybindings) {
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
    }''')

# feed: drop its own footer render (keep header + grid)
s = rep(s, '''        let notice = self.footer_notice.take();
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
        let footer = Paragraph::new(help)
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .style(Style::default().bg(theme.bg_secondary))
                    .padding(ratatui::widgets::Padding::new(0, 0, 1, 0)),
            );
        frame.render_widget(footer, chunks[2]);''',
'''        // footer is rendered by the page-level draw_global_footer''')

# feed layout: shrink footer chunk since global footer takes it
s = rep(s, '''        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(10),
                Constraint::Length(3), // footer: own color, vertically centered
            ])
            .split(area);''',
'''        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(10),
                Constraint::Length(0),
            ])
            .split(area);''')

# add draw_global_footer method
s = rep(s, '''    fn visible_rows(&self, height: u16) -> usize {''',
'''    /// Render the full-width, vertically centered footer with its own surface.
    fn draw_global_footer(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        theme: &Theme,
        keys: &crate::storage::Keybindings,
    ) {
        let notice = self.footer_notice.take();
        let help = shortcut_footer(
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
        let mut help = help;
        if let Some(notice) = notice {
            help.spans.push(Span::styled(
                format!("  {notice}"),
                Style::default().fg(theme.fg_secondary),
            ));
        }
        let footer = Paragraph::new(help)
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .style(Style::default().bg(theme.bg_secondary))
                    .padding(ratatui::widgets::Padding::new(0, 0, 1, 0)),
            );
        frame.render_widget(footer, area);
    }

    fn visible_rows(&self, height: u16) -> usize {''')

open(p, 'w', encoding='utf-8').write(s)
print('5+9 home ok')
