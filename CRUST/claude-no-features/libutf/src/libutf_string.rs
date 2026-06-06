use crate::libutf_utf::*;
use std::cmp::Ordering;

pub struct Utf8String<'a> {
    pub data: &'a [Utf8],
    pub len: usize,
    pub cap: usize,
}

pub struct Utf8StringView<'a> {
    pub data: &'a [Utf8],
    pub len: usize,
}

// Helper to round a `len` up to the next power of two, using the same
// algorithm as the C implementation. Note: an input of 0 stays 0.
fn round_up_pow2(mut len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    len -= 1;
    len |= len >> 1;
    len |= len >> 2;
    len |= len >> 4;
    len |= len >> 8;
    len |= len >> 16;
    if std::mem::size_of::<usize>() == 8 {
        len |= len >> 32;
    }
    len + 1
}

impl<'a> Utf8String<'a> {
    /// Get a mutable view into the underlying buffer. This is `unsafe` because
    /// the struct holds an immutable shared slice reference but the underlying
    /// storage is exclusively owned by this `Utf8String` (it was allocated via
    /// `Box::leak` in `reserve`). Mutating through a raw pointer obtained from
    /// `as_ptr()` is the only way to grow/modify the buffer given the struct
    /// definition we cannot change.
    fn buf_mut(&mut self) -> &mut [u8] {
        if self.cap == 0 {
            &mut []
        } else {
            // SAFETY: The buffer was allocated via Box::leak in `reserve`. While
            // the struct field exposes a `&[u8]` view, we never hand out
            // simultaneous shared and mutable access to the same memory; methods
            // that mutate use `buf_mut` exclusively.
            unsafe {
                std::slice::from_raw_parts_mut(self.data.as_ptr() as *mut u8, self.cap)
            }
        }
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
        self.data = &[];
        self.len = 0;
        self.cap = 0;
    }

    pub fn destroy(&mut self) {
        if self.cap != 0 {
            let ptr = self.data.as_ptr() as *mut u8;
            let cap = self.cap;
            // SAFETY: Buffer was obtained via Box::leak of a Box<[u8]>. We are
            // the sole owner and reconstitute the Box to release the memory.
            unsafe {
                let _ = Box::from_raw(std::slice::from_raw_parts_mut(ptr, cap));
            }
            self.data = &[];
            self.cap = 0;
            self.len = 0;
        }
    }

    pub fn reserve(&mut self, len: usize) -> Result<(), ()> {
        if len <= self.cap {
            return Ok(());
        }

        let new_cap = round_up_pow2(len);

        // Allocate fresh boxed slice.
        let mut new_buf: Box<[u8]> = vec![0u8; new_cap].into_boxed_slice();

        // Copy existing live bytes into the new buffer.
        if self.len > 0 {
            new_buf[..self.len].copy_from_slice(&self.data[..self.len]);
        }

        // Free old buffer if any.
        if self.cap != 0 {
            let old_ptr = self.data.as_ptr() as *mut u8;
            let old_cap = self.cap;
            // SAFETY: The previous buffer was obtained from Box::leak.
            unsafe {
                let _ = Box::from_raw(std::slice::from_raw_parts_mut(old_ptr, old_cap));
            }
        }

        // Leak the new buffer and store the slice.
        let leaked: &'static mut [u8] = Box::leak(new_buf);
        // Coerce &'static [u8] to whatever lifetime the struct expects.
        self.data = leaked as &[u8];
        self.cap = new_cap;

        Ok(())
    }

    pub fn shrink_to_fit(&mut self) -> Result<(), ()> {
        if self.cap == 0 {
            return Ok(());
        }
        if self.len == self.cap {
            return Ok(());
        }

        let new_cap = self.len;
        let mut new_buf: Box<[u8]> = vec![0u8; new_cap].into_boxed_slice();
        if new_cap > 0 {
            new_buf.copy_from_slice(&self.data[..new_cap]);
        }

        // Free old buffer.
        let old_ptr = self.data.as_ptr() as *mut u8;
        let old_cap = self.cap;
        // SAFETY: Buffer was leaked from a Box.
        unsafe {
            let _ = Box::from_raw(std::slice::from_raw_parts_mut(old_ptr, old_cap));
        }

        if new_cap == 0 {
            // Drop the freshly allocated zero-length buffer too.
            drop(new_buf);
            self.data = &[];
            self.cap = 0;
        } else {
            let leaked: &'static mut [u8] = Box::leak(new_buf);
            self.data = leaked as &[u8];
            self.cap = new_cap;
        }

        Ok(())
    }

