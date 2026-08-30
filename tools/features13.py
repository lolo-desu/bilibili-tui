# -*- coding: utf-8 -*-
# Web-style home cards: 16:9 full-width cover, title below with breathing
# room, icon stats row (view/danmaku/reply), UP row, duration ON the cover
# corner (web) rendered as a suffix line under title per user request.
# Card rows fixed-height; no giant empty space.

def rep(s, old, new):
    assert old in s, 'MISSING: ' + old[:70]
    return s.replace(old, new, 1)

p = 'src/ui/home.rs'
s = open(p, encoding='utf-8').read()

# card height: cover(8) + gap(1) + title(2) + duration(1) + up(1) + stats(1) + padding(2 borders) = 14 -> use 14
s = rep(s, '''    /// Height of vertical grid cards used at 3-4 columns (cover + text rows).
    const GRID_CARD_HEIGHT: u16 = 12;''',
'''    /// Height of vertical grid cards used at 3-4 columns (cover + text rows).
    const GRID_CARD_HEIGHT: u16 = 14;''')

# rewrite render_video_card to web style
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
        let card_chunks = if vertical {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(7), // 16:9 cover (full width)
                    Constraint::Length(1), // breathing room
                    Constraint::Min(4),    // text block
                ])
                .split(inner)
        } else {
            Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Length(24), Constraint::Min(20)])
                .split(inner)
        };

        // Cover fills the container (16:9-ish) via crop resize.
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
            .map(format_count);

        // Stats row: view / danmaku / reply, each with an icon; missing
        // reply counts (rcmd API) are simply omitted.
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
        let duration_row = Line::from(Span::styled(
            duration,
            Style::default().fg(theme.success),
        ));

        // Bottom-up: stats last line, UP above it, duration above, title top.
        let rows = if vertical {
            vec![stats_row, up_row, duration_row]
        } else {
            vec![stats_row, up_row]
        };
        let bottom_count = rows.len();
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
        let top_count = title_block
            .len()
            .min(info_lines.saturating_sub(bottom_count));
        lines.extend(title_block.into_iter().take(top_count));
        if vertical {
            // duration already counted in rows (bottom block); add it above UP
        }
        for _ in 0..info_lines.saturating_sub(top_count + bottom_count) {
            lines.push(Line::raw(""));
        }
        lines.extend(rows);

        let info = Paragraph::new(lines);
        frame.render_widget(info, info_area);
    }

'''
s = s.replace(old, new_card, 1)

open(p, 'w', encoding='utf-8').write(s)
print('card ok')
