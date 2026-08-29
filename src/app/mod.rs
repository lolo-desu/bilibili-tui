mod actions;
mod network_events;
mod runtime;

use crate::application::network;
use crate::domain::playback::{PlaybackEvent, PlaybackState};
use crate::infrastructure::{
    bilibili::{ApiClient, LiveDanmakuHub},
    persistence::{self, AppConfig, Credentials, Keybindings},
};
use crate::presentation::tui::{
    BangumiPage, DEFAULT_THEME_ID, HomePage, Page, SettingsPage, Sidebar, Theme, UpPage,
    VideoDetailPage,
};
use crate::ui::icons;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::mpsc;
use tokio::sync::watch;

#[derive(Default)]
struct RequestTracker {
    sequence: u64,
    pending: HashMap<&'static str, u64>,
}

impl RequestTracker {
    fn next(&mut self, key: &'static str) -> u64 {
        self.sequence = self.sequence.saturating_add(1);
        self.pending.insert(key, self.sequence);
        self.sequence
    }

    fn is_latest(&self, key: &'static str, request_id: u64) -> bool {
        self.pending
            .get(key)
            .is_some_and(|latest| *latest == request_id)
    }
}

/// Previous page for back navigation
#[derive(Clone)]
pub enum PreviousPage {
    Home,
    Search,
    Dynamic,
    History,
    Favorites,
    Live,
    Bangumi,
}

/// Main application state
pub struct App {
    pub current_page: Page,
    pub should_quit: bool,
    pub api_client: Arc<ApiClient>,
    pub credentials: Option<Credentials>,
    pub sidebar: Sidebar,
    pub show_sidebar: bool,

    pub previous_page: Option<PreviousPage>,
    /// Full page instances for nested detail navigation (list -> video -> UP).
    pub navigation_stack: Vec<Page>,
    pub theme: Theme,
    pub theme_id: String,
    pub config: AppConfig,
    pub keybindings: Keybindings,
    pub pending_home_notice: Option<String>,
    pub playback: PlaybackState,
    pub live_danmaku_hub: Option<Arc<LiveDanmakuHub>>,
    danmaku_config_tx: watch::Sender<crate::storage::DanmakuConfig>,
    playback_event_tx: mpsc::Sender<PlaybackEvent>,
    playback_event_rx: mpsc::Receiver<PlaybackEvent>,
    auto_return_after_playback: Option<(u64, String)>,
    next_playback_session_id: u64,
    pending_playlist: Option<(
        Vec<crate::domain::playback::PlaylistItem>,
        crate::domain::playback::PlaylistSource,
        usize,
        crate::domain::playback::PlayOrder,
    )>,

    /// Cached home page to avoid refresh when switching tabs
    pub cached_home: Option<HomePage>,
    /// Cached bangumi page to avoid refresh when switching tabs
    pub cached_bangumi: Option<BangumiPage>,
    network_command_tx: mpsc::Sender<network::NetworkCommand>,
    network_event_rx: mpsc::Receiver<network::NetworkEvent>,
    request_tracker: RequestTracker,
}

impl App {
    pub fn new() -> Self {
        Self::new_with_open(None)
    }

    pub fn new_with_open(open_spec: Option<&str>) -> Self {
        let credentials = persistence::load_credentials().ok();
        let api_client = if let Some(ref creds) = credentials {
            ApiClient::with_cookies(creds)
        } else {
            ApiClient::new()
        };
        let api_client = Arc::new(api_client);
        let bridge = network::start_network_worker(api_client.clone());
        let (playback_event_tx, playback_event_rx) = mpsc::channel();

        // Load config and apply saved theme
        let config = persistence::load_config().unwrap_or_default();
        let (danmaku_config_tx, _) = watch::channel(config.danmaku.clone());
        let keybindings = config.keybindings.clone();
        let configured_theme_id = config.theme.clone();
        let (theme, used_fallback) = Theme::load_or_default(&configured_theme_id);
        let theme_id = if used_fallback {
            DEFAULT_THEME_ID.to_string()
        } else {
            configured_theme_id
        };

        // Always start from home. Login is now an optional flow.
        let current_page = Self::page_for_open_spec(
            open_spec,
            &keybindings,
            &theme_id,
            credentials.is_some(),
            &config,
        );

        Self {
            current_page,
            should_quit: false,
            api_client,
            credentials,
            sidebar: Sidebar::new(),
            show_sidebar: true,
            previous_page: None,
            navigation_stack: Vec::new(),
            theme,
            theme_id,
            config,
            keybindings,
            pending_home_notice: used_fallback.then_some(
                format!("{} 旧主题配置无效，请前往设置页重新选择主题", icons::WARN).to_string(),
            ),
            playback: PlaybackState::default(),
            live_danmaku_hub: None,
            danmaku_config_tx,
            playback_event_tx,
            playback_event_rx,
            auto_return_after_playback: None,
            next_playback_session_id: 1,
            pending_playlist: None,
            cached_home: None,
            cached_bangumi: None,
            network_command_tx: bridge.command_tx,
            network_event_rx: bridge.event_rx,
            request_tracker: RequestTracker::default(),
        }
    }

