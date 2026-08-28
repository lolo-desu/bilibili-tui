# -*- coding: utf-8 -*-
# settings.rs: borders -> bg panels

p = 'src/ui/settings.rs'
s = open(p, encoding='utf-8').read()
n0 = s.count('borders(Borders::')

def rep(old, new, count=1):
    global s
    assert old in s, 'MISSING: ' + old[:60]
    s = s.replace(old, new, count)

# 1. header bottom line: keep but use border_subtle already - keep as-is (faint line allowed)

# 2. danmaku section
rep('''        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border_subtle))
            .title(Span::styled(
                format!(" {} 弹幕设置 ", icons::COMMENT),
                Style::default()
                    .fg(theme.bilibili_pink)
                    .add_modifier(Modifier::BOLD),
            ));''',
'''        let block = panel_block(
            theme,
            Some(Line::from(Span::styled(
                format!(" {} 弹幕设置 ", icons::COMMENT),
                Style::default()
                    .fg(theme.bilibili_pink)
                    .add_modifier(Modifier::BOLD),
            ))),
            false,
        );''')

# 3. danmaku input
rep('''            let input = Paragraph::new(format!("{}▏", self.danmaku_input)).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" 输入新值 ")
                    .border_style(Style::default().fg(theme.fg_accent)),
            );''',
'''            let input = Paragraph::new(format!("{}▏", self.danmaku_input)).block(
                Block::default()
                    .style(Style::default().bg(theme.bg_secondary))
                    .title(Line::from(Span::styled(
                        " 输入新值 ",
                        Style::default().fg(theme.fg_accent),
                    ))),
            );''')

# 4. playback section
rep('''        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border_subtle))
            .title(Span::styled(
                " ▶  播放设置 ",
                Style::default()
                    .fg(theme.bilibili_pink)
                    .add_modifier(Modifier::BOLD),
            ));''',
'''        let block = panel_block(
            theme,
            Some(Line::from(Span::styled(
                " ▶  播放设置 ",
                Style::default()
                    .fg(theme.bilibili_pink)
                    .add_modifier(Modifier::BOLD),
            ))),
            false,
        );''')

# 5. theme section
rep('''        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border_subtle))
            .title(Span::styled(
                format!(" {} 选择主题 ", icons::PAINT),
                Style::default()
                    .fg(theme.bilibili_pink)
                    .add_modifier(Modifier::BOLD),
            ));''',
'''        let block = panel_block(
            theme,
            Some(Line::from(Span::styled(
                format!(" {} 选择主题 ", icons::PAINT),
                Style::default()
                    .fg(theme.bilibili_pink)
                    .add_modifier(Modifier::BOLD),
            ))),
            false,
        );''')

# 6. keybindings section
rep('''        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border_subtle))
            .title(Span::styled(
                " ⌨️ 快捷键 ",
                Style::default()
                    .fg(theme.bilibili_pink)
                    .add_modifier(Modifier::BOLD),
            ));''',
'''        let block = panel_block(
            theme,
            Some(Line::from(Span::styled(
                format!(" {} 快捷键 ", icons::KEYBOARD),
                Style::default()
                    .fg(theme.bilibili_pink)
                    .add_modifier(Modifier::BOLD),
            ))),
            false,
        );''')

# 7. keybind capture box
rep('''                Paragraph::new(format!("正在设置「{label}」：请按新的快捷键"))
                    .block(Block::default().borders(Borders::ALL).title(" 快捷键输入 ")),
''',
'''                Paragraph::new(format!("正在设置「{label}」：请按新的快捷键")).block(
                    Block::default()
                        .style(Style::default().bg(theme.bg_secondary))
                        .title(Line::from(Span::styled(
                            " 快捷键输入 ",
                            Style::default().fg(theme.fg_accent),
                        ))),
                ),
''')

# 8. account section
rep('''        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border_subtle))
            .title(Span::styled(
                format!(" {} 账户 ", icons::USER),
                Style::default()
                    .fg(theme.bilibili_pink)
                    .add_modifier(Modifier::BOLD),
            ));''',
'''        let block = panel_block(
            theme,
            Some(Line::from(Span::styled(
                format!(" {} 账户 ", icons::USER),
                Style::default()
                    .fg(theme.bilibili_pink)
                    .add_modifier(Modifier::BOLD),
            ))),
            false,
        );''')

# 9. account action button -> bg block
rep('''            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(action_color)),
            )
            .alignment(Alignment::Center);''',
'''            .block(
                Block::default()
                    .style(Style::default().bg(theme.bg_secondary))
                    .title(Line::from(Span::styled(
                        "  ",
                        Style::default().fg(action_color),
                    ))),
            )
            .alignment(Alignment::Center);''')

open(p, 'w', encoding='utf-8').write(s)
print('settings done, borders left:', s.count('borders(Borders::'), '(was', n0, ')')

# import
if 'use super::' in s and 'panel_block' not in s.split('fn ')[0]:
    pass
import re
m = re.search(r'use super::\{([^}]*)\};', s)
print('super import:', m.group(0) if m else None)
