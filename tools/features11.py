# -*- coding: utf-8 -*-
# Naming: sidebar / tabs / content. This batch:
#  A) tabs column: no title, height aligned with sidebar (logo row on top),
#     shared logo header across both panels, more vertical spacing per tab
#  B) card meta: UP/关注/数据/时长 bottom-aligned rows (bottom-up), not glued to title

def rep(s, old, new):
    assert old in s, 'MISSING: ' + old[:70]
    return s.replace(old, new, 1)

# ================= A. sidebar + tabs unified header =================
p = 'src/ui/sidebar.rs'
s = open(p, encoding='utf-8').read()

old = s[s.find('    pub fn draw(&self, frame: &mut Frame, area: Rect, theme: &Theme) {'):]
old = old[:old.find('\n    pub fn next')]
new_draw = '''    pub fn draw(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        // Background panel instead of border lines (opencode style): the
        // sidebar reads as a colored surface, content area stays on bg_primary.
        let block = Block::default().style(Style::default().bg(theme.bg_secondary));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Split into logo, nav items, footer
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4), // Logo row (mirrored by the tabs panel)
                Constraint::Min(5),    // Nav items
                Constraint::Length(1), // Version
            ])
            .split(inner);

        render_logo(frame, chunks[0], theme);

        // Nav items with modern block selection indicator
        let items: Vec<ListItem> = NavItem::all()
            .iter()
            .map(|item| {
                let is_selected = *item == self.selected;
                let style = if is_selected {
                    Style::default()
                        .fg(theme.bilibili_pink)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.fg_secondary)
                };

                // Use block indicator for selection instead of arrow
                let prefix = if is_selected { " ▌" } else { "  " };
                let suffix = if is_selected { " " } else { "" };
                ListItem::new(format!("{}{}{}", prefix, item.label(), suffix)).style(style)
            })
            .collect();

        let list = List::new(items).highlight_style(Style::default().add_modifier(Modifier::BOLD));

        frame.render_widget(list, chunks[1]);

        // Version tag so it is easy to tell which build is running
        let version = Paragraph::new(Line::from(Span::styled(
            format!("  v{}", env!("CARGO_PKG_VERSION")),
            Style::default().fg(theme.fg_muted),
        )));
        frame.render_widget(version, chunks[2]);
    }
'''
s = s.replace(old, new_draw, 1)

# shared logo renderer (also used by the tabs panel)
s = rep(s, '''pub struct Sidebar {
    pub selected: NavItem,
}''',
'''pub struct Sidebar {
    pub selected: NavItem,
}

/// Shared branding header: Bilibili logo + client tag, rendered by both the
/// sidebar and the tabs panel so the two columns start at the same height.
pub fn render_logo(frame: &mut Frame, area: Rect, theme: &Theme) {
    let brand_lines = vec![
        Line::raw(""),
        Line::from(vec![
            Span::styled(
                "  ▌",
                Style::default()
                    .fg(theme.bilibili_pink)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "B",
                Style::default()
                    .fg(theme.bilibili_pink)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "ilibili",
                Style::default()
                    .fg(theme.fg_primary)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![Span::styled(
            "   TUI Client",
            Style::default()
                .fg(theme.fg_muted)
                .add_modifier(Modifier::ITALIC),
        )]),
    ];
    frame.render_widget(Paragraph::new(brand_lines), area);
}''')
open(p, 'w', encoding='utf-8').write(s)
print('A1 sidebar ok')

# ================= A2. tabs panel: no title, spaced rows =================
p = 'src/ui/home.rs'
s = open(p, encoding='utf-8').read()

old = s[s.find('    fn draw_sources(&self, frame: &mut Frame, area: Rect, theme: &Theme) {'):]
old = old[:old.find('\n    fn ')]
new_fn = '''    fn draw_sources(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        // Vertical tab strip: same surface as the sidebar, logo row on top
        // (aligned with the sidebar header), spaced tab rows below.
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
                Constraint::Length(4), // Logo row (mirrors the sidebar header)
                Constraint::Min(5),    // Tabs
            ])
            .split(outer);
        super::sidebar::render_logo(frame, rows[0], theme);

        // Two terminal rows per tab keeps them readable without feeling cramped.
        let mut constraints: Vec<Constraint> = Vec::new();
        for index in 0..self.source_count() {
            let _ = index;
            constraints.push(Constraint::Length(2));
        }
        constraints.push(Constraint::Min(0));
        let tabs = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(rows[1]);

        for (index, tab_area) in tabs.iter().enumerate() {
            if index >= self.source_count() {
                break;
            }
            let is_selected = index == self.selected_source;
            // Tab pill occupies the upper row; lower row is pure spacing.
            let pill = Rect {
                height: 1,
                ..*tab_area
            };
            let label = self.source_label(index);
            if is_selected {
                let selected = Line::from(vec![
                    Span::styled(
                        "▌",
                        Style::default()
                            .fg(theme.bilibili_pink)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        label,
                        Style::default()
                            .fg(theme.fg_primary)
                            .add_modifier(Modifier::BOLD)
                            .bg(theme.bg_card),
                    ),
                ]);
                frame.render_widget(
                    Block::default().style(Style::default().bg(theme.bg_card)),
                    pill,
                );
                frame.render_widget(Paragraph::new(selected), pill);
            } else {
                let normal = Paragraph::new(Line::from(Span::styled(
                    format!("  {label}"),
                    Style::default().fg(theme.fg_secondary),
                )));
                frame.render_widget(normal, pill);
            }
        }
    }
'''
s = s.replace(old, new_fn, 1)

