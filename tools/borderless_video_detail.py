# -*- coding: utf-8 -*-
# video_detail.rs: border blocks -> background panels

p = 'src/ui/video_detail.rs'
s = open(p, encoding='utf-8').read()
out = s

def rep(old, new, count=1):
    global out
    assert old in out, 'MISSING: ' + old[:70]
    out = out.replace(old, new, count)

# 1. video info panel
rep('''        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border_subtle))
            .title(Span::styled(
                format!(" {} 视频信息 ", icons::PLAY),
                Style::default().fg(theme.bilibili_pink),
            ));''',
'''        let block = panel_block(
            theme,
            Some(Line::from(Span::styled(
                format!(" {} 视频信息 ", icons::PLAY),
                Style::default().fg(theme.bilibili_pink),
            ))),
            false,
        );''')

# 2. comments panel
rep('''        let block = Block::default()
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
                Span::styled(" (t切换) ", Style::default().fg(theme.fg_muted)),
            ]));''',
'''        let block = panel_block(
            theme,
            Some(Line::from(vec![
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
                Span::styled(" (t切换) ", Style::default().fg(theme.fg_muted)),
            ])),
            is_focused,
        );''')

# 3. related panel
rep('''        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(border_style)
            .title(Span::styled(
                format!(" {} 相关推荐 ", icons::TV),
                Style::default().fg(if is_focused {
                    theme.bilibili_pink
                } else {
                    theme.fg_muted
                }),
            ));''',
'''        let block = panel_block(
            theme,
            Some(Line::from(Span::styled(
                format!(" {} 相关推荐 ", icons::TV),
                Style::default().fg(if is_focused {
                    theme.bilibili_pink
                } else {
                    theme.fg_muted
                }),
            ))),
            is_focused,
        );''')

# 4. episodes panel
rep('''        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(border_style)
            .title(Span::styled(
                format!(" {} 选集 ({}) ", icons::LIST, pages.len()),
                Style::default().fg(if is_focused {
                    theme.bilibili_pink
                } else {
                    theme.fg_muted
                }),
            ));''',
'''        let block = panel_block(
            theme,
            Some(Line::from(Span::styled(
                format!(" {} 选集 ({}) ", icons::LIST, pages.len()),
                Style::default().fg(if is_focused {
                    theme.bilibili_pink
                } else {
                    theme.fg_muted
                }),
            ))),
            is_focused,
        );''')

# 5. loading / error center panel
rep('''            let loading = Paragraph::new("⏳ 加载中...")
                .style(Style::default().fg(theme.warning))
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded),
                );''',
'''            let loading = Paragraph::new("⏳ 加载中...")
                .style(Style::default().fg(theme.warning))
                .alignment(Alignment::Center)
                .block(panel_block(theme, None, false));''')

rep('''            let error_widget = Paragraph::new(format!("{} {}", icons::ERROR, error))
                .style(Style::default().fg(theme.error))
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded),
                );''',
'''            let error_widget = Paragraph::new(format!("{} {}", icons::ERROR, error))
                .style(Style::default().fg(theme.error))
                .alignment(Alignment::Center)
                .block(panel_block(theme, None, false));''')

# 6. input box
rep('''            let input_block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.bilibili_pink))
                .title(Span::styled(
                    format!(" {} 发表评论 ", icons::EDIT),
                    Style::default()
                        .fg(theme.bilibili_pink)
                        .add_modifier(Modifier::BOLD),
                ));''',
'''            let input_block = Block::default()
                .style(Style::default().bg(theme.bg_secondary))
                .title(Span::styled(
                    format!(" {} 发表评论 ", icons::EDIT),
                    Style::default()
                        .fg(theme.bilibili_pink)
                        .add_modifier(Modifier::BOLD),
                ));''')

open(p, 'w', encoding='utf-8').write(out)
print('video_detail done')

# add panel_block import
if 'use super::{Component, Theme, shortcut_footer};' in out:
    out2 = out.replace('use super::{Component, Theme, shortcut_footer};',
                       'use super::{Component, Theme, panel_block, shortcut_footer};')
    open(p, 'w', encoding='utf-8').write(out2)
    print('import added')
