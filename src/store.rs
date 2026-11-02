//! Persistent recents list: one emoji per line, most-recent-first,
//! de-duplicated, capped. Stored under $XDG_DATA_HOME/emoxidize/recents.txt.

use std::cell::RefCell;
use std::fs;
use std::path::PathBuf;

pub struct Recents {
    path: PathBuf,
    list: RefCell<Vec<String>>,
}

impl Recents {
    pub const CAP: usize = 40;

    pub fn load() -> Self {
        let dir = std::env::var("XDG_DATA_HOME")
            .ok()
            .filter(|s| !s.is_empty())
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var("HOME")
                    .ok()
                    .filter(|s| !s.is_empty())
                    .map(|h| PathBuf::from(h).join(".local").join("share"))
            })
            .unwrap_or_else(|| PathBuf::from("."));
        let path = dir.join("emoxidize").join("recents.txt");
        let list = fs::read_to_string(&path)
            .map(|s| sanitize(s.lines().map(str::to_string).collect()))
            .unwrap_or_default();
        let recents = Recents {
            path,
            list: RefCell::new(list),
        };
        if let Some(parent) = recents.path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        recents
    }

    pub fn list(&self) -> Vec<String> {
        self.list.borrow().clone()
    }

    pub fn is_empty(&self) -> bool {
        self.list.borrow().is_empty()
    }

    /// Move `ch` to the front (or insert it), persist and return the new list.
    pub fn touch(&self, ch: &str) -> Vec<String> {
        {
            let mut l = self.list.borrow_mut();
            l.retain(|e| e != ch);
            l.insert(0, ch.to_string());
            l.truncate(Self::CAP);
        }
        self.save();
        self.list()
    }

    /// Replace the whole list (external GSettings write), persist and return
    /// the sanitized list.
    pub fn replace(&self, items: Vec<String>) -> Vec<String> {
        let sanitized = sanitize(items);
        *self.list.borrow_mut() = sanitized.clone();
        self.save();
        sanitized
    }

    fn save(&self) {
        if let Some(parent) = self.path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let mut body = self.list.borrow().join("\n");
        if !body.is_empty() {
            body.push('\n');
        }
        let _ = fs::write(&self.path, body);
    }
}

fn sanitize(items: Vec<String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::with_capacity(items.len().min(Recents::CAP));
    for item in items {
        let item = item.trim();
        if item.is_empty() || out.iter().any(|e| e == item) {
            continue;
        }
        out.push(item.to_string());
        if out.len() >= Recents::CAP {
            break;
        }
    }
    out
}
