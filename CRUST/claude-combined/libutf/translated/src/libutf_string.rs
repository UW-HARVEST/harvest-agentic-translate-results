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

fn cmp_bytes(a: &[u8], b: &[u8]) -> i32 {
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

// Free a leaked boxed slice that we previously created with `Box::leak`.
// `data` must point to a buffer of length `cap` previously leaked, or `cap` must be 0.
unsafe fn free_leaked(data: &[Utf8], cap: usize) {
    if cap > 0 {
        let ptr = data.as_ptr() as *mut u8;
        let _ = Box::from_raw(core::slice::from_raw_parts_mut(ptr, cap));
    }
}

impl<'a> Utf8String<'a> {
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
        unsafe {
            free_leaked(self.data, self.cap);
        }
        self.data = &[];
        self.len = 0;
        self.cap = 0;
    }

    pub fn reserve(&mut self, len: usize) -> Result<(), ()> {
        if len <= self.cap {
            return Ok(());
        }
        let new_cap = len.next_power_of_two();
        let mut new_buf: Vec<u8> = vec![0u8; new_cap];
        if self.len > 0 {
            new_buf[..self.len].copy_from_slice(&self.data[..self.len]);
        }
        unsafe {
            free_leaked(self.data, self.cap);
        }
        let boxed = new_buf.into_boxed_slice();
        let leaked: &'static [u8] = Box::leak(boxed);
        self.data = leaked;
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
        // Allocate new buffer of size = self.len, copy contents, free old.
        let new_cap = self.len;
        if new_cap == 0 {
            unsafe {
                free_leaked(self.data, self.cap);
            }
            self.data = &[];
            self.cap = 0;
            return Ok(());
        }
        let mut new_buf: Vec<u8> = vec![0u8; new_cap];
        new_buf[..self.len].copy_from_slice(&self.data[..self.len]);
        unsafe {
            free_leaked(self.data, self.cap);
        }
        let boxed = new_buf.into_boxed_slice();
        let leaked: &'static [u8] = Box::leak(boxed);
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
        self.append_literal(&other.data[..other.len])
    }

    pub fn append_view(&mut self, view: &Utf8StringView) -> Result<(), ()> {
        self.append_literal(&view.data[..view.len])
    }

    pub fn append_character(&mut self, c: Utf8) -> Result<(), ()> {
        self.reserve(self.len + 1)?;
        unsafe {
            let ptr = self.data.as_ptr() as *mut u8;
            *ptr.add(self.len) = c;
        }
        self.len += 1;
        Ok(())
    }

    pub fn append_literal(&mut self, literal: &[Utf8]) -> Result<(), ()> {
        let n = literal.len();
        self.reserve(self.len + n)?;
        unsafe {
            let ptr = self.data.as_ptr() as *mut u8;
            core::ptr::copy_nonoverlapping(literal.as_ptr(), ptr.add(self.len), n);
        }
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
        unsafe {
            let ptr = self.data.as_ptr() as *mut u8;
            core::ptr::copy(ptr, ptr.add(1), self.len);
            *ptr = c;
        }
        self.len += 1;
        Ok(())
    }

    pub fn prepend_literal(&mut self, literal: &[Utf8]) -> Result<(), ()> {
        let n = literal.len();
        self.reserve(self.len + n)?;
        unsafe {
            let ptr = self.data.as_ptr() as *mut u8;
            core::ptr::copy(ptr, ptr.add(n), self.len);
            core::ptr::copy_nonoverlapping(literal.as_ptr(), ptr, n);
        }
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
        unsafe {
            let ptr = self.data.as_ptr() as *mut u8;
            core::ptr::copy(ptr.add(pos), ptr.add(pos + 1), self.len - pos);
            *ptr.add(pos) = c;
        }
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
        unsafe {
            let ptr = self.data.as_ptr() as *mut u8;
            core::ptr::copy(ptr.add(pos), ptr.add(pos + n), self.len - pos);
            core::ptr::copy_nonoverlapping(literal.as_ptr(), ptr.add(pos), n);
        }
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
        let len = if pos + len > self.len {
            self.len - pos
        } else {
            len
        };
        let replaced_len = self.len + 1 - len;
        self.reserve(replaced_len)?;
        unsafe {
            let ptr = self.data.as_ptr() as *mut u8;
            core::ptr::copy(ptr.add(pos + len), ptr.add(pos + 1), self.len - pos - len);
            *ptr.add(pos) = c;
        }
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
        let len = if pos + len > self.len {
            self.len - pos
        } else {
            len
        };
        let n = literal.len();
        let replaced_len = self.len + n - len;
        self.reserve(replaced_len)?;
        unsafe {
            let ptr = self.data.as_ptr() as *mut u8;
            core::ptr::copy(ptr.add(pos + len), ptr.add(pos + n), self.len - pos - len);
            core::ptr::copy_nonoverlapping(literal.as_ptr(), ptr.add(pos), n);
        }
        self.len = replaced_len;
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
        unsafe {
            let ptr = self.data.as_ptr() as *mut u8;
            core::ptr::copy(ptr.add(pos + len), ptr.add(pos), self.len - pos - len);
        }
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
        // The Rust test treats the second argument as a length.
        let total = self.data.len();
        let start = if start > total { total } else { start };
        let avail = total - start;
        let len = if end > avail { avail } else { end };
        Utf8StringView {
            data: &self.data[start..start + len],
            len,
        }
    }

    pub fn substring_copy(&self, start: usize, end: usize) -> Result<Utf8String, ()> {
        let total = self.data.len();
        let start = if start > total { total } else { start };
        let avail = total - start;
        let len = if end > avail { avail } else { end };
        let mut result = Utf8String::new();
        result.reserve(len)?;
        unsafe {
            let dst = result.data.as_ptr() as *mut u8;
            core::ptr::copy_nonoverlapping(self.data.as_ptr().add(start), dst, len);
        }
        result.len = len;
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
        if self.len == 0 {
            return None;
        }
        let pos = if pos >= self.len { self.len - 1 } else { pos };
        let mut i = pos as isize;
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
        cmp_bytes(&self.data[..self.len], &other.data[..other.len])
    }

    pub fn compare_literal(&self, literal: &[Utf8]) -> i32 {
        cmp_bytes(&self.data[..self.len], literal)
    }

    pub fn substring(&self, start: usize, end: usize) -> Utf8StringView<'a> {
        let total = self.data.len();
        let start = if start > total { total } else { start };
        let avail = total - start;
        let len = if end > avail { avail } else { end };
        Utf8StringView {
            data: &self.data[start..start + len],
            len,
        }
    }

    pub fn substring_copy(&self, start: usize, end: usize) -> Result<Utf8String, ()> {
        let total = self.data.len();
        let start = if start > total { total } else { start };
        let avail = total - start;
        let len = if end > avail { avail } else { end };
        let mut result = Utf8String::new();
        result.reserve(len)?;
        unsafe {
            let dst = result.data.as_ptr() as *mut u8;
            core::ptr::copy_nonoverlapping(self.data.as_ptr().add(start), dst, len);
        }
        result.len = len;
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
        if self.len == 0 {
            return None;
        }
        let pos = if pos >= self.len { self.len - 1 } else { pos };
        let mut i = pos as isize;
        while i >= 0 {
            if self.data[i as usize] == c {
                return Some(i as usize);
            }
            i -= 1;
        }
        None
    }
}
