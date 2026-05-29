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

// Compute the next power of 2 >= n (for n >= 1).
// Mirrors the C macro:
//   len--; len |= len>>1; ... len++;
fn round_up_pow2(mut n: usize) -> usize {
    if n <= 1 {
        return 1;
    }
    n -= 1;
    n |= n >> 1;
    n |= n >> 2;
    n |= n >> 4;
    n |= n >> 8;
    n |= n >> 16;
    if core::mem::size_of::<usize>() == 8 {
        n |= n >> 32;
    }
    n + 1
}

// Given a Utf8String whose `data` pointer was produced by `Box::leak`-ing a
// boxed slice of size `cap`, drop that allocation.
//
// Safety: `string.data` must point to memory we own with capacity `cap`,
// and there must be no other live references to it.
unsafe fn free_buffer(data: *const u8, cap: usize) {
    if cap == 0 || data.is_null() {
        return;
    }
    // Reconstruct the Box<[u8]> we leaked and drop it.
    let ptr = data as *mut u8;
    let slice: *mut [u8] = core::ptr::slice_from_raw_parts_mut(ptr, cap);
    drop(Box::from_raw(slice));
}

impl<'a> Utf8String<'a> {
    // Return a writable view of the underlying buffer.
    //
    // Safety: caller must ensure no other references to the buffer are live
    // for the duration of the returned reference. We only call this from
    // methods that take `&mut self`, and we own the buffer (allocated via
    // Box::leak), so there is no aliasing.
    fn buffer_mut(&mut self) -> &mut [u8] {
        if self.cap == 0 {
            return &mut [];
        }
        let ptr = self.data.as_ptr() as *mut u8;
        unsafe { core::slice::from_raw_parts_mut(ptr, self.cap) }
    }
}

