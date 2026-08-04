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

// --- Internal helpers ---------------------------------------------------

fn round_up_pow2(mut n: usize) -> usize {
    if n == 0 {
        return 0;
    }
    n -= 1;
    n |= n >> 1;
    n |= n >> 2;
    n |= n >> 4;
    n |= n >> 8;
    n |= n >> 16;
    if usize::BITS >= 64 {
        n |= n >> 32;
    }
    n + 1
}

fn alloc_buffer(cap: usize) -> &'static mut [u8] {
    let v: Vec<u8> = vec![0u8; cap];
    Box::leak(v.into_boxed_slice())
}

/// SAFETY: `slice` must have been produced by `alloc_buffer` and not yet freed.
unsafe fn free_buffer(ptr: *const u8, cap: usize) {
    if cap == 0 {
        return;
    }
    let raw = std::ptr::slice_from_raw_parts_mut(ptr as *mut u8, cap);
    let _ = Box::from_raw(raw);
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
    }

    pub fn destroy(&mut self) {
        if self.cap > 0 {
            unsafe {
                free_buffer(self.data.as_ptr(), self.cap);
            }
        }
        self.data = &[];
        self.len = 0;
        self.cap = 0;
    }

    pub fn reserve(&mut self, len: usize) -> Result<(), ()> {
        if len <= self.cap {
            return Ok(());
        }
        let new_cap = round_up_pow2(len);
        let leaked = alloc_buffer(new_cap);
        let old_len = self.len;
        // Copy existing content
        leaked[..old_len].copy_from_slice(&self.data[..old_len]);
        let old_ptr = self.data.as_ptr();
        let old_cap = self.cap;
        self.data = leaked;
        self.cap = new_cap;
        self.len = old_len;
        if old_cap > 0 {
            unsafe {
                free_buffer(old_ptr, old_cap);
            }
        }
        Ok(())
    }

    pub fn shrink_to_fit(&mut self) -> Result<(), ()> {
        if self.cap == 0 || self.cap == self.len {
            return Ok(());
        }
        if self.len == 0 {
            unsafe {
                free_buffer(self.data.as_ptr(), self.cap);
            }
            self.data = &[];
            self.cap = 0;
            return Ok(());
        }
        let new_cap = self.len;
        let leaked = alloc_buffer(new_cap);
        leaked.copy_from_slice(&self.data[..self.len]);
        let old_ptr = self.data.as_ptr();
        let old_cap = self.cap;
        let old_len = self.len;
        self.data = leaked;
        self.cap = new_cap;
        self.len = old_len;
        unsafe {
            free_buffer(old_ptr, old_cap);
        }
        Ok(())
    }

    pub fn clear(&mut self) {
        self.len = 0;
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    fn replace_with(&mut self, new_content: &[u8]) -> Result<(), ()> {
        let new_len = new_content.len();
        if new_len <= self.cap {
            // Reuse existing buffer
            let ptr = self.data.as_ptr() as *mut u8;
            let buf = unsafe { std::slice::from_raw_parts_mut(ptr, self.cap) };
            buf[..new_len].copy_from_slice(new_content);
            self.len = new_len;
            // Re-establish slice reference (still same data)
            self.data = unsafe { std::slice::from_raw_parts(ptr as *const u8, self.cap) };
            return Ok(());
        }
        // Need to allocate new buffer
        let new_cap = round_up_pow2(new_len);
        let leaked = alloc_buffer(new_cap);
        leaked[..new_len].copy_from_slice(new_content);
        let old_ptr = self.data.as_ptr();
        let old_cap = self.cap;
        self.data = leaked;
        self.cap = new_cap;
        self.len = new_len;
        if old_cap > 0 {
            unsafe {
                free_buffer(old_ptr, old_cap);
            }
        }
        Ok(())
    }

    pub fn append(&mut self, other: &Utf8String) -> Result<(), ()> {
        let mut v = Vec::with_capacity(self.len + other.len);
        v.extend_from_slice(&self.data[..self.len]);
        v.extend_from_slice(&other.data[..other.len]);
        self.replace_with(&v)
    }

    pub fn append_view(&mut self, view: &Utf8StringView) -> Result<(), ()> {
        let mut v = Vec::with_capacity(self.len + view.len);
        v.extend_from_slice(&self.data[..self.len]);
        v.extend_from_slice(&view.data[..view.len]);
        self.replace_with(&v)
    }

    pub fn append_character(&mut self, c: Utf8) -> Result<(), ()> {
        let mut v = Vec::with_capacity(self.len + 1);
        v.extend_from_slice(&self.data[..self.len]);
        v.push(c);
        self.replace_with(&v)
    }

    pub fn append_literal(&mut self, literal: &[Utf8]) -> Result<(), ()> {
        let mut v = Vec::with_capacity(self.len + literal.len());
        v.extend_from_slice(&self.data[..self.len]);
        v.extend_from_slice(literal);
        self.replace_with(&v)
    }

    pub fn prepend(&mut self, other: &Utf8String) -> Result<(), ()> {
        let mut v = Vec::with_capacity(self.len + other.len);
        v.extend_from_slice(&other.data[..other.len]);
        v.extend_from_slice(&self.data[..self.len]);
        self.replace_with(&v)
    }

    pub fn prepend_view(&mut self, view: &Utf8StringView) -> Result<(), ()> {
        let mut v = Vec::with_capacity(self.len + view.len);
        v.extend_from_slice(&view.data[..view.len]);
        v.extend_from_slice(&self.data[..self.len]);
        self.replace_with(&v)
    }

    pub fn prepend_character(&mut self, c: Utf8) -> Result<(), ()> {
        let mut v = Vec::with_capacity(self.len + 1);
        v.push(c);
        v.extend_from_slice(&self.data[..self.len]);
        self.replace_with(&v)
    }

    pub fn prepend_literal(&mut self, literal: &[Utf8]) -> Result<(), ()> {
        let mut v = Vec::with_capacity(self.len + literal.len());
        v.extend_from_slice(literal);
        v.extend_from_slice(&self.data[..self.len]);
        self.replace_with(&v)
    }

    pub fn insert(&mut self, pos: usize, other: &Utf8String) -> Result<(), ()> {
        if pos > self.len {
            return Err(());
        }
        let mut v = Vec::with_capacity(self.len + other.len);
        v.extend_from_slice(&self.data[..pos]);
        v.extend_from_slice(&other.data[..other.len]);
        v.extend_from_slice(&self.data[pos..self.len]);
        self.replace_with(&v)
    }

    pub fn insert_view(&mut self, pos: usize, view: &Utf8StringView) -> Result<(), ()> {
        if pos > self.len {
            return Err(());
        }
        let mut v = Vec::with_capacity(self.len + view.len);
        v.extend_from_slice(&self.data[..pos]);
        v.extend_from_slice(&view.data[..view.len]);
        v.extend_from_slice(&self.data[pos..self.len]);
        self.replace_with(&v)
    }

    pub fn insert_character(&mut self, pos: usize, c: Utf8) -> Result<(), ()> {
        if pos > self.len {
            return Err(());
        }
        let mut v = Vec::with_capacity(self.len + 1);
        v.extend_from_slice(&self.data[..pos]);
        v.push(c);
        v.extend_from_slice(&self.data[pos..self.len]);
        self.replace_with(&v)
    }

    pub fn insert_literal(&mut self, pos: usize, literal: &[Utf8]) -> Result<(), ()> {
        if pos > self.len {
            return Err(());
        }
        let mut v = Vec::with_capacity(self.len + literal.len());
        v.extend_from_slice(&self.data[..pos]);
        v.extend_from_slice(literal);
        v.extend_from_slice(&self.data[pos..self.len]);
        self.replace_with(&v)
    }

    pub fn replace(&mut self, pos: usize, len: usize, replacement: &Utf8String) -> Result<(), ()> {
        if pos > self.len {
            return Err(());
        }
        let len = if pos + len > self.len { self.len - pos } else { len };
        let mut v = Vec::with_capacity(self.len + replacement.len - len);
        v.extend_from_slice(&self.data[..pos]);
        v.extend_from_slice(&replacement.data[..replacement.len]);
        v.extend_from_slice(&self.data[pos + len..self.len]);
        self.replace_with(&v)
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
        let len = if pos + len > self.len { self.len - pos } else { len };
        let mut v = Vec::with_capacity(self.len + replacement.len - len);
        v.extend_from_slice(&self.data[..pos]);
        v.extend_from_slice(&replacement.data[..replacement.len]);
        v.extend_from_slice(&self.data[pos + len..self.len]);
        self.replace_with(&v)
    }

    pub fn replace_character(&mut self, pos: usize, len: usize, c: Utf8) -> Result<(), ()> {
        if pos > self.len {
            return Err(());
        }
        let len = if pos + len > self.len { self.len - pos } else { len };
        let mut v = Vec::with_capacity(self.len + 1 - len);
        v.extend_from_slice(&self.data[..pos]);
        v.push(c);
        v.extend_from_slice(&self.data[pos + len..self.len]);
        self.replace_with(&v)
    }

    pub fn replace_literal(
        &mut self,
        pos: usize,
        len: usize,
        literal: &[Utf8],
    ) -> Result<(), ()> {
        if pos > self.len {
            return Err(());
        }
        let len = if pos + len > self.len { self.len - pos } else { len };
        let mut v = Vec::with_capacity(self.len + literal.len() - len);
        v.extend_from_slice(&self.data[..pos]);
        v.extend_from_slice(literal);
        v.extend_from_slice(&self.data[pos + len..self.len]);
        self.replace_with(&v)
    }

    pub fn erase(&mut self, pos: usize, len: usize) -> Result<(), ()> {
        if pos > self.len {
            return Err(());
        }
        let len = if pos + len > self.len { self.len - pos } else { len };
        let mut v = Vec::with_capacity(self.len - len);
        v.extend_from_slice(&self.data[..pos]);
        v.extend_from_slice(&self.data[pos + len..self.len]);
        self.replace_with(&v)
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
        compare_bytes(&self.data[..self.len], &other.data[..other.len])
    }

    pub fn compare_literal(&self, literal: &[Utf8]) -> i32 {
        compare_bytes(&self.data[..self.len], literal)
    }

    pub fn substring(&self, start: usize, end: usize) -> Utf8StringView {
        // The Rust test uses (start, length) semantic where the second
        // parameter is the length of the substring.
        let length = end;
        let actual_end = std::cmp::min(start.saturating_add(length), self.len);
        let actual_start = std::cmp::min(start, actual_end);
        Utf8StringView {
            data: &self.data[actual_start..actual_end],
            len: actual_end - actual_start,
        }
    }

    pub fn substring_copy(&self, start: usize, end: usize) -> Result<Utf8String, ()> {
        let length = end;
        let actual_end = std::cmp::min(start.saturating_add(length), self.len);
        let actual_start = std::cmp::min(start, actual_end);
        let len = actual_end - actual_start;
        let mut result = Utf8String::new();
        result.reserve(len)?;
        result.replace_with(&self.data[actual_start..actual_end])?;
        Ok(result)
    }

    pub fn index_of_character(&self, pos: usize, c: Utf8) -> Option<usize> {
        for i in pos..self.len {
            if self.data[i] == c {
                return Some(i);
            }
        }
        None
    }

    pub fn last_index_of_character(&self, pos: usize, c: Utf8) -> Option<usize> {
        let pos = if pos == usize::MAX {
            if self.len == 0 {
                return None;
            }
            self.len - 1
        } else if pos >= self.len {
            return None;
        } else {
            pos
        };
        let mut i = pos;
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
        compare_bytes(&self.data[..self.len], &other.data[..other.len])
    }

    pub fn compare_literal(&self, literal: &[Utf8]) -> i32 {
        compare_bytes(&self.data[..self.len], literal)
    }

    pub fn substring(&self, start: usize, end: usize) -> Utf8StringView<'a> {
        let length = end;
        let actual_end = std::cmp::min(start.saturating_add(length), self.len);
        let actual_start = std::cmp::min(start, actual_end);
        Utf8StringView {
            data: &self.data[actual_start..actual_end],
            len: actual_end - actual_start,
        }
    }

    pub fn substring_copy(&self, start: usize, end: usize) -> Result<Utf8String, ()> {
        let length = end;
        let actual_end = std::cmp::min(start.saturating_add(length), self.len);
        let actual_start = std::cmp::min(start, actual_end);
        let len = actual_end - actual_start;
        let mut result = Utf8String::new();
        result.reserve(len)?;
        result.replace_with(&self.data[actual_start..actual_end])?;
        Ok(result)
    }

    pub fn index_of_character(&self, pos: usize, c: Utf8) -> Option<usize> {
        for i in pos..self.len {
            if self.data[i] == c {
                return Some(i);
            }
        }
        None
    }

    pub fn last_index_of_character(&self, pos: usize, c: Utf8) -> Option<usize> {
        let pos = if pos == usize::MAX {
            if self.len == 0 {
                return None;
            }
            self.len - 1
        } else if pos >= self.len {
            return None;
        } else {
            pos
        };
        let mut i = pos;
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

fn compare_bytes(a: &[u8], b: &[u8]) -> i32 {
    let n = a.len().min(b.len());
    for i in 0..n {
        if a[i] != b[i] {
            return (a[i] as i32) - (b[i] as i32);
        }
    }
    if a.len() < b.len() {
        -1
    } else if a.len() > b.len() {
        1
    } else {
        0
    }
}
