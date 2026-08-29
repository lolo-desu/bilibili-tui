# -*- coding: utf-8 -*-
# Batch: comment replies floor view, tab-style sources, key nav, surfaces

import re

def rep(s, old, new, count=1):
    assert old in s, 'MISSING: ' + old[:70]
    return s.replace(old, new, count)

# =====================================================================
# 1. comment_list: reply floor view with avatars + paged toggles
# =====================================================================
p = 'src/ui/comment_list.rs'
s = open(p, encoding='utf-8').read()

# --- 1a. replies shown in pages (Bilibili-app style): only first 3 when ---
# --- collapsed-preview; expanded shows up to 10 per page with 上一页/下一页 ---
s = rep(s, '''pub struct CommentList {
    /// Top-level comments (hot + recent, in API order).
    pub comments: Vec<CommentItem>,
    /// Fetched replies keyed by root comment rpid.
    pub replies: HashMap<i64, Vec<CommentItem>>,''',
'''/// Replies shown per floor page when a comment is expanded.
pub const REPLIES_PER_PAGE: usize = 10;

pub struct CommentList {
    /// Top-level comments (hot + recent, in API order).
    pub comments: Vec<CommentItem>,
    /// Fetched replies keyed by root comment rpid.
    pub replies: HashMap<i64, Vec<CommentItem>>,
    /// Current floor page (0-based) per expanded root rpid.
    pub reply_pages: HashMap<i64, usize>,''')

s = rep(s, '''        Self {
            comments: Vec::new(),
            replies: HashMap::new(),
            expanded: HashSet::new(),''',
'''        Self {
            comments: Vec::new(),
            replies: HashMap::new(),
            reply_pages: HashMap::new(),
            expanded: HashSet::new(),''')

# clear pages when comments reset
s = rep(s, '''    pub fn set_comments(&mut self, comments: Vec<CommentItem>, total_count: i64) {
        self.comments = comments;
        self.replies.clear();
        self.expanded.clear();''',
'''    pub fn set_comments(&mut self, comments: Vec<CommentItem>, total_count: i64) {
        self.comments = comments;
        self.replies.clear();
        self.reply_pages.clear();
        self.expanded.clear();''')

# --- 1b. EntryKind: ReplyFloor replaces per-reply entries; page entries ---
s = rep(s, '''pub enum EntryKind {
    /// Top-level comment card.
    Comment,
    /// Reply row inside an expanded comment.
    Reply,
    /// "展开/收起回复" or "加载更多回复" toggle row.
    Toggle,
    /// Horizontal rule between top-level cards (not selectable).
    Separator,
}''',
'''pub enum EntryKind {
    /// Top-level comment card.
    Comment,
    /// Reply row inside an expanded comment (floor view).
    Reply,
    /// "展开/收起回复" or page/load-more toggle row.
    Toggle,
    /// Horizontal rule between top-level cards (not selectable).
    Separator,
}''')

# reply_page helper: which slice of replies is visible for root rpid
s = rep(s, '''    /// Collapse (or mark loading) replies for a root comment.''',
'''    /// Visible slice (floor page) of replies for an expanded root comment.
    pub fn visible_replies(&self, root_rpid: i64) -> &[CommentItem] {
        let all = self.replies.get(&root_rpid).map(Vec::as_slice).unwrap_or(&[]);
        if !self.expanded.contains(&root_rpid) {
            return &[];
        }
        let page = *self.reply_pages.get(&root_rpid).unwrap_or(&0);
        let start = (page * REPLIES_PER_PAGE).min(all.len());
        let end = (start + REPLIES_PER_PAGE).min(all.len());
        &all[start..end]
    }

    /// Current floor page number (1-based) for an expanded root, if any.
    pub fn reply_page_info(&self, root_rpid: i64) -> Option<(usize, usize)> {
        if !self.expanded.contains(&root_rpid) {
            return None;
        }
        let total = self.replies.get(&root_rpid)?.len();
        if total == 0 {
            return None;
        }
        let page = *self.reply_pages.get(&root_rpid).unwrap_or(&0);
        let pages = total.div_ceil(REPLIES_PER_PAGE);
        Some((page + 1, pages))
    }

    /// Turn to the next/previous floor page; returns true when moved.
    pub fn page_replies(&mut self, root_rpid: i64, dir: i32) -> bool {
        let Some((page, pages)) = self.reply_page_info(root_rpid) else {
            return false;
        };
        let next = if dir > 0 {
            (page).min(pages - 1)
        } else {
            page.saturating_sub(2)
        };
        if next + 1 == page {
            return false;
        }
        self.reply_pages.insert(root_rpid, next);
        self.entries.clear();
        true
    }

    /// Collapse (or mark loading) replies for a root comment.''')

