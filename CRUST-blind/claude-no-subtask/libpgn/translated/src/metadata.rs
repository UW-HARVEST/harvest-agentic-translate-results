use std::collections::HashMap;

use crate::utils::buffer::PgnBuffer;
use crate::utils::cursor::pgn_cursor_skip_newline;

#[allow(dead_code)]
const PGN_METADATA_INITIAL_SIZE: usize = 8;
#[allow(dead_code)]
const PGN_METADATA_GROW_SIZE: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PgnMetadataItem {
    key: PgnBuffer,
    value: PgnBuffer,
}

impl PgnMetadataItem {
    pub fn new() -> Self {
        PgnMetadataItem {
            key: PgnBuffer::new(),
            value: PgnBuffer::new(),
        }
    }
}

impl Default for PgnMetadataItem {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct PgnMetadata {
    /// Insertion-ordered key-value list. We keep both a Vec for ordering
    /// preservation (matching the C array semantics) and a HashMap to keep
    /// the API contract from the public interface.
    pub items: HashMap<String, String>,
    order: Vec<String>,
}

impl PgnMetadata {
    pub fn new() -> Self {
        PgnMetadata {
            items: HashMap::new(),
            order: Vec::new(),
        }
    }

    pub fn from_string_with_consumption(s: &str, consumed: &mut usize) -> Self {
        let bytes = s.as_bytes();
        let mut cursor: usize = 0;
        let mut metadata = PgnMetadata::new();

        if cursor >= bytes.len() || bytes[cursor] != b'[' {
            // matches the C `if (str[cursor] != '[') return NULL;`
            // Returns an empty metadata (callers should check `items.is_empty()`).
            return metadata;
        }

        loop {
            if cursor >= bytes.len() || bytes[cursor] != b'[' {
                break;
            }
            cursor += 1;

            let mut key = String::new();
            while cursor < bytes.len() && bytes[cursor] != b' ' {
                key.push(bytes[cursor] as char);
                cursor += 1;
            }

            // Expect a space then opening quote
            if cursor >= bytes.len() || bytes[cursor] != b' ' {
                panic!("expected space after key");
            }
            cursor += 1;
            if cursor >= bytes.len() || bytes[cursor] != b'"' {
                panic!("expected opening quote");
            }
            cursor += 1;

            let mut value = String::new();
            while cursor < bytes.len() && bytes[cursor] != b'"' {
                value.push(bytes[cursor] as char);
                cursor += 1;
            }
            if cursor >= bytes.len() || bytes[cursor] != b'"' {
                panic!("expected closing quote");
            }

            metadata.insert(&key, &value);

            // Expect closing ']'
            cursor += 1;
            if cursor >= bytes.len() || bytes[cursor] != b']' {
                panic!("expected ']'");
            }
            cursor += 1;

            // Skip newline (\n or \r\n) if present.
            if cursor < bytes.len() && (bytes[cursor] == b'\n' || bytes[cursor] == b'\r') {
                pgn_cursor_skip_newline(s, &mut cursor);
            }
        }

        *consumed += cursor;
        metadata
    }

    pub fn from_string(s: &str) -> Self {
        let mut consumed: usize = 0;
        Self::from_string_with_consumption(s, &mut consumed)
    }

    pub fn insert(&mut self, key: &str, value: &str) {
        if !self.items.contains_key(key) {
            self.order.push(key.to_string());
        }
        self.items.insert(key.to_string(), value.to_string());
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.items.get(key).map(|s| s.as_str())
    }

    pub fn delete(&mut self, key: &str) {
        if self.items.remove(key).is_some() {
            if let Some(pos) = self.order.iter().position(|k| k == key) {
                self.order.remove(pos);
            }
        }
    }
}

impl Default for PgnMetadata {
    fn default() -> Self {
        Self::new()
    }
}
