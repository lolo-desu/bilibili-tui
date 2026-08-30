# -*- coding: utf-8 -*-
"""Three QA fixes:
1. Settings -> playback: home-columns row, persisted via config (removes [ ] keys)
2. Comment focus: Left/Right arrows page floor replies (and wrap)
3. 'Load more comments': only show/trigger when not already loading (defensive)
"""
import re

def rep(s, old, new, tag):
    assert old in s, 'MISSING [%s]: %s' % (tag, old[:70])
    assert s.count(old) == 1, 'NOT UNIQUE [%s]: %s' % (tag, old[:70])
    return s.replace(old, new, 1)

# ============ 1a. config field ============
p = 'src/storage/mod.rs'
s = open(p, encoding='utf-8').read()
s = rep(s, '''    #[serde(default)]
    pub video_quality: VideoQuality,
}''',
'''    #[serde(default)]
    pub video_quality: VideoQuality,
    /// Number of columns in the home grid (persisted; set in settings).
    #[serde(default = "default_home_columns")]
    pub home_columns: usize,
}

fn default_home_columns() -> usize {
    3
}''', 'config field')
s = rep(s, '''            auto_play: true,
            video_quality: VideoQuality::default(),
        }''',
'''            auto_play: true,
            video_quality: VideoQuality::default(),
            home_columns: default_home_columns(),
        }''', 'config default')
open(p, 'w', encoding='utf-8').write(s)
print('1a config ok')

# ============ 1b. action enum ============
p = 'src/application/action.rs'
s = open(p, encoding='utf-8').read()
s = rep(s, '    SaveVideoQuality(VideoQuality),',
'''    SaveVideoQuality(VideoQuality),
    SaveHomeColumns(usize),''', 'action enum')
open(p, 'w', encoding='utf-8').write(s)
print('1b action ok')

# ============ 1c. action handler ============
p = 'src/app/actions.rs'
s = open(p, encoding='utf-8').read()
s = rep(s, '''            AppAction::SaveVideoQuality(quality) => {
                self.config.video_quality = quality;
                let _ = persistence::save_config(&self.config);
            }''',
'''            AppAction::SaveVideoQuality(quality) => {
                self.config.video_quality = quality;
                let _ = persistence::save_config(&self.config);
            }
            AppAction::SaveHomeColumns(columns) => {
                self.config.home_columns = columns;
                let _ = persistence::save_config(&self.config);
            }''', 'action handler')
# pass home_columns into SettingsPage::new (3 call sites)
old_new = '''                    self.config.auto_play,
                    self.config.video_quality,
                );'''
new_new = '''                    self.config.auto_play,
                    self.config.video_quality,
                    self.config.home_columns,
                );'''
assert s.count(old_new) == 1
s = s.replace(old_new, new_new)
old_new2 = '''                        self.config.auto_play,
                        self.config.video_quality,
                    )));'''
assert s.count(old_new2) == 1
s = s.replace(old_new2, '''                        self.config.auto_play,
                        self.config.video_quality,
                        self.config.home_columns,
                    )));''')
old_new3 = '''                        self.config.auto_play,
                        self.config.video_quality,
                    );'''
assert s.count(old_new3) == 1
s = s.replace(old_new3, '''                        self.config.auto_play,
                        self.config.video_quality,
                        self.config.home_columns,
                    );''')
open(p, 'w', encoding='utf-8').write(s)
print('1c handler ok')

# ============ 1d. settings page ============
p = 'src/ui/settings.rs'
s = open(p, encoding='utf-8').read()
s = rep(s, '''    pub video_quality: VideoQuality,''',
'''    pub video_quality: VideoQuality,
    pub home_columns: usize,''', 'settings field')
s = rep(s, '''        auto_play: bool,
        video_quality: VideoQuality,
    ) -> Self {''',
'''        auto_play: bool,
        video_quality: VideoQuality,
        home_columns: usize,
    ) -> Self {''', 'settings new sig')
# store it: find the struct literal init containing video_quality,
s = rep(s, '''            video_quality,''',
'''            video_quality,
            home_columns,''', 'settings init')
