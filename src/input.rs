use crate::error::{Error, Result};

#[derive(Debug, Clone, PartialEq)]
pub enum Key {
    Char(char),
    Enter,
    Tab,
    Backspace,
    Escape,
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,
    Insert,
    Delete,
    F(u8),
    Ctrl(char),
    Alt(char),
}

#[derive(Debug, Clone, PartialEq)]
pub enum MouseAction {
    Press,
    Release,
    Move,
    ScrollUp,
    ScrollDown,
}

impl Key {
    pub fn to_escape_sequence(&self) -> Vec<u8> {
        match self {
            Key::Char(c) => {
                let mut buf = [0u8; 4];
                let s = c.encode_utf8(&mut buf);
                s.as_bytes().to_vec()
            }
            Key::Enter => vec![13],
            Key::Tab => vec![9],
            Key::Backspace => vec![127],
            Key::Escape => vec![27],
            Key::Up => b"\x1b[A".to_vec(),
            Key::Down => b"\x1b[B".to_vec(),
            Key::Left => b"\x1b[D".to_vec(),
            Key::Right => b"\x1b[C".to_vec(),
            Key::Home => b"\x1b[H".to_vec(),
            Key::End => b"\x1b[F".to_vec(),
            Key::PageUp => b"\x1b[5~".to_vec(),
            Key::PageDown => b"\x1b[6~".to_vec(),
            Key::Insert => b"\x1b[2~".to_vec(),
            Key::Delete => b"\x1b[3~".to_vec(),
            Key::F(n) => match n {
                1 => b"\x1bOP".to_vec(),
                2 => b"\x1bOQ".to_vec(),
                3 => b"\x1bOR".to_vec(),
                4 => b"\x1bOS".to_vec(),
                5 => b"\x1b[15~".to_vec(),
                6 => b"\x1b[17~".to_vec(),
                7 => b"\x1b[18~".to_vec(),
                8 => b"\x1b[19~".to_vec(),
                9 => b"\x1b[20~".to_vec(),
                10 => b"\x1b[21~".to_vec(),
                11 => b"\x1b[23~".to_vec(),
                12 => b"\x1b[24~".to_vec(),
                _ => vec![],
            },
            Key::Ctrl(c) => {
                let byte = (*c as u8).wrapping_sub(b'a').wrapping_add(1);
                vec![byte]
            }
            Key::Alt(c) => {
                let mut buf = vec![27];
                let mut char_buf = [0u8; 4];
                let s = c.encode_utf8(&mut char_buf);
                buf.extend_from_slice(s.as_bytes());
                buf
            }
        }
    }
}

pub fn parse_key_name(name: &str) -> Result<Key> {
    let lower = name.to_lowercase();

    if lower.starts_with("ctrl+") || lower.starts_with("ctrl-") {
        let ch = lower[5..].chars().next().ok_or_else(|| Error::UnknownKey(name.to_string()))?;
        if ch.is_ascii_lowercase() {
            return Ok(Key::Ctrl(ch));
        }
        return Err(Error::UnknownKey(name.to_string()));
    }

    if lower.starts_with("alt+") || lower.starts_with("alt-") {
        let ch = lower[4..].chars().next().ok_or_else(|| Error::UnknownKey(name.to_string()))?;
        return Ok(Key::Alt(ch));
    }

    if lower.starts_with('f') && lower.len() >= 2 {
        if let Ok(n) = lower[1..].parse::<u8>() {
            if (1..=12).contains(&n) {
                return Ok(Key::F(n));
            }
        }
    }

    match lower.as_str() {
        "enter" | "return" => Ok(Key::Enter),
        "tab" => Ok(Key::Tab),
        "backspace" | "bs" => Ok(Key::Backspace),
        "escape" | "esc" => Ok(Key::Escape),
        "up" => Ok(Key::Up),
        "down" => Ok(Key::Down),
        "left" => Ok(Key::Left),
        "right" => Ok(Key::Right),
        "home" => Ok(Key::Home),
        "end" => Ok(Key::End),
        "pageup" | "pgup" => Ok(Key::PageUp),
        "pagedown" | "pgdn" => Ok(Key::PageDown),
        "insert" | "ins" => Ok(Key::Insert),
        "delete" | "del" => Ok(Key::Delete),
        "space" => Ok(Key::Char(' ')),
        _ => Err(Error::UnknownKey(name.to_string())),
    }
}

