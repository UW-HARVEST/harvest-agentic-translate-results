use crate::libutf_utf::*;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

pub struct Utf8String<'a> {
    pub data: &'a [Utf8],
    pub len: usize,
    pub cap: usize,
}

pub struct Utf8StringView<'a> {
    pub data: &'a [Utf8],
    pub len: usize,
}

fn buffers() -> &'static Mutex<HashMap<usize, Box<[u8]>>> {
    static BUFFERS: OnceLock<Mutex<HashMap<usize, Box<[u8]>>>> = OnceLock::new();
    BUFFERS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn round_capacity(mut len: usize) -> usize {
    if len <= 8 {
        return 8;
    }

    len -= 1;
    len |= len >> 1;
    len |= len >> 2;
    len |= len >> 4;
    len |= len >> 8;
    len |= len >> 16;
    if usize::BITS == 64 {
        len |= len >> 32;
    }
    len + 1
}

fn string_bytes<'a>(string: &'a Utf8String<'a>) -> &'a [u8] {
    &string.data[..string.len.min(string.data.len())]
}

fn view_bytes<'a>(view: &'a Utf8StringView<'a>) -> &'a [u8] {
    &view.data[..view.len.min(view.data.len())]
}

fn current_capacity(string: &Utf8String<'_>) -> usize {
    string.cap
}

fn release_buffer(string: &Utf8String<'_>) {
    if string.cap == 0 || string.data.is_empty() {
        return;
    }
    let key = string.data.as_ptr() as usize;
    buffers().lock().expect("buffer mutex poisoned").remove(&key);
}

fn update_storage<'a>(
    string: &mut Utf8String<'a>,
    bytes: &[u8],
    capacity: usize,
) -> Result<(), ()> {
    let capacity = capacity.max(bytes.len());
    let mut buffer = vec![0; capacity].into_boxed_slice();
    buffer[..bytes.len()].copy_from_slice(bytes);
    let key = buffer.as_ptr() as usize;

    release_buffer(string);

    let slice = {
        let mut guard = buffers().lock().expect("buffer mutex poisoned");
        guard.insert(key, buffer);
        let stored = guard.get(&key).expect("stored buffer missing");
        // The buffer remains owned by the global map until the next reallocation or destroy.
        unsafe { std::mem::transmute::<&[u8], &'a [u8]>(&stored[..capacity]) }
    };

    string.data = slice;
    string.len = bytes.len();
    string.cap = capacity;
    Ok(())
}

fn ensure_capacity<'a>(string: &mut Utf8String<'a>, len: usize) -> Result<(), ()> {
    if len <= current_capacity(string) {
        return Ok(());
    }

    let bytes = string_bytes(string).to_vec();
    update_storage(string, &bytes, round_capacity(len))
}

fn clamp_range(len: usize, start: usize, end: usize) -> (usize, usize) {
    let end = if end == usize::MAX || end > len { len } else { end };
    let start = start.min(end);
    (start, end)
}

fn strncmp_like(a: &[u8], b: &[u8], n: usize) -> i32 {
    let mut i = 0;
    while i < n {
        let av = a[i];
        let bv = b[i];
        if av != bv {
            return (av as i32) - (bv as i32);
        }
        if av == 0 {
            return 0;
        }
        i += 1;
    }
    0
}

fn compare_bytes(a: &[u8], b: &[u8]) -> i32 {
    let result = strncmp_like(a, b, a.len().min(b.len()));
    if result == 0 {
        match a.len().cmp(&b.len()) {
            Ordering::Less => -1,
            Ordering::Equal => 0,
            Ordering::Greater => 1,
        }
    } else {
        result
    }
}

