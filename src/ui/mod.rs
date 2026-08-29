mod article_detail;
mod bangumi;
mod bangumi_detail;
pub mod comment_list;
mod dynamic;
mod dynamic_detail;
mod favorites;
mod history;
mod home;
pub mod icons;
pub(crate) mod image_picker;
mod live;
mod live_detail;
mod login;
mod search;
mod settings;
mod sidebar;
pub mod theme;
mod up;
mod video_card;
mod video_detail;

pub use article_detail::ArticleDetailPage;
pub use bangumi::BangumiPage;
pub use bangumi_detail::BangumiDetailPage;
pub use dynamic::{DynamicPage, DynamicTab};
pub use dynamic_detail::DynamicDetailPage;
pub use favorites::FavoritesPage;
pub use history::HistoryPage;
pub use home::HomePage;
pub use live::LivePage;
pub use live_detail::LiveDetailPage;
pub use login::LoginPage;
pub use search::SearchPage;
pub use settings::SettingsPage;
pub use sidebar::{NavItem, Sidebar};
pub use theme::{DEFAULT_THEME_ID, Theme, ThemeChoice};
pub use up::UpPage;
pub use video_card::{VideoCard, VideoCardGrid};
pub use video_detail::VideoDetailPage;

use crate::application::AppAction;
use crate::storage::Keybindings;
use ratatui::{
    Frame,
    crossterm::event::{KeyCode, KeyModifiers, MouseEvent},
    prelude::{Color, Line, Modifier, Rect, Span, Style},
    widgets::{Block, BorderType, Borders},
};

/// Build the centered, bracketed shortcut footer used across list pages.
/// Each tuple is `(shortcut, label, color)`; shortcut text is emphasized while
/// labels and brackets use the secondary foreground.
pub fn shortcut_footer(
    theme: &Theme,
    items: impl IntoIterator<Item = (String, String, Color)>,
) -> Line<'static> {
    let dim = Style::default().fg(theme.border_subtle);
    let key = Style::default().fg(theme.fg_muted);
    let mut spans = Vec::new();
    for (index, (shortcut, label, color)) in items.into_iter().enumerate() {
        if index > 0 {
            spans.push(Span::styled("   ", dim));
            spans.push(Span::styled("│ ", dim));
        }
        spans.push(Span::styled("[", dim));
        spans.push(Span::styled(
            shortcut,
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled("] ", dim));
        spans.push(Span::styled(label, key));
    }
    Line::from(spans)
}

/// Download an image and center-crop it to a uniform 16:9 cover so every
/// card in a grid shows the same aspect ratio (hard crop, no distortion).
pub async fn download_cover(url: &str) -> Option<image::DynamicImage> {
    let response = reqwest::get(url).await.ok()?;
    let bytes = response.bytes().await.ok()?;
    let img = image::load_from_memory(&bytes).ok()?;
    Some(crop_cover(img))
}

/// Center-crop an image to the widest 16:9 rectangle that fits inside it.
/// For taller images this crops away top/bottom (favoring the upper part,
/// where covers usually put their subject); for wider ones, the sides.
pub fn crop_cover(img: image::DynamicImage) -> image::DynamicImage {
    use image::GenericImageView;
    const TARGET: f32 = 16.0 / 9.0;
    const TOP_BIAS: f32 = 0.35; // keep the crop window toward the top
    let (w, h) = img.dimensions();
    if w == 0 || h == 0 {
        return img;
    }
    let ratio = w as f32 / h as f32;
    let (cw, ch, x, y) = if ratio > TARGET {
        let cw = ((h as f32 * TARGET).round() as u32).min(w);
        (cw, h, (w - cw) / 2, 0)
    } else {
        let ch = ((w as f32 / TARGET).round() as u32).min(h);
        let offset = h.saturating_sub(ch);
        let y = (offset as f32 * TOP_BIAS).round() as u32;
        (w, ch, 0, y)
    };
    img.crop_imm(x, y, cw, ch)
}

/// Build a panel that separates content with a background color block
/// (opencode style) instead of strong border lines. A faint title row sits
/// at the top inside the panel. Focus is expressed by a thin highlighted
/// outline (border_focused), never by brightening the whole block: unfocused
/// panels draw their "border" in the panel's own background color so it
/// stays invisible and only the focused panel shows an outline.
pub fn panel_block<'a>(theme: &Theme, title: Option<Line<'a>>, focused: bool) -> Block<'a> {
    let mut block = Block::default()
        .style(Style::default().bg(theme.bg_card))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(if focused {
            theme.border_focused
        } else {
            theme.bg_card
        }));
    if let Some(title) = title {
        block = block.title(title.style(Style::default().fg(theme.fg_muted)));
    }
    block
}

/// UI Component trait
pub trait Component {
    fn draw(&mut self, frame: &mut Frame, area: Rect, theme: &Theme, keys: &Keybindings);
    fn handle_input(&mut self, key: KeyCode, keys: &Keybindings) -> Option<AppAction> {
        let _ = (key, keys);
        None
    }
    fn handle_input_with_modifiers(
        &mut self,
        key: KeyCode,
        modifiers: KeyModifiers,
        keys: &Keybindings,
    ) -> Option<AppAction> {
        let _ = modifiers;
        self.handle_input(key, keys)
    }
    fn handle_mouse(&mut self, event: MouseEvent, area: Rect) -> Option<AppAction> {
        let _ = (event, area);
        None
    }
}

/// Application pages
pub enum Page {
    Login(LoginPage),
    Home(HomePage),
    Search(SearchPage),
    Dynamic(DynamicPage),
    DynamicDetail(Box<DynamicDetailPage>),
    ArticleDetail(Box<ArticleDetailPage>),
    VideoDetail(Box<VideoDetailPage>),
    Up(Box<UpPage>),
    History(HistoryPage),
    Favorites(FavoritesPage),
    Live(LivePage),
    LiveDetail(Box<LiveDetailPage>),
    Settings(Box<SettingsPage>),
    Bangumi(Box<BangumiPage>),
    BangumiDetail(Box<BangumiDetailPage>),
}
