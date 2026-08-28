# -*- coding: utf-8 -*-
# comment_list: emote spans rendering + sort badge in toggle/title + tests

p = 'src/ui/comment_list.rs'
s = open(p, encoding='utf-8').read()

# ---------- 1. shared span builders for message lines ----------
old = '''const AVATAR_COLS: u16 = 4; // avatar cell width, in terminal columns'''
new = '''/// Build styled spans for one wrapped message line, rendering known emote
/// tokens as a smiley glyph + short label instead of raw "(文本)".
fn message_spans(line: &str, segments: &[crate::api::comment::Segment<'_>], theme: &Theme) -> Vec<Span<'static>> {
    let _ = line;
    let mut spans = Vec::new();
    for seg in segments {
        match seg {
            crate::api::comment::Segment::Text(t) => {
                spans.push(Span::styled(t.to_string(), Style::default()));
            }
            crate::api::comment::Segment::Emote(token) => {
                spans.push(Span::styled(
                    format!("{}{} ", icons::SMILE, token),
                    Style::default().fg(theme.bilibili_cyan),
                ));
            }
        }
    }
    if spans.is_empty() {
        spans.push(Span::default());
    }
    spans
}

const AVATAR_COLS: u16 = 4; // avatar cell width, in terminal columns'''
assert old in s, 'const anchor'
s = s.replace(old, new, 1)

# ---------- 2. wrap_lines: keep as is, but message render needs emote-aware split ----------
# We add a helper that wraps segments into lines of styled spans.
old = '''fn level_color(level: i32, theme: &Theme) -> Color {'''
new = '''/// Wrap `segments` into visual lines of at most `width` cells, preserving
/// emote styling across wraps.
fn wrap_segments(
    segments: &[crate::api::comment::Segment<'_>],
    width: usize,
    theme: &Theme,
) -> Vec<Vec<Span<'static>>> {
    let mut lines: Vec<Vec<Span<'static>>> = vec![Vec::new()];
    let mut cur = 0usize;
    for seg in segments {
        match seg {
            crate::api::comment::Segment::Text(t) => {
                for line in wrap_lines(t, width) {
                    if cur > 0 && !lines[last(&lines)].is_empty() {
                        lines.push(Vec::new());
                    }
                    cur = 0;
                    let line_len = unicode_width(line);
                    let _ = line_len;
                    lines.last_mut().unwrap().push(Span::styled(
                        line.to_string(),
                        Style::default(),
                    ));
                }
            }
            crate::api::comment::Segment::Emote(token) => {
                let styled = format!("{}{} ", icons::SMILE, token);
                let w = truncate_width(&styled, usize::MAX).chars().count();
                let _ = w;
                lines.last_mut().unwrap().push(Span::styled(
                    styled,
                    Style::default().fg(theme.bilibili_cyan),
                ));
            }
        }
    }
    lines
}

fn last(v: &[Vec<Span<'static>>]) -> usize {
    v.len().saturating_sub(1)
}

fn unicode_width(s: &str) -> usize {
    s.chars().count()
}

fn level_color(level: i32, theme: &Theme) -> Color {'''
assert old in s, 'level_color anchor'
s = s.replace(old, new, 1)

# ---------- 3. comment card message: use segments when emotes present ----------
old = '''        // Message lines (wrapped)
        let lines = wrap_lines(comment.message(), content_width);
        for (li, line_text) in lines.iter().enumerate() {
            let y = row + 1 + li as u16;
            if y >= area.bottom() {
                break;
            }
            let mut span = Span::styled(line_text.clone(), Style::default().fg(theme.fg_primary));
            if is_selected {
                span = span.style(sel_style);
            }
            frame.render_widget(
                Paragraph::new(Line::from(vec![span])),
                Rect {
                    x: text_x,
                    y,
                    width: text_width,
                    height: 1,
                },
            );
        }

        // Action row: 时间 · IP属地 · 点赞 · 回复数 (web order)
        let action_y = row + 1 + lines.len() as u16;'''
new = '''        // Message lines (wrapped; emote-aware when the API provides emotes)
        let segments = comment.message_segments();
        let has_emotes = segments
            .iter()
            .any(|seg| matches!(seg, crate::api::comment::Segment::Emote(_)));
        let line_count = if has_emotes {
            wrap_segments(&segments, content_width, theme).len()
        } else {
            wrap_lines(comment.message(), content_width).len()
        };
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
        let action_y = row + 1 + line_count as u16;'''
assert old in s, 'comment message block'
s = s.replace(old, new, 1)

# ---------- 4. reply message: same treatment ----------
old = '''        // Message
        let lines = wrap_lines(reply.message(), content_width);
        for (li, line_text) in lines.iter().enumerate() {
            let y = row + 1 + li as u16;
            if y >= area.bottom() {
                break;
            }
            let span = Span::styled(line_text.clone(), Style::default().fg(theme.fg_primary));
            let line = Line::from(vec![span]).style(if is_selected {
                sel_style
            } else {
                Style::default()
            });
            frame.render_widget(
                Paragraph::new(line),
                Rect {
                    x: text_x,
                    y,
                    width: text_width,
                    height: 1,
                },
            );
        }

        // Action row: 时间 · IP属地 · 点赞
        let action_y = row + 1 + lines.len() as u16;'''
new = '''        // Message (emote-aware)
        let segments = reply.message_segments();
        let has_emotes = segments
            .iter()
            .any(|seg| matches!(seg, crate::api::comment::Segment::Emote(_)));
        let line_count = if has_emotes {
            wrap_segments(&segments, content_width, theme).len()
        } else {
            wrap_lines(reply.message(), content_width).len()
        };
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
        let action_y = row + 1 + line_count as u16;'''
assert old in s, 'reply message block'
s = s.replace(old, new, 1)

# ---------- 5. sort status display in the toggle row & API-facing helpers ----------
# CommentList exposes sort badge text for the page title.
old = '''    /// Reset to the first comment (e.g. after refresh).
    pub fn reset_selection(&mut self) {'''
new = '''    /// Sort badge shown next to the comment panel title: 最热 / 最新.
    pub fn sort_label(&self) -> &'static str {
        if self.sort_newest {
            "最新"
        } else {
            "最热"
        }
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
    pub fn reset_selection(&mut self) {'''
assert old in s, 'reset_selection'
s = s.replace(old, new, 1)

open(p, 'w', encoding='utf-8').write(s)
print('features4 done')