pub fn parse_mouse_action(action: &str) -> Result<MouseAction> {
    match action.to_lowercase().as_str() {
        "press" | "click" => Ok(MouseAction::Press),
        "release" => Ok(MouseAction::Release),
        "move" => Ok(MouseAction::Move),
        "scrollup" | "scroll-up" => Ok(MouseAction::ScrollUp),
        "scrolldown" | "scroll-down" => Ok(MouseAction::ScrollDown),
        _ => Err(Error::UnknownMouseAction(action.to_string())),
    }
}

pub fn mouse_sgr_sequence(action: &MouseAction, col: u16, row: u16) -> Vec<u8> {
    let (button, suffix) = match action {
        MouseAction::Press => (0, 'M'),
        MouseAction::Release => (0, 'm'),
        MouseAction::Move => (32, 'M'),
        MouseAction::ScrollUp => (64, 'M'),
        MouseAction::ScrollDown => (65, 'M'),
    };
    format!("\x1b[<{};{};{}{}", button, col + 1, row + 1, suffix).into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic_keys() {
        assert_eq!(parse_key_name("enter").unwrap(), Key::Enter);
        assert_eq!(parse_key_name("Enter").unwrap(), Key::Enter);
        assert_eq!(parse_key_name("return").unwrap(), Key::Enter);
        assert_eq!(parse_key_name("tab").unwrap(), Key::Tab);
        assert_eq!(parse_key_name("escape").unwrap(), Key::Escape);
        assert_eq!(parse_key_name("esc").unwrap(), Key::Escape);
        assert_eq!(parse_key_name("space").unwrap(), Key::Char(' '));
    }

    #[test]
    fn test_parse_arrow_keys() {
        assert_eq!(parse_key_name("up").unwrap(), Key::Up);
        assert_eq!(parse_key_name("down").unwrap(), Key::Down);
        assert_eq!(parse_key_name("left").unwrap(), Key::Left);
        assert_eq!(parse_key_name("right").unwrap(), Key::Right);
    }

    #[test]
    fn test_parse_function_keys() {
        assert_eq!(parse_key_name("f1").unwrap(), Key::F(1));
        assert_eq!(parse_key_name("F5").unwrap(), Key::F(5));
        assert_eq!(parse_key_name("f12").unwrap(), Key::F(12));
    }

    #[test]
    fn test_parse_ctrl_keys() {
        assert_eq!(parse_key_name("ctrl+c").unwrap(), Key::Ctrl('c'));
        assert_eq!(parse_key_name("ctrl-z").unwrap(), Key::Ctrl('z'));
        assert_eq!(parse_key_name("Ctrl+A").unwrap(), Key::Ctrl('a'));
    }

    #[test]
    fn test_parse_alt_keys() {
        assert_eq!(parse_key_name("alt+x").unwrap(), Key::Alt('x'));
        assert_eq!(parse_key_name("Alt-F").unwrap(), Key::Alt('f'));
    }

    #[test]
    fn test_unknown_key() {
        assert!(parse_key_name("nonexistent").is_err());
    }

    #[test]
    fn test_escape_sequences() {
        assert_eq!(Key::Enter.to_escape_sequence(), vec![13]);
        assert_eq!(Key::Tab.to_escape_sequence(), vec![9]);
        assert_eq!(Key::Up.to_escape_sequence(), b"\x1b[A".to_vec());
        assert_eq!(Key::Ctrl('c').to_escape_sequence(), vec![3]);
        assert_eq!(Key::Ctrl('a').to_escape_sequence(), vec![1]);
        assert_eq!(Key::F(1).to_escape_sequence(), b"\x1bOP".to_vec());
        assert_eq!(Key::Char('a').to_escape_sequence(), b"a".to_vec());
    }

    #[test]
    fn test_mouse_sgr() {
        let seq = mouse_sgr_sequence(&MouseAction::Press, 10, 5);
        assert_eq!(seq, b"\x1b[<0;11;6M".to_vec());

        let seq = mouse_sgr_sequence(&MouseAction::Release, 10, 5);
        assert_eq!(seq, b"\x1b[<0;11;6m".to_vec());

        let seq = mouse_sgr_sequence(&MouseAction::ScrollUp, 0, 0);
        assert_eq!(seq, b"\x1b[<64;1;1M".to_vec());
    }

    #[test]
    fn test_parse_mouse_action() {
        assert_eq!(parse_mouse_action("press").unwrap(), MouseAction::Press);
        assert_eq!(parse_mouse_action("click").unwrap(), MouseAction::Press);
        assert_eq!(parse_mouse_action("release").unwrap(), MouseAction::Release);
        assert_eq!(parse_mouse_action("scrollup").unwrap(), MouseAction::ScrollUp);
        assert!(parse_mouse_action("invalid").is_err());
    }
}