# ================= A3. page-level: tabs pane aligned with sidebar ============
# The tabs column must start at the very top (no feed header above), which it
# already does. Nothing to change in draw(); sources pane == full height.

# ================= B. card meta rows: bottom-aligned =================
old = s[s.find('    fn render_video_card('):]
old = old[:old.find('\n    fn ', 10)]
new_card = '''    fn render_video_card(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        video_idx: usize,
        is_selected: bool,
        theme: &Theme,
    ) {
        let title_span = if is_selected {
            Span::styled(
                " ▶ ",
                Style::default()
                    .fg(theme.fg_accent)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::raw("")
        };

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
            }))
            .title(title_span);

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // 3-4 columns: vertical web-style card (full-width cover on top);
        // 1-2 columns: horizontal list card (cover left, info right)
        let vertical = self.columns >= 3;
        let card_chunks = if vertical {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(4), Constraint::Length(4)])
                .split(inner)
        } else {
            Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Length(28), Constraint::Min(30)])
                .split(inner)
        };

        // Cover container: fill it edge-to-edge with a 16:9 center-crop so
        // every cover shares the same size and aspect ratio.
        let cover_area = card_chunks[0];
        if let Some(cover) = &mut self.videos[video_idx].cover {
            let image_widget = StatefulImage::new();
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

        // Video info: title block on top, meta rows glued to the bottom edge
        // (bottom-up: duration/stats on the last line, UP line above it).
        let info_area = card_chunks[1];
        let card = &self.videos[video_idx];

        let title = card.video.title.as_deref().unwrap_or("无标题");
        let author = card.video.author_name();
        let views = card.video.format_views();
        let duration = card.video.format_duration();

        let title_width = (info_area.width as usize).saturating_sub(2);
        let title_lines: Vec<String> = wrap_title(title, title_width.max(8), 2);

        let meta_style = Style::default().fg(theme.fg_secondary);

        let follower = card
            .video
            .owner
            .as_ref()
            .and_then(|owner| owner.follower)
            .map(format_count)
            .unwrap_or_else(|| "-".to_string());
        let danmaku = card
            .video
            .stat
            .as_ref()
            .and_then(|stat| stat.danmaku)
            .map(format_count)
            .unwrap_or_else(|| "-".to_string());
        let replies = card
            .video
            .stat
            .as_ref()
            .and_then(|stat| stat.reply)
            .map(format_count)
            .unwrap_or_else(|| "-".to_string());

        // bottom-up rows; the list is rendered bottom-anchored below
        let rows: Vec<Line> = vec![
            Line::from(vec![
                Span::styled(format!("▶ {views}"), meta_style),
                Span::styled(format!("  弹幕 {danmaku}"), meta_style),
                Span::styled(format!("  评论 {replies}"), meta_style),
                Span::styled(format!("  {duration}"), Style::default().fg(theme.success)),
            ]),
            Line::from(vec![
                Span::styled("UP ", meta_style),
                Span::styled(author, Style::default().fg(theme.bilibili_cyan)),
                Span::styled(format!("  ·  {follower} 关注"), meta_style),
            ]),
        ];

        let info_lines = info_area.height as usize;
        let meta_count = rows.len();
        let title_capacity = info_lines.saturating_sub(meta_count);
        let title_gap = title_capacity.saturating_sub(title_lines.len().min(title_capacity));

        let mut lines: Vec<Line> = Vec::new();
        for text in title_lines.iter().take(title_capacity) {
            lines.push(Line::from(Span::styled(
                text,
                if is_selected {
                    Style::default()
                        .fg(theme.fg_primary)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.fg_secondary)
                },
            )));
        }
        for _ in 0..title_gap {
            lines.push(Line::raw(""));
        }
        lines.extend(rows);

        let info = Paragraph::new(lines);
        frame.render_widget(info, info_area);
    }

    /// Word-safe title wrapping with a hard line cap.
    fn wrap_title(text: &str, width: usize, max_lines: usize) -> Vec<String> {
        wrap_text_chars(text, width, max_lines)
    }
'''
s = s.replace(old, new_card, 1)

open(p, 'w', encoding='utf-8').write(s)
print('B home card ok')
