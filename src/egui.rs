//! egui olaylarını `less-input` tuşlarına çevirir (`egui` özelliği).

use crate::{Key, KeyEvent, Mods};

/// Bu karedeki egui olaylarından tuş listesi.
///
/// - Yazı üreten tuşlar `Event::Text`'ten gelir (Key olayı atlanır), böylece klavye düzeni (Türkçe Q) doğru çalışır.
/// - Cmd / Ctrl kombinasyonları Key olayından; rakamlarda fiziksel tuş kullanılır (Cmd+Shift+0 düzenden bağımsız, #42).
/// - Option + harf: ardından ASCII yazı gelirse (Türkçe düzende Option+Q = `@`) yazıdır, ASCII dışıysa (`∆`)
///   `<M-j>` gibi Option kısayoludur.
/// - egui'nin Copy / Cut / Paste olayları `<D-c>` / `<D-x>` / `<D-v>` olur; yapıştırma metni panodan editör okur.
pub fn translate(events: &[egui::Event]) -> Vec<KeyEvent> {
    let mut out = Vec::new();
    let mut pending_alt: Option<KeyEvent> = None;
    let mut skip_text = false;
    for ev in events {
        if !matches!(ev, egui::Event::Text(_))
            && let Some(alt) = pending_alt.take()
        {
            out.push(alt);
        }
        match ev {
            egui::Event::Key { key, physical_key, pressed: true, modifiers, .. } => {
                skip_text = false;
                let mods = mods(*modifiers);
                if let Some(k) = special(*key) {
                    out.push(KeyEvent::new(k, mods));
                } else if let Some(c) = char_of(*key, *physical_key, mods) {
                    let ev = KeyEvent::new(Key::Char(c), mods);
                    if mods.cmd || mods.ctrl {
                        out.push(ev);
                        skip_text = true;
                    } else if mods.alt {
                        pending_alt = Some(ev);
                    }
                }
            }
            egui::Event::Text(text) => {
                if let Some(alt) = pending_alt.take() {
                    if !text.is_ascii() {
                        out.push(alt);
                        continue;
                    }
                } else if std::mem::take(&mut skip_text) {
                    continue;
                }
                out.extend(text.chars().map(KeyEvent::char));
            }
            egui::Event::Copy => out.push(KeyEvent::new(Key::Char('c'), Mods::CMD)),
            egui::Event::Cut => out.push(KeyEvent::new(Key::Char('x'), Mods::CMD)),
            egui::Event::Paste(_) => out.push(KeyEvent::new(Key::Char('v'), Mods::CMD)),
            _ => {}
        }
    }
    out.extend(pending_alt);
    out
}

/// egui değiştiricileri. `cmd` macOS Command tuşudur.
fn mods(m: egui::Modifiers) -> Mods {
    // shortcut: Linux/Windows'ta Super tuşu `cmd` sayılmaz (egui `mac_cmd` yalnız macOS'ta); oralara port edilince bakılır.
    Mods { shift: m.shift, ctrl: m.ctrl, alt: m.alt, cmd: m.mac_cmd }
}

/// Yazı üretmeyen tuşlar.
fn special(key: egui::Key) -> Option<Key> {
    use egui::Key as E;
    Some(match key {
        E::Enter => Key::Enter,
        E::Escape => Key::Esc,
        E::Tab => Key::Tab,
        E::Backspace => Key::Backspace,
        E::Delete => Key::Delete,
        E::ArrowLeft => Key::Left,
        E::ArrowRight => Key::Right,
        E::ArrowUp => Key::Up,
        E::ArrowDown => Key::Down,
        E::Home => Key::Home,
        E::End => Key::End,
        E::PageUp => Key::PageUp,
        E::PageDown => Key::PageDown,
        E::F1 => Key::F(1),
        E::F2 => Key::F(2),
        E::F3 => Key::F(3),
        E::F4 => Key::F(4),
        E::F5 => Key::F(5),
        E::F6 => Key::F(6),
        E::F7 => Key::F(7),
        E::F8 => Key::F(8),
        E::F9 => Key::F(9),
        E::F10 => Key::F(10),
        E::F11 => Key::F(11),
        E::F12 => Key::F(12),
        _ => return None,
    })
}

