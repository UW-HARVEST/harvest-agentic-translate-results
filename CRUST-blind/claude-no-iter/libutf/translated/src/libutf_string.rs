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

/// Compare two byte slices using C `strncmp`-style semantics.
fn strncmp_like(a: &[u8], b: &[u8]) -> i32 {
    let n = a.len().min(b.len());
    for i in 0..n {
        if a[i] != b[i] {
            return (a[i] as i32) - (b[i] as i32);
        }
    }
    0
}

fn final_compare(initial: i32, a_len: usize, b_len: usize) -> i32 {
    if initial == 0 {
        if a_len < b_len {
            -1
        } else if a_len > b_len {
            1
        } else {
            0
        }
    } else {
        initial
    }
}

impl Utf8String<'_> {
    /// Free any owned heap buffer (raw pointer round-trip via `Box::from_raw`).
    fn free_buffer(&mut self) {
        if self.cap > 0 {
            // Safety: We own the slice that `data` currently points at because
            // `self.cap > 0` always corresponds to a previously `Box::leak`-ed
            // boxed slice of length exactly `self.cap`.
            unsafe {
                let ptr = self.data.as_ptr() as *mut u8;
                let slice = std::slice::from_raw_parts_mut(ptr, self.cap);
                let _ = Box::from_raw(slice as *mut [u8]);
            }
            self.data = &[];
            self.cap = 0;
        }
    }

    /// Returns a mutable view of the entire allocated capacity.
    fn buf_mut(&mut self) -> &mut [u8] {
        if self.cap == 0 {
            return &mut [];
        }
        // Safety: When `cap > 0`, `data` points to a buffer of size `cap`
        // that we exclusively own (via `Box::leak`).
        unsafe {
            let ptr = self.data.as_ptr() as *mut u8;
            std::slice::from_raw_parts_mut(ptr, self.cap)
        }
    }

    pub fn new() -> Self {
        Self {
            data: &[],
            len: 0,
            cap: 0,
        }
    }

    pub fn init(&mut self) {
        self.free_buffer();
        self.data = &[];
        self.len = 0;
        self.cap = 0;
    }

    pub fn destroy(&mut self) {
        self.free_buffer();
        self.len = 0;
    }

    pub fn reserve(&mut self, len: usize) -> Result<(), ()> {
        if len <= self.cap {
            return Ok(());
        }

        // Round up to the next power of two (C bit hack equivalent).
        let new_cap = if len <= 1 { 1 } else { len.next_power_of_two() };

        let mut new_buf: Vec<u8> = vec![0u8; new_cap];
        if self.len > 0 {
            new_buf[..self.len].copy_from_slice(&self.data[..self.len]);
        }

        // Free the old buffer (if any) before installing the new leaked one.
        self.free_buffer();

        let leaked: &'static mut [u8] = Box::leak(new_buf.into_boxed_slice());
        self.data = leaked;
        self.cap = new_cap;

        Ok(())
    }

    pub fn shrink_to_fit(&mut self) -> Result<(), ()> {
        if self.cap == 0 {
            return Ok(());
        }
        if self.cap == self.len {
            return Ok(());
        }

        if self.len == 0 {
            self.free_buffer();
            return Ok(());
        }

        let new_cap = self.len;
        let mut new_buf: Vec<u8> = vec![0u8; new_cap];
        new_buf.copy_from_slice(&self.data[..self.len]);

        self.free_buffer();

        let leaked: &'static mut [u8] = Box::leak(new_buf.into_boxed_slice());
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
        let other_data: Vec<u8> = other.data[..other_len].to_vec();
        self.reserve(self.len + other_len)?;
        let len = self.len;
        self.buf_mut()[len..len + other_len].copy_from_slice(&other_data);
        self.len += other_len;
        Ok(())
    }

    pub fn append_view(&mut self, view: &Utf8StringView) -> Result<(), ()> {
        let view_len = view.len;
        let view_data: Vec<u8> = view.data[..view_len].to_vec();
        self.reserve(self.len + view_len)?;
        let len = self.len;
        self.buf_mut()[len..len + view_len].copy_from_slice(&view_data);
        self.len += view_len;
        Ok(())
    }

    pub fn append_character(&mut self, c: Utf8) -> Result<(), ()> {
        self.reserve(self.len + 1)?;
        let len = self.len;
        self.buf_mut()[len] = c;
        self.len += 1;
        Ok(())
    }

    pub fn append_literal(&mut self, literal: &[Utf8]) -> Result<(), ()> {
        let n = literal.len();
        let lit_copy: Vec<u8> = literal.to_vec();
        self.reserve(self.len + n)?;
        let len = self.len;
        self.buf_mut()[len..len + n].copy_from_slice(&lit_copy);
        self.len += n;
        Ok(())
    }

    pub fn prepend(&mut self, other: &Utf8String) -> Result<(), ()> {
        let other_len = other.len;
        let other_data: Vec<u8> = other.data[..other_len].to_vec();
        self.reserve(self.len + other_len)?;
        let cur_len = self.len;
        let buf = self.buf_mut();
        buf.copy_within(0..cur_len, other_len);
        buf[..other_len].copy_from_slice(&other_data);
        self.len += other_len;
        Ok(())
    }

    pub fn prepend_view(&mut self, view: &Utf8StringView) -> Result<(), ()> {
        let view_len = view.len;
        let view_data: Vec<u8> = view.data[..view_len].to_vec();
        self.reserve(self.len + view_len)?;
        let cur_len = self.len;
        let buf = self.buf_mut();
        buf.copy_within(0..cur_len, view_len);
        buf[..view_len].copy_from_slice(&view_data);
        self.len += view_len;
        Ok(())
    }

    pub fn prepend_character(&mut self, c: Utf8) -> Result<(), ()> {
        self.reserve(self.len + 1)?;
        let cur_len = self.len;
        let buf = self.buf_mut();
        buf.copy_within(0..cur_len, 1);
        buf[0] = c;
        self.len += 1;
        Ok(())
    }

    pub fn prepend_literal(&mut self, literal: &[Utf8]) -> Result<(), ()> {
        let n = literal.len();
        let lit_copy: Vec<u8> = literal.to_vec();
        self.reserve(self.len + n)?;
        let cur_len = self.len;
        let buf = self.buf_mut();
        buf.copy_within(0..cur_len, n);
        buf[..n].copy_from_slice(&lit_copy);
        self.len += n;
        Ok(())
    }

    pub fn insert(&mut self, pos: usize, other: &Utf8String) -> Result<(), ()> {
        if pos > self.len {
            return Err(());
        }
        let other_len = other.len;
        let other_data: Vec<u8> = other.data[..other_len].to_vec();
        let inserted_len = self.len + other_len;
        self.reserve(inserted_len)?;
        let cur_len = self.len;
        let buf = self.buf_mut();
        buf.copy_within(pos..cur_len, pos + other_len);
        buf[pos..pos + other_len].copy_from_slice(&other_data);
        self.len = inserted_len;
        Ok(())
    }

    pub fn insert_view(&mut self, pos: usize, view: &Utf8StringView) -> Result<(), ()> {
        if pos > self.len {
            return Err(());
        }
        let view_len = view.len;
        let view_data: Vec<u8> = view.data[..view_len].to_vec();
        let inserted_len = self.len + view_len;
        self.reserve(inserted_len)?;
        let cur_len = self.len;
        let buf = self.buf_mut();
        buf.copy_within(pos..cur_len, pos + view_len);
        buf[pos..pos + view_len].copy_from_slice(&view_data);
        self.len = inserted_len;
        Ok(())
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
        let lit_copy: Vec<u8> = literal.to_vec();
        let inserted_len = self.len + n;
        self.reserve(inserted_len)?;
        let cur_len = self.len;
        let buf = self.buf_mut();
        buf.copy_within(pos..cur_len, pos + n);
        buf[pos..pos + n].copy_from_slice(&lit_copy);
        self.len = inserted_len;
        Ok(())
    }

    pub fn replace(
        &mut self,
        pos: usize,
        len: usize,
        replacement: &Utf8String,
    ) -> Result<(), ()> {
        if pos > self.len {
            return Err(());
        }
        let mut len = len;
        if pos + len > self.len {
            len = self.len - pos;
        }
        let rep_len = replacement.len;
        let rep_data: Vec<u8> = replacement.data[..rep_len].to_vec();
        let replaced_len = self.len + rep_len - len;
        self.reserve(replaced_len)?;
        let cur_len = self.len;
        let buf = self.buf_mut();
        buf.copy_within(pos + len..cur_len, pos + rep_len);
        buf[pos..pos + rep_len].copy_from_slice(&rep_data);
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
        let rep_data: Vec<u8> = replacement.data[..rep_len].to_vec();
        let replaced_len = self.len + rep_len - len;
        self.reserve(replaced_len)?;
        let cur_len = self.len;
        let buf = self.buf_mut();
        buf.copy_within(pos + len..cur_len, pos + rep_len);
        buf[pos..pos + rep_len].copy_from_slice(&rep_data);
        self.len = replaced_len;
        Ok(())
    }

    pub fn replace_character(
        &mut self,
        pos: usize,
        len: usize,
        c: Utf8,
    ) -> Result<(), ()> {
        if pos > self.len {
            return Err(());
        }
        // Note: matches C version which does NOT clamp `len` here.
        let replaced_len = self.len + 1 - len;
        self.reserve(replaced_len)?;
        let cur_len = self.len;
        let buf = self.buf_mut();
        // `cur_len - pos - len` bytes to move.
        let move_count = cur_len - pos - len;
        buf.copy_within(pos + len..pos + len + move_count, pos + 1);
        buf[pos] = c;
        self.len = replaced_len;
        Ok(())
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
        let mut len = len;
        if pos + len > self.len {
            len = self.len - pos;
        }
        let n = literal.len();
        let lit_copy: Vec<u8> = literal.to_vec();
        let replaced_len = self.len + n - len;
        self.reserve(replaced_len)?;
        let cur_len = self.len;
        let buf = self.buf_mut();
        buf.copy_within(pos + len..cur_len, pos + n);
        buf[pos..pos + n].copy_from_slice(&lit_copy);
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
        let cur_len = self.len;
        let buf = self.buf_mut();
        buf.copy_within(pos + len..cur_len, pos);
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
        let a = &self.data[..self.len];
        let b = &other.data[..other.len];
        let initial = strncmp_like(a, b);
        final_compare(initial, self.len, other.len)
    }

    pub fn compare_literal(&self, literal: &[Utf8]) -> i32 {
        let a = &self.data[..self.len];
        let initial = strncmp_like(a, literal);
        final_compare(initial, self.len, literal.len())
    }

    pub fn substring(&self, start: usize, end: usize) -> Utf8StringView {
        let mut end = end;
        if end > self.len {
            end = self.len;
        }
        let start = if start > end { end } else { start };
        Utf8StringView {
            data: &self.data[start..end],
            len: end - start,
        }
    }

    pub fn substring_copy(&self, start: usize, end: usize) -> Result<Utf8String, ()> {
        let mut end = end;
        if end > self.len {
            end = self.len;
        }
        let start = if start > end { end } else { start };
        let len = end - start;
        let snapshot: Vec<u8> = self.data[start..end].to_vec();
        let mut result = Utf8String::new();
        result.reserve(len)?;
        if len > 0 {
            result.buf_mut()[..len].copy_from_slice(&snapshot);
        }
        result.len = len;
        Ok(result)
    }

    pub fn index_of_character(&self, pos: usize, c: Utf8) -> Option<usize> {
        let mut i = pos;
        while i < self.len {
            if self.data[i] == c {
                return Some(i);
            }
            i += 1;
        }
        None
    }

    pub fn last_index_of_character(&self, pos: usize, c: Utf8) -> Option<usize> {
        if self.len == 0 {
            return None;
        }
        let pos = if pos == usize::MAX {
            self.len - 1
        } else if pos >= self.len {
            return None;
        } else {
            pos
        };
        let mut i: isize = pos as isize;
        while i >= 0 {
            if self.data[i as usize] == c {
                return Some(i as usize);
            }
            i -= 1;
        }
        None
    }
}

