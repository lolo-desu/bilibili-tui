# -*- coding: utf-8 -*-
# video_detail: sort badge in comments title + new open_spec arms unchanged

p = 'src/ui/video_detail.rs'
s = open(p, encoding='utf-8').read()

old = '''        let total = self.comment_list.comments.len();
        let more_hint = if self.comment_list.has_more { "+" } else { "" };
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(border_style)
            .title(Span::styled(
                format!(" {} 评论 {}{} ", icons::COMMENT, total, more_hint),
                Style::default().fg(if is_focused {
                    theme.bilibili_pink
                } else {
                    theme.fg_muted
                }),
            ));'''
new = '''        let total = self.comment_list.comments.len();
        let more_hint = if self.comment_list.has_more { "+" } else { "" };
        let sort_icon = self.comment_list.sort_icon();
        let sort_label = self.comment_list.sort_label();
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(border_style)
            .title(Line::from(vec![
                Span::styled(
                    format!(" {} 评论 {}{} ", icons::COMMENT, total, more_hint),
                    Style::default().fg(if is_focused {
                        theme.bilibili_pink
                    } else {
                        theme.fg_muted
                    }),
                ),
                Span::styled(
                    format!(" {}·{} ", sort_icon, sort_label),
                    Style::default().fg(if is_focused {
                        theme.bilibili_cyan
                    } else {
                        theme.fg_muted
                    }),
                ),
                Span::styled(
                    " (t切换) ",
                    Style::default().fg(theme.fg_muted),
                ),
            ]));'''
assert old in s, 'comments title'
s = s.replace(old, new, 1)
open(p, 'w', encoding='utf-8').write(s)
print('video_detail sort badge done')