/// Yazı üreten tuşun karakteri (değiştiricili kısayollar için). Harfler Shift ile büyük harf olur.
fn char_of(key: egui::Key, physical: Option<egui::Key>, mods: Mods) -> Option<char> {
    use egui::Key as E;
    let key = match physical {
        Some(p) if (mods.cmd || mods.ctrl) && p.name().len() == 1 && p.name().as_bytes()[0].is_ascii_digit() => p,
        _ => key,
    };
    let c = match key {
        E::Space => ' ',
        E::Minus => '-',
        E::Quote => '\'',
        _ => {
            let s = key.symbol_or_name();
            let mut chars = s.chars();
            match (chars.next(), chars.next()) {
                (Some(c), None) => c,
                _ => return None,
            }
        }
    };
    Some(if c.is_ascii_alphabetic() && !mods.shift { c.to_ascii_lowercase() } else { c })
}

/// Tek bir egui tuşu (Text olayı olmadan); yazı üreten tuşlar düzenden bağımsız egui adıyla gelir.
pub fn to_less_key(key: egui::Key, modifiers: egui::Modifiers) -> Option<KeyEvent> {
    let m = mods(modifiers);
    let k = special(key).or_else(|| char_of(key, None, m).map(Key::Char))?;
    Some(KeyEvent::new(k, m))
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Event, Key as E, Modifiers};

    fn key(k: E, m: Modifiers) -> Event {
        Event::Key { key: k, physical_key: Some(k), pressed: true, repeat: false, modifiers: m }
    }

    fn cmd() -> Modifiers {
        Modifiers { mac_cmd: true, command: true, ..Default::default() }
    }

    #[test]
    fn plain_typing_comes_from_text_events() {
        let evs = [key(E::D, Modifiers::NONE), Event::Text("d".into()), key(E::Num2, Modifiers::SHIFT)];
        let evs: Vec<Event> = evs.into_iter().chain([Event::Text("'".into())]).collect();
        assert_eq!(translate(&evs), vec![KeyEvent::char('d'), KeyEvent::char('\'')]);
    }

    #[test]
    fn cmd_digit_uses_physical_key_42() {
        let ev = Event::Key {
            key: E::Equals,
            physical_key: Some(E::Num0),
            pressed: true,
            repeat: false,
            modifiers: Modifiers { shift: true, ..cmd() },
        };
        let out = translate(&[ev]);
        assert_eq!(out, vec![KeyEvent::new(Key::Char('0'), Mods { shift: true, ..Mods::CMD })]);
    }

    #[test]
    fn option_letter_is_alt_unless_layout_types_ascii() {
        let alt = Modifiers::ALT;
        let us = translate(&[key(E::J, alt), Event::Text("∆".into())]);
        assert_eq!(us, vec![KeyEvent::new(Key::Char('j'), Mods::ALT)]);
        let tr = translate(&[key(E::Q, alt), Event::Text("@".into())]);
        assert_eq!(tr, vec![KeyEvent::char('@')]);
        let bare = translate(&[key(E::K, alt)]);
        assert_eq!(bare, vec![KeyEvent::new(Key::Char('k'), Mods::ALT)]);
    }

    #[test]
    fn clipboard_events_become_cmd_keys() {
        let out = translate(&[Event::Copy, Event::Paste("x".into())]);
        assert_eq!(out, vec![KeyEvent::new(Key::Char('c'), Mods::CMD), KeyEvent::new(Key::Char('v'), Mods::CMD)]);
    }

    #[test]
    fn special_keys_and_released_events() {
        let released =
            Event::Key { key: E::A, physical_key: None, pressed: false, repeat: false, modifiers: Modifiers::NONE };
        let out = translate(&[key(E::Escape, Modifiers::NONE), released, key(E::Tab, Modifiers::SHIFT)]);
        assert_eq!(out, vec![KeyEvent::plain(Key::Esc), KeyEvent::new(Key::Tab, Mods::SHIFT)]);
    }

    #[test]
    fn single_key_without_text_event() {
        assert_eq!(to_less_key(E::G, Modifiers::SHIFT), Some(KeyEvent::new(Key::Char('G'), Mods::SHIFT)));
        assert_eq!(to_less_key(E::J, Modifiers::NONE), Some(KeyEvent::char('j')));
        assert_eq!(to_less_key(E::Enter, Modifiers::NONE), Some(KeyEvent::plain(Key::Enter)));
        assert_eq!(to_less_key(E::F13, Modifiers::NONE), None);
    }
}
