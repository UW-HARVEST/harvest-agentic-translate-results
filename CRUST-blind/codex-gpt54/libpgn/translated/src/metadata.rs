use crate::utils::buffer::PgnBuffer;
const PGN_METADATA_INITIAL_SIZE: usize = 8;
const PGN_METADATA_GROW_SIZE: usize = 8;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PgnMetadataItem {
    key: PgnBuffer,
    value: PgnBuffer,
}
impl PgnMetadataItem {
    pub fn new() -> Self {
        Self {
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
        Self {
            items: HashMap::with_capacity(PGN_METADATA_INITIAL_SIZE),
        }
    }
    pub fn from_string_with_consumption(s: &str, consumed: &mut usize) -> Self {
        let bytes = s.as_bytes();
        let mut cursor = 0;

        if !matches!(bytes.get(cursor), Some(b'[')) {
            return Self::new();
        }

        let mut metadata = Self::new();
        while matches!(bytes.get(cursor), Some(b'[')) {
            cursor += 1;
            let key_start = cursor;
            while !matches!(bytes.get(cursor), Some(b' ')) {
                cursor += 1;
            }
            let key = &s[key_start..cursor];

            cursor += 1;
            if matches!(bytes.get(cursor), Some(b'"')) {
                cursor += 1;
            }
            let value_start = cursor;
            while !matches!(bytes.get(cursor), Some(b'"')) {
                cursor += 1;
            }
            let value = &s[value_start..cursor];

            metadata.insert(key, value);

            cursor += 1;
            if matches!(bytes.get(cursor), Some(b']')) {
                cursor += 1;
            }

            if matches!(bytes.get(cursor), Some(b'\r')) && matches!(bytes.get(cursor + 1), Some(b'\n')) {
                cursor += 2;
            } else if matches!(bytes.get(cursor), Some(b'\n')) {
                cursor += 1;
            } else {
                break;
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
        if self.items.len() == self.items.capacity() {
            self.items.reserve(PGN_METADATA_GROW_SIZE);
        }
        self.items.insert(key.to_string(), value.to_string());
    }
    pub fn get(&self, key: &str) -> Option<&str> {
        self.items.get(key).map(String::as_str)
    }
    pub fn delete(&mut self, key: &str) {
        self.items.remove(key);
    }
}