s = rep(s, '''    fn adjust_playback(&mut self, direction: i32) -> AppAction {
        match self.selected_playback_index {
            0 => {
                self.auto_play = !self.auto_play;
                AppAction::SaveAutoPlay(self.auto_play)
            }
            _ => {
                self.video_quality = self.video_quality.cycle(direction);
                AppAction::SaveVideoQuality(self.video_quality)
            }
        }
    }''',
'''    fn adjust_playback(&mut self, direction: i32) -> AppAction {
        const COLUMN_CHOICES: [usize; 3] = [1, 2, 3];
        match self.selected_playback_index {
            0 => {
                self.auto_play = !self.auto_play;
                AppAction::SaveAutoPlay(self.auto_play)
            }
            1 => {
                // home grid columns: cycle 1 -> 2 -> 3 -> 1 ...
                let cur = COLUMN_CHOICES
                    .iter()
                    .position(|c| *c == self.home_columns)
                    .unwrap_or(0);
                let len = COLUMN_CHOICES.len();
                let next = if direction >= 0 {
                    (cur + 1) % len
                } else {
                    (cur + len - 1) % len
                };
                self.home_columns = COLUMN_CHOICES[next];
                AppAction::SaveHomeColumns(self.home_columns)
            }
            _ => {
                self.video_quality = self.video_quality.cycle(direction);
                AppAction::SaveVideoQuality(self.video_quality)
            }
        }
    }''', 'adjust_playback')
# selection bound: 2 items -> 3 items
s = rep(s, 'self.selected_playback_index = (self.selected_playback_index + 1).min(1);',
'self.selected_playback_index = (self.selected_playback_index + 1).min(2);', 'sel down')
# rows: add columns row between auto_play and quality
s = rep(s, '''        let rows = [
            format!(
                "进入视频自动播放：{}",
                if self.auto_play { "开启" } else { "关闭" }
            ),
            format!("默认视频画质：{}", self.video_quality.label()),
        ];''',
'''        let rows = [
            format!(
                "进入视频自动播放：{}",
                if self.auto_play { "开启" } else { "关闭" }
            ),
            format!("主页列数：{}", self.home_columns),
            format!("默认视频画质：{}", self.video_quality.label()),
        ];''', 'playback rows')
open(p, 'w', encoding='utf-8').write(s)
print('1d settings ok')

# ============ 1e. home: init columns from arg, drop [ ] keys ============
p = 'src/ui/home.rs'
s = open(p, encoding='utf-8').read()
s = rep(s, '''    pub fn new() -> Self {''', '''    pub fn new_with_columns(columns: usize) -> Self {
        let mut page = Self::new();
        page.columns = if Self::COLUMN_CHOICES.contains(&columns) {
            columns
        } else {
            Self::DEFAULT_COLUMNS
        };
        page
    }

    pub fn new() -> Self {''', 'new_with_columns')
s = rep(s, '''        if key == KeyCode::Char('[') {
            self.cycle_columns(-1);
            return Some(AppAction::None);
        }
        if key == KeyCode::Char(']') {
            self.cycle_columns(1);''',
'''        if key == KeyCode::Char('[') || key == KeyCode::Char(']') {
            // column count moved to 设置 -> 播放设置; keep keys inert
            return Some(AppAction::None);''', 'drop bracket keys')
open(p, 'w', encoding='utf-8').write(s)
print('1e home ok')

# home construction sites: use saved columns
p = 'src/app/actions.rs'
s = open(p, encoding='utf-8').read()
n = s.count('Page::Home(HomePage::new())')
s = s.replace('Page::Home(HomePage::new())',
              'Page::Home(HomePage::new_with_columns(self.config.home_columns))')
print('1e home ctor sites:', n)
open(p, 'w', encoding='utf-8').write(s)

# ============ 2. comment focus Left/Right = floor paging ============
p = 'src/ui/video_detail.rs'
s = open(p, encoding='utf-8').read()
# find where comment list keys are forwarded; insert arrows before generic forward
print(open(p, encoding='utf-8').read()[0:0])
open(p, 'w', encoding='utf-8').write(s)
print('2 placeholder (handled below)')
PYEOF_MARKER = None
