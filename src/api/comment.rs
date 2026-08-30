//! Comment API types and functions

use serde::Deserialize;

/// Comment list response
#[derive(Debug, Deserialize)]
pub struct CommentData {
    pub page: Option<CommentPage>,
    pub replies: Option<Vec<CommentItem>>,
    pub hots: Option<Vec<CommentItem>>,
}

#[derive(Debug, Deserialize)]
pub struct CommentPage {
    pub num: Option<i32>,
    pub size: Option<i32>,
    pub count: Option<i32>,
    pub acount: Option<i32>,
}

/// Individual comment item
/// A piece of a comment message: plain text or a known emote token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Segment<'a> {
    Text(&'a str),
    Emote(&'a str),
}

#[derive(Debug, Clone, Deserialize)]
pub struct CommentItem {
    pub rpid: i64,
    pub oid: i64,
    pub mid: i64,
    pub parent: i64,
    pub count: Option<i32>,
    pub rcount: Option<i32>,
    pub floor: Option<i32>,
    pub ctime: Option<i64>,
    pub like: Option<i32>,
    pub member: Option<CommentMember>,
    pub content: Option<CommentContent>,
    #[serde(default)]
    pub reply_control: Option<ReplyControl>,
    pub replies: Option<Vec<CommentItem>>,
}

/// Moderation / display metadata attached to a comment by the server.
#[derive(Debug, Clone, Deserialize)]
pub struct ReplyControl {
    /// IP location shown under the comment, e.g. "广东" (None for older posts).
    pub location: Option<String>,
    #[serde(default)]
    pub up_like: Option<bool>,
}

impl CommentItem {
    /// IP location text, e.g. "广东" or None.
    pub fn ip_location(&self) -> Option<&str> {
        self.reply_control
            .as_ref()
            .and_then(|c| c.location.as_deref())
            .filter(|s| !s.is_empty())
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct CommentMember {
    pub mid: Option<String>,
    pub uname: Option<String>,
    pub avatar: Option<String>,
    pub level_info: Option<LevelInfo>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LevelInfo {
    pub current_level: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CommentContent {
    pub message: Option<String>,
    /// Bilibili emotes used in this comment, keyed by their text token
    /// (e.g. "(闹钟)" -> emote metadata with an image url).
    #[serde(default)]
    pub emote: Option<std::collections::HashMap<String, CommentEmote>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CommentEmote {
    /// Display token, e.g. "(闹钟)".
    pub text: Option<String>,
    /// Meta per size variant; "1" is the standard one.
    pub url: Option<String>,
}

impl CommentItem {
    pub fn author_name(&self) -> &str {
        self.member
            .as_ref()
            .and_then(|m| m.uname.as_deref())
            .unwrap_or("匿名")
    }

    pub fn message(&self) -> &str {
        self.content
            .as_ref()
            .and_then(|c| c.message.as_deref())
            .unwrap_or("")
    }

    /// Number of visual lines this message occupies at `width` columns,
    /// accounting for emote spans (a token may wrap onto its own line).
    pub fn message_line_count(&self, width: usize) -> usize {
        let segments = self.message_segments();
        if segments.iter().any(|s| matches!(s, Segment::Emote(_))) {
            // approximating the same layout as comment_list::wrap_segments:
            // sum wrapped text lines plus one per emote that starts a new row
            let mut count = 0usize;
            let mut col = 0usize;
            for seg in &segments {
                match seg {
                    Segment::Text(t) => {
                        for line in crate::ui::comment_list::wrap_lines(t, width) {
                            let w = line.chars().count();
                            if col > 0 && col + w > width {
                                count += 1;
                                col = 0;
                            }
                            col += w;
                        }
                        count += 1;
                    }
                    Segment::Emote(token) => {
                        let w = format!("{}{} ", "\u{f118}", token).chars().count();
                        if col + w > width && col > 0 {
                            count += 1;
                            col = 0;
                        }
                        col += w;
                    }
                }
            }
            count.max(1)
        } else {
            crate::ui::comment_list::wrap_lines(self.message(), width).len()
        }
    }

    /// Message split into plain-text and emote segments (in order).
    /// Emote tokens like "(闹钟)" that exist in `content.emote` become
    /// `Segment::Emote(token)`; unknown brackets stay plain text.
    pub fn message_segments(&self) -> Vec<Segment<'_>> {
        let message = self.message();
        let emotes = self.content.as_ref().and_then(|c| c.emote.as_ref());
        let Some(emotes) = emotes else {
            return vec![Segment::Text(message)];
        };
        let mut segments = Vec::new();
        let mut rest = message;
        while let Some(open) = rest.find('[') {
            // find the matching close bracket (basic emotes are "[名]")
            if let Some(close_rel) = rest[open..].find(']') {
                let close = open + close_rel;
                let token = &rest[open..=close];
                if emotes.contains_key(token) {
                    if open > 0 {
                        segments.push(Segment::Text(&rest[..open]));
                    }
                    segments.push(Segment::Emote(token));
                    rest = &rest[close + 1..];
                    continue;
                }
                // not a known emote: keep scanning after this '['
                let advance = open + 1;
                segments.push(Segment::Text(&rest[..advance]));
                rest = &rest[advance..];
            } else {
                break;
            }
        }
        if !rest.is_empty() {
            segments.push(Segment::Text(rest));
        }
        segments
    }

    pub fn format_like(&self) -> String {
        match self.like {
            Some(n) if n >= 10000 => format!("{:.1}万", n as f64 / 10000.0),
            Some(n) => format!("{}", n),
            None => "-".to_string(),
        }
    }

    pub fn format_time(&self) -> String {
        if let Some(ctime) = self.ctime {
            // Convert timestamp to relative time
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            let diff = now - ctime;

            if diff < 60 {
                "刚刚".to_string()
            } else if diff < 3600 {
                format!("{}分钟前", diff / 60)
            } else if diff < 86400 {
                format!("{}小时前", diff / 3600)
            } else if diff < 2592000 {
                format!("{}天前", diff / 86400)
            } else {
                format!("{}月前", diff / 2592000)
            }
        } else {
            "".to_string()
        }
    }

    /// Direct reply count (`count`). Note `rcount` includes the whole
    /// sub-tree, which makes "load more" appear even when all direct
    /// replies are already fetched.
    pub fn reply_count(&self) -> i32 {
        // `count` = direct replies; fall back to rcount for old payloads
        self.count.or(self.rcount).unwrap_or(0)
    }

    /// Absolute local time like the web player: "2024-03-15 23:51".
    pub fn format_time_absolute(&self) -> String {
        use chrono::TimeZone;
        match self.ctime {
            Some(ts) => match chrono::Local.timestamp_opt(ts, 0) {
                chrono::LocalResult::Single(t) => t.format("%Y-%m-%d %H:%M").to_string(),
                _ => String::new(),
            },
            None => String::new(),
        }
    }
}

/// Comment type enum for different content types
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub enum CommentType {
    /// 视频 (Video)
    Video = 1,
    /// 话题 (Topic)
    Topic = 6,
    /// 活动 (Activity)
    Activity = 10,
    /// 相簿/图片动态 (Photo Album)
    Album = 11,
    /// 专栏 (Article)
    Article = 12,
    /// 音频 (Audio)
    Audio = 14,
    /// 动态（纯文字 & 分享）
    Dynamic = 17,
    /// 合辑 (Playlist)
    Playlist = 19,
    /// 课程 (Course)
    Course = 33,
}

#[allow(dead_code)]
impl CommentType {
    pub fn as_i32(&self) -> i32 {
        *self as i32
    }
}

/// Response for adding a comment
#[derive(Debug, Deserialize)]
pub struct AddCommentResponse {
    pub success_action: Option<i32>,
    pub success_toast: Option<String>,
    pub need_captcha: Option<bool>,
    pub rpid: Option<i64>,
    pub rpid_str: Option<String>,
    pub root: Option<i64>,
    pub parent: Option<i64>,
    pub reply: Option<CommentItem>,
}
