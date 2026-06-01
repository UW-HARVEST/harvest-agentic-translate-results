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

impl Default for PgnMetadataItem {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct PgnMetadata {
    items: Vec<(String, String)>,
}

impl PgnMetadata {
    pub fn new() -> Self {
        // Pre-allocate using the initial size to match C semantics.
        let _ = (PGN_METADATA_INITIAL_SIZE, PGN_METADATA_GROW_SIZE);
        PgnMetadata {
            items: Vec::with_capacity(PGN_METADATA_INITIAL_SIZE),
        }
    }

    pub fn from_string_with_consumption(s: &str, consumed: &mut usize) -> Self {
        let bytes = s.as_bytes();
        let mut cursor = 0usize;

        let mut metadata = PgnMetadata::new();

        // Allow returning empty metadata if the string doesn't start with `[`,
        // matching the C `if (str[cursor] != '[') return NULL;` semantics
        // (we return an empty metadata struct since Rust's API requires a
        // value).
        if cursor >= bytes.len() || bytes[cursor] != b'[' {
            return metadata;
        }

        let mut key_buffer = String::new();
        let mut value_buffer = String::new();

        loop {
            if cursor >= bytes.len() || bytes[cursor] != b'[' {
                break;
            }
            cursor += 1;

            while cursor < bytes.len() && bytes[cursor] != b' ' {
                key_buffer.push(bytes[cursor] as char);
                cursor += 1;
            }

            cursor += 1; // skip space
            assert!(cursor < bytes.len() && bytes[cursor] == b'"');
            cursor += 1;

            while cursor < bytes.len() && bytes[cursor] != b'"' {
                value_buffer.push(bytes[cursor] as char);
                cursor += 1;
            }

            metadata.insert(&key_buffer, &value_buffer);
            key_buffer.clear();
            value_buffer.clear();

            cursor += 1; // skip closing quote
            assert!(cursor < bytes.len() && bytes[cursor] == b']');
            cursor += 1;

            // skip newline if present
            if cursor < bytes.len() && (bytes[cursor] == b'\n' || bytes[cursor] == b'\r') {
                pgn_cursor_skip_newline(s, &mut cursor);
            } else {
                break;
            }
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
        if let Some(pos) = self.items.iter().position(|(k, _)| k == key) {
            self.items.remove(pos);
        }
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

impl Default for PgnMetadata {
    fn default() -> Self {
        Self::new()
    }
}
