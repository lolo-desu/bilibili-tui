# -*- coding: utf-8 -*-
# comment emote support + sort status display

# ---------- 1. API: emote struct + message_segments ----------
p = 'src/api/comment.rs'
s = open(p, encoding='utf-8').read()

old = '''#[derive(Debug, Clone, Deserialize)]
pub struct CommentContent {
    pub message: Option<String>,
}'''
new = '''#[derive(Debug, Clone, Deserialize)]
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
}'''
assert old in s, 'CommentContent'
s = s.replace(old, new, 1)

# message_segments: split message into text / emote pieces
old = '''    pub fn message(&self) -> &str {
        self.content
            .as_ref()
            .and_then(|c| c.message.as_deref())
            .unwrap_or("")
    }'''
new = '''    pub fn message(&self) -> &str {
        self.content
            .as_ref()
            .and_then(|c| c.message.as_deref())
            .unwrap_or("")
    }

    /// Message split into plain-text and emote segments (in order).
    /// Emote tokens like "(闹钟)" that exist in `content.emote` become
    /// `Segment::Emote(token)`; unknown brackets stay plain text.
    pub fn message_segments(&self) -> Vec<Segment<'_>> {
        let message = self.message();
        let emotes = self
            .content
            .as_ref()
            .and_then(|c| c.emote.as_ref());
        let Some(emotes) = emotes else {
            return vec![Segment::Text(message)];
        };
        let mut segments = Vec::new();
        let mut rest = message;
        while let Some(open) = rest.find('(') {
            // find matching close bracket
            if let Some(close_rel) = rest[open..].find(')') {
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
                // not a known emote: keep scanning after this '('
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
    }'''
assert old in s, 'message fn'
s = s.replace(old, new, 1)

# Segment enum at module level
s = s.replace('''#[derive(Debug, Clone, Deserialize)]
pub struct CommentItem {''', '''/// A piece of a comment message: plain text or a known emote token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Segment<'a> {
    Text(&'a str),
    Emote(&'a str),
}

#[derive(Debug, Clone, Deserialize)]
pub struct CommentItem {''')
open(p, 'w', encoding='utf-8').write(s)
print('api emote done')