impl Utf8String<'_> {
    pub fn new() -> Self {
        Utf8String {
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
        unsafe {
            free_buffer(self.data.as_ptr(), self.cap);
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

        // Allocate a new buffer, copy over current contents.
        let mut vec: Vec<u8> = vec![0u8; new_cap];
        if self.len > 0 {
            vec[..self.len].copy_from_slice(&self.data[..self.len]);
        }
        let boxed: Box<[u8]> = vec.into_boxed_slice();
        let leaked: &'static mut [u8] = Box::leak(boxed);
        let new_ptr = leaked.as_ptr();

        // Free the old buffer (if any).
        unsafe {
            free_buffer(self.data.as_ptr(), self.cap);
        }

        // Install the new buffer.
        self.data = unsafe { core::slice::from_raw_parts(new_ptr, new_cap) };
        self.cap = new_cap;
        Ok(())
    }

    pub fn shrink_to_fit(&mut self) -> Result<(), ()> {
        if self.cap == 0 || self.cap == self.len {
            return Ok(());
        }

        let new_cap = self.len;

        if new_cap == 0 {
            // Free buffer entirely.
            unsafe {
                free_buffer(self.data.as_ptr(), self.cap);
            }
            self.data = &[];
            self.cap = 0;
            return Ok(());
        }

        let mut vec: Vec<u8> = vec![0u8; new_cap];
        vec[..self.len].copy_from_slice(&self.data[..self.len]);
        let boxed: Box<[u8]> = vec.into_boxed_slice();
        let leaked: &'static mut [u8] = Box::leak(boxed);
        let new_ptr = leaked.as_ptr();

        unsafe {
            free_buffer(self.data.as_ptr(), self.cap);
        }

        self.data = unsafe { core::slice::from_raw_parts(new_ptr, new_cap) };
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
        let cur_len = self.len;
        let buf = self.buffer_mut();
        buf[cur_len..cur_len + other_len].copy_from_slice(&other.data[..other_len]);
        self.len = new_len;
        Ok(())
    }

    pub fn append_view(&mut self, view: &Utf8StringView) -> Result<(), ()> {
        let other_len = view.len;
        let new_len = self.len + other_len;
        self.reserve(new_len)?;
        let cur_len = self.len;
        let buf = self.buffer_mut();
        buf[cur_len..cur_len + other_len].copy_from_slice(&view.data[..other_len]);
        self.len = new_len;
        Ok(())
    }

    pub fn append_character(&mut self, c: Utf8) -> Result<(), ()> {
        let new_len = self.len + 1;
        self.reserve(new_len)?;
        let cur_len = self.len;
        let buf = self.buffer_mut();
        buf[cur_len] = c;
        self.len = new_len;
        Ok(())
    }

    pub fn append_literal(&mut self, literal: &[Utf8]) -> Result<(), ()> {
        let n = literal.len();
        let new_len = self.len + n;
        self.reserve(new_len)?;
        let cur_len = self.len;
        let buf = self.buffer_mut();
        buf[cur_len..cur_len + n].copy_from_slice(literal);
        self.len = new_len;
        Ok(())
    }

    pub fn prepend(&mut self, other: &Utf8String) -> Result<(), ()> {
        let other_len = other.len;
        let new_len = self.len + other_len;
        self.reserve(new_len)?;
        let cur_len = self.len;
        // We need to copy other.data first since it might alias buffer in pathological cases,
        // but here other is a different string object, so just do safe ops.
        // However, `other.data` may share the buffer if same string -- C version uses memmove.
        // We'll be safe: capture other.data into a temporary if necessary.
        let other_bytes: Vec<u8> = other.data[..other_len].to_vec();
        let buf = self.buffer_mut();
        buf.copy_within(0..cur_len, other_len);
        buf[..other_len].copy_from_slice(&other_bytes);
        self.len = new_len;
        Ok(())
    }

    pub fn prepend_view(&mut self, view: &Utf8StringView) -> Result<(), ()> {
        let other_len = view.len;
        let new_len = self.len + other_len;
        self.reserve(new_len)?;
        let cur_len = self.len;
        let other_bytes: Vec<u8> = view.data[..other_len].to_vec();
        let buf = self.buffer_mut();
        buf.copy_within(0..cur_len, other_len);
        buf[..other_len].copy_from_slice(&other_bytes);
        self.len = new_len;
        Ok(())
    }

    pub fn prepend_character(&mut self, c: Utf8) -> Result<(), ()> {
        let new_len = self.len + 1;
        self.reserve(new_len)?;
        let cur_len = self.len;
        let buf = self.buffer_mut();
        buf.copy_within(0..cur_len, 1);
        buf[0] = c;
        self.len = new_len;
        Ok(())
    }

    pub fn prepend_literal(&mut self, literal: &[Utf8]) -> Result<(), ()> {
        let n = literal.len();
        let new_len = self.len + n;
        self.reserve(new_len)?;
        let cur_len = self.len;
        let lit_copy: Vec<u8> = literal.to_vec();
        let buf = self.buffer_mut();
        buf.copy_within(0..cur_len, n);
        buf[..n].copy_from_slice(&lit_copy);
        self.len = new_len;
        Ok(())
    }

    pub fn insert(&mut self, pos: usize, other: &Utf8String) -> Result<(), ()> {
        if pos > self.len {
            return Err(());
        }
        let other_len = other.len;
        let new_len = self.len + other_len;
        self.reserve(new_len)?;
        let cur_len = self.len;
        let other_bytes: Vec<u8> = other.data[..other_len].to_vec();
        let buf = self.buffer_mut();
        buf.copy_within(pos..cur_len, pos + other_len);
        buf[pos..pos + other_len].copy_from_slice(&other_bytes);
        self.len = new_len;
        Ok(())
    }

    pub fn insert_view(&mut self, pos: usize, view: &Utf8StringView) -> Result<(), ()> {
        if pos > self.len {
            return Err(());
        }
        let other_len = view.len;
        let new_len = self.len + other_len;
        self.reserve(new_len)?;
        let cur_len = self.len;
        let other_bytes: Vec<u8> = view.data[..other_len].to_vec();
        let buf = self.buffer_mut();
        buf.copy_within(pos..cur_len, pos + other_len);
        buf[pos..pos + other_len].copy_from_slice(&other_bytes);
        self.len = new_len;
        Ok(())
    }

    pub fn insert_character(&mut self, pos: usize, c: Utf8) -> Result<(), ()> {
        if pos > self.len {
            return Err(());
        }
        let new_len = self.len + 1;
        self.reserve(new_len)?;
        let cur_len = self.len;
        let buf = self.buffer_mut();
        buf.copy_within(pos..cur_len, pos + 1);
        buf[pos] = c;
        self.len = new_len;
        Ok(())
    }

    pub fn insert_literal(&mut self, pos: usize, literal: &[Utf8]) -> Result<(), ()> {
        if pos > self.len {
            return Err(());
        }
        let n = literal.len();
        let new_len = self.len + n;
        self.reserve(new_len)?;
        let cur_len = self.len;
        let lit_copy: Vec<u8> = literal.to_vec();
        let buf = self.buffer_mut();
        buf.copy_within(pos..cur_len, pos + n);
        buf[pos..pos + n].copy_from_slice(&lit_copy);
        self.len = new_len;
        Ok(())
    }

    pub fn replace(
        &mut self,
        pos: usize,
        mut len: usize,
        replacement: &Utf8String,
    ) -> Result<(), ()> {
        if pos > self.len {
            return Err(());
        }
        if pos + len > self.len {
            len = self.len - pos;
        }
        let rep_len = replacement.len;
        let new_len = self.len + rep_len - len;
        self.reserve(new_len)?;
        let cur_len = self.len;
        let rep_bytes: Vec<u8> = replacement.data[..rep_len].to_vec();
        let buf = self.buffer_mut();
        let tail_start = pos + len;
        let tail_end = cur_len;
        buf.copy_within(tail_start..tail_end, pos + rep_len);
        buf[pos..pos + rep_len].copy_from_slice(&rep_bytes);
        self.len = new_len;
        Ok(())
    }

    pub fn replace_view(
        &mut self,
        pos: usize,
        mut len: usize,
        replacement: &Utf8StringView,
    ) -> Result<(), ()> {
        if pos > self.len {
            return Err(());
        }
        if pos + len > self.len {
            len = self.len - pos;
        }
        let rep_len = replacement.len;
        let new_len = self.len + rep_len - len;
        self.reserve(new_len)?;
        let cur_len = self.len;
        let rep_bytes: Vec<u8> = replacement.data[..rep_len].to_vec();
        let buf = self.buffer_mut();
        let tail_start = pos + len;
        let tail_end = cur_len;
        buf.copy_within(tail_start..tail_end, pos + rep_len);
        buf[pos..pos + rep_len].copy_from_slice(&rep_bytes);
        self.len = new_len;
        Ok(())
    }

    pub fn replace_character(&mut self, pos: usize, len: usize, c: Utf8) -> Result<(), ()> {
        if pos > self.len {
            return Err(());
        }
        // Note: matches C semantics; does not clamp `len` like the slice/literal variants.
        let new_len = self.len + 1 - len;
        self.reserve(new_len)?;
        let cur_len = self.len;
        let buf = self.buffer_mut();
        let tail_start = pos + len;
        let tail_end = cur_len;
        buf.copy_within(tail_start..tail_end, pos + 1);
        buf[pos] = c;
        self.len = new_len;
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
        let new_len = self.len + n - len;
        self.reserve(new_len)?;
        let cur_len = self.len;
        let lit_copy: Vec<u8> = literal.to_vec();
        let buf = self.buffer_mut();
        let tail_start = pos + len;
        let tail_end = cur_len;
        buf.copy_within(tail_start..tail_end, pos + n);
        buf[pos..pos + n].copy_from_slice(&lit_copy);
        self.len = new_len;
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
        let buf = self.buffer_mut();
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
        compare_bytes(&self.data[..self.len], &other.data[..other.len])
    }

    pub fn compare_literal(&self, literal: &[Utf8]) -> i32 {
        compare_bytes(&self.data[..self.len], literal)
    }

    pub fn substring(&self, start: usize, end: usize) -> Utf8StringView {
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
        let len = e - s;
        let mut result = Utf8String::new();
        result.reserve(len)?;
        if len > 0 {
            let buf = result.buffer_mut();
            buf[..len].copy_from_slice(&self.data[s..e]);
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
        let mut i = if pos == usize::MAX {
            self.len - 1
        } else if pos >= self.len {
            return None;
        } else {
            pos
        };
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
        let len = e - s;
        let mut result = Utf8String::new();
        result.reserve(len)?;
        if len > 0 {
            let buf = result.buffer_mut();
            buf[..len].copy_from_slice(&self.data[s..e]);
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
        let mut i = if pos == usize::MAX {
            self.len - 1
        } else if pos >= self.len {
            return None;
        } else {
            pos
        };
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

// Implements the C `strncmp` + length tie-break behavior used by all
// `*_compare` functions in the original code:
//   - compare bytes up to min(a_len, b_len)
//   - on equal prefix, the shorter one compares less
fn compare_bytes(a: &[u8], b: &[u8]) -> i32 {
    let n = a.len().min(b.len());
    for i in 0..n {
        if a[i] != b[i] {
            // C strncmp returns negative/positive based on byte difference.
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
