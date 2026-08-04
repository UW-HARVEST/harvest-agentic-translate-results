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
        let mut cursor: usize = 0;
        let mut metadata = PgnMetadata::new();

        if bytes.is_empty() || bytes[cursor] != b'[' {
            return metadata;
        }

        loop {
            if cursor >= bytes.len() || bytes[cursor] != b'[' {
                break;
            }
            cursor += 1;

            // read key until space
            let key_start = cursor;
            while cursor < bytes.len() && bytes[cursor] != b' ' {
                cursor += 1;
            }
            let key = s[key_start..cursor].to_string();

            // skip the space, expect '"'
            assert!(cursor + 1 < bytes.len());
            cursor += 1;
            assert_eq!(bytes[cursor], b'"');
            cursor += 1;

            // read value until '"'
            let val_start = cursor;
            while cursor < bytes.len() && bytes[cursor] != b'"' {
                cursor += 1;
            }
            let value = s[val_start..cursor].to_string();

            metadata.insert(&key, &value);

            // we are at the closing '"'; advance past it
            // C: assert(str[++cursor] == ']');
            cursor += 1;
            assert!(cursor < bytes.len() && bytes[cursor] == b']');
            cursor += 1;

            if cursor < bytes.len() {
                pgn_cursor_skip_newline(s, &mut cursor);
            }
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
}
