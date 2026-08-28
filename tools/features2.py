# -*- coding: utf-8 -*-
# comment_list: emote rendering, sort-status, avatar-keyed-by-mid fix

p = 'src/ui/comment_list.rs'
s = open(p, encoding='utf-8').read()

# ---------- 1. Avatar keying fix: protocols aligned by mid+uname instead of index ----------
old = '''/// Async avatar loader: downloads avatar images in the background and keeps
/// one rendered protocol per comment (index-aligned with `comments`).
///
/// The terminal picker is created lazily on first use — `Picker::from_query_stdio`
/// performs terminal capability queries that must never run at page-construction
/// time (it blocks non-TTY test environments).
pub struct AvatarLoader {
    pub protocols: Vec<Option<StatefulProtocol>>,
    pending: HashSet<usize>,
    tx: mpsc::Sender<AvatarResult>,
    rx: mpsc::Receiver<AvatarResult>,
    picker: Option<Arc<Picker>>,
    supports_images: bool,
}'''
new = '''/// Async avatar loader: downloads avatar images in the background and keeps
/// one rendered protocol per author, keyed by `(mid, uname)` instead of list
/// index — comment refreshes reorder/re-paginate the list, and index-keyed
/// caches showed the previous holder's avatar under a new name.
///
/// The terminal picker is created lazily on first use — `Picker::from_query_stdio`
/// performs terminal capability queries that must never run at page-construction
/// time (it blocks non-TTY test environments).
pub struct AvatarLoader {
    /// Rendered image protocols keyed by author identity (mid, uname).
    pub protocols: HashMap<(Option<i64>, String), StatefulProtocol>,
    pending: HashSet<(Option<i64>, String)>,
    tx: mpsc::Sender<AvatarResult>,
    rx: mpsc::Receiver<AvatarResult>,
    picker: Option<Arc<Picker>>,
    supports_images: bool,
}

/// Stable author identity used to key avatar cache entries.
fn author_key(member: Option<&crate::api::comment::CommentMember>) -> Option<(Option<i64>, String)> {
    let member = member?;
    let mid = member
        .mid
        .clone()
        .and_then(|m| m.parse::<i64>().ok());
    let name = member.uname.clone().or_else(|| member.avatar.clone())?;
    Some((mid, name))
}'''
assert old in s, 'loader struct'
s = s.replace(old, new, 1)

old = '''/// Avatar download result message.
pub struct AvatarResult {
    pub index: usize,
    pub protocol: StatefulProtocol,
}'''
new = '''/// Avatar download result message.
pub struct AvatarResult {
    pub key: (Option<i64>, String),
    pub protocol: StatefulProtocol,
}'''
assert old in s, 'result struct'
s = s.replace(old, new, 1)

# loader methods
old = '''    /// Sync list size after comments change (keeps index alignment).
    pub fn sync_len(&mut self, len: usize) {
        while self.protocols.len() > len {
            self.protocols.pop();
        }
        while self.protocols.len() < len {
            self.protocols.push(None);
        }
    }

    pub fn get(&self, index: usize) -> Option<&StatefulProtocol> {
        self.protocols.get(index).and_then(|p| p.as_ref())
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut StatefulProtocol> {
        self.protocols.get_mut(index).and_then(|p| p.as_mut())
    }

    fn is_loaded_or_pending(&self, index: usize) -> bool {
        self.pending.contains(&index)
            || self
                .protocols
                .get(index)
                .map(|p| p.is_some())
                .unwrap_or(true)
    }

    /// Request downloads for the given comment indices.
    pub fn request(&mut self, indices: impl IntoIterator<Item = usize>, urls: &[Option<String>]) {
        let Some(picker) = self.ensure_picker() else {
            return;
        };
        for idx in indices {
            if self.is_loaded_or_pending(idx) {
                continue;
            }
            let Some(url) = urls.get(idx).and_then(|u| u.as_ref()) else {
                continue;
            };
            self.pending.insert(idx);
            let tx = self.tx.clone();
            let picker = Arc::clone(&picker);
            let url = normalize_avatar_url(url);
            tokio::spawn(async move {
                if let Some(img) = download_image(&url).await {
                    let protocol = picker.new_resize_protocol(img);
                    let _ = tx
                        .send(AvatarResult {
                            index: idx,
                            protocol,
                        })
                        .await;
                }
            });
        }
    }

    /// Drain finished downloads; returns true if anything new arrived.
    pub fn poll(&mut self) -> bool {
        let mut updated = false;
        while let Ok(result) = self.rx.try_recv() {
            self.pending.remove(&result.index);
            if let Some(slot) = self.protocols.get_mut(result.index) {
                *slot = Some(result.protocol);
                updated = true;
            }
        }
        updated
    }'''