# build_entries: paged replies
s = rep(s, '''            let is_expanded = self.expanded.contains(&comment.rpid);
            if is_expanded {
                if let Some(replies) = self.replies.get(&comment.rpid) {
                    for (ri, reply) in replies.iter().enumerate() {
                        let reply_msg_lines = reply.message_line_count(content_width).max(1);
                        let height = 1 + reply_msg_lines + 1 + 1; // header+msg+actions+blank
                        entries.push(Entry {
                            kind: EntryKind::Reply,
                            comment_index: ci,
                            reply_index: ri,
                            start_line: line,
                            height: height as u16,
                        });
                        line += height;
                    }
                    // "load more replies" toggle when more exist on server
                    if comment.reply_count() as usize > replies.len() {
                        entries.push(Entry {
                            kind: EntryKind::Toggle,
                            comment_index: ci,
                            reply_index: 1,
                            start_line: line,
                            height: 2,
                        });
                        line += 2;
                    }
                } else if self.loading_replies_for == Some(comment.rpid) {
                    entries.push(Entry {
                        kind: EntryKind::Toggle,
                        comment_index: ci,
                        reply_index: 0,
                        start_line: line,
                        height: 1,
                    });
                    line += 1;
                }
                // collapse row
                entries.push(Entry {
                    kind: EntryKind::Toggle,
                    comment_index: ci,
                    reply_index: 0,
                    start_line: line,
                    height: 2,
                });
                line += 2;
            } else if comment.reply_count() > 0 {
                // single-row preview toggle
                entries.push(Entry {
                    kind: EntryKind::Toggle,
                    comment_index: ci,
                    reply_index: 0,
                    start_line: line,
                    height: 1,
                });
                line += 1;
            }''',
'''            let is_expanded = self.expanded.contains(&comment.rpid);
            if is_expanded {
                if self.replies.contains_key(&comment.rpid) {
                    let floor_replies = self.visible_replies(comment.rpid);
                    let page_base = self.reply_pages.get(&comment.rpid).copied().unwrap_or(0)
                        * REPLIES_PER_PAGE;
                    for (ri, reply) in floor_replies.iter().enumerate() {
                        let reply_msg_lines = reply.message_line_count(content_width).max(1);
                        let height = 1 + reply_msg_lines + 1 + 1; // header+msg+actions+blank
                        entries.push(Entry {
                            kind: EntryKind::Reply,
                            comment_index: ci,
                            reply_index: page_base + ri,
                            start_line: line,
                            height: height as u16,
                        });
                        line += height;
                    }
                    // floor pager: 上一页 / 第x/y页 / 下一页 (+加载更多 when server has more)
                    let total_fetched = self.replies.get(&comment.rpid).map_or(0, |r| r.len());
                    let has_more_server = comment.reply_count() as usize > total_fetched;
                    let pager_height = if has_more_server { 2 } else { 2 };
                    entries.push(Entry {
                        kind: EntryKind::Toggle,
                        comment_index: ci,
                        reply_index: 2, // pager row
                        start_line: line,
                        height: pager_height as u16,
                    });
                    line += pager_height;
                } else if self.loading_replies_for == Some(comment.rpid) {
                    entries.push(Entry {
                        kind: EntryKind::Toggle,
                        comment_index: ci,
                        reply_index: 0,
                        start_line: line,
                        height: 1,
                    });
                    line += 1;
                }
                // collapse row
                entries.push(Entry {
                    kind: EntryKind::Toggle,
                    comment_index: ci,
                    reply_index: 0,
                    start_line: line,
                    height: 2,
                });
                line += 2;
            } else if comment.reply_count() > 0 {
                // single-row preview toggle
                entries.push(Entry {
                    kind: EntryKind::Toggle,
                    comment_index: ci,
                    reply_index: 0,
                    start_line: line,
                    height: 1,
                });
                line += 1;
            }''')

# activate_selected: pager intent
s = rep(s, '''            EntryKind::Toggle => {
                let comment = self.comments.get(entry.comment_index)?;
                if self.expanded.contains(&comment.rpid) && entry.reply_index == 0 {
                    Some(CommentIntent::ToggleReplies {
                        comment_index: entry.comment_index,
                    })
                } else if entry.reply_index == 1 {
                    Some(CommentIntent::LoadMoreReplies {
                        comment_index: entry.comment_index,
                    })
                } else {
                    Some(CommentIntent::ToggleReplies {
                        comment_index: entry.comment_index,
                    })
                }
            }''',
'''            EntryKind::Toggle => {
                let comment = self.comments.get(entry.comment_index)?;
                if self.expanded.contains(&comment.rpid) {
                    if entry.reply_index == 2 {
                        // pager row cycles to the next floor page (wraps)
                        Some(CommentIntent::PageReplies {
                            comment_index: entry.comment_index,
                        })
                    } else {
                        // reply_index 0 = collapse row
                        Some(CommentIntent::ToggleReplies {
                            comment_index: entry.comment_index,
                        })
                    }
                } else {
                    Some(CommentIntent::ToggleReplies {
                        comment_index: entry.comment_index,
                    })
                }
            }''')

# CommentIntent: add PageReplies
s = rep(s, '''    /// Fetch the next page of replies for the expanded comment.
    LoadMoreReplies { comment_index: usize },''',
'''    /// Fetch the next page of replies for the expanded comment.
    LoadMoreReplies { comment_index: usize },
    /// Turn the floor page of the expanded comment's replies.
    PageReplies { comment_index: usize },''')

open(p, 'w', encoding='utf-8').write(s)
print('comment_list part1 ok')