    pub fn clear(&mut self) {
        self.len = 0;
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn append(&mut self, other: &Utf8String) -> Result<(), ()> {
        self.append_literal(&other.data[..other.len])
    }

    pub fn append_view(&mut self, view: &Utf8StringView) -> Result<(), ()> {
        self.append_literal(&view.data[..view.len])
    }

    pub fn append_character(&mut self, c: Utf8) -> Result<(), ()> {
        self.reserve(self.len + 1)?;
        let pos = self.len;
        let buf = self.buf_mut();
        buf[pos] = c;
        self.len += 1;
        Ok(())
    }

    pub fn append_literal(&mut self, literal: &[Utf8]) -> Result<(), ()> {
        let n = literal.len();
        self.reserve(self.len + n)?;
        let pos = self.len;
        let buf = self.buf_mut();
        buf[pos..pos + n].copy_from_slice(literal);
        self.len += n;
        Ok(())
    }

    pub fn prepend(&mut self, other: &Utf8String) -> Result<(), ()> {
        self.prepend_literal(&other.data[..other.len])
    }

    pub fn prepend_view(&mut self, view: &Utf8StringView) -> Result<(), ()> {
        self.prepend_literal(&view.data[..view.len])
    }

    pub fn prepend_character(&mut self, c: Utf8) -> Result<(), ()> {
        self.reserve(self.len + 1)?;
        let len = self.len;
        let buf = self.buf_mut();
        buf.copy_within(0..len, 1);
        buf[0] = c;
        self.len += 1;
        Ok(())
    }

    pub fn prepend_literal(&mut self, literal: &[Utf8]) -> Result<(), ()> {
        let n = literal.len();
        self.reserve(self.len + n)?;
        let len = self.len;
        let buf = self.buf_mut();
        buf.copy_within(0..len, n);
        buf[..n].copy_from_slice(literal);
        self.len += n;
        Ok(())
    }

    pub fn insert(&mut self, pos: usize, other: &Utf8String) -> Result<(), ()> {
        self.insert_literal(pos, &other.data[..other.len])
    }

    pub fn insert_view(&mut self, pos: usize, view: &Utf8StringView) -> Result<(), ()> {
        self.insert_literal(pos, &view.data[..view.len])
    }

    pub fn insert_character(&mut self, pos: usize, c: Utf8) -> Result<(), ()> {
        if pos > self.len {
            return Err(());
        }
        let inserted_len = self.len + 1;
        self.reserve(inserted_len)?;
        let cur_len = self.len;
        let buf = self.buf_mut();
        buf.copy_within(pos..cur_len, pos + 1);
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
        self.reserve(inserted_len)?;
        let cur_len = self.len;
        let buf = self.buf_mut();
        buf.copy_within(pos..cur_len, pos + n);
        buf[pos..pos + n].copy_from_slice(literal);
        self.len = inserted_len;
        Ok(())
    }

    pub fn replace(&mut self, pos: usize, len: usize, replacement: &Utf8String) -> Result<(), ()> {
        self.replace_literal(pos, len, &replacement.data[..replacement.len])
    }

    pub fn replace_view(
        &mut self,
        pos: usize,
        len: usize,
        replacement: &Utf8StringView,
    ) -> Result<(), ()> {
        self.replace_literal(pos, len, &replacement.data[..replacement.len])
    }

    pub fn replace_character(&mut self, pos: usize, len: usize, c: Utf8) -> Result<(), ()> {
        if pos > self.len {
            return Err(());
        }
        // C version doesn't clamp len here, but we need to be careful.
        let replaced_len = self.len + 1 - len;
        self.reserve(replaced_len)?;
        let cur_len = self.len;
        let buf = self.buf_mut();
        // memmove(&data[pos+1], &data[pos+len], len - pos - len)
        let src_start = pos + len;
        let copy_len = cur_len - pos - len;
        buf.copy_within(src_start..src_start + copy_len, pos + 1);
        buf[pos] = c;
        self.len = replaced_len;
        Ok(())
    }

    pub fn replace_literal(
        &mut self,
        pos: usize,
        mut len: usize,
        literal: &[Utf8],
    ) -> Result<(), ()> {
        if pos > self.len {
            return Err(());
        }
        if pos + len > self.len {
            len = self.len - pos;
        }
        let n = literal.len();
        let replaced_len = self.len + n - len;
        self.reserve(replaced_len)?;
        let cur_len = self.len;
        let buf = self.buf_mut();
        let src_start = pos + len;
        let copy_len = cur_len - pos - len;
        buf.copy_within(src_start..src_start + copy_len, pos + n);
        buf[pos..pos + n].copy_from_slice(literal);
        self.len = replaced_len;
        Ok(())
    }

    pub fn erase(&mut self, pos: usize, mut len: usize) -> Result<(), ()> {
        if pos > self.len {
            return Err(());
        }
        if pos + len > self.len {
            len = self.len - pos;
        }
        let cur_len = self.len;
        let buf = self.buf_mut();
        let src_start = pos + len;
        let copy_len = cur_len - pos - len;
        buf.copy_within(src_start..src_start + copy_len, pos);
        self.len -= len;
        Ok(())
    }

    pub fn concat(&self, other: &Utf8String) -> Result<Utf8String, ()> {
        let mut result = Utf8String::new();
        result.reserve(self.len + other.len)?;
        result.append_literal(&self.data[..self.len])?;
        result.append_literal(&other.data[..other.len])?;
        Ok(result)
    }

    pub fn concat_view(&self, other: &Utf8StringView) -> Result<Utf8String, ()> {
        let mut result = Utf8String::new();
        result.reserve(self.len + other.len)?;
        result.append_literal(&self.data[..self.len])?;
        result.append_literal(&other.data[..other.len])?;
        Ok(result)
    }

    pub fn concat_character(&self, c: Utf8) -> Result<Utf8String, ()> {
        let mut result = Utf8String::new();
        result.reserve(self.len + 1)?;
        result.append_literal(&self.data[..self.len])?;
        result.append_character(c)?;
        Ok(result)
    }

    pub fn concat_literal(&self, literal: &[Utf8]) -> Result<Utf8String, ()> {
        let mut result = Utf8String::new();
        result.reserve(self.len + literal.len())?;
        result.append_literal(&self.data[..self.len])?;
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
        // The C library uses (start, end_exclusive) but the Rust binding
        // here treats the second parameter as a length: substring(2, 8)
        // returns 8 bytes starting at position 2.
        let s = if start > self.len { self.len } else { start };
        let mut n = end;
        if s + n > self.len {
            n = self.len - s;
        }
        Utf8StringView {
            data: &self.data[s..s + n],
            len: n,
        }
    }

    pub fn substring_copy(&self, start: usize, end: usize) -> Result<Utf8String, ()> {
        let s = if start > self.len { self.len } else { start };
        let mut n = end;
        if s + n > self.len {
            n = self.len - s;
        }
        let mut result = Utf8String::new();
        result.reserve(n)?;
        if n > 0 {
            let buf = result.buf_mut();
            buf[..n].copy_from_slice(&self.data[s..s + n]);
        }
        result.len = n;
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
        let start = if pos == usize::MAX {
            if self.len == 0 {
                return None;
            }
            self.len - 1
        } else if pos >= self.len {
            return None;
        } else {
            pos
        };

        let mut i = start;
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

impl Default for Utf8String<'_> {
    fn default() -> Self {
        Self::new()
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
        let mut e = end;
        if e > self.len {
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
        if e > self.len {
            e = self.len;
        }
        let s = if start > e { e } else { start };
        let mut result = Utf8String::new();
        let n = e - s;
        result.reserve(n)?;
        if n > 0 {
            let buf = result.buf_mut();
            buf[..n].copy_from_slice(&self.data[s..e]);
        }
        result.len = n;
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
        let start = if pos == usize::MAX {
            if self.len == 0 {
                return None;
            }
            self.len - 1
        } else if pos >= self.len {
            return None;
        } else {
            pos
        };

        let mut i = start;
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

// Mimic strncmp + the libutf "shorter is less" rule.
fn compare_bytes(a: &[u8], b: &[u8]) -> i32 {
    let n = a.len().min(b.len());
    for i in 0..n {
        match a[i].cmp(&b[i]) {
            Ordering::Less => return (a[i] as i32) - (b[i] as i32),
            Ordering::Greater => return (a[i] as i32) - (b[i] as i32),
            Ordering::Equal => {}
        }
    }
    match a.len().cmp(&b.len()) {
        Ordering::Less => -1,
        Ordering::Greater => 1,
        Ordering::Equal => 0,
    }
}
