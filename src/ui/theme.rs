use opaline::OpalineColor;
use ratatui::style::Color;

pub const DEFAULT_THEME_ID: &str = "silkcircuit-neon";

const BUILTIN_THEME_IDS: [&str; 39] = [
    "silkcircuit-neon",
    "silkcircuit-soft",
    "silkcircuit-glow",
    "silkcircuit-vibrant",
    "silkcircuit-dawn",
    "catppuccin-mocha",
    "catppuccin-macchiato",
    "catppuccin-frappe",
    "catppuccin-latte",
    "dracula",
    "nord",
    "tokyo-night",
    "tokyo-night-storm",
    "tokyo-night-moon",
    "rose-pine",
    "rose-pine-moon",
    "rose-pine-dawn",
    "kanagawa-wave",
    "kanagawa-dragon",
    "kanagawa-lotus",
    "everforest-dark",
    "everforest-light",
    "gruvbox-dark",
    "gruvbox-light",
    "solarized-dark",
    "solarized-light",
    "one-dark",
    "one-light",
    "monokai-pro",
    "github-dark-dimmed",
    "github-light",
    "night-owl",
    "light-owl",
    "ayu-dark",
    "ayu-mirage",
    "ayu-light",
    "flexoki-dark",
    "flexoki-light",
    "palenight",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeChoice {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone)]
pub struct Theme {
    pub bg_primary: Color,
    pub bg_secondary: Color,
    pub bg_modal: Color,
    pub bg_card: Color,
    pub bg_highlight: Color,
    pub bg_overlay: Color,

    pub fg_primary: Color,
    pub fg_secondary: Color,
    pub fg_accent: Color,
    pub fg_muted: Color,

    pub border_focused: Color,
    pub border_unfocused: Color,
    pub border_subtle: Color,

    pub selection_bg: Color,
    pub selection_fg: Color,

    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub info: Color,

    pub bilibili_pink: Color,
    pub bilibili_blue: Color,
    pub bilibili_cyan: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self::load_or_default(DEFAULT_THEME_ID).0
    }
}

impl Theme {
    pub fn available_theme_choices() -> Vec<ThemeChoice> {
        BUILTIN_THEME_IDS
            .iter()
            .filter_map(|id| {
                opaline::load_by_name(id).map(|theme| ThemeChoice {
                    id: (*id).to_string(),
                    label: theme.meta.name.clone(),
                })
            })
            .collect()
    }

    pub fn next_theme_id(current_theme_id: &str) -> String {
        if let Some(idx) = BUILTIN_THEME_IDS
            .iter()
            .position(|id| *id == current_theme_id)
        {
            let next = (idx + 1) % BUILTIN_THEME_IDS.len();
            BUILTIN_THEME_IDS[next].to_string()
        } else {
            BUILTIN_THEME_IDS[0].to_string()
        }
    }

    pub fn load(theme_id: &str) -> Option<Self> {
        opaline::load_by_name(theme_id).map(|theme| Self::from_opaline(&theme))
    }

    pub fn load_or_default(theme_id: &str) -> (Self, bool) {
        match Self::load(theme_id) {
            Some(theme) => (theme, false),
            None => {
                let fallback = opaline::load_by_name(DEFAULT_THEME_ID).unwrap_or_default();
                (Self::from_opaline(&fallback), true)
            }
        }
    }

    fn from_opaline(theme: &opaline::Theme) -> Self {
        let bg_base = theme.color("bg.base");

        // Distinct surfaces for the three-column layout: sidebar uses the
        // darker code/mantle tone, panels/cards use the panel/base tone so
        // columns read as separate blocks instead of one flat color.
        let bg_secondary = {
            let code = theme.color("bg.code");
            let base = theme.color("bg.panel");
            if code != base {
                code
            } else {
                bg_base.darken(0.35)
            }
        };
        let bg_card = theme.color("bg.panel");

        Self {
            bg_primary: Self::to_ratatui(bg_base),
            bg_secondary: Self::to_ratatui(bg_secondary),
            bg_modal: Self::color(theme, "bg.code"),
            bg_card: Self::to_ratatui(bg_card),
            bg_highlight: Self::color(theme, "bg.highlight"),
            bg_overlay: Self::to_ratatui(bg_base.darken(0.2)),

            fg_primary: Self::color(theme, "text.primary"),
            fg_secondary: Self::color(theme, "text.secondary"),
            fg_accent: Self::color(theme, "accent.primary"),
            fg_muted: Self::color(theme, "text.muted"),

            border_focused: Self::color(theme, "border.focused"),
            border_unfocused: Self::color(theme, "border.unfocused"),
            border_subtle: Self::color(theme, "text.dim"),

            selection_bg: Self::color(theme, "bg.selection"),
            selection_fg: Self::color(theme, "text.primary"),

            success: Self::color(theme, "success"),
            warning: Self::color(theme, "warning"),
            error: Self::color(theme, "error"),
            info: Self::color(theme, "info"),

            bilibili_pink: Self::color(theme, "accent.primary"),
            bilibili_blue: Self::color(theme, "accent.secondary"),
            bilibili_cyan: Self::color(theme, "accent.tertiary"),
        }
    }

    fn color(theme: &opaline::Theme, token: &str) -> Color {
        Self::to_ratatui(theme.color(token))
    }

    fn to_ratatui(color: OpalineColor) -> Color {
        Color::Rgb(color.r, color.g, color.b)
    }
}
