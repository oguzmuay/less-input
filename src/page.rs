//! Tarayıcı/sayfa kaydırma tuş haritası (`j k d u gg G`, sayı öneki, Ctrl-d/u, oklar).

use crate::keys::{Key, KeyEvent};

/// Sayfada yapılacak kaydırma; sayı öneki işaretli değere katılır (`5j` → `ScrollLines(5)`, `3k` → `ScrollLines(-3)`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PageAction {
    /// Satır (j/k, ↓/↑); negatif yukarı.
    ScrollLines(i32),
    /// Yarım sayfa (d/u, Ctrl-d/Ctrl-u); negatif yukarı.
    HalfPages(i32),
    /// `gg`.
    Top,
    /// `G`.
    Bottom,
}

/// Sayfa kaydırma motoru: bekleyen `g` ve sayı önekini tutar. Cmd/Alt'lı tuşlar uygulamaya bırakılır (`None`).
#[derive(Clone, Debug, Default)]
pub struct PageEngine {
    count: Option<i32>,
    pending_g: bool,
}

impl PageEngine {
    /// Motor bir sonraki tuşu bekleyen bir dizi (`g`, sayı) ortasındaysa `true`; uygulama o tuşu yutmalı.
    pub fn is_pending(&self) -> bool {
        self.pending_g || self.count.is_some()
    }

    /// Tuşu işler; kaydırma tamamlandıysa eylemi döndürür. Tanınmayan tuş bekleyen durumu sıfırlar.
    pub fn feed(&mut self, key: KeyEvent) -> Option<PageAction> {
        let after_g = std::mem::take(&mut self.pending_g);
        let count = self.count;
        let m = key.mods;
        if m.cmd || m.alt || key.key == Key::Esc {
            self.count = None;
            return None;
        }
        let n = count.unwrap_or(1);
        let action = if m.ctrl {
            match key.ctrl_char() {
                Some('d') => Some(PageAction::HalfPages(n)),
                Some('u') => Some(PageAction::HalfPages(-n)),
                _ => None,
            }
        } else {
            match key.key {
                Key::Down => Some(PageAction::ScrollLines(n)),
                Key::Up => Some(PageAction::ScrollLines(-n)),
                Key::Char(c @ '0'..='9') if c != '0' || count.is_some() => {
                    let d = c as i32 - '0' as i32;
                    self.count = Some(count.unwrap_or(0).saturating_mul(10).saturating_add(d).min(100_000));
                    return None;
                }
                Key::Char('j') => Some(PageAction::ScrollLines(n)),
                Key::Char('k') => Some(PageAction::ScrollLines(-n)),
                Key::Char('d') => Some(PageAction::HalfPages(n)),
                Key::Char('u') => Some(PageAction::HalfPages(-n)),
                Key::Char('G') => Some(PageAction::Bottom),
                Key::Char('g') if after_g => Some(PageAction::Top),
                Key::Char('g') => {
                    self.pending_g = true;
                    return None;
                }
                _ => None,
            }
        };
        self.count = None;
        action
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::Mods;

    fn feed_all(e: &mut PageEngine, keys: &str) -> Option<PageAction> {
        keys.chars().fold(None, |_, c| e.feed(KeyEvent::char(c)))
    }

    #[test]
    fn lines_and_counts() {
        let mut e = PageEngine::default();
        assert_eq!(feed_all(&mut e, "j"), Some(PageAction::ScrollLines(1)));
        assert_eq!(feed_all(&mut e, "k"), Some(PageAction::ScrollLines(-1)));
        assert_eq!(feed_all(&mut e, "3j"), Some(PageAction::ScrollLines(3)));
        assert_eq!(feed_all(&mut e, "10k"), Some(PageAction::ScrollLines(-10)));
        assert_eq!(e.feed(KeyEvent::plain(Key::Down)), Some(PageAction::ScrollLines(1)));
        assert!(!e.is_pending());
    }

    #[test]
    fn top_bottom_half_pages() {
        let mut e = PageEngine::default();
        assert_eq!(feed_all(&mut e, "g"), None);
        assert!(e.is_pending());
        assert_eq!(feed_all(&mut e, "g"), Some(PageAction::Top));
        assert_eq!(feed_all(&mut e, "G"), Some(PageAction::Bottom));
        assert_eq!(feed_all(&mut e, "d"), Some(PageAction::HalfPages(1)));
        assert_eq!(feed_all(&mut e, "2u"), Some(PageAction::HalfPages(-2)));
        assert_eq!(e.feed(KeyEvent::new(Key::Char('d'), Mods::CTRL)), Some(PageAction::HalfPages(1)));
        assert_eq!(e.feed(KeyEvent::new(Key::Char('u'), Mods::CTRL)), Some(PageAction::HalfPages(-1)));
    }

    #[test]
    fn pending_resets() {
        let mut e = PageEngine::default();
        feed_all(&mut e, "g");
        assert_eq!(feed_all(&mut e, "x"), None);
        assert_eq!(feed_all(&mut e, "g"), None);
        feed_all(&mut e, "5");
        assert_eq!(feed_all(&mut e, "x"), None);
        assert_eq!(feed_all(&mut e, "j"), Some(PageAction::ScrollLines(1)));
        feed_all(&mut e, "5");
        e.feed(KeyEvent::plain(Key::Esc));
        assert!(!e.is_pending());
        assert_eq!(feed_all(&mut e, "0"), None);
    }

    #[test]
    fn cmd_and_alt_ignored() {
        let mut e = PageEngine::default();
        assert_eq!(e.feed(KeyEvent::new(Key::Char('j'), Mods::CMD)), None);
        assert_eq!(e.feed(KeyEvent::new(Key::Char('j'), Mods::ALT)), None);
    }
}
