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

// Helpers for buffer management.
//
// The struct's `data` field is a shared reference, so we use `Box::leak` to
// hand out a `'static` slice for the backing storage. The `cap` field stores
// the allocation length, allowing us to reconstruct and drop the `Box` in
// `destroy`. While the buffer is owned by the string, we briefly obtain a
// mutable view into it via raw pointers so we can modify its contents in
// place (matching the C behavior).

fn alloc_buffer(cap: usize) -> &'static mut [u8] {
    let v: Vec<u8> = vec![0u8; cap];
    Box::leak(v.into_boxed_slice())
}

fn free_buffer(ptr: *const u8, cap: usize) {
    if cap == 0 {
        return;
    }
    // SAFETY: `ptr` was obtained from `Box::leak` of a `Box<[u8]>` of length
    // `cap`, and we have not handed out any other references to this region
    // (it is only reachable through the `Utf8String` we are dropping the
    // backing for).
    unsafe {
        let raw = std::slice::from_raw_parts_mut(ptr as *mut u8, cap);
        drop(Box::from_raw(raw as *mut [u8]));
    }
}

impl<'a> Utf8String<'a> {
    fn buf_mut(&mut self) -> &mut [u8] {
        let ptr = self.data.as_ptr() as *mut u8;
        let cap = self.cap;
        // SAFETY: We have `&mut self`, so no other references to the backing
        // buffer exist. The buffer is exactly `cap` bytes long. If `cap == 0`,
        // we return an empty slice; in that case `ptr` may be a dangling but
        // properly aligned pointer (as produced for empty slices), and an
        // empty `from_raw_parts_mut` is sound.
        unsafe { std::slice::from_raw_parts_mut(ptr, cap) }
    }

    fn ensure_cap(&mut self, needed: usize) -> Result<(), ()> {
        if needed <= self.cap {
            return Ok(());
        }
        let new_cap = needed.next_power_of_two();
        let new_buf = alloc_buffer(new_cap);

        // Copy any existing data over.
        if self.len > 0 {
            new_buf[..self.len].copy_from_slice(&self.data[..self.len]);
        }

        // Free old buffer if any.
        let old_ptr = self.data.as_ptr();
        let old_cap = self.cap;

        // Reborrow so we can downgrade to a shared slice with arbitrary
        // lifetime (the buffer is leaked and effectively `'static`).
        let new_shared: &'static [u8] = &*new_buf;
        self.data = new_shared;
        self.cap = new_cap;

        free_buffer(old_ptr, old_cap);
        Ok(())
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
        let ptr = self.data.as_ptr();
        let cap = self.cap;
        self.data = &[];
        self.cap = 0;
        self.len = 0;
        free_buffer(ptr, cap);
    }

