use crate::utils::buffer::PgnBuffer;
use crate::utils::cursor;

const PGN_METADATA_INITIAL_SIZE: usize = 8;
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
use std::collections::HashMap;
#[derive(Debug)]
pub struct PgnMetadata {
    items: HashMap<String, String>,
}
impl PgnMetadata {
    pub fn new() -> Self {
        PgnMetadata {
            items: HashMap::new(),
        }
    }
    pub fn from_string_with_consumption(s: &str, consumed: &mut usize) -> Self {
        let bytes = s.as_bytes();
        let mut cursor_pos = 0usize;

        if cursor_pos >= bytes.len() || bytes[cursor_pos] != b'[' {
            return PgnMetadata::new();
        }

        let mut metadata = PgnMetadata::new();

        loop {
            if cursor_pos >= bytes.len() || bytes[cursor_pos] != b'[' {
                break;
            }
            cursor_pos += 1;

            // Parse key
            let mut key = String::new();
            while cursor_pos < bytes.len() && bytes[cursor_pos] != b' ' {
                key.push(bytes[cursor_pos] as char);
                cursor_pos += 1;
            }

            // Skip space and opening quote
            cursor_pos += 1; // space
            assert!(cursor_pos < bytes.len() && bytes[cursor_pos] == b'"');
            cursor_pos += 1;

            // Parse value
            let mut value = String::new();
            while cursor_pos < bytes.len() && bytes[cursor_pos] != b'"' {
                value.push(bytes[cursor_pos] as char);
                cursor_pos += 1;
            }

            // Skip closing quote
            cursor_pos += 1;

            metadata.insert(&key, &value);

            assert!(cursor_pos < bytes.len() && bytes[cursor_pos] == b']');
            cursor_pos += 1;

            cursor::pgn_cursor_skip_newline(s, &mut cursor_pos);
        }

        *consumed += cursor_pos;
        metadata
    }
    pub fn from_string(s: &str) -> Self {
        let mut consumed = 0;
        PgnMetadata::from_string_with_consumption(s, &mut consumed)
    }
    pub fn insert(&mut self, key: &str, value: &str) {
        self.items.insert(key.to_string(), value.to_string());
    }
    pub fn get(&self, key: &str) -> Option<&str> {
        self.items.get(key).map(|s| s.as_str())
    }
    pub fn delete(&mut self, key: &str) {
        self.items.remove(key);
    }
}