impl<'a> Utf8StringView<'a> {
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn compare(&self, other: &Utf8StringView) -> i32 {
        let a = &self.data[..self.len];
        let b = &other.data[..other.len];
        let initial = strncmp_like(a, b);
        final_compare(initial, self.len, other.len)
    }

    pub fn compare_literal(&self, literal: &[Utf8]) -> i32 {
        let a = &self.data[..self.len];
        let initial = strncmp_like(a, literal);
        final_compare(initial, self.len, literal.len())
    }

    pub fn substring(&self, start: usize, end: usize) -> Utf8StringView<'a> {
        let mut end = end;
        if end > self.len {
            end = self.len;
        }
        let start = if start > end { end } else { start };
        Utf8StringView {
            data: &self.data[start..end],
            len: end - start,
        }
    }

    pub fn substring_copy(&self, start: usize, end: usize) -> Result<Utf8String, ()> {
        let mut end = end;
        if end > self.len {
            end = self.len;
        }
        let start = if start > end { end } else { start };
        let len = end - start;
        let snapshot: Vec<u8> = self.data[start..end].to_vec();
        let mut result = Utf8String::new();
        result.reserve(len)?;
        if len > 0 {
            result.buf_mut()[..len].copy_from_slice(&snapshot);
        }
        result.len = len;
        Ok(result)
    }

    pub fn index_of_character(&self, pos: usize, c: Utf8) -> Option<usize> {
        let mut i = pos;
        while i < self.len {
            if self.data[i] == c {
                return Some(i);
            }
            i += 1;
        }
        None
    }

    pub fn last_index_of_character(&self, pos: usize, c: Utf8) -> Option<usize> {
        if self.len == 0 {
            return None;
        }
        let pos = if pos == usize::MAX {
            self.len - 1
        } else if pos >= self.len {
            return None;
        } else {
            pos
        };
        let mut i: isize = pos as isize;
        while i >= 0 {
            if self.data[i as usize] == c {
                return Some(i as usize);
            }
            i -= 1;
        }
        None
    }
}