    /// Resolve a `--open` deep-link spec into the initial page.
    fn page_for_open_spec(
        spec: Option<&str>,
        keybindings: &Keybindings,
        theme_id: &str,
        is_logged_in: bool,
        config: &crate::storage::AppConfig,
    ) -> Page {
        match spec {
            Some("settings") => Page::Settings(Box::new(SettingsPage::new(
                keybindings.clone(),
                theme_id.to_string(),
                is_logged_in,
                config.danmaku.clone(),
                config.auto_play,
                config.video_quality,
            ))),
            Some(s) if s.starts_with("video:") => {
                let mut it = s[6..].split(',');
                let bvid = it.next().unwrap_or("").to_string();
                let aid: i64 = it.next().and_then(|v| v.parse().ok()).unwrap_or(0);
                let page = VideoDetailPage::new(bvid, aid);
                // video:<bvid>,<aid>,<root_rpid>,<focus_rpid> opens the
                // conversation view directly (dev/screenshot helper).
                let page = match (it.next(), it.next()) {
                    (Some(root), Some(focus)) => {
                        if let (Ok(root), Ok(focus)) = (root.parse(), focus.parse()) {
                            let mut page = page;
                            page.comment_list.sub_thread = Some((root, focus));
                            page.loading = false;
                            page
                        } else {
                            page
                        }
                    }
                    _ => page,
                };
                Page::VideoDetail(Box::new(page))
            }
            Some(s) if s.starts_with("up:") => {
                let mid: i64 = s[3..].parse().unwrap_or(0);
                Page::Up(Box::new(UpPage::new(mid)))
            }
            Some(s) if s.starts_with("home:") => {
                let cols: usize = s[5..].parse().unwrap_or(1);
                let mut page = HomePage::new();
                // cycle from the default (1 col) until we reach the target
                while page.column_count() != cols.clamp(1, 4) {
                    page.cycle_columns(1);
                }
                page.focus_sources = false;
                Page::Home(page)
            }
            _ => Page::Home(HomePage::new()),
        }
    }

    fn next_request_id(&mut self, key: &'static str) -> u64 {
        self.request_tracker.next(key)
    }

    fn is_latest_request(&self, key: &'static str, req_id: u64) -> bool {
        self.request_tracker.is_latest(key, req_id)
    }

    fn send_network_command(&self, command: network::NetworkCommand) {
        let _ = self.network_command_tx.send(command);
    }

    fn allocate_playback_session(&mut self) -> u64 {
        let id = self.next_playback_session_id;
        self.next_playback_session_id = self.next_playback_session_id.saturating_add(1);
        id
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{App, RequestTracker};
    use crate::application::AppAction;
    use crate::domain::playback::PlaybackEvent;
    use crate::presentation::tui::{FavoritesPage, HomePage, NavItem, Page, VideoDetailPage};

    #[test]
    fn request_tracking_latest_wins_per_key() {
        let mut tracker = RequestTracker::default();
        let first = tracker.next("search");
        let second = tracker.next("search");

        assert!(!tracker.is_latest("search", first));
        assert!(tracker.is_latest("search", second));
    }

    #[test]
    fn request_tracking_isolated_by_key() {
        let mut tracker = RequestTracker::default();
        let search_id = tracker.next("search");
        let home_id = tracker.next("home");

        assert!(tracker.is_latest("search", search_id));
        assert!(tracker.is_latest("home", home_id));
        assert!(!tracker.is_latest("home", search_id));
    }

    #[tokio::test]
    async fn tab_continues_from_favorites_to_live() {
        let mut app = App::new();
        app.sidebar.select(NavItem::Favorites);
        app.current_page = Page::Favorites(FavoritesPage::new(1));
        app.handle_action(AppAction::NavNext).await;
        assert_eq!(app.sidebar.selected, NavItem::Live);
        assert!(matches!(app.current_page, Page::Live(_)));
    }

    #[tokio::test]
    async fn completed_auto_play_returns_to_the_previous_page() {
        let mut app = App::new();
        app.navigation_stack.push(Page::Home(HomePage::new()));
        app.current_page =
            Page::VideoDetail(Box::new(VideoDetailPage::new("BV1test".to_string(), 1)));
        app.playback.begin_session(7);
        app.auto_return_after_playback = Some((7, "BV1test".to_string()));
        app.playback_event_tx
            .send(PlaybackEvent::Finished {
                session_id: 7,
                bvid: Some("BV1test".to_string()),
            })
            .unwrap();

        app.tick().await;

        assert!(matches!(app.current_page, Page::Home(_)));
        assert!(app.navigation_stack.is_empty());
        assert!(app.auto_return_after_playback.is_none());
    }

    #[tokio::test]
    async fn stale_playback_session_does_not_return() {
        let mut app = App::new();
        app.navigation_stack.push(Page::Home(HomePage::new()));
        app.current_page =
            Page::VideoDetail(Box::new(VideoDetailPage::new("BV1test".to_string(), 1)));
        app.playback.begin_session(8);
        app.auto_return_after_playback = Some((8, "BV1test".to_string()));
        app.playback_event_tx
            .send(PlaybackEvent::Finished {
                session_id: 7,
                bvid: Some("BV1test".to_string()),
            })
            .unwrap();
        app.tick().await;
        assert!(matches!(app.current_page, Page::VideoDetail(_)));
        assert_eq!(
            app.playback.status,
            crate::domain::playback::PlaybackStatus::Playing
        );
    }
}
