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

fn alloc_buf(cap: usize) -> *mut Utf8 {
    let mut v = Vec::<Utf8>::with_capacity(cap);
    let ptr = v.as_mut_ptr();
    std::mem::forget(v);
    ptr
}

fn realloc_buf(old_ptr: *mut Utf8, old_cap: usize, new_cap: usize) -> *mut Utf8 {
    let mut v = unsafe { Vec::from_raw_parts(old_ptr, 0, old_cap) };
    v.reserve_exact(new_cap.saturating_sub(old_cap));
    let ptr = v.as_mut_ptr();
    let actual_cap = v.capacity();
    std::mem::forget(v);
    let _ = actual_cap;
    ptr
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

fn ptr_from_slice(s: &[Utf8]) -> *mut Utf8 {
    s.as_ptr() as *mut Utf8
}

fn make_slice<'a>(ptr: *const Utf8, cap: usize) -> &'a [Utf8] {
    if cap == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(ptr, cap) }
    }
}

impl Utf8String<'_> {
    pub fn new() -> Self {
        let cap = 8usize;
        let ptr = alloc_buf(cap);
        Utf8String {
            data: make_slice(ptr, cap),
            len: 0,
            cap,
        }
    }

    pub fn init(&mut self) {
        let cap = 8usize;
        let ptr = alloc_buf(cap);
        self.data = make_slice(ptr, cap);
        self.len = 0;
        self.cap = cap;
    }

    pub fn destroy(&mut self) {
        if self.cap > 0 {
            let ptr = ptr_from_slice(self.data);
            unsafe { let _ = Vec::from_raw_parts(ptr, 0, self.cap); }
        }
        self.data = &[];
        self.len = 0;
        self.cap = 0;
    }

    pub fn reserve(&mut self, len: usize) -> Result<(), ()> {
        if len <= self.cap { return Ok(()); }
        let new_cap = round_up_pow2(len);
        let old_ptr = ptr_from_slice(self.data);
        let new_ptr = alloc_buf(new_cap);
        unsafe {
            std::ptr::copy_nonoverlapping(old_ptr, new_ptr, self.len);
            let _ = Vec::from_raw_parts(old_ptr, 0, self.cap);
        }
        self.data = make_slice(new_ptr, new_cap);
        self.cap = new_cap;
        Ok(())
    }

    pub fn shrink_to_fit(&mut self) -> Result<(), ()> {
        let new_cap = self.len;
        if new_cap >= self.cap { return Ok(()); }
        if new_cap == 0 {
            let old_ptr = ptr_from_slice(self.data);
            unsafe { let _ = Vec::from_raw_parts(old_ptr, 0, self.cap); }
            let ptr = alloc_buf(0);
            self.data = make_slice(ptr, 0);
            self.cap = 0;
            return Ok(());
        }
        let old_ptr = ptr_from_slice(self.data);
        let new_ptr = alloc_buf(new_cap);
        unsafe {
            std::ptr::copy_nonoverlapping(old_ptr, new_ptr, self.len);
            let _ = Vec::from_raw_parts(old_ptr, 0, self.cap);
        }
        self.data = make_slice(new_ptr, new_cap);
        self.cap = new_cap;
        Ok(())
    }

    pub fn clear(&mut self) {
        self.len = 0;
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    fn data_ptr(&self) -> *mut Utf8 {
        ptr_from_slice(self.data)
    }

    pub fn append(&mut self, other: &Utf8String) -> Result<(), ()> {
        self.reserve(self.len + other.len)?;
        unsafe {
            std::ptr::copy_nonoverlapping(
                other.data.as_ptr(),
                self.data_ptr().add(self.len),
                other.len,
            );
        }
        self.len += other.len;
        Ok(())
    }

    pub fn append_view(&mut self, view: &Utf8StringView) -> Result<(), ()> {
        self.reserve(self.len + view.len)?;
        unsafe {
            std::ptr::copy_nonoverlapping(
                view.data.as_ptr(),
                self.data_ptr().add(self.len),
                view.len,
            );
        }
        self.len += view.len;
        Ok(())
    }

    pub fn append_character(&mut self, c: Utf8) -> Result<(), ()> {
        self.reserve(self.len + 1)?;
        unsafe { *self.data_ptr().add(self.len) = c; }
        self.len += 1;
        Ok(())
    }

    pub fn append_literal(&mut self, literal: &[Utf8]) -> Result<(), ()> {
        let n = literal.len();
        self.reserve(self.len + n)?;
        unsafe {
            std::ptr::copy_nonoverlapping(literal.as_ptr(), self.data_ptr().add(self.len), n);
        }
        self.len += n;
        Ok(())
    }

    pub fn prepend(&mut self, other: &Utf8String) -> Result<(), ()> {
        self.reserve(self.len + other.len)?;
        unsafe {
            let ptr = self.data_ptr();
            std::ptr::copy(ptr, ptr.add(other.len), self.len);
            std::ptr::copy_nonoverlapping(other.data.as_ptr(), ptr, other.len);
        }
        self.len += other.len;
        Ok(())
    }

    pub fn prepend_view(&mut self, view: &Utf8StringView) -> Result<(), ()> {
        self.reserve(self.len + view.len)?;
        unsafe {
            let ptr = self.data_ptr();
            std::ptr::copy(ptr, ptr.add(view.len), self.len);
            std::ptr::copy_nonoverlapping(view.data.as_ptr(), ptr, view.len);
        }
        self.len += view.len;
        Ok(())
    }

    pub fn prepend_character(&mut self, c: Utf8) -> Result<(), ()> {
        self.reserve(self.len + 1)?;
        unsafe {
            let ptr = self.data_ptr();
            std::ptr::copy(ptr, ptr.add(1), self.len);
            *ptr = c;
        }
        self.len += 1;
        Ok(())
    }

    pub fn prepend_literal(&mut self, literal: &[Utf8]) -> Result<(), ()> {
        let n = literal.len();
        self.reserve(self.len + n)?;
        unsafe {
            let ptr = self.data_ptr();
            std::ptr::copy(ptr, ptr.add(n), self.len);
            std::ptr::copy_nonoverlapping(literal.as_ptr(), ptr, n);
        }
        self.len += n;
        Ok(())
    }

    pub fn insert(&mut self, pos: usize, other: &Utf8String) -> Result<(), ()> {
        if pos > self.len { return Err(()); }
        let new_len = self.len + other.len;
        self.reserve(new_len)?;
        unsafe {
            let ptr = self.data_ptr();
            std::ptr::copy(ptr.add(pos), ptr.add(pos + other.len), self.len - pos);
            std::ptr::copy_nonoverlapping(other.data.as_ptr(), ptr.add(pos), other.len);
        }
        self.len = new_len;
        Ok(())
    }

    pub fn insert_view(&mut self, pos: usize, view: &Utf8StringView) -> Result<(), ()> {
        if pos > self.len { return Err(()); }
        let new_len = self.len + view.len;
        self.reserve(new_len)?;
        unsafe {
            let ptr = self.data_ptr();
            std::ptr::copy(ptr.add(pos), ptr.add(pos + view.len), self.len - pos);
            std::ptr::copy_nonoverlapping(view.data.as_ptr(), ptr.add(pos), view.len);
        }
        self.len = new_len;
        Ok(())
    }

    pub fn insert_character(&mut self, pos: usize, c: Utf8) -> Result<(), ()> {
        if pos > self.len { return Err(()); }
        let new_len = self.len + 1;
        self.reserve(new_len)?;
        unsafe {
            let ptr = self.data_ptr();
            std::ptr::copy(ptr.add(pos), ptr.add(pos + 1), self.len - pos);
            *ptr.add(pos) = c;
        }
        self.len = new_len;
        Ok(())
    }

    pub fn insert_literal(&mut self, pos: usize, literal: &[Utf8]) -> Result<(), ()> {
        if pos > self.len { return Err(()); }
        let n = literal.len();
        let new_len = self.len + n;
        self.reserve(new_len)?;
        unsafe {
            let ptr = self.data_ptr();
            std::ptr::copy(ptr.add(pos), ptr.add(pos + n), self.len - pos);
            std::ptr::copy_nonoverlapping(literal.as_ptr(), ptr.add(pos), n);
        }
        self.len = new_len;
        Ok(())
    }

    pub fn replace(&mut self, pos: usize, mut len: usize, replacement: &Utf8String) -> Result<(), ()> {
        if pos > self.len { return Err(()); }
        if pos + len > self.len { len = self.len - pos; }
        let new_len = self.len + replacement.len - len;
        self.reserve(new_len)?;
        unsafe {
            let ptr = self.data_ptr();
            std::ptr::copy(ptr.add(pos + len), ptr.add(pos + replacement.len), self.len - pos - len);
            std::ptr::copy_nonoverlapping(replacement.data.as_ptr(), ptr.add(pos), replacement.len);
        }
        self.len = new_len;
        Ok(())
    }

    pub fn replace_view(&mut self, pos: usize, mut len: usize, replacement: &Utf8StringView) -> Result<(), ()> {
        if pos > self.len { return Err(()); }
        if pos + len > self.len { len = self.len - pos; }
        let new_len = self.len + replacement.len - len;
        self.reserve(new_len)?;
        unsafe {
            let ptr = self.data_ptr();
            std::ptr::copy(ptr.add(pos + len), ptr.add(pos + replacement.len), self.len - pos - len);
            std::ptr::copy_nonoverlapping(replacement.data.as_ptr(), ptr.add(pos), replacement.len);
        }
        self.len = new_len;
        Ok(())
    }

    pub fn replace_character(&mut self, pos: usize, len: usize, c: Utf8) -> Result<(), ()> {
        if pos > self.len { return Err(()); }
        let new_len = self.len + 1 - len;
        self.reserve(new_len)?;
        unsafe {
            let ptr = self.data_ptr();
            std::ptr::copy(ptr.add(pos + len), ptr.add(pos + 1), self.len - pos - len);
            *ptr.add(pos) = c;
        }
        self.len = new_len;
        Ok(())
    }

    pub fn replace_literal(&mut self, pos: usize, mut len: usize, literal: &[Utf8]) -> Result<(), ()> {
        if pos > self.len { return Err(()); }
        if pos + len > self.len { len = self.len - pos; }
        let n = literal.len();
        let new_len = self.len + n - len;
        self.reserve(new_len)?;
        unsafe {
            let ptr = self.data_ptr();
            std::ptr::copy(ptr.add(pos + len), ptr.add(pos + n), self.len - pos - len);
            std::ptr::copy_nonoverlapping(literal.as_ptr(), ptr.add(pos), n);
        }
        self.len = new_len;
        Ok(())
    }

    pub fn erase(&mut self, pos: usize, mut len: usize) -> Result<(), ()> {
        if pos > self.len { return Err(()); }
        if pos + len > self.len { len = self.len - pos; }
        unsafe {
            let ptr = self.data_ptr();
            std::ptr::copy(ptr.add(pos + len), ptr.add(pos), self.len - pos - len);
        }
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
        let s = start.min(self.len);
        let e = (s + end).min(self.len);
        Utf8StringView {
            data: &self.data[s..e],
            len: e - s,
        }
    }

    pub fn substring_copy(&self, start: usize, end: usize) -> Result<Utf8String, ()> {
        let s = start.min(self.len);
        let e = (s + end).min(self.len);
        let slen = e - s;
        let mut result = Utf8String::new();
        result.reserve(slen)?;
        result.append_literal(&self.data[s..e])?;
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
        let s = start.min(self.len);
        let e = (s + end).min(self.len);
        Utf8StringView {
            data: &self.data[s..e],
            len: e - s,
        }
    }

    pub fn substring_copy(&self, start: usize, end: usize) -> Result<Utf8String, ()> {
        let s = start.min(self.len);
        let e = (s + end).min(self.len);
        let slen = e - s;
        let mut result = Utf8String::new();
        result.reserve(slen)?;
        result.append_literal(&self.data[s..e])?;
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

fn compare_bytes(a: &[u8], b: &[u8]) -> i32 {
    let min_len = a.len().min(b.len());
    for i in 0..min_len {
        if a[i] != b[i] {
            return if a[i] < b[i] { -1 } else { 1 };
        }
    }
    if a.len() < b.len() { -1 }
    else if a.len() > b.len() { 1 }
    else { 0 }
}
