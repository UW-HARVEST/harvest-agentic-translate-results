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
        PgnMetadataItem::new()
    }
}

#[derive(Debug)]
pub struct PgnMetadata {
    // We preserve insertion order to mirror the C `items` array. The
    // public API uses string keys, so a `Vec<(String, String)>` is the
    // natural representation.
    items: Vec<(String, String)>,
}

impl PgnMetadata {
    pub fn new() -> Self {
        PgnMetadata { items: Vec::new() }
    }

    pub fn from_string_with_consumption(s: &str, consumed: &mut usize) -> Self {
        let bytes = s.as_bytes();
        let mut metadata = PgnMetadata::new();
        let mut cursor: usize = 0;

        if cursor >= bytes.len() || bytes[cursor] != b'[' {
            // Match the C behavior of returning a NULL pointer — but our API
            // returns a `PgnMetadata`. The caller must check whether the
            // returned struct has any items.
            return metadata;
        }

        loop {
            if cursor >= bytes.len() || bytes[cursor] != b'[' {
                break;
            }
            cursor += 1;

            // Read key (until space)
            let key_start = cursor;
            while cursor < bytes.len() && bytes[cursor] != b' ' {
                cursor += 1;
            }
            let key = std::str::from_utf8(&bytes[key_start..cursor])
                .unwrap_or("")
                .to_string();

            // Skip space and assert opening quote
            assert!(cursor + 1 < bytes.len() && bytes[cursor + 1] == b'"');
            cursor += 1; // space
            cursor += 1; // opening quote

            // Read value (until closing quote)
            let val_start = cursor;
            while cursor < bytes.len() && bytes[cursor] != b'"' {
                cursor += 1;
            }
            let value = std::str::from_utf8(&bytes[val_start..cursor])
                .unwrap_or("")
                .to_string();

            metadata.insert(&key, &value);

            // Skip closing quote, expect ']'
            assert!(cursor + 1 < bytes.len() && bytes[cursor + 1] == b']');
            cursor += 1; // closing quote
            cursor += 1; // ']'

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
        self.items.push((key.to_string(), value.to_string()));
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        for (k, v) in self.items.iter() {
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
        PgnMetadata::new()
    }
}
