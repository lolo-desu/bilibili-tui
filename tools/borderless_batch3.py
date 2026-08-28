# -*- coding: utf-8 -*-
# live_detail, bangumi_detail, article_detail, login: borders -> bg panels

import re

def rep(s, old, new, count=1):
    assert old in s, 'MISSING: ' + old[:60]
    return s.replace(old, new, count)

# ---------- live_detail.rs ----------
p = 'src/ui/live_detail.rs'
s = open(p, encoding='utf-8').read()

# 1. main panel (title already built above)
s = rep(s, '''                title,
                Style::default()
                    .fg(theme.bilibili_pink)
                    .add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border_subtle));''',
'''                title,
                Style::default()
                    .fg(theme.bilibili_pink)
                    .add_modifier(Modifier::BOLD),
            ));''')

# find the block start to add bg style: it starts with Block::default() then .title? check context later.
# 2. room info
s = rep(s, '''                " 房间信息 ",
                Style::default()
                    .fg(theme.fg_secondary)
                    .add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border_subtle));''',
'''                " 房间信息 ",
                Style::default()
                    .fg(theme.fg_secondary)
                    .add_modifier(Modifier::BOLD),
            ))
            .style(Style::default().bg(theme.bg_card));''')

# 3. danmaku panel
s = rep(s, '''                format!(" 弹幕 ({}) ", self.danmakus.len()),
                Style::default()
                    .fg(theme.fg_secondary)
                    .add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border_subtle));''',
'''                format!(" 弹幕 ({}) ", self.danmakus.len()),
                Style::default()
                    .fg(theme.fg_secondary)
                    .add_modifier(Modifier::BOLD),
            ))
            .style(Style::default().bg(theme.bg_card));''')

# 4. entry panel
s = rep(s, '''                " 入场 ",
                Style::default()
                    .fg(theme.fg_secondary)
                    .add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border_subtle));''',
'''                " 入场 ",
                Style::default()
                    .fg(theme.fg_secondary)
                    .add_modifier(Modifier::BOLD),
            ))
            .style(Style::default().bg(theme.bg_card));''')

open(p, 'w', encoding='utf-8').write(s)
print('live_detail done, left:', s.count('borders(Borders::'))

# ---------- bangumi_detail.rs ----------
p = 'src/ui/bangumi_detail.rs'
s = open(p, encoding='utf-8').read()

s = rep(s, '''        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border_subtle))
            .title(Span::styled(
                format!(" {} 番剧信息 ", icons::TV),
                Style::default().fg(theme.bilibili_pink),
            ));''',
'''        let block = panel_block(
            theme,
            Some(Line::from(Span::styled(
                format!(" {} 番剧信息 ", icons::TV),
                Style::default().fg(theme.bilibili_pink),
            ))),
            false,
        );''')

s = rep(s, '''        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border_subtle))
            .title(Span::styled(
                format!(" {} 选集 ", icons::LIST),
                Style::default().fg(theme.bilibili_pink),
            ));''',
'''        let block = panel_block(
            theme,
            Some(Line::from(Span::styled(
                format!(" {} 选集 ", icons::LIST),
                Style::default().fg(theme.bilibili_pink),
            ))),
            false,
        );''')

open(p, 'w', encoding='utf-8').write(s)
print('bangumi_detail done, left:', s.count('borders(Borders::'))

# ---------- article_detail.rs ----------
p = 'src/ui/article_detail.rs'
s = open(p, encoding='utf-8').read()

s = rep(s, '''        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border_subtle))
            .title(" 正文 ");''',
'''        let block = panel_block(
            theme,
            Some(Line::from(Span::styled(
                " 正文 ",
                Style::default().fg(theme.bilibili_pink),
            ))),
            false,
        );''')

s = rep(s, '''        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border_subtle))
            .title(format!(" {alt} "));''',
'''        let block = Block::default().style(Style::default().bg(theme.bg_secondary))
            .title(Line::from(Span::styled(
                format!(" {alt} "),
                Style::default().fg(theme.fg_muted),
            )));''')

