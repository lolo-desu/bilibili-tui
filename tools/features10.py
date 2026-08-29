# -*- coding: utf-8 -*-
# 7. tab-style source list  10. selected card: bg + square outline  11. square corners

def rep(s, old, new, count=1):
    assert old in s, 'MISSING: ' + old[:70]
    return s.replace(old, new, count)

# ---- 7. tab-style category column ----
p = 'src/ui/home.rs'
s = open(p, encoding='utf-8').read()
old = s[s.find('    fn draw_sources(&self, frame: &mut Frame, area: Rect, theme: &Theme) {'):]
old = old[:old.find('\n    fn ')]
new_fn = '''    fn draw_sources(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        // Vertical tab strip: selected tab is a full-width pill on the
        // content surface with a left accent bar (web tab style).
        let block = Block::default()
            .style(Style::default().bg(theme.bg_secondary))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(if self.focus_sources {
                theme.border_focused
            } else {
                theme.bg_secondary
            }))
            .title(Line::from(Span::styled(
                " 首页 ",
                Style::default().fg(theme.fg_muted),
            )));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        for (index, row) in inner.rows().enumerate() {
            if index >= self.source_count() {
                break;
            }
            let is_selected = index == self.selected_source;
            let area_row = Rect {
                x: inner.x,
                y: row.y,
                width: inner.width,
                height: 1,
            };
            let label = self.source_label(index);
            if is_selected {
                let selected = Line::from(vec![
                    Span::styled(
                        "▌",
                        Style::default()
                            .fg(theme.bilibili_pink)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        label,
                        Style::default()
                            .fg(theme.fg_primary)
                            .add_modifier(Modifier::BOLD)
                            .bg(theme.bg_card),
                    ),
                ]);
                // paint the tab pill full width, then the text over it
                frame.render_widget(
                    Block::default().style(Style::default().bg(theme.bg_card)),
                    area_row,
                );
                frame.render_widget(Paragraph::new(selected), area_row);
            } else {
                let normal = Paragraph::new(Line::from(Span::styled(
                    format!("  {label}"),
                    Style::default().fg(theme.fg_secondary),
                )));
                frame.render_widget(normal, area_row);
            }
        }
    }
'''
s = s.replace(old, new_fn, 1)
open(p, 'w', encoding='utf-8').write(s)
print('7. tabs ok')

# ---- 10. selected video: bg highlight + square border; 11. square corners ----
p = 'src/ui/home.rs'
s = open(p, encoding='utf-8').read()
# square border on home cards
s = s.replace('.border_type(BorderType::Rounded)', '.border_type(BorderType::Plain)')
open(p, 'w', encoding='utf-8').write(s)
print('10a. home square ok')

p = 'src/ui/video_card.rs'
s = open(p, encoding='utf-8').read()
s = s.replace('.border_type(BorderType::Rounded)', '.border_type(BorderType::Plain)')
open(p, 'w', encoding='utf-8').write(s)
print('10b. video_card square ok')

# selection: border_focused outline stays, plus bg_highlight on selected card
for p, old, new in [
    ('src/ui/video_card.rs',
     '''        // selection reads as a thin outline + pink marker; blocks stay calm
        let border_color = if is_selected {
            theme.border_focused
        } else {
            theme.bg_card
        };''',
     '''        // selection = outline + subtle background lift, both at once
        let border_color = if is_selected {
            theme.border_focused
        } else {
            theme.bg_card
        };
        let card_bg = if is_selected {
            theme.bg_highlight
        } else {
            theme.bg_card
        };'''),
]:
    s = open(p, encoding='utf-8').read()
    s = rep(s, old, new)
    s = s.replace('.style(Style::default().bg(theme.bg_card))', '.style(Style::default().bg(card_bg))')
    open(p, 'w', encoding='utf-8').write(s)
print('10c. video_card bg ok')

# home list card: same treatment
p = 'src/ui/home.rs'
s = open(p, encoding='utf-8').read()
s = rep(s, '''        let block = Block::default()
            .style(Style::default().bg(theme.bg_card))
            .borders(Borders::ALL)
            .border_type(BorderType::Plain)
            .border_style(Style::default().fg(if is_selected {
                theme.border_focused
            } else {
                theme.bg_card
            }))
            .title(title_span);''',
'''        let card_bg = if is_selected {
            theme.bg_highlight
        } else {
            theme.bg_card
        };
        let block = Block::default()
            .style(Style::default().bg(card_bg))
            .borders(Borders::ALL)
            .border_type(BorderType::Plain)
            .border_style(Style::default().fg(if is_selected {
                theme.border_focused
            } else {
                theme.bg_card
            }))
            .title(title_span);''')
open(p, 'w', encoding='utf-8').write(s)
print('10d. home card bg ok')
