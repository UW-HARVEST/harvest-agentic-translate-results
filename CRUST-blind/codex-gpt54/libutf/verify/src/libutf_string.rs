use crate::libutf_utf::*;

const INLINE_CAPACITY: usize = 8;
static EMPTY_BYTES: [u8; 0] = [];

fn next_capacity(len: usize) -> Option<usize> {
    if len <= INLINE_CAPACITY {
        return Some(INLINE_CAPACITY);
    }

    let rounded = len.checked_next_power_of_two()?;
    Some(rounded)
}

fn leaked_slice(bytes: Vec<u8>) -> &'static [u8] {
    if bytes.is_empty() {
        &EMPTY_BYTES
    } else {
        Box::leak(bytes.into_boxed_slice())
    }
}

fn clamp_range(start: usize, end: usize, len: usize) -> (usize, usize) {
    let end = if end == usize::MAX || end > len { len } else { end };
    let start = start.min(end);
    (start, end)
}

fn strncmp_like(a: &[u8], b: &[u8], n: usize) -> i32 {
    for i in 0..n {
        let ac = a[i];
        let bc = b[i];
        if ac != bc {
            return i32::from(ac) - i32::from(bc);
        }
        if ac == 0 {
            return 0;
        }
    }
    0
}

fn compare_bytes(a: &[u8], b: &[u8]) -> i32 {
    let result = strncmp_like(a, b, a.len().min(b.len()));
    if result == 0 {
        if a.len() < b.len() {
            -1
        } else if a.len() > b.len() {
            1
        } else {
            0
        }
    } else {
        result
    }
}

fn new_string_from_bytes(bytes: &[u8]) -> Result<Utf8String<'static>, ()> {
    let mut string = Utf8String::new();
    string.reserve(bytes.len())?;
    string.data = leaked_slice(bytes.to_vec());
    string.len = bytes.len();
    Ok(string)
}

pub struct Utf8String<'a> {
    pub data: &'a [Utf8],
    pub len: usize,
    pub cap: usize,
}

pub struct Utf8StringView<'a> {
    pub data: &'a [Utf8],
    pub len: usize,
}

