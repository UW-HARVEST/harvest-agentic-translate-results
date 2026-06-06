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

#[derive(Debug)]
pub struct PgnMetadata {
    // Use Vec to preserve insertion order — matches C semantics.
    items: Vec<(String, String)>,
}

impl PgnMetadata {
    pub fn new() -> Self {
        let mut v: Vec<(String, String)> = Vec::new();
        v.reserve(PGN_METADATA_INITIAL_SIZE);
        let _ = PGN_METADATA_GROW_SIZE; // keep referenced
        PgnMetadata { items: v }
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

            let mut key = String::new();
            while cursor < bytes.len() && bytes[cursor] != b' ' {
                key.push(bytes[cursor] as char);
                cursor += 1;
            }
            // Skip the space
            assert!(cursor < bytes.len() && bytes[cursor] == b' ');
            cursor += 1;

            // Expect a quote
            assert!(cursor < bytes.len() && bytes[cursor] == b'"');
            cursor += 1;

            let mut value = String::new();
            while cursor < bytes.len() && bytes[cursor] != b'"' {
                value.push(bytes[cursor] as char);
                cursor += 1;
            }
            // Closing quote
            assert!(cursor < bytes.len() && bytes[cursor] == b'"');
            cursor += 1;

            metadata.insert(&key, &value);

            // Closing ]
            assert!(cursor < bytes.len() && bytes[cursor] == b']');
            cursor += 1;

            // Optional newline
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
        if let Some(idx) = self.items.iter().position(|(k, _)| k == key) {
            self.items.remove(idx);
        }
    }
}

impl Default for PgnMetadata {
    fn default() -> Self {
        Self::new()
    }
}