impl Utf8String<'_> {
    pub fn new() -> Self {
        Self {
            data: &[],
            len: 0,
            cap: 0,
        }
    }

    pub fn init(&mut self) {
        self.destroy();
        self.data = &[];
        self.len = 0;
        self.cap = 0;
    }

    pub fn destroy(&mut self) {
        release_buffer(self);
        self.data = &[];
        self.len = 0;
        self.cap = 0;
    }

    pub fn reserve(&mut self, len: usize) -> Result<(), ()> {
        ensure_capacity(self, len)
    }

    pub fn shrink_to_fit(&mut self) -> Result<(), ()> {
        if self.cap == 0 {
            return Ok(());
        }
        if self.len == 0 {
            self.destroy();
            return Ok(());
        }
        let bytes = string_bytes(self).to_vec();
        update_storage(self, &bytes, self.len)
    }

    pub fn clear(&mut self) {
        self.len = 0;
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn append(&mut self, other: &Utf8String) -> Result<(), ()> {
        self.append_literal(string_bytes(other))
    }

    pub fn append_view(&mut self, view: &Utf8StringView) -> Result<(), ()> {
        self.append_literal(view_bytes(view))
    }

    pub fn append_character(&mut self, c: Utf8) -> Result<(), ()> {
        let new_len = self.len.checked_add(1).ok_or(())?;
        let cap = current_capacity(self).max(round_capacity(new_len));
        let mut bytes = string_bytes(self).to_vec();
        bytes.push(c);
        update_storage(self, &bytes, cap)
    }

    pub fn append_literal(&mut self, literal: &[Utf8]) -> Result<(), ()> {
        let new_len = self.len.checked_add(literal.len()).ok_or(())?;
        let cap = current_capacity(self).max(round_capacity(new_len));
        let mut bytes = string_bytes(self).to_vec();
        bytes.extend_from_slice(literal);
        update_storage(self, &bytes, cap)
    }

    pub fn prepend(&mut self, other: &Utf8String) -> Result<(), ()> {
        self.prepend_literal(string_bytes(other))
    }

    pub fn prepend_view(&mut self, view: &Utf8StringView) -> Result<(), ()> {
        self.prepend_literal(view_bytes(view))
    }

    pub fn prepend_character(&mut self, c: Utf8) -> Result<(), ()> {
        let new_len = self.len.checked_add(1).ok_or(())?;
        let cap = current_capacity(self).max(round_capacity(new_len));
        let mut bytes = Vec::with_capacity(new_len);
        bytes.push(c);
        bytes.extend_from_slice(string_bytes(self));
        update_storage(self, &bytes, cap)
    }

    pub fn prepend_literal(&mut self, literal: &[Utf8]) -> Result<(), ()> {
        let new_len = self.len.checked_add(literal.len()).ok_or(())?;
        let cap = current_capacity(self).max(round_capacity(new_len));
        let mut bytes = Vec::with_capacity(new_len);
        bytes.extend_from_slice(literal);
        bytes.extend_from_slice(string_bytes(self));
        update_storage(self, &bytes, cap)
    }

    pub fn insert(&mut self, pos: usize, other: &Utf8String) -> Result<(), ()> {
        self.insert_literal(pos, string_bytes(other))
    }

    pub fn insert_view(&mut self, pos: usize, view: &Utf8StringView) -> Result<(), ()> {
        self.insert_literal(pos, view_bytes(view))
    }

    pub fn insert_character(&mut self, pos: usize, c: Utf8) -> Result<(), ()> {
        if pos > self.len {
            return Err(());
        }
        let new_len = self.len.checked_add(1).ok_or(())?;
        let cap = current_capacity(self).max(round_capacity(new_len));
        let current = string_bytes(self);
        let mut bytes = Vec::with_capacity(new_len);
        bytes.extend_from_slice(&current[..pos]);
        bytes.push(c);
        bytes.extend_from_slice(&current[pos..]);
        update_storage(self, &bytes, cap)
    }

    pub fn insert_literal(&mut self, pos: usize, literal: &[Utf8]) -> Result<(), ()> {
        if pos > self.len {
            return Err(());
        }
        let new_len = self.len.checked_add(literal.len()).ok_or(())?;
        let cap = current_capacity(self).max(round_capacity(new_len));
        let current = string_bytes(self);
        let mut bytes = Vec::with_capacity(new_len);
        bytes.extend_from_slice(&current[..pos]);
        bytes.extend_from_slice(literal);
        bytes.extend_from_slice(&current[pos..]);
        update_storage(self, &bytes, cap)
    }

    pub fn replace(&mut self, pos: usize, len: usize, replacement: &Utf8String) -> Result<(), ()> {
        self.replace_literal(pos, len, string_bytes(replacement))
    }

    pub fn replace_view(
        &mut self,
        pos: usize,
        len: usize,
        replacement: &Utf8StringView,
    ) -> Result<(), ()> {
        self.replace_literal(pos, len, view_bytes(replacement))
    }

    pub fn replace_character(&mut self, pos: usize, len: usize, c: Utf8) -> Result<(), ()> {
        if pos > self.len {
            return Err(());
        }
        let replaced = len.min(self.len.saturating_sub(pos));
        let new_len = self.len + 1 - replaced;
        let cap = current_capacity(self).max(round_capacity(new_len));
        let current = string_bytes(self);
        let mut bytes = Vec::with_capacity(new_len);
        bytes.extend_from_slice(&current[..pos]);
        bytes.push(c);
        bytes.extend_from_slice(&current[pos + replaced..]);
        update_storage(self, &bytes, cap)
    }

    pub fn replace_literal(&mut self, pos: usize, len: usize, literal: &[Utf8]) -> Result<(), ()> {
        if pos > self.len {
            return Err(());
        }
        let replaced = len.min(self.len - pos);
        let new_len = self
            .len
            .checked_add(literal.len())
            .and_then(|v| v.checked_sub(replaced))
            .ok_or(())?;
        let cap = current_capacity(self).max(round_capacity(new_len));
        let current = string_bytes(self);
        let mut bytes = Vec::with_capacity(new_len);
        bytes.extend_from_slice(&current[..pos]);
        bytes.extend_from_slice(literal);
        bytes.extend_from_slice(&current[pos + replaced..]);
        update_storage(self, &bytes, cap)
    }

    pub fn erase(&mut self, pos: usize, len: usize) -> Result<(), ()> {
        if pos > self.len {
            return Err(());
        }
        let erased = len.min(self.len - pos);
        let current = string_bytes(self);
        let mut bytes = Vec::with_capacity(self.len - erased);
        bytes.extend_from_slice(&current[..pos]);
        bytes.extend_from_slice(&current[pos + erased..]);
        update_storage(self, &bytes, current_capacity(self))
    }

    pub fn concat(&self, other: &Utf8String) -> Result<Utf8String, ()> {
        self.concat_literal(string_bytes(other))
    }

    pub fn concat_view(&self, other: &Utf8StringView) -> Result<Utf8String, ()> {
        self.concat_literal(view_bytes(other))
    }

    pub fn concat_character(&self, c: Utf8) -> Result<Utf8String, ()> {
        let mut result = Utf8String::new();
        result.append_literal(string_bytes(self))?;
        result.append_character(c)?;
        Ok(result)
    }

    pub fn concat_literal(&self, literal: &[Utf8]) -> Result<Utf8String, ()> {
        let mut result = Utf8String::new();
        result.append_literal(string_bytes(self))?;
        result.append_literal(literal)?;
        Ok(result)
    }

    pub fn compare(&self, other: &Utf8String) -> i32 {
        compare_bytes(string_bytes(self), string_bytes(other))
    }

    pub fn compare_literal(&self, literal: &[Utf8]) -> i32 {
        compare_bytes(string_bytes(self), literal)
    }

    pub fn substring(&self, start: usize, end: usize) -> Utf8StringView {
        let bytes = string_bytes(self);
        let (start, end) = clamp_range(bytes.len(), start, end);
        Utf8StringView {
            data: &bytes[start..end],
            len: end - start,
        }
    }

    pub fn substring_copy(&self, start: usize, end: usize) -> Result<Utf8String, ()> {
        let view = self.substring(start, end);
        let mut result = Utf8String::new();
        result.append_view(&view)?;
        Ok(result)
    }

    pub fn index_of_character(&self, pos: usize, c: Utf8) -> Option<usize> {
        let bytes = string_bytes(self);
        (pos..bytes.len()).find(|&i| bytes[i] == c)
    }

    pub fn last_index_of_character(&self, pos: usize, c: Utf8) -> Option<usize> {
        let bytes = string_bytes(self);
        if bytes.is_empty() {
            return None;
        }
        let start = if pos == usize::MAX {
            bytes.len() - 1
        } else if pos >= bytes.len() {
            return None;
        } else {
            pos
        };
        (0..=start).rev().find(|&i| bytes[i] == c)
    }
}