impl Utf8String<'_> {
    fn bytes(&self) -> &[u8] {
        &self.data[..self.len.min(self.data.len())]
    }

    fn replace_bytes(&mut self, bytes: Vec<u8>) -> Result<(), ()> {
        self.reserve(bytes.len())?;
        self.data = leaked_slice(bytes);
        self.len = self.data.len();
        Ok(())
    }

    pub fn new() -> Self {
        Self {
            data: &EMPTY_BYTES,
            len: 0,
            cap: INLINE_CAPACITY,
        }
    }

    pub fn init(&mut self) {
        *self = Self::new();
    }

    pub fn destroy(&mut self) {
        self.init();
    }

    pub fn reserve(&mut self, len: usize) -> Result<(), ()> {
        if len <= self.cap {
            return Ok(());
        }

        self.cap = next_capacity(len).ok_or(())?;
        Ok(())
    }

    pub fn shrink_to_fit(&mut self) -> Result<(), ()> {
        self.cap = if self.len <= INLINE_CAPACITY {
            INLINE_CAPACITY
        } else {
            self.len
        };
        Ok(())
    }

    pub fn clear(&mut self) {
        self.len = 0;
        self.data = &EMPTY_BYTES;
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn append(&mut self, other: &Utf8String) -> Result<(), ()> {
        let mut bytes = self.bytes().to_vec();
        bytes.extend_from_slice(other.bytes());
        self.replace_bytes(bytes)
    }

    pub fn append_view(&mut self, view: &Utf8StringView) -> Result<(), ()> {
        let mut bytes = self.bytes().to_vec();
        bytes.extend_from_slice(&view.data[..view.len.min(view.data.len())]);
        self.replace_bytes(bytes)
    }

    pub fn append_character(&mut self, c: Utf8) -> Result<(), ()> {
        let mut bytes = self.bytes().to_vec();
        bytes.push(c);
        self.replace_bytes(bytes)
    }

    pub fn append_literal(&mut self, literal: &[Utf8]) -> Result<(), ()> {
        let mut bytes = self.bytes().to_vec();
        bytes.extend_from_slice(literal);
        self.replace_bytes(bytes)
    }

    pub fn prepend(&mut self, other: &Utf8String) -> Result<(), ()> {
        let mut bytes = other.bytes().to_vec();
        bytes.extend_from_slice(self.bytes());
        self.replace_bytes(bytes)
    }

    pub fn prepend_view(&mut self, view: &Utf8StringView) -> Result<(), ()> {
        let mut bytes = view.data[..view.len.min(view.data.len())].to_vec();
        bytes.extend_from_slice(self.bytes());
        self.replace_bytes(bytes)
    }

    pub fn prepend_character(&mut self, c: Utf8) -> Result<(), ()> {
        let mut bytes = Vec::with_capacity(self.len + 1);
        bytes.push(c);
        bytes.extend_from_slice(self.bytes());
        self.replace_bytes(bytes)
    }

    pub fn prepend_literal(&mut self, literal: &[Utf8]) -> Result<(), ()> {
        let mut bytes = literal.to_vec();
        bytes.extend_from_slice(self.bytes());
        self.replace_bytes(bytes)
    }

    pub fn insert(&mut self, pos: usize, other: &Utf8String) -> Result<(), ()> {
        self.insert_literal(pos, other.bytes())
    }

    pub fn insert_view(&mut self, pos: usize, view: &Utf8StringView) -> Result<(), ()> {
        self.insert_literal(pos, &view.data[..view.len.min(view.data.len())])
    }

    pub fn insert_character(&mut self, pos: usize, c: Utf8) -> Result<(), ()> {
        self.insert_literal(pos, &[c])
    }

    pub fn insert_literal(&mut self, pos: usize, literal: &[Utf8]) -> Result<(), ()> {
        if pos > self.len {
            return Err(());
        }

        let current = self.bytes();
        let mut bytes = Vec::with_capacity(current.len() + literal.len());
        bytes.extend_from_slice(&current[..pos]);
        bytes.extend_from_slice(literal);
        bytes.extend_from_slice(&current[pos..]);
        self.replace_bytes(bytes)
    }

    pub fn replace(&mut self, pos: usize, len: usize, replacement: &Utf8String) -> Result<(), ()> {
        self.replace_literal(pos, len, replacement.bytes())
    }

    pub fn replace_view(
        &mut self,
        pos: usize,
        len: usize,
        replacement: &Utf8StringView,
    ) -> Result<(), ()> {
        self.replace_literal(pos, len, &replacement.data[..replacement.len.min(replacement.data.len())])
    }

    pub fn replace_character(&mut self, pos: usize, len: usize, c: Utf8) -> Result<(), ()> {
        self.replace_literal(pos, len, &[c])
    }

    pub fn replace_literal(&mut self, pos: usize, len: usize, literal: &[Utf8]) -> Result<(), ()> {
        if pos > self.len {
            return Err(());
        }

        let current = self.bytes();
        let actual_len = len.min(self.len - pos);
        let mut bytes = Vec::with_capacity(current.len() + literal.len().saturating_sub(actual_len));
        bytes.extend_from_slice(&current[..pos]);
        bytes.extend_from_slice(literal);
        bytes.extend_from_slice(&current[pos + actual_len..]);
        self.replace_bytes(bytes)
    }

    pub fn erase(&mut self, pos: usize, len: usize) -> Result<(), ()> {
        self.replace_literal(pos, len, &[])
    }

    pub fn concat(&self, other: &Utf8String) -> Result<Utf8String, ()> {
        let mut bytes = self.bytes().to_vec();
        bytes.extend_from_slice(other.bytes());
        new_string_from_bytes(&bytes)
    }

    pub fn concat_view(&self, other: &Utf8StringView) -> Result<Utf8String, ()> {
        let mut bytes = self.bytes().to_vec();
        bytes.extend_from_slice(&other.data[..other.len.min(other.data.len())]);
        new_string_from_bytes(&bytes)
    }

    pub fn concat_character(&self, c: Utf8) -> Result<Utf8String, ()> {
        let mut bytes = self.bytes().to_vec();
        bytes.push(c);
        new_string_from_bytes(&bytes)
    }

    pub fn concat_literal(&self, literal: &[Utf8]) -> Result<Utf8String, ()> {
        let mut bytes = self.bytes().to_vec();
        bytes.extend_from_slice(literal);
        new_string_from_bytes(&bytes)
    }

    pub fn compare(&self, other: &Utf8String) -> i32 {
        compare_bytes(self.bytes(), other.bytes())
    }

    pub fn compare_literal(&self, literal: &[Utf8]) -> i32 {
        compare_bytes(self.bytes(), literal)
    }

    pub fn substring(&self, start: usize, end: usize) -> Utf8StringView {
        let (start, end) = clamp_range(start, end, self.len);
        Utf8StringView {
            data: &self.data[start..end],
            len: end - start,
        }
    }

    pub fn substring_copy(&self, start: usize, end: usize) -> Result<Utf8String, ()> {
        let view = self.substring(start, end);
        new_string_from_bytes(&view.data[..view.len])
    }

    pub fn index_of_character(&self, pos: usize, c: Utf8) -> Option<usize> {
        self.bytes()
            .iter()
            .enumerate()
            .skip(pos)
            .find_map(|(idx, &byte)| (byte == c).then_some(idx))
    }

    pub fn last_index_of_character(&self, pos: usize, c: Utf8) -> Option<usize> {
        if self.len == 0 {
            return None;
        }

        let start = if pos == usize::MAX {
            self.len - 1
        } else if pos >= self.len {
            return None;
        } else {
            pos
        };

        (0..=start).rev().find(|&idx| self.bytes()[idx] == c)
    }
}

impl<'a> Utf8StringView<'a> {
    fn bytes(&self) -> &[u8] {
        &self.data[..self.len.min(self.data.len())]
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn compare(&self, other: &Utf8StringView) -> i32 {
        compare_bytes(self.bytes(), other.bytes())
    }

    pub fn compare_literal(&self, literal: &[Utf8]) -> i32 {
        compare_bytes(self.bytes(), literal)
    }

    pub fn substring(&self, start: usize, end: usize) -> Utf8StringView<'a> {
        let (start, end) = clamp_range(start, end, self.len);
        Utf8StringView {
            data: &self.data[start..end],
            len: end - start,
        }
    }

    pub fn substring_copy(&self, start: usize, end: usize) -> Result<Utf8String, ()> {
        let view = self.substring(start, end);
        new_string_from_bytes(&view.data[..view.len])
    }

    pub fn index_of_character(&self, pos: usize, c: Utf8) -> Option<usize> {
        self.bytes()
            .iter()
            .enumerate()
            .skip(pos)
            .find_map(|(idx, &byte)| (byte == c).then_some(idx))
    }

    pub fn last_index_of_character(&self, pos: usize, c: Utf8) -> Option<usize> {
        if self.len == 0 {
            return None;
        }

        let start = if pos == usize::MAX {
            self.len - 1
        } else if pos >= self.len {
            return None;
        } else {
            pos
        };

        (0..=start).rev().find(|&idx| self.bytes()[idx] == c)
    }
}
