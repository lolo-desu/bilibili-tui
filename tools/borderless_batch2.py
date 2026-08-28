# -*- coding: utf-8 -*-
# history.rs + favorites.rs + live.rs + bangumi.rs: borders -> bg panels

def rep(s, old, new, count=1):
    assert old in s, 'MISSING: ' + old[:60]
    return s.replace(old, new, count)

# ---------- history.rs ----------
p = 'src/ui/history.rs'
s = open(p, encoding='utf-8').read()

# main page panel
s = rep(s, '''        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border_subtle))
            .title(Span::styled(
                format!(" {} 观看历史 ", icons::FEED),
                Style::default()
                    .fg(theme.bilibili_pink)
                    .add_modifier(Modifier::BOLD),
            ))''',
'''        let block = panel_block(
            theme,
            Some(Line::from(Span::styled(
                format!(" {} 观看历史 ", icons::FEED),
                Style::default()
                    .fg(theme.bilibili_pink)
                    .add_modifier(Modifier::BOLD),
            ))),
            false,
        )''')

# item card -> bg block
s = rep(s, '''        let mut block = Block::default()
            .borders(Borders::ALL)
            .border_type(if is_selected {
                BorderType::Thick
            } else {
                BorderType::Rounded
            })
            .border_style(Style::default().fg(border_color));''',
'''        let _ = border_color;
        let mut block = Block::default().style(Style::default().bg(if is_selected {
            theme.bg_highlight
        } else {
            theme.bg_card
        }));''')

# delete confirm popup: keep as elevated dialog with bg (rare strong line OK -> use bg only)
s = rep(s, '''                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(theme.warning))
                        .title(" 删除历史记录 "),
                ),''',
'''                .block(
                    Block::default()
                        .style(Style::default().bg(theme.bg_secondary))
                        .title(Span::styled(
                            " 删除历史记录 ",
                            Style::default().fg(theme.warning),
                        )),
                ),''')

open(p, 'w', encoding='utf-8').write(s)
print('history done, left:', s.count('borders(Borders::'))

# ---------- favorites.rs ----------
p = 'src/ui/favorites.rs'
s = open(p, encoding='utf-8').read()

s = rep(s, '.block(Block::default().borders(Borders::ALL).title(" 收藏 "))',
'''            .block(
                Block::default()
                    .style(Style::default().bg(theme.bg_secondary))
                    .title(Line::from(Span::styled(
                        " 收藏 ",
                        Style::default().fg(theme.fg_muted),
                    ))),
            )''')

s = rep(s, '.block(Block::default().borders(Borders::ALL)),\n            right[0],',
        '.block(panel_block(theme, None, false)),\n            right[0],')

open(p, 'w', encoding='utf-8').write(s)
print('favorites done, left:', s.count('borders(Borders::'))

# ---------- live.rs ----------
p = 'src/ui/live.rs'
s = open(p, encoding='utf-8').read()

s = rep(s, '''            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(theme.border_subtle))
                    .title(Span::styled(
                        " 直播 ",
                        Style::default()
                            .fg(theme.fg_accent)
                            .add_modifier(Modifier::BOLD),
                    )),''',
'''            .block(
                panel_block(
                    theme,
                    Some(Line::from(Span::styled(
                        " 直播 ",
                        Style::default()
                            .fg(theme.fg_accent)
                            .add_modifier(Modifier::BOLD),
                    ))),
                    false,
                ),''')

open(p, 'w', encoding='utf-8').write(s)
print('live partial done, left:', s.count('borders(Borders::'))

# ---------- bangumi.rs ----------
p = 'src/ui/bangumi.rs'
s = open(p, encoding='utf-8').read()

s = rep(s, '''        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.border_subtle)),
        )
        .alignment(Alignment::Center);''',
'''        .block(panel_block(theme, None, false))
        .alignment(Alignment::Center);''')

open(p, 'w', encoding='utf-8').write(s)
print('bangumi done, left:', s.count('borders(Borders::'))
