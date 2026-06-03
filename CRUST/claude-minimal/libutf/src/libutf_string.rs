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

fn allocate_buffer(cap: usize) -> &'static mut [u8] {
    let boxed: Box<[u8]> = vec![0u8; cap].into_boxed_slice();
    Box::leak(boxed)
}

unsafe fn deallocate_buffer(ptr: *mut u8, cap: usize) {
    if cap > 0 {
        let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr, cap));
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
        if self.cap > 0 {
            let ptr = self.data.as_ptr() as *mut u8;
            unsafe {
                deallocate_buffer(ptr, self.cap);
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
        let new_cap = if len == 0 { 1 } else { len.next_power_of_two() };
        let new_slice: &'static mut [u8] = allocate_buffer(new_cap);
        if self.len > 0 {
            new_slice[..self.len].copy_from_slice(&self.data[..self.len]);
        }
        let old_ptr = self.data.as_ptr() as *mut u8;
        let old_cap = self.cap;
        self.data = new_slice;
        self.cap = new_cap;
        if old_cap > 0 {
            unsafe {
                deallocate_buffer(old_ptr, old_cap);
            }
        }
        Ok(())
    }

    pub fn shrink_to_fit(&mut self) -> Result<(), ()> {
        if self.cap == self.len {
            return Ok(());
        }
        if self.len == 0 {
            self.destroy();
            return Ok(());
        }
        let new_cap = self.len;
        let new_slice: &'static mut [u8] = allocate_buffer(new_cap);
        new_slice[..self.len].copy_from_slice(&self.data[..self.len]);
        let old_ptr = self.data.as_ptr() as *mut u8;
        let old_cap = self.cap;
        self.data = new_slice;
        self.cap = new_cap;
        if old_cap > 0 {
            unsafe {
                deallocate_buffer(old_ptr, old_cap);
            }
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
            std::ptr::copy_nonoverlapping(literal.as_ptr(), ptr.add(self.len), n);
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
            let ptr = self.data.as_ptr() as *mut u8;
            std::ptr::copy(ptr, ptr.add(n), self.len);
            std::ptr::copy_nonoverlapping(literal.as_ptr(), ptr, n);
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
            std::ptr::copy(ptr.add(pos), ptr.add(pos + 1), self.len - pos);
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
            std::ptr::copy(ptr.add(pos), ptr.add(pos + n), self.len - pos);
            std::ptr::copy_nonoverlapping(literal.as_ptr(), ptr.add(pos), n);
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
        let mut len = len;
        if pos + len > self.len {
            len = self.len - pos;
        }
        let replaced_len = self.len + 1 - len;
        self.reserve(replaced_len)?;
        unsafe {
            let ptr = self.data.as_ptr() as *mut u8;
            std::ptr::copy(
                ptr.add(pos + len),
                ptr.add(pos + 1),
                self.len - pos - len,
            );
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
        let mut len = len;
        if pos + len > self.len {
            len = self.len - pos;
        }
        let n = literal.len();
        let replaced_len = self.len + n - len;
        self.reserve(replaced_len)?;
        unsafe {
            let ptr = self.data.as_ptr() as *mut u8;
            std::ptr::copy(
                ptr.add(pos + len),
                ptr.add(pos + n),
                self.len - pos - len,
            );
            std::ptr::copy_nonoverlapping(literal.as_ptr(), ptr.add(pos), n);
        }
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
        unsafe {
            let ptr = self.data.as_ptr() as *mut u8;
            std::ptr::copy(
                ptr.add(pos + len),
                ptr.add(pos),
                self.len - pos - len,
            );
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
        let a = &self.data[..self.len];
        let b = &other.data[..other.len];
        compare_bytes(a, b)
    }

    pub fn compare_literal(&self, literal: &[Utf8]) -> i32 {
        let a = &self.data[..self.len];
        compare_bytes(a, literal)
    }

    pub fn substring(&self, start: usize, end: usize) -> Utf8StringView {
        let mut end = end;
        if end == usize::MAX || end > self.len {
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
        if end == usize::MAX || end > self.len {
            end = self.len;
        }
        let mut start = start;
        if start > end {
            start = end;
        }
        let len = end - start;
        let mut result = Utf8String::new();
        result.reserve(len)?;
        unsafe {
            let dst = result.data.as_ptr() as *mut u8;
            std::ptr::copy_nonoverlapping(self.data.as_ptr().add(start), dst, len);
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
        let mut pos = pos;
        if pos == usize::MAX {
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
                break;
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
        let mut end = end;
        if end == usize::MAX || end > self.len {
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
        if end == usize::MAX || end > self.len {
            end = self.len;
        }
        let mut start = start;
        if start > end {
            start = end;
        }
        let len = end - start;
        let mut result = Utf8String::new();
        result.reserve(len)?;
        unsafe {
            let dst = result.data.as_ptr() as *mut u8;
            std::ptr::copy_nonoverlapping(self.data.as_ptr().add(start), dst, len);
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
        let mut pos = pos;
        if pos == usize::MAX {
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
                break;
            }
            i -= 1;
        }
        None
    }
}

fn compare_bytes(a: &[u8], b: &[u8]) -> i32 {
    let a_len = a.len();
    let b_len = b.len();
    let n = a_len.min(b_len);
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
