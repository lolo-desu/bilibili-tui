# -*- coding: utf-8 -*-
# Conversation view (APP-style 查看对话): build_entries renders the focused
# reply + its children; entry kinds; intents; Esc/v handling.

def rep(s, old, new):
    assert old in s, 'MISSING: ' + old[:80]
    return s.replace(old, new, 1)

p = 'src/ui/comment_list.rs'
s = open(p, encoding='utf-8').read()

# ---------------- EntryKind::SubReply ----------------
if 'SubReply' not in s:
    s = rep(s, '''    /// Reply row inside an expanded comment (floor view).
    Reply,''',
'''    /// Reply row inside an expanded comment (floor view).
    Reply,
    /// Reply-to-reply row inside the conversation view.
    SubReply,''')

# ---------------- build_entries: conversation view prefix ----------------
s = rep(s, '''    fn build_entries(&mut self, width: u16) {
        let mut entries = Vec::new();
        let mut line = 0usize;
        let content_width = width.saturating_sub(AVATAR_COLS + 1).max(8) as usize;

        for (ci, comment) in self.comments.iter().enumerate() {''',
'''    fn build_entries(&mut self, width: u16) {
        let mut entries = Vec::new();
        let mut line = 0usize;
        let content_width = width.saturating_sub(AVATAR_COLS + 1).max(8) as usize;

        // Conversation view: focused reply on top, its children below,
        // a back row at the bottom (APP-style 对话页).
        if let Some((root_rpid, focus_rpid)) = self.sub_thread {
            let ci = self
                .comments
                .iter()
                .position(|c| c.rpid == root_rpid)
                .unwrap_or(0);
            if let Some(focus) = self
                .replies
                .get(&root_rpid)
                .and_then(|rs| rs.iter().find(|r| r.rpid == focus_rpid))
            {
                let msg = focus.message_line_count(content_width).max(1);
                let h = 1 + msg + 1 + CARD_TRAIL_BLANK as usize + 1;
                entries.push(Entry {
                    kind: EntryKind::Reply,
                    comment_index: ci,
                    reply_index: usize::MAX, // focus marker
                    start_line: line,
                    height: h as u16,
                });
                line += h;
                if let Some(children) = self.sub_replies.get(&focus_rpid) {
                    for (si, child) in children.iter().enumerate() {
                        let msg = child.message_line_count(content_width).max(1);
                        let h = 1 + msg + 1 + CARD_TRAIL_BLANK as usize + 1;
                        entries.push(Entry {
                            kind: EntryKind::SubReply,
                            comment_index: ci,
                            reply_index: si,
                            start_line: line,
                            height: h as u16,
                        });
                        line += h;
                    }
                }
                entries.push(Entry {
                    kind: EntryKind::Toggle,
                    comment_index: ci,
                    reply_index: 3, // back row
                    start_line: line,
                    height: 2,
                });
                line += 2;
            }
            self.entries = entries;
            self.total_lines = line;
            self.last_width = width;
            return;
        }

        for (ci, comment) in self.comments.iter().enumerate() {''')

# ---------------- render dispatch ----------------
s = rep(s, '''                EntryKind::Reply => {
                    let comment = &self.comments[entry.comment_index];
                    if let Some(replies) = self.replies.get(&comment.rpid)
                        && let Some(reply) = replies.get(entry.reply_index)
                    {
                        self.draw_reply_row(frame, area, row, reply, theme, is_selected, sel_style);
                    }
                }''',
'''                EntryKind::Reply => {
                    let comment = &self.comments[entry.comment_index];
                    // usize::MAX marks the focused reply atop a conversation.
                    if entry.reply_index == usize::MAX {
                        if let Some(focus_rpid) = self.sub_thread.map(|(_, r)| r)
                            && let Some(replies) = self.replies.get(&comment.rpid)
                            && let Some(reply) =
                                replies.iter().find(|r| r.rpid == focus_rpid)
                        {
                            self.draw_reply_row(
                                frame,
                                area,
                                row,
                                reply,
                                theme,
                                is_selected,
                                sel_style,
                            );
                        }
                    } else if let Some(replies) = self.replies.get(&comment.rpid)
                        && let Some(reply) = replies.get(entry.reply_index)
                    {
                        self.draw_reply_row(frame, area, row, reply, theme, is_selected, sel_style);
                    }
                }
                EntryKind::SubReply => {
                    if let Some((_, focus_rpid)) = self.sub_thread
                        && let Some(children) = self.sub_replies.get(&focus_rpid)
                        && let Some(child) = children.get(entry.reply_index)
                    {
                        self.draw_sub_reply_row(
                            frame,
                            area,
                            row,
                            child,
                            theme,
                            is_selected,
                            sel_style,
                        );
                    }
                }''')

# toggle row: back row label when in conversation
s = rep(s, '''        let (label, icon, color) = if self.expanded.contains(&comment.rpid) {
            ("收起回复".to_string(), icons::FOLD_OPEN, theme.fg_muted)''',
'''        let (label, icon, color) = if entry.reply_index == 3 {
            ("‹ 返回评论列表".to_string(), icons::LEFT_ARROW, theme.bilibili_blue)
        } else if self.expanded.contains(&comment.rpid) {
            ("收起回复".to_string(), icons::FOLD_OPEN, theme.fg_muted)''')

open(p, 'w', encoding='utf-8').write(s)
print('entries ok')
