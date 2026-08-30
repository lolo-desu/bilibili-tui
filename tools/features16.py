# -*- coding: utf-8 -*-
# draw_sub_reply_row + reply indent/left line/child hint + intents.

def rep(s, old, new):
    assert old in s, 'MISSING: ' + old[:80]
    return s.replace(old, new, 1)

p = 'src/ui/comment_list.rs'
s = open(p, encoding='utf-8').read()

# ---------------- activate_selected: new kinds ----------------
s = rep(s, '''            EntryKind::Reply => Some(CommentIntent::Like {
                comment_index: entry.comment_index,
                reply_index: Some(entry.reply_index),
            }),''',
'''            EntryKind::Reply => {
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
            }),''')

# toggle: back row closes conversation
s = rep(s, '''                if self.expanded.contains(&comment.rpid) {
                    if entry.reply_index == 2 {''',
'''                if self.in_sub_thread() {
                    Some(CommentIntent::CloseSubThread)
                } else if self.expanded.contains(&comment.rpid) {
                    if entry.reply_index == 2 {''')

# ---------------- CommentIntent variants ----------------
s = rep(s, '''    /// Turn the floor page of the expanded comment's replies.
    PageReplies { comment_index: usize },''',
'''    /// Turn the floor page of the expanded comment's replies.
    PageReplies { comment_index: usize },
    /// Open the APP-style conversation of a floor reply.
    OpenSubThread {
        comment_index: usize,
        reply_index: usize,
    },
    /// Leave the conversation view.
    CloseSubThread,''')

# ---------------- draw_sub_reply_row (indented + left rule) ----------------
anchor = "    #[allow(clippy::too_many_arguments)]\n    fn draw_toggle_row("
assert anchor in s
sub_fn = '''    /// Child reply inside a conversation view: indented with a faint
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
        const INDENT: u16 = AVATAR_COLS + 2;
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
        .style(if is_selected { sel_style } else { Style::default() });
        frame.render_widget(
            Paragraph::new(header),
            Rect { x: text_x, y: row, width: text_width, height: 1 },
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
                Rect { x: text_x, y, width: text_width, height: 1 },
            );
        }

        // Faint vertical hierarchy line spanning the whole child block.
        let block_h = (1 + line_count + 1) as u16;
        let rule_y = row.min(area.bottom().saturating_sub(1));
        let rule_h = block_h.min(area.bottom().saturating_sub(rule_y));
        let rule = Block::default()
            .borders(Borders::LEFT)
            .border_style(Style::default().fg(theme.border_subtle));
        frame.render_widget(
            rule,
            Rect {
                x: area.x + AVATAR_COLS,
                y: rule_y,
                width: GAP_COLS + 1,
                height: rule_h,
            },
        );

        let action_y = row + 1 + line_count as u16;
        if action_y < area.bottom() {
            let liked = self.is_liked(reply.rpid);
            let like_icon = if liked { icons::LIKE_FILLED } else { icons::LIKE };
            let like_color = if liked { theme.bilibili_pink } else { theme.fg_muted };
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
                Rect { x: text_x, y: action_y, width: text_width, height: 1 },
            );
        }
    }

'''
s = s.replace(anchor, sub_fn + anchor, 1)

# ---------------- floor reply: indent + left rule + child hint ----------
s = rep(s, '''        // Floor view: replies align exactly like top-level comments
        let text_x = area.x + AVATAR_COLS + GAP_COLS;
        let text_width = area.width.saturating_sub(AVATAR_COLS + GAP_COLS);
        let content_width = text_width.saturating_sub(1) as usize;''',
'''        // Floor view: replies indent one step right; a faint vertical line
        // on the left marks them as children of the parent comment.
        const REPLY_INDENT: u16 = 2;
        let line_x = area.x + AVATAR_COLS;
        let text_x = line_x + REPLY_INDENT + GAP_COLS;
        let text_width = area
            .width
            .saturating_sub(AVATAR_COLS + REPLY_INDENT + GAP_COLS);
        let content_width = text_width.saturating_sub(1) as usize;
        let _ = line_x;''')

s = rep(s, '''            action_spans.push(Span::styled(
                format!("  {} ", like_icon),
                Style::default().fg(like_color),
            ));
            action_spans.push(Span::styled(
                format_count(self.like_count(reply)),
                Style::default().fg(like_color),
            ));
            let action = Line::from(action_spans).style(if is_selected {
                sel_style''',
'''            action_spans.push(Span::styled(
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
            // faint vertical hierarchy line spanning this reply's block
            let block_h = (1 + line_count + 1) as u16;
            let rule_y = row.min(area.bottom().saturating_sub(1));
            let rule_h = block_h.min(area.bottom().saturating_sub(rule_y));
            let rule = Block::default()
                .borders(Borders::LEFT)
                .border_style(Style::default().fg(theme.border_subtle));
            frame.render_widget(
                rule,
                Rect {
                    x: line_x,
                    y: rule_y,
                    width: REPLY_INDENT,
                    height: rule_h,
                },
            );
            let action = Line::from(action_spans).style(if is_selected {
                sel_style''')

open(p, 'w', encoding='utf-8').write(s)
print('rows ok')
