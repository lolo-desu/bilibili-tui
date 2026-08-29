# -*- coding: utf-8 -*-
# comment_list part2: render floors with avatars + pager row

def rep(s, old, new, count=1):
    assert old in s, 'MISSING: ' + old[:70]
    return s.replace(old, new, count)

p = 'src/ui/comment_list.rs'
s = open(p, encoding='utf-8').read()

# --- avatar prefetch: include visible floor replies' authors ---
s = rep(s, '''        // avatar prefetch for visible comments
        let visible_comments = self.visible_comment_indices(viewport);
        let authors = self.visible_authors(&visible_comments);
        self.avatars.request(authors);
        self.avatars.poll();''',
'''        // avatar prefetch for visible comments and their floor replies
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
        self.avatars.request(authors);
        self.avatars.poll();''')

# --- unified avatar rendering for Comment AND Reply rows ---
old_avatar = '''            // Avatar first (needs &mut self for protocol render state)
            if entry.kind == EntryKind::Comment && avatars_supported {
                let avatar_rect = Rect {
                    x: area.x,
                    y: row,
                    width: AVATAR_COLS,
                    height: AVATAR_ROWS.min(area.bottom().saturating_sub(row)),
                };
                let protocol = self
                    .comments
                    .get(entry.comment_index)
                    .and_then(|c| author_key(c.member.as_ref()))
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
            }'''
new_avatar = '''            // Avatar first (needs &mut self for protocol render state).
            // Replies get one too (floor view = same layout as top comments).
            let avatar_member = match entry.kind {
                EntryKind::Comment => self
                    .comments
                    .get(entry.comment_index)
                    .and_then(|c| c.member.as_ref()),
                EntryKind::Reply => self
                    .comments
                    .get(entry.comment_index)
                    .and_then(|c| self.visible_replies(c.rpid).get(entry.reply_index))
                    .and_then(|r| r.member.as_ref()),
                _ => None,
            };
            if avatars_supported && let Some(member) = avatar_member {
                let avatar_rect = Rect {
                    x: area.x,
                    y: row,
                    width: AVATAR_COLS,
                    height: AVATAR_ROWS.min(area.bottom().saturating_sub(row)),
                };
                let protocol = author_key(member)
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
            }'''
s = rep(s, old_avatar, new_avatar)

# --- reply row: align with avatar column (same as comment card) ---
s = rep(s, '''        // Replies indent under the parent's text column (web style)
        let text_x = area.x + AVATAR_COLS + GAP_COLS + 2;
        let text_width = area.width.saturating_sub(AVATAR_COLS + GAP_COLS + 2);
        let content_width = text_width.saturating_sub(1) as usize;''',
'''        // Floor view: replies align exactly like top-level comments
        let text_x = area.x + AVATAR_COLS + GAP_COLS;
        let text_width = area.width.saturating_sub(AVATAR_COLS + GAP_COLS);
        let content_width = text_width.saturating_sub(1) as usize;''')

# --- pager row rendering ---
s = rep(s, '''        let (label, icon, color) = if entry.reply_index == 1 {
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
    }''',
'''        // Floor pager row (reply_index == 2): 上一页 | 第x/y页 | 下一页 (+加载更多)
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
                    // single page: show load-more from server instead
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

        let (label, icon, color) = if self.expanded.contains(&comment.rpid) {
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
    }''')

open(p, 'w', encoding='utf-8').write(s)
print('part2 ok')
