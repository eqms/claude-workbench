use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn map_key_to_pty(key: KeyEvent) -> Option<Vec<u8>> {
    let mut bytes = Vec::new();

    // Handle Control + Char
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        if let KeyCode::Char(c) = key.code {
            let ch = c.to_ascii_lowercase();
            // a=1 ... z=26
            if ('a'..='z').contains(&ch) {
                bytes.push(ch as u8 - b'a' + 1);
                return Some(bytes);
            }
             match c {
                '[' => return Some(vec![27]),
                '\\' => return Some(vec![28]),
                ']' => return Some(vec![29]),
                '^' => return Some(vec![30]),
                '_' => return Some(vec![31]),
                '?' => return Some(vec![127]),
                _ => {}
            }
        }
    }

    match key.code {
        KeyCode::Char(c) => bytes.extend_from_slice(c.to_string().as_bytes()),
        KeyCode::Enter => bytes.push(b'\r'),
        KeyCode::Backspace => bytes.push(127),
        KeyCode::Tab => bytes.push(9),
        KeyCode::BackTab => bytes.extend_from_slice(b"\x1b[Z"),
        KeyCode::Esc => bytes.push(27),
        
        KeyCode::Up => bytes.extend_from_slice(b"\x1b[A"),
        KeyCode::Down => bytes.extend_from_slice(b"\x1b[B"),
        KeyCode::Right => bytes.extend_from_slice(b"\x1b[C"),
        KeyCode::Left => bytes.extend_from_slice(b"\x1b[D"),
        
        KeyCode::Home => bytes.extend_from_slice(b"\x1b[H"),
        KeyCode::End => bytes.extend_from_slice(b"\x1b[F"),
        
        KeyCode::PageUp => bytes.extend_from_slice(b"\x1b[5~"),
        KeyCode::PageDown => bytes.extend_from_slice(b"\x1b[6~"),
        KeyCode::Delete => bytes.extend_from_slice(b"\x1b[3~"),
        KeyCode::Insert => bytes.extend_from_slice(b"\x1b[2~"),
        
        _ => return None,
    }

    Some(bytes)
}
