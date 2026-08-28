# -*- coding: utf-8 -*-
# dynamic.rs + dynamic_detail.rs: borders -> bg panels

import re

def rep(s, old, new, count=1):
    assert old in s, 'MISSING: ' + old[:60]
    return s.replace(old, new, count)

# ---------- dynamic.rs ----------
p = 'src/ui/dynamic.rs'
s = open(p, encoding='utf-8').read()

# UP list -> bg panel
s = rep(s, '''        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(theme.border_subtle))
                    .title(" 关注的UP主 "),
            )''',
'''        let list = List::new(items)
            .block(
                Block::default()
                    .style(Style::default().bg(theme.bg_secondary))
                    .title(Line::from(Span::styled(
                        " 关注的UP主 ",
                        Style::default().fg(theme.fg_muted),
                    ))),
            )''')

# header title block (TOP|LEFT|RIGHT) -> plain
s = rep(s, '.block(Block::default().borders(Borders::TOP | Borders::LEFT | Borders::RIGHT));',
        '.block(Block::default());')

# tabs block (BOTTOM|LEFT|RIGHT) -> faint bottom separator line (allowed)
s = rep(s, '.block(Block::default().borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT))',
        '.block(\n                Block::default().borders(Borders::BOTTOM).border_style(Style::default().fg(theme.border_subtle)),\n            )')

open(p, 'w', encoding='utf-8').write(s)
print('dynamic done, left:', s.count('borders(Borders::'))

# ---------- dynamic_detail.rs ----------
p = 'src/ui/dynamic_detail.rs'
s = open(p, encoding='utf-8').read()

# title block
s = rep(s, '''            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(theme.border_unfocused)),
            );
        frame.render_widget(title, chunks[0]);''',
'''            .block(panel_block(theme, None, false));
        frame.render_widget(title, chunks[0]);''')

# loading block
s = rep(s, '''                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(theme.border_focused)),
                );''',
'''                .block(panel_block(theme, None, false));''')

# error block
s = rep(s, '''                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(Color::Red)),
                );''',
'''                .block(panel_block(theme, None, false));''')

# input box
s = rep(s, '''            let input_block = Block::default()
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

# images panel
s = rep(s, '''        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border_focused))
            .title(format!(
                " 图片 {}/{} [h/l 切换] ",
                self.current_image_index + 1,
                self.image_urls.len()
            ));''',
'''        let block = panel_block(
            theme,
            Some(Line::from(Span::styled(
                format!(
                    " 图片 {}/{} [h/l 切换] ",
                    self.current_image_index + 1,
                    self.image_urls.len()
                ),
                Style::default().fg(theme.bilibili_pink),
            ))),
            true,
        );''')

open(p, 'w', encoding='utf-8').write(s)
print('dynamic_detail done, left:', s.count('borders(Borders::'))
m = re.search(r'use super::\{([^}]*)\};', s)
print('dd super import:', m.group(0) if m else None)
