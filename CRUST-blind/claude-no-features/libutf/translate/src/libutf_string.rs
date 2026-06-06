use crate::libutf_utf::*;

pub struct Utf8String<'a> {
    pub data: &'a [Utf8],
    pub len: usize,
    pub cap: usize,
}

pub struct Utf8StringView<'a> {
    pub data: &'a [Utf8],
    pub len: usize,
}

fn round_up_pow2(len: usize) -> usize {
    if len <= 1 {
        return 1;
    }
    let mut n = len - 1;
    n |= n >> 1;
    n |= n >> 2;
    n |= n >> 4;
    n |= n >> 8;
    n |= n >> 16;
    n |= n >> 32;
    n + 1
}

fn cmp_bytes(a: &[u8], b: &[u8]) -> i32 {
    use std::cmp::Ordering;
    match a.cmp(b) {
        Ordering::Less => -1,
        Ordering::Greater => 1,
        Ordering::Equal => 0,
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
        // Free any existing buffer first.
        free_buffer(self.data, self.cap);
        self.data = &[];
        self.len = 0;
        self.cap = 0;
    }

    pub fn destroy(&mut self) {
        free_buffer(self.data, self.cap);
        self.data = &[];
        self.len = 0;
        self.cap = 0;
    }

    pub fn reserve(&mut self, len: usize) -> Result<(), ()> {
        if len <= self.cap {
            return Ok(());
        }
        let new_cap = round_up_pow2(len);
        let mut new_buf: Vec<Utf8> = vec![0; new_cap];
        if self.len > 0 {
            new_buf[..self.len].copy_from_slice(&self.data[..self.len]);
        }
        let leaked: &'static mut [Utf8] = Box::leak(new_buf.into_boxed_slice());
        free_buffer(self.data, self.cap);
        self.data = leaked;
        self.cap = new_cap;
        Ok(())
    }

    pub fn shrink_to_fit(&mut self) -> Result<(), ()> {
        if self.cap == 0 {
            return Ok(());
        }
        let new_cap = self.len;
        if new_cap == self.cap {
            return Ok(());
        }
        if new_cap == 0 {
            free_buffer(self.data, self.cap);
            self.data = &[];
            self.cap = 0;
            return Ok(());
        }
        let mut new_buf: Vec<Utf8> = vec![0; new_cap];
        new_buf[..self.len].copy_from_slice(&self.data[..self.len]);
        let leaked: &'static mut [Utf8] = Box::leak(new_buf.into_boxed_slice());
        free_buffer(self.data, self.cap);
        self.data = leaked;
        self.cap = new_cap;
        Ok(())
    }

    pub fn clear(&mut self) {
        self.len = 0;
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn append(&mut self, other: &Utf8String) -> Result<(), ()> {
        let other_len = other.len;
        let new_len = self.len + other_len;
        self.reserve(new_len)?;
        // Copy from other into self at offset self.len.
        let other_bytes = unsafe {
            std::slice::from_raw_parts(other.data.as_ptr(), other_len)
        };
        let buf = data_mut(self.data, self.cap);
        buf[self.len..new_len].copy_from_slice(other_bytes);
        self.len = new_len;
        Ok(())
    }

    pub fn append_view(&mut self, view: &Utf8StringView) -> Result<(), ()> {
        let other_len = view.len;
        let new_len = self.len + other_len;
        self.reserve(new_len)?;
        let view_bytes = unsafe {
            std::slice::from_raw_parts(view.data.as_ptr(), other_len)
        };
        let buf = data_mut(self.data, self.cap);
        buf[self.len..new_len].copy_from_slice(view_bytes);
        self.len = new_len;
        Ok(())
    }

    pub fn append_character(&mut self, c: Utf8) -> Result<(), ()> {
        let new_len = self.len + 1;
        self.reserve(new_len)?;
        let buf = data_mut(self.data, self.cap);
        buf[self.len] = c;
        self.len = new_len;
        Ok(())
    }

    pub fn append_literal(&mut self, literal: &[Utf8]) -> Result<(), ()> {
        let n = literal.len();
        let new_len = self.len + n;
        self.reserve(new_len)?;
        let buf = data_mut(self.data, self.cap);
        buf[self.len..new_len].copy_from_slice(literal);
        self.len = new_len;
        Ok(())
    }

    pub fn prepend(&mut self, other: &Utf8String) -> Result<(), ()> {
        let other_len = other.len;
        let new_len = self.len + other_len;
        // Snapshot other.data first because reserve may free overlapping data
        // (but other shouldn't share buf with self in normal usage).
        let other_bytes: Vec<u8> = other.data[..other_len].to_vec();
        self.reserve(new_len)?;
        let buf = data_mut(self.data, self.cap);
        buf.copy_within(0..self.len, other_len);
        buf[..other_len].copy_from_slice(&other_bytes);
        self.len = new_len;
        Ok(())
    }

    pub fn prepend_view(&mut self, view: &Utf8StringView) -> Result<(), ()> {
        let view_len = view.len;
        let new_len = self.len + view_len;
        let view_bytes: Vec<u8> = view.data[..view_len].to_vec();
        self.reserve(new_len)?;
        let buf = data_mut(self.data, self.cap);
        buf.copy_within(0..self.len, view_len);
        buf[..view_len].copy_from_slice(&view_bytes);
        self.len = new_len;
        Ok(())
    }

    pub fn prepend_character(&mut self, c: Utf8) -> Result<(), ()> {
        let new_len = self.len + 1;
        self.reserve(new_len)?;
        let buf = data_mut(self.data, self.cap);
        buf.copy_within(0..self.len, 1);
        buf[0] = c;
        self.len = new_len;
        Ok(())
    }

    pub fn prepend_literal(&mut self, literal: &[Utf8]) -> Result<(), ()> {
        let n = literal.len();
        let new_len = self.len + n;
        let lit_owned: Vec<u8> = literal.to_vec();
        self.reserve(new_len)?;
        let buf = data_mut(self.data, self.cap);
        buf.copy_within(0..self.len, n);
        buf[..n].copy_from_slice(&lit_owned);
        self.len = new_len;
        Ok(())
    }

    pub fn insert(&mut self, pos: usize, other: &Utf8String) -> Result<(), ()> {
        if pos > self.len {
            return Err(());
        }
        let other_len = other.len;
        let inserted_len = self.len + other_len;
        let other_bytes: Vec<u8> = other.data[..other_len].to_vec();
        self.reserve(inserted_len)?;
        let old_len = self.len;
        let buf = data_mut(self.data, self.cap);
        buf.copy_within(pos..old_len, pos + other_len);
        buf[pos..pos + other_len].copy_from_slice(&other_bytes);
        self.len = inserted_len;
        Ok(())
    }

    pub fn insert_view(&mut self, pos: usize, view: &Utf8StringView) -> Result<(), ()> {
        if pos > self.len {
            return Err(());
        }
        let view_len = view.len;
        let inserted_len = self.len + view_len;
        let view_bytes: Vec<u8> = view.data[..view_len].to_vec();
        self.reserve(inserted_len)?;
        let old_len = self.len;
        let buf = data_mut(self.data, self.cap);
        buf.copy_within(pos..old_len, pos + view_len);
        buf[pos..pos + view_len].copy_from_slice(&view_bytes);
        self.len = inserted_len;
        Ok(())
    }

    pub fn insert_character(&mut self, pos: usize, c: Utf8) -> Result<(), ()> {
        if pos > self.len {
            return Err(());
        }
        let inserted_len = self.len + 1;
        self.reserve(inserted_len)?;
        let old_len = self.len;
        let buf = data_mut(self.data, self.cap);
        buf.copy_within(pos..old_len, pos + 1);
        buf[pos] = c;
        self.len = inserted_len;
        Ok(())
    }

    pub fn insert_literal(&mut self, pos: usize, literal: &[Utf8]) -> Result<(), ()> {
        if pos > self.len {
            return Err(());
        }
        let n = literal.len();
        let inserted_len = self.len + n;
        let lit_owned: Vec<u8> = literal.to_vec();
        self.reserve(inserted_len)?;
        let old_len = self.len;
        let buf = data_mut(self.data, self.cap);
        buf.copy_within(pos..old_len, pos + n);
        buf[pos..pos + n].copy_from_slice(&lit_owned);
        self.len = inserted_len;
        Ok(())
    }

    pub fn replace(&mut self, pos: usize, len: usize, replacement: &Utf8String) -> Result<(), ()> {
        if pos > self.len {
            return Err(());
        }
        let mut len = len;
        if pos + len > self.len {
            len = self.len - pos;
        }
        let rep_len = replacement.len;
        let replaced_len = self.len + rep_len - len;
        let rep_bytes: Vec<u8> = replacement.data[..rep_len].to_vec();
        self.reserve(replaced_len)?;
        let old_len = self.len;
        let buf = data_mut(self.data, self.cap);
        // Move tail [pos+len..old_len] to start at pos+rep_len
        buf.copy_within(pos + len..old_len, pos + rep_len);
        buf[pos..pos + rep_len].copy_from_slice(&rep_bytes);
        self.len = replaced_len;
        Ok(())
    }

    pub fn replace_view(
        &mut self,
        pos: usize,
        len: usize,
        replacement: &Utf8StringView,
    ) -> Result<(), ()> {
        if pos > self.len {
            return Err(());
        }
        let mut len = len;
        if pos + len > self.len {
            len = self.len - pos;
        }
        let rep_len = replacement.len;
        let replaced_len = self.len + rep_len - len;
        let rep_bytes: Vec<u8> = replacement.data[..rep_len].to_vec();
        self.reserve(replaced_len)?;
        let old_len = self.len;
        let buf = data_mut(self.data, self.cap);
        buf.copy_within(pos + len..old_len, pos + rep_len);
        buf[pos..pos + rep_len].copy_from_slice(&rep_bytes);
        self.len = replaced_len;
        Ok(())
    }

    pub fn replace_character(&mut self, pos: usize, len: usize, c: Utf8) -> Result<(), ()> {
        if pos > self.len {
            return Err(());
        }
        // Note: C does not bound len here for replace_character.
        let replaced_len = self.len + 1 - len;
        self.reserve(replaced_len)?;
        let old_len = self.len;
        let buf = data_mut(self.data, self.cap);
        buf.copy_within(pos + len..old_len, pos + 1);
        buf[pos] = c;
        self.len = replaced_len;
        Ok(())
    }

    pub fn replace_literal(&mut self, pos: usize, len: usize, literal: &[Utf8]) -> Result<(), ()> {
        if pos > self.len {
            return Err(());
        }
        let mut len = len;
        if pos + len > self.len {
            len = self.len - pos;
        }
        let n = literal.len();
        let replaced_len = self.len + n - len;
        let lit_owned: Vec<u8> = literal.to_vec();
        self.reserve(replaced_len)?;
        let old_len = self.len;
        let buf = data_mut(self.data, self.cap);
        buf.copy_within(pos + len..old_len, pos + n);
        buf[pos..pos + n].copy_from_slice(&lit_owned);
        self.len = replaced_len;
        Ok(())
    }

    pub fn erase(&mut self, pos: usize, len: usize) -> Result<(), ()> {
        if pos > self.len {
            return Err(());
        }
        let mut len = len;
        if pos + len > self.len {
            len = self.len - pos;
        }
        let old_len = self.len;
        let buf = data_mut(self.data, self.cap);
        buf.copy_within(pos + len..old_len, pos);
        self.len -= len;
        Ok(())
    }

    pub fn concat(&self, other: &Utf8String) -> Result<Utf8String, ()> {
        let mut result = Utf8String::new();
        result.reserve(self.len + other.len)?;
        result.append(self)?;
        result.append(other)?;
        Ok(result)
    }

    pub fn concat_view(&self, other: &Utf8StringView) -> Result<Utf8String, ()> {
        let mut result = Utf8String::new();
        result.reserve(self.len + other.len)?;
        result.append(self)?;
        result.append_view(other)?;
        Ok(result)
    }

    pub fn concat_character(&self, c: Utf8) -> Result<Utf8String, ()> {
        let mut result = Utf8String::new();
        result.reserve(self.len + 1)?;
        result.append(self)?;
        result.append_character(c)?;
        Ok(result)
    }

    pub fn concat_literal(&self, literal: &[Utf8]) -> Result<Utf8String, ()> {
        let mut result = Utf8String::new();
        result.reserve(self.len + literal.len())?;
        result.append(self)?;
        result.append_literal(literal)?;
        Ok(result)
    }

    pub fn compare(&self, other: &Utf8String) -> i32 {
        cmp_bytes(&self.data[..self.len], &other.data[..other.len])
    }

    pub fn compare_literal(&self, literal: &[Utf8]) -> i32 {
        cmp_bytes(&self.data[..self.len], literal)
    }

    pub fn substring(&self, start: usize, end: usize) -> Utf8StringView {
        let mut e = end;
        if e == usize::MAX || e > self.len {
            e = self.len;
        }
        let s = if start > e { e } else { start };
        let bytes = &self.data[s..e];
        Utf8StringView {
            data: bytes,
            len: e - s,
        }
    }

    pub fn substring_copy(&self, start: usize, end: usize) -> Result<Utf8String, ()> {
        let mut e = end;
        if e == usize::MAX || e > self.len {
            e = self.len;
        }
        let s = if start > e { e } else { start };
        let len = e - s;
        let mut result = Utf8String::new();
        result.reserve(len)?;
        if len > 0 {
            let src: Vec<u8> = self.data[s..e].to_vec();
            let buf = data_mut(result.data, result.cap);
            buf[..len].copy_from_slice(&src);
        }
        result.len = len;
        Ok(result)
    }

    pub fn index_of_character(&self, pos: usize, c: Utf8) -> Option<usize> {
        let n = self.len;
        let mut i = pos;
        while i < n {
            if self.data[i] == c {
                return Some(i);
            }
            i += 1;
        }
        None
    }

    pub fn last_index_of_character(&self, pos: usize, c: Utf8) -> Option<usize> {
        let mut p = pos;
        if p == usize::MAX {
            if self.len == 0 {
                return None;
            }
            p = self.len - 1;
        } else if p >= self.len {
            return None;
        }
        let mut i = p;
        loop {
            if self.data[i] == c {
                return Some(i);
            }
            if i == 0 {
                return None;
            }
            i -= 1;
        }
    }
}

impl<'a> Utf8StringView<'a> {
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn compare(&self, other: &Utf8StringView) -> i32 {
        cmp_bytes(&self.data[..self.len], &other.data[..other.len])
    }

    pub fn compare_literal(&self, literal: &[Utf8]) -> i32 {
        cmp_bytes(&self.data[..self.len], literal)
    }

    pub fn substring(&self, start: usize, end: usize) -> Utf8StringView<'a> {
        let mut e = end;
        if e == usize::MAX || e > self.len {
            e = self.len;
        }
        let s = if start > e { e } else { start };
        Utf8StringView {
            data: &self.data[s..e],
            len: e - s,
        }
    }

    pub fn substring_copy(&self, start: usize, end: usize) -> Result<Utf8String, ()> {
        let mut e = end;
        if e == usize::MAX || e > self.len {
            e = self.len;
        }
        let s = if start > e { e } else { start };
        let len = e - s;
        let mut result = Utf8String::new();
        result.reserve(len)?;
        if len > 0 {
            let src: Vec<u8> = self.data[s..e].to_vec();
            let buf = data_mut(result.data, result.cap);
            buf[..len].copy_from_slice(&src);
        }
        result.len = len;
        Ok(result)
    }

    pub fn index_of_character(&self, pos: usize, c: Utf8) -> Option<usize> {
        let n = self.len;
        let mut i = pos;
        while i < n {
            if self.data[i] == c {
                return Some(i);
            }
            i += 1;
        }
        None
    }

    pub fn last_index_of_character(&self, pos: usize, c: Utf8) -> Option<usize> {
        let mut p = pos;
        if p == usize::MAX {
            if self.len == 0 {
                return None;
            }
            p = self.len - 1;
        } else if p >= self.len {
            return None;
        }
        let mut i = p;
        loop {
            if self.data[i] == c {
                return Some(i);
            }
            if i == 0 {
                return None;
            }
            i -= 1;
        }
    }
}

// Helpers for managing the heap-backed buffer that the &[Utf8] field points
// into. The struct logically owns the buffer, but Rust borrow tracking can't
// express that through an immutable slice reference, so we use Box::leak/
// Box::from_raw pairs to manage the allocation explicitly.

fn data_mut<'b>(data: &[Utf8], cap: usize) -> &'b mut [Utf8] {
    if cap == 0 {
        return &mut [];
    }
    unsafe {
        let ptr = data.as_ptr() as *mut Utf8;
        std::slice::from_raw_parts_mut(ptr, cap)
    }
}

fn free_buffer(data: &[Utf8], cap: usize) {
    if cap == 0 {
        return;
    }
    if data.is_empty() {
        return;
    }
    unsafe {
        let ptr = data.as_ptr() as *mut Utf8;
        let slice = std::slice::from_raw_parts_mut(ptr, cap);
        let _ = Box::from_raw(slice as *mut [Utf8]);
    }
}
