//! Left sidebar navigation component

use super::Theme;
use super::icons;
use ratatui::{prelude::*, widgets::*};

/// Navigation menu items
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavItem {
    Home,
    Search,
    Dynamic,
    History,
    Favorites,
    Live,
    Bangumi,
    Settings,
}

impl NavItem {
    pub fn label(&self) -> String {
        match self {
            NavItem::Home => format!("{} 首页", icons::HOME),
            NavItem::Search => format!("{} 搜索", icons::SEARCH),
            NavItem::Dynamic => format!("{} 动态", icons::TV),
            NavItem::History => format!("{} 历史", icons::FEED),
            NavItem::Favorites => format!("{} 收藏夹", icons::STAR),
            NavItem::Live => format!("{} 直播", icons::SIGNAL),
            NavItem::Bangumi => format!("{} 番剧", icons::PLAY),
            NavItem::Settings => format!("{} 设置", icons::GEAR),
        }
    }

    pub fn all() -> &'static [NavItem] {
        &[
            NavItem::Home,
            NavItem::Dynamic,
            NavItem::History,
            NavItem::Favorites,
            NavItem::Live,
            NavItem::Bangumi,
            NavItem::Settings,
        ]
    }
}

pub struct Sidebar {
    pub selected: NavItem,
}

impl Sidebar {
    pub fn new() -> Self {
        Self {
            selected: NavItem::Home,
        }
    }

    pub fn draw(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        // Background panel instead of border lines (opencode style): the
        // sidebar reads as a colored surface, content area stays on bg_primary.
        let block = Block::default().style(Style::default().bg(theme.bg_secondary));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Split into header and nav items
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4), // Header with branding
                Constraint::Length(1), // Separator
                Constraint::Min(5),    // Nav items
                Constraint::Length(1), // Footer separator
                Constraint::Length(1), // Version
            ])
            .split(inner);

        // Bilibili branding header with modern styling
        let brand_lines = vec![
            Line::raw(""),
            Line::from(vec![
                Span::styled(
                    "  ▌",
                    Style::default()
                        .fg(theme.bilibili_pink)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "B",
                    Style::default()
                        .fg(theme.bilibili_pink)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "ilibili",
                    Style::default()
                        .fg(theme.fg_primary)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![Span::styled(
                "   TUI Client",
                Style::default()
                    .fg(theme.fg_muted)
                    .add_modifier(Modifier::ITALIC),
            )]),
        ];
        let brand = Paragraph::new(brand_lines);
        frame.render_widget(brand, chunks[0]);

        // Separator line with gradient effect
        let separator =
            Paragraph::new("  ────────────").style(Style::default().fg(theme.border_subtle));
        frame.render_widget(separator, chunks[1]);

        // Nav items with modern block selection indicator
        let items: Vec<ListItem> = NavItem::all()
            .iter()
            .map(|item| {
                let is_selected = *item == self.selected;
                let style = if is_selected {
                    Style::default()
                        .fg(theme.bilibili_pink)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.fg_secondary)
                };

                // Use block indicator for selection instead of arrow
                let prefix = if is_selected { " ▌" } else { "  " };
                let suffix = if is_selected { " " } else { "" };
                ListItem::new(format!("{}{}{}", prefix, item.label(), suffix)).style(style)
            })
            .collect();

        let list = List::new(items).highlight_style(Style::default().add_modifier(Modifier::BOLD));

        frame.render_widget(list, chunks[2]);

        // Version tag so it is easy to tell which build is running
        let version = Paragraph::new(Line::from(Span::styled(
            format!("  v{}", env!("CARGO_PKG_VERSION")),
            Style::default().fg(theme.fg_muted),
        )));
        frame.render_widget(version, chunks[4]);
    }

    pub fn next(&mut self) {
        let items = NavItem::all();
        let current_idx = items.iter().position(|i| *i == self.selected).unwrap_or(0);
        let next_idx = (current_idx + 1) % items.len();
        self.selected = items[next_idx];
    }

    pub fn prev(&mut self) {
        let items = NavItem::all();
        let current_idx = items.iter().position(|i| *i == self.selected).unwrap_or(0);
        let prev_idx = if current_idx == 0 {
            items.len() - 1
        } else {
            current_idx - 1
        };
        self.selected = items[prev_idx];
    }

    pub fn select(&mut self, item: NavItem) {
        self.selected = item;
    }
}

impl Default for Sidebar {
    fn default() -> Self {
        Self::new()
    }
}
