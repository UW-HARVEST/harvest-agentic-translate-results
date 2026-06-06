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

// Internal helpers ---------------------------------------------------------

fn empty_static_slice() -> &'static [Utf8] {
    &[]
}

/// Allocate a heap buffer of `cap` bytes, zero-initialized, return as
/// a leaked static slice. Caller is responsible for freeing it.
fn alloc_buffer(cap: usize) -> &'static mut [Utf8] {
    let v: Vec<u8> = vec![0u8; cap];
    Box::leak(v.into_boxed_slice())
}

/// Reclaim a previously-leaked buffer.
unsafe fn free_buffer(ptr: *const Utf8, cap: usize) {
    if cap == 0 || ptr.is_null() {
        return;
    }
    let slice = core::slice::from_raw_parts_mut(ptr as *mut Utf8, cap);
    let _ = Box::from_raw(slice as *mut [Utf8]);
}

impl Utf8String<'_> {
    pub fn new() -> Self {
        Self {
            data: empty_static_slice(),
            len: 0,
            cap: 0,
        }
    }

    pub fn init(&mut self) {
        // Initialize an existing string to empty state. If there was a
        // pre-existing allocation, drop it.
        if self.cap > 0 && !self.data.is_empty() {
            unsafe {
                free_buffer(self.data.as_ptr(), self.cap);
            }
        }
        self.data = empty_static_slice();
        self.len = 0;
        self.cap = 0;
    }

    pub fn destroy(&mut self) {
        if self.cap > 0 {
            unsafe {
                free_buffer(self.data.as_ptr(), self.cap);
            }
        }
        self.data = empty_static_slice();
        self.len = 0;
        self.cap = 0;
    }

    pub fn reserve(&mut self, len: usize) -> Result<(), ()> {
        if len <= self.cap {
            return Ok(());
        }
        // Round up to next power of two, matching C bithack.
        let mut new_cap = if len == 0 { 1 } else { len };
        new_cap = new_cap.next_power_of_two();

        let new_buf = alloc_buffer(new_cap);
        // Copy existing valid bytes.
        if self.len > 0 {
            new_buf[..self.len].copy_from_slice(&self.data[..self.len]);
        }

        // Free old allocation, if any.
        if self.cap > 0 {
            unsafe {
                free_buffer(self.data.as_ptr(), self.cap);
            }
        }

        // Convert `&'static mut [u8]` to `&'static [u8]` and assign,
        // transmuting the lifetime to match the struct. This is safe
        // because we own the buffer and free it on destroy/reserve.
        let static_ref: &'static [Utf8] = &*new_buf;
        self.data = unsafe { core::mem::transmute::<&'static [Utf8], &[Utf8]>(static_ref) };
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
            // Free buffer entirely.
            unsafe {
                free_buffer(self.data.as_ptr(), self.cap);
            }
            self.data = empty_static_slice();
            self.cap = 0;
            return Ok(());
        }
        let new_buf = alloc_buffer(new_cap);
        new_buf[..new_cap].copy_from_slice(&self.data[..new_cap]);

        unsafe {
            free_buffer(self.data.as_ptr(), self.cap);
        }
        let static_ref: &'static [Utf8] = &*new_buf;
        self.data = unsafe { core::mem::transmute::<&'static [Utf8], &[Utf8]>(static_ref) };
        self.cap = new_cap;
        Ok(())
    }

    pub fn clear(&mut self) {
        self.len = 0;
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    fn data_mut(&mut self) -> &mut [Utf8] {
        // SAFETY: we own the underlying buffer of size self.cap.
        unsafe {
            core::slice::from_raw_parts_mut(
                self.data.as_ptr() as *mut Utf8,
                self.cap.max(self.len),
            )
        }
    }

    pub fn append(&mut self, other: &Utf8String) -> Result<(), ()> {
        let new_len = self.len + other.len;
        self.reserve(new_len)?;
        let other_data = &other.data[..other.len];
        let start = self.len;
        let buf = self.data_mut();
        buf[start..start + other.len].copy_from_slice(other_data);
        self.len = new_len;
        Ok(())
    }

    pub fn append_view(&mut self, view: &Utf8StringView) -> Result<(), ()> {
        let new_len = self.len + view.len;
        self.reserve(new_len)?;
        let view_data = &view.data[..view.len];
        let start = self.len;
        let buf = self.data_mut();
        buf[start..start + view.len].copy_from_slice(view_data);
        self.len = new_len;
        Ok(())
    }

    pub fn append_character(&mut self, c: Utf8) -> Result<(), ()> {
        let new_len = self.len + 1;
        self.reserve(new_len)?;
        let start = self.len;
        let buf = self.data_mut();
        buf[start] = c;
        self.len = new_len;
        Ok(())
    }

    pub fn append_literal(&mut self, literal: &[Utf8]) -> Result<(), ()> {
        let n = literal.len();
        let new_len = self.len + n;
        self.reserve(new_len)?;
        let start = self.len;
        let buf = self.data_mut();
        buf[start..start + n].copy_from_slice(literal);
        self.len = new_len;
        Ok(())
    }

    pub fn prepend(&mut self, other: &Utf8String) -> Result<(), ()> {
        let other_len = other.len;
        let new_len = self.len + other_len;
        // Save other's bytes in case other == self via aliasing concerns
        let other_bytes: Vec<u8> = other.data[..other_len].to_vec();
        self.reserve(new_len)?;
        let old_len = self.len;
        let buf = self.data_mut();
        buf.copy_within(0..old_len, other_len);
        buf[..other_len].copy_from_slice(&other_bytes);
        self.len = new_len;
        Ok(())
    }

    pub fn prepend_view(&mut self, view: &Utf8StringView) -> Result<(), ()> {
        let v_len = view.len;
        let new_len = self.len + v_len;
        let view_bytes: Vec<u8> = view.data[..v_len].to_vec();
        self.reserve(new_len)?;
        let old_len = self.len;
        let buf = self.data_mut();
        buf.copy_within(0..old_len, v_len);
        buf[..v_len].copy_from_slice(&view_bytes);
        self.len = new_len;
        Ok(())
    }

    pub fn prepend_character(&mut self, c: Utf8) -> Result<(), ()> {
        let new_len = self.len + 1;
        self.reserve(new_len)?;
        let old_len = self.len;
        let buf = self.data_mut();
        buf.copy_within(0..old_len, 1);
        buf[0] = c;
        self.len = new_len;
        Ok(())
    }

    pub fn prepend_literal(&mut self, literal: &[Utf8]) -> Result<(), ()> {
        let n = literal.len();
        let new_len = self.len + n;
        let lit_bytes: Vec<u8> = literal.to_vec();
        self.reserve(new_len)?;
        let old_len = self.len;
        let buf = self.data_mut();
        buf.copy_within(0..old_len, n);
        buf[..n].copy_from_slice(&lit_bytes);
        self.len = new_len;
        Ok(())
    }

    pub fn insert(&mut self, pos: usize, other: &Utf8String) -> Result<(), ()> {
        if pos > self.len {
            return Err(());
        }
        let other_len = other.len;
        let other_bytes: Vec<u8> = other.data[..other_len].to_vec();
        let new_len = self.len + other_len;
        self.reserve(new_len)?;
        let old_len = self.len;
        let buf = self.data_mut();
        buf.copy_within(pos..old_len, pos + other_len);
        buf[pos..pos + other_len].copy_from_slice(&other_bytes);
        self.len = new_len;
        Ok(())
    }

    pub fn insert_view(&mut self, pos: usize, view: &Utf8StringView) -> Result<(), ()> {
        if pos > self.len {
            return Err(());
        }
        let v_len = view.len;
        let view_bytes: Vec<u8> = view.data[..v_len].to_vec();
        let new_len = self.len + v_len;
        self.reserve(new_len)?;
        let old_len = self.len;
        let buf = self.data_mut();
        buf.copy_within(pos..old_len, pos + v_len);
        buf[pos..pos + v_len].copy_from_slice(&view_bytes);
        self.len = new_len;
        Ok(())
    }

    pub fn insert_character(&mut self, pos: usize, c: Utf8) -> Result<(), ()> {
        if pos > self.len {
            return Err(());
        }
        let new_len = self.len + 1;
        self.reserve(new_len)?;
        let old_len = self.len;
        let buf = self.data_mut();
        buf.copy_within(pos..old_len, pos + 1);
        buf[pos] = c;
        self.len = new_len;
        Ok(())
    }

    pub fn insert_literal(&mut self, pos: usize, literal: &[Utf8]) -> Result<(), ()> {
        if pos > self.len {
            return Err(());
        }
        let n = literal.len();
        let lit_bytes: Vec<u8> = literal.to_vec();
        let new_len = self.len + n;
        self.reserve(new_len)?;
        let old_len = self.len;
        let buf = self.data_mut();
        buf.copy_within(pos..old_len, pos + n);
        buf[pos..pos + n].copy_from_slice(&lit_bytes);
        self.len = new_len;
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
        let len = if pos + len > self.len {
            self.len - pos
        } else {
            len
        };
        let r_len = replacement.len;
        let repl_bytes: Vec<u8> = replacement.data[..r_len].to_vec();
        let new_len = self.len + r_len - len;
        self.reserve(new_len)?;
        let old_len = self.len;
        let buf = self.data_mut();
        buf.copy_within(pos + len..old_len, pos + r_len);
        buf[pos..pos + r_len].copy_from_slice(&repl_bytes);
        self.len = new_len;
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
        let len = if pos + len > self.len {
            self.len - pos
        } else {
            len
        };
        let r_len = replacement.len;
        let repl_bytes: Vec<u8> = replacement.data[..r_len].to_vec();
        let new_len = self.len + r_len - len;
        self.reserve(new_len)?;
        let old_len = self.len;
        let buf = self.data_mut();
        buf.copy_within(pos + len..old_len, pos + r_len);
        buf[pos..pos + r_len].copy_from_slice(&repl_bytes);
        self.len = new_len;
        Ok(())
    }

    pub fn replace_character(&mut self, pos: usize, len: usize, c: Utf8) -> Result<(), ()> {
        if pos > self.len {
            return Err(());
        }
        // Note: matches C exactly — no clamp on len here, but C also relies on
        // `string->len - pos - len` not underflowing; replicate that behavior.
        let new_len = self.len + 1 - len;
        self.reserve(new_len)?;
        let old_len = self.len;
        let buf = self.data_mut();
        // memmove(&data[pos+1], &data[pos+len], len-pos-len)
        let count = old_len.saturating_sub(pos + len);
        if count > 0 {
            buf.copy_within(pos + len..pos + len + count, pos + 1);
        }
        buf[pos] = c;
        self.len = new_len;
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
        let len = if pos + len > self.len {
            self.len - pos
        } else {
            len
        };
        let n = literal.len();
        let lit_bytes: Vec<u8> = literal.to_vec();
        let new_len = self.len + n - len;
        self.reserve(new_len)?;
        let old_len = self.len;
        let buf = self.data_mut();
        buf.copy_within(pos + len..old_len, pos + n);
        buf[pos..pos + n].copy_from_slice(&lit_bytes);
        self.len = new_len;
        Ok(())
    }

    pub fn erase(&mut self, pos: usize, len: usize) -> Result<(), ()> {
        if pos > self.len {
            return Err(());
        }
        let len = if pos + len > self.len {
            self.len - pos
        } else {
            len
        };
        let old_len = self.len;
        let buf = self.data_mut();
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
        let a = &self.data[..self.len];
        let b = &other.data[..other.len];
        compare_bytes(a, b)
    }

    pub fn compare_literal(&self, literal: &[Utf8]) -> i32 {
        let a = &self.data[..self.len];
        compare_bytes(a, literal)
    }

    pub fn substring(&self, start: usize, end: usize) -> Utf8StringView {
        let end = if end > self.len { self.len } else { end };
        let start = if start > end { end } else { start };
        let len = end - start;
        Utf8StringView {
            data: &self.data[start..start + len],
            len,
        }
    }

    pub fn substring_copy(&self, start: usize, end: usize) -> Result<Utf8String, ()> {
        let end = if end > self.len { self.len } else { end };
        let start = if start > end { end } else { start };
        let len = end - start;
        let mut result = Utf8String::new();
        result.reserve(len)?;
        if len > 0 {
            let src = self.data[start..start + len].to_vec();
            let buf = result.data_mut();
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
        if self.len == 0 {
            return None;
        }
        let pos = if pos >= self.len { return None; } else { pos };
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
        compare_bytes(a, b)
    }

    pub fn compare_literal(&self, literal: &[Utf8]) -> i32 {
        let a = &self.data[..self.len];
        compare_bytes(a, literal)
    }

    pub fn substring(&self, start: usize, end: usize) -> Utf8StringView<'a> {
        let end = if end > self.len { self.len } else { end };
        let start = if start > end { end } else { start };
        let len = end - start;
        Utf8StringView {
            data: &self.data[start..start + len],
            len,
        }
    }

    pub fn substring_copy(&self, start: usize, end: usize) -> Result<Utf8String, ()> {
        let end = if end > self.len { self.len } else { end };
        let start = if start > end { end } else { start };
        let len = end - start;
        let mut result = Utf8String::new();
        result.reserve(len)?;
        if len > 0 {
            let src = self.data[start..start + len].to_vec();
            let buf = result.data_mut();
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
        if self.len == 0 {
            return None;
        }
        let pos = if pos >= self.len { return None; } else { pos };
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

fn compare_bytes(a: &[Utf8], b: &[Utf8]) -> i32 {
    let a_len = a.len();
    let b_len = b.len();
    let n = if a_len < b_len { a_len } else { b_len };
    for i in 0..n {
        if a[i] < b[i] {
            return -1;
        } else if a[i] > b[i] {
            return 1;
        }
    }
    if a_len < b_len {
        -1
    } else if a_len > b_len {
        1
    } else {
        0
    }
}
