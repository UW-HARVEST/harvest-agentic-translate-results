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

// Helper: allocate a new Vec of given capacity, leak it, return static slice
fn alloc_buf(cap: usize) -> &'static [Utf8] {
    let v = vec![0u8; cap];
    let boxed = v.into_boxed_slice();
    Box::leak(boxed)
}

// Helper: get mutable access to the underlying data
fn data_mut<'a>(s: &'a Utf8String<'a>) -> &'a mut [Utf8] {
    let ptr = s.data.as_ptr() as *mut Utf8;
    unsafe { std::slice::from_raw_parts_mut(ptr, s.cap) }
}

fn round_up_pow2(mut n: usize) -> usize {
    n = n.wrapping_sub(1);
    n |= n >> 1;
    n |= n >> 2;
    n |= n >> 4;
    n |= n >> 8;
    n |= n >> 16;
    if std::mem::size_of::<usize>() == 8 {
        n |= n >> 32;
    }
    n.wrapping_add(1)
}

impl Utf8String<'_> {
    pub fn new() -> Self {
        Utf8String { data: &[], len: 0, cap: 0 }
    }

    pub fn init(&mut self) {
        self.data = &[];
        self.len = 0;
        self.cap = 0;
    }

    pub fn destroy(&mut self) {
        if self.cap > 0 {
            let ptr = self.data.as_ptr() as *mut Utf8;
            unsafe {
                let _ = Box::from_raw(std::slice::from_raw_parts_mut(ptr, self.cap));
            }
        }
        self.data = &[];
        self.len = 0;
        self.cap = 0;
    }

    pub fn reserve(&mut self, len: usize) -> Result<(), ()> {
        if len <= self.cap { return Ok(()); }
        let new_cap = round_up_pow2(len);
        let new_buf = alloc_buf(new_cap);
        let dst = new_buf.as_ptr() as *mut Utf8;
        if self.len > 0 {
            unsafe {
                std::ptr::copy_nonoverlapping(self.data.as_ptr(), dst, self.len);
            }
        }
        if self.cap > 0 {
            let old_ptr = self.data.as_ptr() as *mut Utf8;
            unsafe {
                let _ = Box::from_raw(std::slice::from_raw_parts_mut(old_ptr, self.cap));
            }
        }
        self.data = new_buf;
        self.cap = new_cap;
        Ok(())
    }

    pub fn shrink_to_fit(&mut self) -> Result<(), ()> {
        if self.cap == 0 { return Ok(()); }
        let new_cap = self.len;
        if new_cap == 0 {
            if self.cap > 0 {
                let ptr = self.data.as_ptr() as *mut Utf8;
                unsafe {
                    let _ = Box::from_raw(std::slice::from_raw_parts_mut(ptr, self.cap));
                }
            }
            self.data = &[];
            self.cap = 0;
            return Ok(());
        }
        let new_buf = alloc_buf(new_cap);
        let dst = new_buf.as_ptr() as *mut Utf8;
        unsafe {
            std::ptr::copy_nonoverlapping(self.data.as_ptr(), dst, self.len);
        }
        if self.cap > 0 {
            let old_ptr = self.data.as_ptr() as *mut Utf8;
            unsafe {
                let _ = Box::from_raw(std::slice::from_raw_parts_mut(old_ptr, self.cap));
            }
        }
        self.data = new_buf;
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
        data_mut(self)[self.len] = c;
        self.len += 1;
        Ok(())
    }

    pub fn append_literal(&mut self, literal: &[Utf8]) -> Result<(), ()> {
        let n = literal.len();
        self.reserve(self.len + n)?;
        let buf = data_mut(self);
        buf[self.len..self.len + n].copy_from_slice(literal);
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
        let buf = data_mut(self);
        buf.copy_within(0..self.len, 1);
        buf[0] = c;
        self.len += 1;
        Ok(())
    }

    pub fn prepend_literal(&mut self, literal: &[Utf8]) -> Result<(), ()> {
        let n = literal.len();
        self.reserve(self.len + n)?;
        let buf = data_mut(self);
        buf.copy_within(0..self.len, n);
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
        if pos > self.len { return Err(()); }
        self.reserve(self.len + 1)?;
        let buf = data_mut(self);
        buf.copy_within(pos..self.len, pos + 1);
        buf[pos] = c;
        self.len += 1;
        Ok(())
    }

    pub fn insert_literal(&mut self, pos: usize, literal: &[Utf8]) -> Result<(), ()> {
        if pos > self.len { return Err(()); }
        let n = literal.len();
        self.reserve(self.len + n)?;
        let buf = data_mut(self);
        buf.copy_within(pos..self.len, pos + n);
        buf[pos..pos + n].copy_from_slice(literal);
        self.len += n;
        Ok(())
    }

    pub fn replace(&mut self, pos: usize, len: usize, replacement: &Utf8String) -> Result<(), ()> {
        self.replace_literal(pos, len, &replacement.data[..replacement.len])
    }

    pub fn replace_view(&mut self, pos: usize, len: usize, replacement: &Utf8StringView) -> Result<(), ()> {
        self.replace_literal(pos, len, &replacement.data[..replacement.len])
    }

    pub fn replace_character(&mut self, pos: usize, mut len: usize, c: Utf8) -> Result<(), ()> {
        if pos > self.len { return Err(()); }
        if pos + len > self.len { len = self.len - pos; }
        let new_len = self.len + 1 - len;
        self.reserve(new_len)?;
        let buf = data_mut(self);
        buf.copy_within(pos + len..self.len, pos + 1);
        buf[pos] = c;
        self.len = new_len;
        Ok(())
    }

    pub fn replace_literal(&mut self, pos: usize, mut len: usize, literal: &[Utf8]) -> Result<(), ()> {
        if pos > self.len { return Err(()); }
        if pos + len > self.len { len = self.len - pos; }
        let n = literal.len();
        let new_len = self.len + n - len;
        self.reserve(new_len)?;
        let buf = data_mut(self);
        buf.copy_within(pos + len..self.len, pos + n);
        buf[pos..pos + n].copy_from_slice(literal);
        self.len = new_len;
        Ok(())
    }

    pub fn erase(&mut self, pos: usize, mut len: usize) -> Result<(), ()> {
        if pos > self.len { return Err(()); }
        if pos + len > self.len { len = self.len - pos; }
        let buf = data_mut(self);
        buf.copy_within(pos + len..self.len, pos);
        self.len -= len;
        Ok(())
    }

    pub fn concat(&self, other: &Utf8String) -> Result<Utf8String, ()> {
        self.concat_literal(&other.data[..other.len])
    }

    pub fn concat_view(&self, other: &Utf8StringView) -> Result<Utf8String, ()> {
        self.concat_literal(&other.data[..other.len])
    }

    pub fn concat_character(&self, c: Utf8) -> Result<Utf8String, ()> {
        let mut result = Utf8String::new();
        result.reserve(self.len + 1)?;
        result.append_literal(&self.data[..self.len]).unwrap();
        result.append_character(c).unwrap();
        Ok(result)
    }

    pub fn concat_literal(&self, literal: &[Utf8]) -> Result<Utf8String, ()> {
        let mut result = Utf8String::new();
        result.reserve(self.len + literal.len())?;
        result.append_literal(&self.data[..self.len]).unwrap();
        result.append_literal(literal).unwrap();
        Ok(result)
    }

    pub fn compare(&self, other: &Utf8String) -> i32 {
        compare_slices(&self.data[..self.len], &other.data[..other.len])
    }

    pub fn compare_literal(&self, literal: &[Utf8]) -> i32 {
        compare_slices(&self.data[..self.len], literal)
    }

    pub fn substring(&self, start: usize, end: usize) -> Utf8StringView {
        let mut e = end;
        if e == usize::MAX || e > self.len { e = self.len; }
        let s = if start > e { e } else { start };
        Utf8StringView { data: &self.data[s..e], len: e - s }
    }

    pub fn substring_copy(&self, start: usize, end: usize) -> Result<Utf8String, ()> {
        let mut e = end;
        if e == usize::MAX || e > self.len { e = self.len; }
        let s = if start > e { e } else { start };
        let slice = &self.data[s..e];
        let mut result = Utf8String::new();
        result.reserve(slice.len())?;
        result.append_literal(slice).unwrap();
        Ok(result)
    }

    pub fn index_of_character(&self, pos: usize, c: Utf8) -> Option<usize> {
        for i in pos..self.len {
            if self.data[i] == c { return Some(i); }
        }
        None
    }

    pub fn last_index_of_character(&self, pos: usize, c: Utf8) -> Option<usize> {
        let p = if pos == usize::MAX {
            if self.len == 0 { return None; }
            self.len - 1
        } else if pos >= self.len {
            return None;
        } else {
            pos
        };
        for i in (0..=p).rev() {
            if self.data[i] == c { return Some(i); }
        }
        None
    }
}

