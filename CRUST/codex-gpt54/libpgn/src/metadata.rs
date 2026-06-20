use crate::utils::buffer::PgnBuffer;
const PGN_METADATA_INITIAL_SIZE: usize = 8;
const PGN_METADATA_GROW_SIZE: usize = 8;

fn skip_newline(s: &str, cursor: &mut usize) -> bool {
    let bytes = s.as_bytes();
    if bytes.get(*cursor) == Some(&b'\r') {
        assert_eq!(bytes.get(*cursor), Some(&b'\r'));
        *cursor += 1;
        assert_eq!(bytes.get(*cursor), Some(&b'\n'));
        *cursor += 1;
        return true;
    }

    assert_eq!(bytes.get(*cursor), Some(&b'\n'));
    *cursor += 1;
    true
}

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
        let mut cursor = 0usize;

        if bytes.get(cursor) != Some(&b'[') {
            return Self::new();
        }

        let mut metadata = Self::new();
        let mut key_buffer = PgnBuffer::new();
        let mut value_buffer = PgnBuffer::new();

        loop {
            if bytes.get(cursor) != Some(&b'[') {
                break;
            }
            cursor += 1;

            while bytes.get(cursor) != Some(&b' ') {
                key_buffer.append(bytes[cursor] as char);
                cursor += 1;
            }

            assert_eq!(bytes.get(cursor + 1), Some(&b'"'));
            cursor += 2;

            while bytes.get(cursor) != Some(&b'"') {
                value_buffer.append(bytes[cursor] as char);
                cursor += 1;
            }

            metadata.insert(&key_buffer.clone().detach(), &value_buffer.clone().detach());
            key_buffer.reset();
            value_buffer.reset();

            cursor += 1;
            assert_eq!(bytes.get(cursor), Some(&b']'));
            cursor += 1;

            if bytes.get(cursor).is_some_and(|b| *b == b'\n' || *b == b'\r') {
                skip_newline(s, &mut cursor);
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
        if self.items.len() >= self.items.capacity() {
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
