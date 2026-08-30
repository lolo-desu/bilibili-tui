# -*- coding: utf-8 -*-

def rep(s, old, new):
    assert old in s, 'MISSING: ' + old[:80]
    return s.replace(old, new, 1)

# 1) icons: PLAY/DANMAKU/COMMENT -> glyphs proven in the user's font
p = 'src/ui/icons.rs'
s = open(p, encoding='utf-8').read()
s = rep(s, '''/// Play (nf-md-play)
pub const PLAY: &str = "\\u{f040b}";''',
'''/// Play (nf-fa-play, common fa range)
pub const PLAY: &str = "\\u{f04b}";''')
s = rep(s, '''/// Comment bubble (nf-md-comment-text_multiple_outline)
pub const COMMENT: &str = "\\u{f0192}"; // nf-fa-comments, broader support''',
'''/// Comment bubble (nf-fa-comment_o, common fa range)
pub const COMMENT: &str = "\\u{f0e5}";''')
s = rep(s, '''/// Danmaku list (nf-fa-comment_dots) - chat bubbles
pub const DANMAKU: &str = "\\u{f27b}";''',
'''/// Danmaku list (nf-fa-commenting_o, common fa range)
pub const DANMAKU: &str = "\\u{f27a}";''')
open(p, 'w', encoding='utf-8').write(s)
print('icons ok')

# 2) home grid rows: Length so cards keep 16 lines (no vertical stretch);
#    cover area becomes adaptive: height = max(7, width/2 + 1) for 16:9 feel.
p = 'src/ui/home.rs'
s = open(p, encoding='utf-8').read()
s = rep(s, '''            .map(|_| Constraint::Min(self.effective_card_height()))''',
'''            .map(|_| Constraint::Length(self.effective_card_height()))''')
s = rep(s, '''                .constraints([
                    Constraint::Length(7), // 16:9 cover (full width)
                    Constraint::Length(1), // breathing room between cover and text
                    Constraint::Min(4),    // text block
                ])''',
'''                .constraints([
                    Constraint::Length(cover_h), // ~16:9 cover (full width)
                    Constraint::Length(1), // breathing room between cover and text
                    Constraint::Min(5),    // text block
                ])''')
s = rep(s, '''        let vertical = self.columns >= 3;
        let card_chunks = if vertical {''',
'''        let vertical = self.columns >= 3;
        // Cover aspect: terminal cells are ~1:2, so a 16:9 image fills
        // roughly (width/2)+1 rows; clamp for narrow cards.
        let cover_h = if vertical {
            ((inner.width as usize / 2) + 1).clamp(7, 9) as u16
        } else {
            7
        };
        let card_chunks = if vertical {''')
# GRID_CARD_HEIGHT 14 -> 16 (cover up to 9 rows + 5 text + 2 borders)
s = rep(s, '''    /// Height of vertical grid cards used at 3-4 columns (cover + text rows).
    const GRID_CARD_HEIGHT: u16 = 14;''',
'''    /// Height of vertical grid cards used at 3-4 columns (cover + text rows).
    const GRID_CARD_HEIGHT: u16 = 16;''')
open(p, 'w', encoding='utf-8').write(s)
print('home ok')

# 3) detail: 62/38 percentage leaves a gap column; use Min/Min split.
p = 'src/ui/video_detail.rs'
s = open(p, encoding='user' if False else 'utf-8').read()
s = rep(s, '''        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(62), Constraint::Percentage(38)])
            .split(area);''',
'''        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(40), Constraint::Length(56)])
            .split(area);''')
open(p, 'w', encoding='utf-8').write(s)
print('detail split ok')