fn compare_slices(a: &[Utf8], b: &[Utf8]) -> i32 {
    let min_len = a.len().min(b.len());
    for i in 0..min_len {
        if a[i] != b[i] {
            return if (a[i] as i8) < (b[i] as i8) { -1 } else { 1 };
        }
    }
    if a.len() < b.len() { -1 }
    else if a.len() > b.len() { 1 }
    else { 0 }
}

impl<'a> Utf8StringView<'a> {
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn compare(&self, other: &Utf8StringView) -> i32 {
        compare_slices(&self.data[..self.len], &other.data[..other.len])
    }

    pub fn compare_literal(&self, literal: &[Utf8]) -> i32 {
        compare_slices(&self.data[..self.len], literal)
    }

    pub fn substring(&self, start: usize, end: usize) -> Utf8StringView<'a> {
        let mut e = end;
        if e == usize::MAX || e > self.len { e = self.len; }
        let s = if start > e { e } else { start };
        Utf8StringView { data: &self.data[s..e], len: e - s }
    }

    pub fn substring_copy(&self, start: usize, end: usize) -> Result<Utf8String, ()> {
        let mut e = end;
        if e == usize::MAX || e > self.len { e = self.len; }
        let s = if start > e { e } else { start };
        let slice = &self.data[s..e];
        let mut result = Utf8String::new();
        result.reserve(slice.len())?;
        let buf = data_mut(&result);
        buf[..slice.len()].copy_from_slice(slice);
        result.len = slice.len();
        Ok(result)
    }

    pub fn index_of_character(&self, pos: usize, c: Utf8) -> Option<usize> {
        for i in pos..self.len {
            if self.data[i] == c { return Some(i); }
        }
        None
    }

    pub fn last_index_of_character(&self, pos: usize, c: Utf8) -> Option<usize> {
        let p = if pos == usize::MAX {
            if self.len == 0 { return None; }
            self.len - 1
        } else if pos >= self.len {
            return None;
        } else {
            pos
        };
        for i in (0..=p).rev() {
            if self.data[i] == c { return Some(i); }
        }
        None
    }
}