s = rep(s, '''        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border_subtle))
            .title(format!(" 评论 {} ", self.comments.len()));''',
'''        let block = panel_block(
            theme,
            Some(Line::from(Span::styled(
                format!(" 评论 {} ", self.comments.len()),
                Style::default().fg(theme.bilibili_pink),
            ))),
            false,
        );''')

# header underline: keep but faint (already Borders::BOTTOM, plain style) - restyle to subtle
s = rep(s, '.block(Block::default().borders(Borders::BOTTOM)),\n            chunks[0],',
        '.block(\n                Block::default().borders(Borders::BOTTOM).border_style(Style::default().fg(theme.border_subtle)),\n            ),\n            chunks[0],')

open(p, 'w', encoding='utf-8').write(s)
print('article_detail done, left:', s.count('borders(Borders::'))

# ---------- login.rs ----------
p = 'src/ui/login.rs'
s = open(p, encoding='utf-8').read()

# character qr
s = rep(s, '''        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border_unfocused))
            .title(Span::styled(
                " 字符二维码 ",
                Style::default().fg(theme.fg_secondary),
            ));''',
'''        let block = Block::default()
            .style(Style::default().bg(theme.bg_card))
            .title(Span::styled(
                " 字符二维码 ",
                Style::default().fg(theme.fg_secondary),
            ));''')

# image qr
s = rep(s, '''            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border_unfocused))
            .title(Span::styled(
                " 图片二维码 ",
                Style::default().fg(theme.fg_secondary),
            ));''',
'''            .style(Style::default().bg(theme.bg_card))
            .title(Span::styled(
                " 图片二维码 ",
                Style::default().fg(theme.fg_secondary),
            ));''')

# Login main panel
s = rep(s, '''                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(theme.border_subtle))
                    .title(Span::styled(
                        " Login ",
                        Style::default()
                            .fg(theme.bilibili_pink)
                            .add_modifier(Modifier::BOLD),
                    )),''',
'''                    .title(Span::styled(
                        " Login ",
                        Style::default()
                            .fg(theme.bilibili_pink)
                            .add_modifier(Modifier::BOLD),
                    ))''')
# note: this block also has .style? check next lines after rep - may need bg. handled below by checking.

# scan login panel
s = rep(s, '''            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border_unfocused))
            .title(Span::styled(
                " 扫码登录 ",
                Style::default().fg(theme.fg_secondary),
            ));''',
'''            .style(Style::default().bg(theme.bg_card))
            .title(Span::styled(
                " 扫码登录 ",
                Style::default().fg(theme.fg_secondary),
            ));''')

# status panel
s = rep(s, '''                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(theme.border_unfocused))
                    .title(Span::styled(
                        " 状态 ",
                        Style::default().fg(theme.fg_secondary),
                    )),''',
'''                    .title(Span::styled(
                        " 状态 ",
                        Style::default().fg(theme.fg_secondary),
                    ))''')

open(p, 'w', encoding='utf-8').write(s)
print('login done, left:', s.count('borders(Borders::'))

# add style(bg_card) to Login main + status blocks (they start with Block::default() before .border_type etc.)
s2 = s
# status block: find "Block::default()\n                    .title(Span::styled(\n                        \" 状态 \""
s2 = s2.replace('''Block::default()
                    .title(Span::styled(
                        " 状态 ",''',
'''Block::default()
                    .style(Style::default().bg(theme.bg_card))
                    .title(Span::styled(
                        " 状态 ",''')
open(p, 'w', encoding='utf-8').write(s2)
print('login status styled')

# imports
for p in ['src/ui/live_detail.rs', 'src/ui/bangumi_detail.rs', 'src/ui/article_detail.rs', 'src/ui/login.rs']:
    s = open(p, encoding='utf-8').read()
    if 'panel_block' in s and 'use super::{Component, Theme, shortcut_footer};' in s:
        s = s.replace('use super::{Component, Theme, shortcut_footer};',
                      'use super::{Component, Theme, panel_block, shortcut_footer};')
        open(p, 'w', encoding='utf-8').write(s)
        print('import:', p)