impl<'a> Utf8StringView<'a> {
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn compare(&self, other: &Utf8StringView) -> i32 {
        compare_bytes(view_bytes(self), view_bytes(other))
    }

    pub fn compare_literal(&self, literal: &[Utf8]) -> i32 {
        compare_bytes(view_bytes(self), literal)
    }

    pub fn substring(&self, start: usize, end: usize) -> Utf8StringView<'a> {
        let bytes = &self.data[..self.len.min(self.data.len())];
        let (start, end) = clamp_range(bytes.len(), start, end);
        Utf8StringView {
            data: &bytes[start..end],
            len: end - start,
        }
    }

    pub fn substring_copy(&self, start: usize, end: usize) -> Result<Utf8String, ()> {
        let view = self.substring(start, end);
        let mut result = Utf8String::new();
        result.append_view(&view)?;
        Ok(result)
    }

    pub fn index_of_character(&self, pos: usize, c: Utf8) -> Option<usize> {
        let bytes = view_bytes(self);
        (pos..bytes.len()).find(|&i| bytes[i] == c)
    }

    pub fn last_index_of_character(&self, pos: usize, c: Utf8) -> Option<usize> {
        let bytes = view_bytes(self);
        if bytes.is_empty() {
            return None;
        }
        let start = if pos == usize::MAX {
            bytes.len() - 1
        } else if pos >= bytes.len() {
            return None;
        } else {
            pos
        };
        (0..=start).rev().find(|&i| bytes[i] == c)
    }
}
