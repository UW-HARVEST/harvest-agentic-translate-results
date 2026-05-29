use std::collections::HashMap;

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
        let _ = PGN_METADATA_INITIAL_SIZE;
        let _ = PGN_METADATA_GROW_SIZE;
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

#[derive(Debug, Clone)]
pub struct PgnMetadata {
    items: Vec<(String, String)>,
    // keep a HashMap for O(1) lookups but mirror with vec to preserve insertion order if needed
    map: HashMap<String, String>,
}

impl PgnMetadata {
    pub fn new() -> Self {
        PgnMetadata {
            items: Vec::new(),
            map: HashMap::new(),
        }
    }

    pub fn from_string_with_consumption(s: &str, consumed: &mut usize) -> Self {
        let bytes = s.as_bytes();
        let mut cursor: usize = 0;
        let mut metadata = PgnMetadata::new();

        if cursor >= bytes.len() || bytes[cursor] != b'[' {
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

            // skip the space
            cursor += 1;
            // expect '"'
            assert_eq!(bytes[cursor], b'"');
            cursor += 1;

            let mut value = String::new();
            while cursor < bytes.len() && bytes[cursor] != b'"' {
                value.push(bytes[cursor] as char);
                cursor += 1;
            }

            metadata.insert(&key, &value);

            // expect closing '"'
            cursor += 1;
            assert_eq!(bytes[cursor], b']');
            cursor += 1;

            // skip newline (handles \n and \r\n)
            pgn_cursor_skip_newline(s, &mut cursor);
        }

        *consumed += cursor;
        metadata
    }

    pub fn from_string(s: &str) -> Self {
        let mut consumed = 0;
        Self::from_string_with_consumption(s, &mut consumed)
    }

    pub fn insert(&mut self, key: &str, value: &str) {
        self.items.push((key.to_string(), value.to_string()));
        self.map.insert(key.to_string(), value.to_string());
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.map.get(key).map(|s| s.as_str())
    }

    pub fn delete(&mut self, key: &str) {
        self.map.remove(key);
        if let Some(pos) = self.items.iter().position(|(k, _)| k == key) {
            self.items.remove(pos);
        }
    }
}

impl Default for PgnMetadata {
    fn default() -> Self {
        Self::new()
    }
}
