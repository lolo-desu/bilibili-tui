# -*- coding: utf-8 -*-
# Batch feature edits for bilibili-tui (run: python -X utf8 tools/features1.py)

# ============ 1. icons: add missing glyphs ============
p = 'src/ui/icons.rs'
s = open(p, encoding='utf-8').read()
s += r'''
// Settings section icons
pub const KEYBOARD: &str = "\u{f11c}"; // nf-fa-keyboard
pub const SLIDERS: &str = "\u{f1de}"; // nf-fa-sliders
// Layout / sort icons
pub const GRID: &str = "\u{f00a}"; // nf-fa-th (3x3 grid)
pub const SORT_AMT: &str = "\u{f160}"; // nf-fa-sort-amount-asc
pub const FIRE_ALT: &str = "\u{f7e4}"; // nf-fa-fire_alt
pub const CLOCK_O: &str = "\u{f017}"; // nf-fa-clock_o
// Emote / smiley
pub const SMILE: &str = "\u{f118}"; // nf-fa-smile_o
'''
open(p, 'w', encoding='utf-8').write(s)

# ============ 2. home.rs: column cycling with [ ] ============
p = 'src/ui/home.rs'
s = open(p, encoding='utf-8').read()

s = s.replace('const DEFAULT_COLUMNS: usize = 1;', '''const DEFAULT_COLUMNS: usize = 1;
    const COLUMN_CHOICES: [usize; 4] = [1, 2, 3, 4];''')

old = '    fn handle_mouse(&mut self, event: MouseEvent, area: Rect) -> Option<AppAction> {'
new = '''    /// Switch grid column count (1/2/3/4) in the given direction.
    pub fn cycle_columns(&mut self, direction: i32) {
        let cur = Self::COLUMN_CHOICES
            .iter()
            .position(|c| *c == self.columns)
            .unwrap_or(0);
        let len = Self::COLUMN_CHOICES.len();
        let next = if direction >= 0 {
            (cur + 1) % len
        } else {
            (cur + len - 1) % len
        };
        self.columns = Self::COLUMN_CHOICES[next];
        self.scroll_row = self.selected_index / self.columns.max(1);
        self.update_scroll(self.cached_visible_rows);
    }

    fn handle_mouse(&mut self, event: MouseEvent, area: Rect) -> Option<AppAction> {'''
assert old in s, 'home handle_mouse anchor'
s = s.replace(old, new, 1)

old2 = '''        if keys.matches_next_theme(key) {
            return Some(AppAction::NextTheme);
        }
        if keys.matches_open_settings(key) {
            return Some(AppAction::SwitchToSettings);
        }
        Some(AppAction::None)
    }'''
new2 = '''        if key == KeyCode::Char('[') {
            self.cycle_columns(-1);
            return Some(AppAction::None);
        }
        if key == KeyCode::Char(']') {
            self.cycle_columns(1);
            return Some(AppAction::None);
        }
        if keys.matches_next_theme(key) {
            return Some(AppAction::NextTheme);
        }
        if keys.matches_open_settings(key) {
            return Some(AppAction::SwitchToSettings);
        }
        Some(AppAction::None)
    }'''
assert old2 in s, 'home next_theme anchor'
s = s.replace(old2, new2, 1)
open(p, 'w', encoding='utf-8').write(s)

# ============ 3. settings.rs: section icons + [ ] direct section nav ============
p = 'src/ui/settings.rs'
s = open(p, encoding='utf-8').read()

old3 = '''        SettingsSection::Theme => format!("{} 主题", icons::PAINT),
            SettingsSection::Danmaku => format!("{} 弹幕", icons::COMMENT),
            SettingsSection::Playback => "播放".to_string(),
            SettingsSection::Keybindings => "快捷键".to_string(),
            SettingsSection::Account => format!("{} 账户", icons::USER),'''
new3 = '''        SettingsSection::Theme => format!("{} 主题", icons::PAINT),
            SettingsSection::Danmaku => format!("{} 弹幕", icons::COMMENT),
            SettingsSection::Playback => format!("{} 播放", icons::SLIDERS),
            SettingsSection::Keybindings => format!("{} 快捷键", icons::KEYBOARD),
            SettingsSection::Account => format!("{} 账户", icons::USER),'''
assert old3 in s, 'settings labels'
s = s.replace(old3, new3, 1)

# handle_input: [ ] switch section directly; nav_left/right only adjust values
old4 = '''        if keys.matches_section_prev(key) {
            // Cycle through sections backwards
            let sections = SettingsSection::all();
            self.section_index = if self.section_index == 0 {
                sections.len() - 1
            } else {
                self.section_index - 1
            };
            self.current_section = sections[self.section_index];
            return Some(AppAction::None);
        }
        if keys.matches_section_next(key) {
            // Cycle through sections forwards
            let sections = SettingsSection::all();
            self.section_index = (self.section_index + 1) % sections.len();
            self.current_section = sections[self.section_index];
            return Some(AppAction::None);
        }
        if (keys.matches_left(key) || keys.matches_right(key))
            && self.current_section == SettingsSection::Playback
        {
            return Some(self.adjust_playback(if keys.matches_right(key) { 1 } else { -1 }));
        }
        if keys.matches_left(key) || keys.matches_right(key) {
            self.change_section(if keys.matches_right(key) { 1 } else { -1 });
            return Some(AppAction::None);
        }'''
new4 = '''        // [ / ] move between sections directly - never conflicts with
        // left/right which adjust option values inside a section.
        if key == KeyCode::Char('[') {
            self.change_section(-1);
            return Some(AppAction::None);
        }
        if key == KeyCode::Char(']') {
            self.change_section(1);
            return Some(AppAction::None);
        }
        if keys.matches_section_prev(key) {
            self.change_section(-1);
            return Some(AppAction::None);
        }
        if keys.matches_section_next(key) {
            self.change_section(1);
            return Some(AppAction::None);
        }
        if (keys.matches_left(key) || keys.matches_right(key))
            && self.current_section == SettingsSection::Playback
        {
            return Some(self.adjust_playback(if keys.matches_right(key) { 1 } else { -1 }));
        }'''
assert old4 in s, 'settings sections'
s = s.replace(old4, new4, 1)

# footer hints: show [ ] for sections
old5 = '''                Span::styled(
                    format!("{}{}", keys.section_prev, keys.section_next),
                    Style::default()
                        .fg(theme.fg_accent)
                        .add_modifier(Modifier::BOLD),
                ),'''
new5 = '''                Span::styled(
                    "[ ]".to_string(),
                    Style::default()
                        .fg(theme.fg_accent)
                        .add_modifier(Modifier::BOLD),
                ),'''
assert old5 in s, 'settings footer'
s = s.replace(old5, new5, 1)
open(p, 'w', encoding='utf-8').write(s)
print('features1 done')
