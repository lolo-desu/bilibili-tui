//! Nerd Font icons used across the app.
//!
//! All UI glyphs come from Nerd Font (v3) codepoints and render as
//! single-width symbols in a properly patched font (e.g. JetBrainsMono
//! Nerd Font, CaskaydiaCove NF, MesloLGS NF). If your font is not a Nerd
//! Font the glyphs will show as tofu boxes - switch terminal font to any
//! "* Nerd Font" to fix.
//!
//! Reference: https://www.nerdfonts.com/cheat-sheet
#![allow(dead_code)]

// ---- Media ----
/// Monitor / TV (nf-md-television, \uf10fc region)
pub const TV: &str = "\u{f03eb}"; // nf-md-television_classic
/// Play (nf-md-play)
pub const PLAY: &str = "\u{f040b}";
/// Pause (nf-md-pause)
pub const PAUSE: &str = "\u{f03e0}";
/// Broadcast / live (nf-md-broadcast)
pub const LIVE: &str = "\u{f0e8e}";

// ---- Social ----
/// Comment bubble (nf-md-comment-text_multiple_outline)
pub const COMMENT: &str = "\u{f0192}"; // nf-fa-comments, broader support
/// Thumbs-up outline (nf-md-thumb_up_outline)
pub const LIKE: &str = "\u{f514}"; // nf-fa-thumbs_up
/// Thumbs-up filled (nf-fa-thumbs_up filled variant \u{f164})
pub const LIKE_FILLED: &str = "\u{f164}";
/// Heart outline (nf-fa-heart_o)
pub const HEART: &str = "\u{f08a}"; // nf-fa-heart_o
/// Heart filled (nf-fa-heart)
pub const HEART_FILLED: &str = "\u{f004}";
/// Eye (nf-fa-eye)
pub const VIEW: &str = "\u{f06e}";
/// Star (nf-fa-star)
pub const STAR: &str = "\u{f005}";
/// Coin (nf-md-cash)
pub const COIN: &str = "\u{f0d6}";
/// User (nf-fa-user)
pub const USER: &str = "\u{f007}";
/// Users (nf-fa-users)
pub const USERS: &str = "\u{f0c0}";
/// Send (nf-fa-paper_plane)
pub const SEND: &str = "\u{f1d8}";
/// Danmaku list (nf-fa-comment_dots) - chat bubbles
pub const DANMAKU: &str = "\u{f27b}";

// ---- Objects ----
/// Search (nf-fa-search)
pub const SEARCH: &str = "\u{f002}";
/// Home (nf-fa-home)
pub const HOME: &str = "\u{f015}";
/// History / clock (nf-fa-clock_o)
pub const HISTORY: &str = "\u{f017}";
/// Fire / trending (nf-fa-fire)
pub const FIRE: &str = "\u{f06d}";
/// Image / photo (nf-fa-picture_o)
pub const IMAGE: &str = "\u{f03e}";
/// Article / file-text (nf-fa-file_text_o)
pub const ARTICLE: &str = "\u{f0f6}";
/// Settings gear (nf-fa-cog)
pub const GEAR: &str = "\u{f013}";
/// Paint / theme (nf-fa-paint_brush)
pub const PAINT: &str = "\u{f1fc}";
/// Camera (nf-fa-camera)
pub const CAMERA: &str = "\u{f030}";
/// Mobile (nf-fa-mobile)
pub const MOBILE: &str = "\u{f10b}";
/// Inbox empty (nf-fa-inbox)
pub const INBOX: &str = "\u{f01c}";
/// List / episodes (nf-fa-list_ul)
pub const LIST: &str = "\u{f0ca}";
/// Feed / scroll (nf-fa-align_left)
pub const FEED: &str = "\u{f036}";
/// QR code (nf-fa-qrcode)
pub const QRCODE: &str = "\u{f029}";
/// Warning (nf-fa-warning)
pub const WARN: &str = "\u{f071}";
/// Error / times-circle (nf-fa-times_circle)
pub const ERROR: &str = "\u{f057}";
/// Check / success (nf-fa-check_circle)
pub const CHECK: &str = "\u{f058}";
/// Question (nf-fa-question_circle)
pub const QUESTION: &str = "\u{f059}";
/// Info (nf-fa-info_circle)
pub const INFO: &str = "\u{f05a}";
/// Edit / pencil (nf-fa-pencil_square_o)
pub const EDIT: &str = "\u{f044}";
/// Trophy / ranking (nf-fa-trophy)
pub const TROPHY: &str = "\u{f091}";
/// Download (nf-fa-download)
pub const DOWNLOAD: &str = "\u{f019}";
/// Satellite dish (nf-fa-signal) for live/connected
pub const SIGNAL: &str = "\u{f012}";

// ---- Fold / expand & tree ----
/// Expanded node (nf-fa-caret_down)
pub const FOLD_OPEN: &str = "\u{f0d7}";
/// Collapsed node (nf-fa-caret_right)
pub const FOLD_CLOSED: &str = "\u{f0da}";
/// Reply thread arrow (nf-fa-share / turn-down)
pub const REPLY_ARROW: &str = "\u{f064}"; // nf-fa-share
/// Selected marker (nf-fa-angle_right)
pub const SELECTOR: &str = "\u{f105}";
/// Spinner (nf-fa-spinner) - use with animation-less static frame
pub const SPINNER: &str = "\u{f110}";
