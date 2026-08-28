# -*- coding: utf-8 -*-
# up.rs + search.rs: borders -> bg panels

import re

# ---------- up.rs ----------
p = 'src/ui/up.rs'
s = open(p, encoding='utf-8').read()

def rep(s, old, new, count=1):
    assert old in s, 'MISSING: ' + old[:60]
    return s.replace(old, new, count)

# banner: drop borders, keep bg_card
s = rep(s, '''        let banner = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border_subtle))
            .style(Style::default().bg(theme.bg_card))
            .title(Span::styled(
                format!(" {} UP主空间 ", icons::USER),
                Style::default().fg(theme.bilibili_pink),
            ));''',
'''        let banner = Block::default()
            .style(Style::default().bg(theme.bg_card))
            .title(Span::styled(
                format!(" {} UP主空间 ", icons::USER),
                Style::default().fg(theme.bilibili_pink),
            ));''')

# tabs block: borderless
s = rep(s, '''            .block(Block::default().borders(Borders::ALL).title(format!(
                " {} · {play_order} ",
                if self.tab == UpTab::Videos {
                    sort
                } else {
                    favorite_sort
                }
            )));''',
'''            .block(Block::default().title(format!(
                " {} · {play_order} ",
                if self.tab == UpTab::Videos {
                    sort
                } else {
                    favorite_sort
                }
            )));''')

open(p, 'w', encoding='utf-8').write(s)
print('up done, borders left:', s.count('borders(Borders::'))

# ---------- search.rs ----------
p = 'src/ui/search.rs'
s = open(p, encoding='utf-8').read()

# hot list
s = rep(s, '''        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border_subtle))
            .title(Span::styled(
                " 热搜榜 ",
                Style::default().fg(theme.bilibili_pink),
            ));''',
'''        let block = panel_block(
            theme,
            Some(Line::from(Span::styled(
                " 热搜榜 ",
                Style::default().fg(theme.bilibili_pink),
            ))),
            false,
        );''')

# search input
s = rep(s, '''        let input_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(if self.input_mode {
                Style::default().fg(theme.bilibili_pink)
            } else {
                Style::default().fg(theme.border_subtle)
            })
            .title(Span::styled(
                format!(" {} 搜索视频 ", icons::SEARCH),
                Style::default().fg(theme.bilibili_pink),''',
'''        let input_block = Block::default()
            .style(Style::default().bg(if self.input_mode {
                theme.bg_highlight
            } else {
                theme.bg_secondary
            }))
            .title(Span::styled(
                format!(" {} 搜索视频 ", icons::SEARCH),
                Style::default().fg(theme.bilibili_pink),''')

# loading
s = rep(s, '''                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(theme.border_unfocused))
                        .title(Span::styled(
                            format!(" 结果 ({}) ", self.total_results),
                            Style::default().fg(theme.fg_secondary),
                        )),
                );''',
'''                .block(
                    panel_block(
                        theme,
                        Some(Line::from(Span::styled(
                            format!(" 结果 ({}) ", self.total_results),
                            Style::default().fg(theme.fg_secondary),
                        ))),
                        false,
                    ),
                );''')

# error
s = rep(s, '''                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(theme.border_unfocused)),
                );''',
'''                .block(panel_block(theme, None, false));''')

# empty
s = rep(s, '''            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(theme.border_unfocused)),
            );''',
'''            .block(panel_block(theme, None, false));''')

# results header: was TOP|LEFT|RIGHT borders; make it a bg strip
s = rep(s, '''            .block(
                Block::default()
                    .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(theme.border_subtle)),
            );''',
'''            .block(Block::default().style(Style::default().bg(theme.bg_secondary)));''')

open(p, 'w', encoding='utf-8').write(s)
print('search done, borders left:', s.count('borders(Borders::'))

m = re.search(r'use super::\{([^}]*)\};', s)
print('search super import:', m.group(0) if m else None)