new = '''    pub fn get(&self, key: &(Option<i64>, String)) -> Option<&StatefulProtocol> {
        self.protocols.get(key)
    }

    pub fn get_mut(&mut self, key: &(Option<i64>, String)) -> Option<&mut StatefulProtocol> {
        self.protocols.get_mut(key)
    }

    fn is_loaded_or_pending(&self, key: &(Option<i64>, String)) -> bool {
        self.pending.contains(key) || self.protocols.contains_key(key)
    }

    /// Request downloads for the given authors (identity + avatar url).
    pub fn request(
        &mut self,
        authors: impl IntoIterator<Item = ((Option<i64>, String), Option<String>)>,
    ) {
        let Some(picker) = self.ensure_picker() else {
            return;
        };
        for (key, url) in authors {
            if self.is_loaded_or_pending(&key) {
                continue;
            }
            let Some(url) = url else {
                continue;
            };
            self.pending.insert(key.clone());
            let tx = self.tx.clone();
            let picker = Arc::clone(&picker);
            let url = normalize_avatar_url(&url);
            tokio::spawn(async move {
                if let Some(img) = download_image(&url).await {
                    let protocol = picker.new_resize_protocol(img);
                    let _ = tx.send(AvatarResult { key, protocol }).await;
                }
            });
        }
    }

    /// Drain finished downloads; returns true if anything new arrived.
    pub fn poll(&mut self) -> bool {
        let mut updated = false;
        while let Ok(result) = self.rx.try_recv() {
            self.pending.remove(&result.key);
            self.protocols.insert(result.key, result.protocol);
            updated = true;
        }
        updated
    }'''
assert old in s, 'loader methods'
s = s.replace(old, new, 1)

# sync_len call sites -> no-op removal
s = s.replace('        self.avatars.sync_len(self.comments.len());\n', '', 2)

# ---------- 2. avatar_urls -> author request builder ----------
old = '''    fn avatar_urls(&self) -> Vec<Option<String>> {
        self.comments
            .iter()
            .map(|c| c.member.as_ref().and_then(|m| m.avatar.clone()))
            .collect()
    }'''
new = '''    /// Collect visible authors' identities + avatar urls for prefetch.
    fn visible_authors(&self, indices: &[usize]) -> Vec<((Option<i64>, String), Option<String>)> {
        indices
            .iter()
            .filter_map(|i| {
                let c = self.comments.get(*i)?;
                let key = author_key(c.member.as_ref())?;
                Some((key, c.member.as_ref().and_then(|m| m.avatar.clone())))
            })
            .collect()
    }'''
assert old in s, 'avatar_urls'
s = s.replace(old, new, 1)

old = '''        // avatar prefetch for visible comments
        let visible_comments = self.visible_comment_indices(viewport);
        let urls = self.avatar_urls();
        self.avatars
            .request(visible_comments.iter().copied(), &urls);
        let avatars_updated = self.avatars.poll();
        let _ = avatars_updated;'''
new = '''        // avatar prefetch for visible comments
        let visible_comments = self.visible_comment_indices(viewport);
        let authors = self.visible_authors(&visible_comments);
        self.avatars.request(authors);
        self.avatars.poll();'''
assert old in s, 'prefetch'
s = s.replace(old, new, 1)

# ---------- 3. render: avatar draw uses author key ----------
old = '''                let comment_idx = entry.comment_index;
                if let Some(protocol) = self.avatars.get_mut(comment_idx) {'''
new = '''                let protocol = self.comments
                    .get(entry.comment_index)
                    .and_then(|c| author_key(c.member.as_ref()))
                    .and_then(|key| self.avatars.get_mut(&key).map(|p| p as *mut _));
                // SAFETY: protocol points into self.avatars, which we borrow
                // mutably only here; no other aliasing borrow is live.
                if let Some(protocol) = protocol.map(|p| unsafe { &mut *p }) {'''
assert old in s, 'avatar draw'
s = s.replace(old, new, 1)

# supports_avatars: protocols.any() check -> len check
old = '''    fn supports_avatars(&mut self) -> bool {
        self.avatars.supports_images()'''
# find the body of supports_avatars to patch .iter().any pattern
s = s.replace('''        if self.protocols.iter().any(|p| p.is_some()) {
            return true;
        }''', '''        if !self.protocols.is_empty() {
            return true;
        }''')

open(p, 'w', encoding='utf-8').write(s)
print('comment_list avatar fix done')
