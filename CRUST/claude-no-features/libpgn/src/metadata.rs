use crate::utils::buffer::PgnBuffer;
use crate::utils::cursor::pgn_cursor_skip_newline;
use std::collections::HashMap;

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

#[derive(Debug)]
pub struct PgnMetadata {
    items: HashMap<String, String>,
}

impl PgnMetadata {
    pub fn new() -> Self {
        let _ = (PGN_METADATA_INITIAL_SIZE, PGN_METADATA_GROW_SIZE);
        PgnMetadata {
            items: HashMap::with_capacity(PGN_METADATA_INITIAL_SIZE),
        }
    }

    pub fn from_string_with_consumption(s: &str, consumed: &mut usize) -> Self {
        let bytes = s.as_bytes();
        let mut cursor = 0usize;
        let mut metadata = PgnMetadata::new();

        if cursor >= bytes.len() || bytes[cursor] != b'[' {
            return metadata;
        }

        loop {
            if cursor >= bytes.len() || bytes[cursor] != b'[' {
                break;
            }
            cursor += 1;

            // Read key until space.
            let key_start = cursor;
            while cursor < bytes.len() && bytes[cursor] != b' ' {
                cursor += 1;
            }
            let key = s[key_start..cursor].to_string();

            // Skip space.
            if cursor < bytes.len() && bytes[cursor] == b' ' {
                cursor += 1;
            }

            // Expect quote.
            if cursor >= bytes.len() || bytes[cursor] != b'"' {
                break;
            }
            cursor += 1;

            // Read value until quote.
            let value_start = cursor;
            while cursor < bytes.len() && bytes[cursor] != b'"' {
                cursor += 1;
            }
            let value = s[value_start..cursor].to_string();

            metadata.insert(&key, &value);

            // Skip closing quote.
            if cursor < bytes.len() && bytes[cursor] == b'"' {
                cursor += 1;
            }
            // Skip closing bracket.
            if cursor < bytes.len() && bytes[cursor] == b']' {
                cursor += 1;
            }

            // Skip newline.
            pgn_cursor_skip_newline(s, &mut cursor);
        }

        *consumed += cursor;
        metadata
    }

    pub fn from_string(s: &str) -> Self {
        let mut consumed = 0usize;
        Self::from_string_with_consumption(s, &mut consumed)
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

impl Default for PgnMetadata {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for PgnMetadataItem {
    fn default() -> Self {
        Self::new()
    }
}