    pub fn reserve(&mut self, len: usize) -> Result<(), ()> {
        self.ensure_cap(len)
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
            // Just free.
            let ptr = self.data.as_ptr();
            let cap = self.cap;
            self.data = &[];
            self.cap = 0;
            free_buffer(ptr, cap);
            return Ok(());
        }
        let new_buf = alloc_buffer(new_cap);
        new_buf[..self.len].copy_from_slice(&self.data[..self.len]);
        let old_ptr = self.data.as_ptr();
        let old_cap = self.cap;
        let new_shared: &'static [u8] = &*new_buf;
        self.data = new_shared;
        self.cap = new_cap;
        free_buffer(old_ptr, old_cap);
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
        self.ensure_cap(self.len + other_len)?;
        // Need to copy other's bytes into our buffer. Since `other` may alias
        // self if it's the same string, we copy via a separate temporary if
        // needed. Here we assume distinct strings (matches C semantics where
        // memcpy is used).
        let other_data: Vec<u8> = other.data[..other_len].to_vec();
        let len = self.len;
        let buf = self.buf_mut();
        buf[len..len + other_len].copy_from_slice(&other_data);
        self.len += other_len;
        Ok(())
    }

    pub fn append_view(&mut self, view: &Utf8StringView) -> Result<(), ()> {
        let view_len = view.len;
        self.ensure_cap(self.len + view_len)?;
        let view_data: Vec<u8> = view.data[..view_len].to_vec();
        let len = self.len;
        let buf = self.buf_mut();
        buf[len..len + view_len].copy_from_slice(&view_data);
        self.len += view_len;
        Ok(())
    }

    pub fn append_character(&mut self, c: Utf8) -> Result<(), ()> {
        self.ensure_cap(self.len + 1)?;
        let len = self.len;
        let buf = self.buf_mut();
        buf[len] = c;
        self.len += 1;
        Ok(())
    }

    pub fn append_literal(&mut self, literal: &[Utf8]) -> Result<(), ()> {
        let n = literal.len();
        self.ensure_cap(self.len + n)?;
        let lit: Vec<u8> = literal.to_vec();
        let len = self.len;
        let buf = self.buf_mut();
        buf[len..len + n].copy_from_slice(&lit);
        self.len += n;
        Ok(())
    }

    pub fn prepend(&mut self, other: &Utf8String) -> Result<(), ()> {
        let other_len = other.len;
        self.ensure_cap(self.len + other_len)?;
        let other_data: Vec<u8> = other.data[..other_len].to_vec();
        let len = self.len;
        let buf = self.buf_mut();
        buf.copy_within(0..len, other_len);
        buf[..other_len].copy_from_slice(&other_data);
        self.len += other_len;
        Ok(())
    }

    pub fn prepend_view(&mut self, view: &Utf8StringView) -> Result<(), ()> {
        let view_len = view.len;
        self.ensure_cap(self.len + view_len)?;
        let view_data: Vec<u8> = view.data[..view_len].to_vec();
        let len = self.len;
        let buf = self.buf_mut();
        buf.copy_within(0..len, view_len);
        buf[..view_len].copy_from_slice(&view_data);
        self.len += view_len;
        Ok(())
    }

    pub fn prepend_character(&mut self, c: Utf8) -> Result<(), ()> {
        self.ensure_cap(self.len + 1)?;
        let len = self.len;
        let buf = self.buf_mut();
        buf.copy_within(0..len, 1);
        buf[0] = c;
        self.len += 1;
        Ok(())
    }

    pub fn prepend_literal(&mut self, literal: &[Utf8]) -> Result<(), ()> {
        let n = literal.len();
        self.ensure_cap(self.len + n)?;
        let lit: Vec<u8> = literal.to_vec();
        let len = self.len;
        let buf = self.buf_mut();
        buf.copy_within(0..len, n);
        buf[..n].copy_from_slice(&lit);
        self.len += n;
        Ok(())
    }

    pub fn insert(&mut self, pos: usize, other: &Utf8String) -> Result<(), ()> {
        if pos > self.len {
            return Err(());
        }
        let other_len = other.len;
        let inserted_len = self.len + other_len;
        self.ensure_cap(inserted_len)?;
        let other_data: Vec<u8> = other.data[..other_len].to_vec();
        let len = self.len;
        let buf = self.buf_mut();
        buf.copy_within(pos..len, pos + other_len);
        buf[pos..pos + other_len].copy_from_slice(&other_data);
        self.len = inserted_len;
        Ok(())
    }

    pub fn insert_view(&mut self, pos: usize, view: &Utf8StringView) -> Result<(), ()> {
        if pos > self.len {
            return Err(());
        }
        let view_len = view.len;
        let inserted_len = self.len + view_len;
        self.ensure_cap(inserted_len)?;
        let view_data: Vec<u8> = view.data[..view_len].to_vec();
        let len = self.len;
        let buf = self.buf_mut();
        buf.copy_within(pos..len, pos + view_len);
        buf[pos..pos + view_len].copy_from_slice(&view_data);
        self.len = inserted_len;
        Ok(())
    }

    pub fn insert_character(&mut self, pos: usize, c: Utf8) -> Result<(), ()> {
        if pos > self.len {
            return Err(());
        }
        let inserted_len = self.len + 1;
        self.ensure_cap(inserted_len)?;
        let len = self.len;
        let buf = self.buf_mut();
        buf.copy_within(pos..len, pos + 1);
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
        self.ensure_cap(inserted_len)?;
        let lit: Vec<u8> = literal.to_vec();
        let len = self.len;
        let buf = self.buf_mut();
        buf.copy_within(pos..len, pos + n);
        buf[pos..pos + n].copy_from_slice(&lit);
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
        self.ensure_cap(replaced_len)?;
        let rep_data: Vec<u8> = replacement.data[..rep_len].to_vec();
        let cur_len = self.len;
        let buf = self.buf_mut();
        // Move the tail after the removed segment.
        let tail_src_start = pos + len;
        let tail_len = cur_len - pos - len;
        buf.copy_within(tail_src_start..tail_src_start + tail_len, pos + rep_len);
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
        let replaced_len = self.len + rep_len - len;
        self.ensure_cap(replaced_len)?;
        let rep_data: Vec<u8> = replacement.data[..rep_len].to_vec();
        let cur_len = self.len;
        let buf = self.buf_mut();
        let tail_src_start = pos + len;
        let tail_len = cur_len - pos - len;
        buf.copy_within(tail_src_start..tail_src_start + tail_len, pos + rep_len);
        buf[pos..pos + rep_len].copy_from_slice(&rep_data);
        self.len = replaced_len;
        Ok(())
    }

    pub fn replace_character(&mut self, pos: usize, len: usize, c: Utf8) -> Result<(), ()> {
        if pos > self.len {
            return Err(());
        }
        // Note: matching C, no clamp on len here for character replacement.
        let replaced_len = self.len + 1 - len;
        self.ensure_cap(replaced_len)?;
        let cur_len = self.len;
        let buf = self.buf_mut();
        let tail_src_start = pos + len;
        let tail_len = cur_len - pos - len;
        buf.copy_within(tail_src_start..tail_src_start + tail_len, pos + 1);
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
        self.ensure_cap(replaced_len)?;
        let lit: Vec<u8> = literal.to_vec();
        let cur_len = self.len;
        let buf = self.buf_mut();
        let tail_src_start = pos + len;
        let tail_len = cur_len - pos - len;
        buf.copy_within(tail_src_start..tail_src_start + tail_len, pos + n);
        buf[pos..pos + n].copy_from_slice(&lit);
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
        let tail_src_start = pos + len;
        let tail_len = cur_len - pos - len;
        buf.copy_within(tail_src_start..tail_src_start + tail_len, pos);
        self.len -= len;
        Ok(())
    }

    pub fn concat(&self, other: &Utf8String) -> Result<Utf8String, ()> {
        let mut result = Utf8String::new();
        result.ensure_cap(self.len + other.len)?;
        result.append(self)?;
        result.append(other)?;
        Ok(result)
    }

    pub fn concat_view(&self, other: &Utf8StringView) -> Result<Utf8String, ()> {
        let mut result = Utf8String::new();
        result.ensure_cap(self.len + other.len)?;
        result.append(self)?;
        result.append_view(other)?;
        Ok(result)
    }

    pub fn concat_character(&self, c: Utf8) -> Result<Utf8String, ()> {
        let mut result = Utf8String::new();
        result.ensure_cap(self.len + 1)?;
        result.append(self)?;
        result.append_character(c)?;
        Ok(result)
    }

    pub fn concat_literal(&self, literal: &[Utf8]) -> Result<Utf8String, ()> {
        let mut result = Utf8String::new();
        result.ensure_cap(self.len + literal.len())?;
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
        let mut end = end;
        if end > self.len {
            end = self.len;
        }
        let mut start = start;
        if start > end {
            start = end;
        }
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
        let mut start = start;
        if start > end {
            start = end;
        }
        let len = end - start;
        let mut result = Utf8String::new();
        result.ensure_cap(len)?;
        if len > 0 {
            // Copy from self.data into result's buffer.
            let src: Vec<u8> = self.data[start..end].to_vec();
            let buf = result.buf_mut();
            buf[..len].copy_from_slice(&src);
        }
        result.len = len;
        Ok(result)
    }

    pub fn index_of_character(&self, pos: usize, c: Utf8) -> Option<usize> {
        let n = self.len;
        for i in pos..n {
            if self.data[i] == c {
                return Some(i);
            }
        }
        None
    }

    pub fn last_index_of_character(&self, pos: usize, c: Utf8) -> Option<usize> {
        let mut pos = pos;
        if pos == usize::MAX {
            if self.len == 0 {
                return None;
            }
            pos = self.len - 1;
        } else if pos >= self.len {
            return None;
        }
        // Iterate from pos down to 0, inclusive.
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
        let mut end = end;
        if end > self.len {
            end = self.len;
        }
        let mut start = start;
        if start > end {
            start = end;
        }
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
        let mut start = start;
        if start > end {
            start = end;
        }
        let len = end - start;
        let mut result = Utf8String::new();
        result.ensure_cap(len)?;
        if len > 0 {
            let src: Vec<u8> = self.data[start..end].to_vec();
            let buf = result.buf_mut();
            buf[..len].copy_from_slice(&src);
        }
        result.len = len;
        Ok(result)
    }

    pub fn index_of_character(&self, pos: usize, c: Utf8) -> Option<usize> {
        let n = self.len;
        for i in pos..n {
            if self.data[i] == c {
                return Some(i);
            }
        }
        None
    }

    pub fn last_index_of_character(&self, pos: usize, c: Utf8) -> Option<usize> {
        let mut pos = pos;
        if pos == usize::MAX {
            if self.len == 0 {
                return None;
            }
            pos = self.len - 1;
        } else if pos >= self.len {
            return None;
        }
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
    let a_len = a.len();
    let b_len = b.len();
    let n = a_len.min(b_len);
    for i in 0..n {
        let av = a[i];
        let bv = b[i];
        if av != bv {
            return (av as i32) - (bv as i32);
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
