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
    // We use a Vec<(String, String)> to preserve insertion order, the same way
    // the C version stores items as a dynamic array.
    items: Vec<(String, String)>,
}

impl PgnMetadata {
    pub fn new() -> Self {
        PgnMetadata {
            items: Vec::with_capacity(PGN_METADATA_INITIAL_SIZE),
        }
    }

    pub fn from_string_with_consumption(s: &str, consumed: &mut usize) -> Self {
        let bytes = s.as_bytes();
        let mut metadata = PgnMetadata::new();
        let mut cursor = 0usize;

        if bytes.is_empty() || bytes[0] != b'[' {
            return metadata;
        }

        loop {
            if cursor >= bytes.len() || bytes[cursor] != b'[' {
                break;
            }
            cursor += 1;

            let key_start = cursor;
            while cursor < bytes.len() && bytes[cursor] != b' ' {
                cursor += 1;
            }
            let key = std::str::from_utf8(&bytes[key_start..cursor])
                .unwrap_or("")
                .to_string();

            // Skip the space.
            debug_assert!(cursor < bytes.len() && bytes[cursor] == b' ');
            cursor += 1;
            // Expect opening quote.
            debug_assert!(cursor < bytes.len() && bytes[cursor] == b'"');
            cursor += 1;

            let value_start = cursor;
            while cursor < bytes.len() && bytes[cursor] != b'"' {
                cursor += 1;
            }
            let value = std::str::from_utf8(&bytes[value_start..cursor])
                .unwrap_or("")
                .to_string();

            metadata.insert(&key, &value);

            // Skip closing quote.
            debug_assert!(cursor < bytes.len() && bytes[cursor] == b'"');
            cursor += 1;
            // Expect closing bracket.
            debug_assert!(cursor < bytes.len() && bytes[cursor] == b']');
            cursor += 1;

            // Skip newline (if present).
            if cursor < bytes.len() && (bytes[cursor] == b'\n' || bytes[cursor] == b'\r') {
                pgn_cursor_skip_newline(s, &mut cursor);
            }
        }

        *consumed += cursor;
        metadata
    }

    pub fn from_string(s: &str) -> Self {
        let mut consumed = 0usize;
        PgnMetadata::from_string_with_consumption(s, &mut consumed)
    }

    pub fn insert(&mut self, key: &str, value: &str) {
        self.items.push((key.to_string(), value.to_string()));
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        for (k, v) in &self.items {
            if k == key {
                return Some(v.as_str());
            }
        }
        None
    }

    pub fn delete(&mut self, key: &str) {
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
