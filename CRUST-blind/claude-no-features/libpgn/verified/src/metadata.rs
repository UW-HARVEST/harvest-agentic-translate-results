use std::collections::HashMap;

use crate::utils::buffer::PgnBuffer;
use crate::utils::cursor::pgn_cursor_skip_newline;

const PGN_METADATA_INITIAL_SIZE: usize = 8;
const PGN_METADATA_GROW_SIZE: usize = 8;

#[allow(dead_code)]
fn _silence_constants() {
    let _ = (PGN_METADATA_INITIAL_SIZE, PGN_METADATA_GROW_SIZE);
}

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

/// Preserves both the original insertion order and the values, so that
/// metadata behaves like a Pythonic ordered dictionary.
#[derive(Debug)]
pub struct PgnMetadata {
    items: HashMap<String, String>,
    order: Vec<String>,
}
impl PgnMetadata {
    pub fn new() -> Self {
        PgnMetadata {
            items: HashMap::new(),
            order: Vec::new(),
        }
    }

    /// Parses metadata tags `[Key "Value"]` from `s`, advancing `consumed`.
    pub fn from_string_with_consumption(s: &str, consumed: &mut usize) -> Self {
        let bytes = s.as_bytes();
        let mut cursor: usize = 0;

        let mut metadata = PgnMetadata::new();

        if bytes.first().copied() != Some(b'[') {
            return metadata;
        }

        let mut key_buffer = String::new();
        let mut value_buffer = String::new();

        loop {
            if bytes.get(cursor).copied() != Some(b'[') {
                break;
            }
            cursor += 1;

            // Read the key (up to a space).
            while let Some(b) = bytes.get(cursor).copied() {
                if b == b' ' {
                    break;
                }
                key_buffer.push(b as char);
                cursor += 1;
            }

            // Skip the single space.
            cursor += 1;
            assert_eq!(bytes.get(cursor).copied(), Some(b'"'));
            cursor += 1;

            // Read the value (up to the closing quote).
            while let Some(b) = bytes.get(cursor).copied() {
                if b == b'"' {
                    break;
                }
                value_buffer.push(b as char);
                cursor += 1;
            }

            metadata.insert(&key_buffer, &value_buffer);
            key_buffer.clear();
            value_buffer.clear();

            // Consume the closing quote and the closing bracket.
            cursor += 1;
            assert_eq!(bytes.get(cursor).copied(), Some(b']'));
            cursor += 1;

            // Skip the trailing newline (if any).
            match bytes.get(cursor).copied() {
                Some(b'\n') | Some(b'\r') => {
                    pgn_cursor_skip_newline(s, &mut cursor);
                }
                _ => break,
            }
        }

        *consumed += cursor;
        metadata
    }

    pub fn from_string(s: &str) -> Self {
        let mut consumed = 0;
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
            self.order.retain(|k| k != key);
        }
    }
}

impl Default for PgnMetadata {
    fn default() -> Self {
        Self::new()
    }
}
