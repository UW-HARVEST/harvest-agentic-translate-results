use crate::utils::buffer::PgnBuffer;
use crate::utils::cursor::pgn_cursor_skip_newline;

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
        let mut cursor = 0usize;

        if cursor >= bytes.len() || bytes[cursor] != b'[' {
            return PgnMetadata::new();
        }

        let mut metadata = PgnMetadata::new();

        loop {
            if cursor >= bytes.len() || bytes[cursor] != b'[' { break; }
            cursor += 1;

            let mut key = String::new();
            while cursor < bytes.len() && bytes[cursor] != b' ' {
                key.push(bytes[cursor] as char);
                cursor += 1;
            }

            // skip space then opening quote
            cursor += 1;
            assert!(cursor < bytes.len() && bytes[cursor] == b'"');
            cursor += 1;

            let mut value = String::new();
            while cursor < bytes.len() && bytes[cursor] != b'"' {
                value.push(bytes[cursor] as char);
                cursor += 1;
            }

            // skip closing quote
            cursor += 1;
            assert!(cursor < bytes.len() && bytes[cursor] == b']');
            cursor += 1;

            metadata.insert(&key, &value);
            pgn_cursor_skip_newline(s, &mut cursor);
        }

        *consumed += cursor;
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
    pub fn items_is_empty(&self) -> bool {
        self.items.is_empty()
    }
}
