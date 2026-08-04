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

fn alloc_zero_buf(cap: usize) -> &'static mut [u8] {
    let v: Vec<u8> = vec![0u8; cap];
    Box::leak(v.into_boxed_slice())
}

fn round_up_pow2(len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    len.next_power_of_two()
}

impl Utf8String<'_> {
    pub fn new() -> Self {
        let buf = alloc_zero_buf(8);
        Self {
            data: buf,
            len: 0,
            cap: 8,
        }
    }

    pub fn init(&mut self) {
        let buf = alloc_zero_buf(8);
        self.data = buf;
        self.len = 0;
        self.cap = 8;
    }

    pub fn destroy(&mut self) {
        // The buffer was leaked from a Box<[u8]>. We won't reclaim here to avoid
        // tricky lifetime management; subsequent uses must call init() again.
        self.len = 0;
    }

    pub fn reserve(&mut self, len: usize) -> Result<(), ()> {
        if len <= self.cap {
            return Ok(());
        }
        let new_cap = round_up_pow2(len);
        let new_buf = alloc_zero_buf(new_cap);
        // SAFETY: copy current valid data into the new buffer.
        unsafe {
            std::ptr::copy_nonoverlapping(self.data.as_ptr(), new_buf.as_mut_ptr(), self.len);
        }
        self.data = new_buf;
        self.cap = new_cap;
        Ok(())
    }

    pub fn shrink_to_fit(&mut self) -> Result<(), ()> {
        // Always allocate a new buffer of size = self.len, copy data, replace.
        // The C version uses a small-string buffer optimization that we don't
        // model exactly. We at least ensure cap matches len.
        let cap = self.len;
        if cap == 0 {
            // Provide a non-zero buffer to avoid empty slice issues
            let new_buf = alloc_zero_buf(0);
            self.data = new_buf;
            self.cap = 0;
            return Ok(());
        }
        let new_buf = alloc_zero_buf(cap);
        unsafe {
            std::ptr::copy_nonoverlapping(self.data.as_ptr(), new_buf.as_mut_ptr(), self.len);
        }
        self.data = new_buf;
        self.cap = cap;
        Ok(())
    }

    pub fn clear(&mut self) {
        self.len = 0;
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn append(&mut self, other: &Utf8String) -> Result<(), ()> {
        let new_len = self.len + other.len;
        self.reserve(new_len)?;
        unsafe {
            let dst = (self.data.as_ptr() as *mut u8).add(self.len);
            std::ptr::copy_nonoverlapping(other.data.as_ptr(), dst, other.len);
        }
        self.len = new_len;
        Ok(())
    }

    pub fn append_view(&mut self, view: &Utf8StringView) -> Result<(), ()> {
        let new_len = self.len + view.len;
        self.reserve(new_len)?;
        unsafe {
            let dst = (self.data.as_ptr() as *mut u8).add(self.len);
            std::ptr::copy_nonoverlapping(view.data.as_ptr(), dst, view.len);
        }
        self.len = new_len;
        Ok(())
    }

    pub fn append_character(&mut self, c: Utf8) -> Result<(), ()> {
        let new_len = self.len + 1;
        self.reserve(new_len)?;
        unsafe {
            let dst = (self.data.as_ptr() as *mut u8).add(self.len);
            *dst = c;
        }
        self.len = new_len;
        Ok(())
    }

    pub fn append_literal(&mut self, literal: &[Utf8]) -> Result<(), ()> {
        let n = literal.len();
        let new_len = self.len + n;
        self.reserve(new_len)?;
        unsafe {
            let dst = (self.data.as_ptr() as *mut u8).add(self.len);
            std::ptr::copy_nonoverlapping(literal.as_ptr(), dst, n);
        }
        self.len = new_len;
        Ok(())
    }

    pub fn prepend(&mut self, other: &Utf8String) -> Result<(), ()> {
        let new_len = self.len + other.len;
        self.reserve(new_len)?;
        unsafe {
            let base = self.data.as_ptr() as *mut u8;
            // memmove old contents forward by other.len
            std::ptr::copy(base, base.add(other.len), self.len);
            // copy other.data into beginning
            std::ptr::copy_nonoverlapping(other.data.as_ptr(), base, other.len);
        }
        self.len = new_len;
        Ok(())
    }

    pub fn prepend_view(&mut self, view: &Utf8StringView) -> Result<(), ()> {
        let new_len = self.len + view.len;
        self.reserve(new_len)?;
        unsafe {
            let base = self.data.as_ptr() as *mut u8;
            std::ptr::copy(base, base.add(view.len), self.len);
            std::ptr::copy_nonoverlapping(view.data.as_ptr(), base, view.len);
        }
        self.len = new_len;
        Ok(())
    }

    pub fn prepend_character(&mut self, c: Utf8) -> Result<(), ()> {
        let new_len = self.len + 1;
        self.reserve(new_len)?;
        unsafe {
            let base = self.data.as_ptr() as *mut u8;
            std::ptr::copy(base, base.add(1), self.len);
            *base = c;
        }
        self.len = new_len;
        Ok(())
    }

    pub fn prepend_literal(&mut self, literal: &[Utf8]) -> Result<(), ()> {
        let n = literal.len();
        let new_len = self.len + n;
        self.reserve(new_len)?;
        unsafe {
            let base = self.data.as_ptr() as *mut u8;
            std::ptr::copy(base, base.add(n), self.len);
            std::ptr::copy_nonoverlapping(literal.as_ptr(), base, n);
        }
        self.len = new_len;
        Ok(())
    }

    pub fn insert(&mut self, pos: usize, other: &Utf8String) -> Result<(), ()> {
        if pos > self.len {
            return Err(());
        }
        let inserted_len = self.len + other.len;
        self.reserve(inserted_len)?;
        unsafe {
            let base = self.data.as_ptr() as *mut u8;
            std::ptr::copy(base.add(pos), base.add(pos + other.len), self.len - pos);
            std::ptr::copy_nonoverlapping(other.data.as_ptr(), base.add(pos), other.len);
        }
        self.len = inserted_len;
        Ok(())
    }

    pub fn insert_view(&mut self, pos: usize, view: &Utf8StringView) -> Result<(), ()> {
        if pos > self.len {
            return Err(());
        }
        let inserted_len = self.len + view.len;
        self.reserve(inserted_len)?;
        unsafe {
            let base = self.data.as_ptr() as *mut u8;
            std::ptr::copy(base.add(pos), base.add(pos + view.len), self.len - pos);
            std::ptr::copy_nonoverlapping(view.data.as_ptr(), base.add(pos), view.len);
        }
        self.len = inserted_len;
        Ok(())
    }

    pub fn insert_character(&mut self, pos: usize, c: Utf8) -> Result<(), ()> {
        if pos > self.len {
            return Err(());
        }
        let inserted_len = self.len + 1;
        self.reserve(inserted_len)?;
        unsafe {
            let base = self.data.as_ptr() as *mut u8;
            std::ptr::copy(base.add(pos), base.add(pos + 1), self.len - pos);
            *base.add(pos) = c;
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
            let base = self.data.as_ptr() as *mut u8;
            std::ptr::copy(base.add(pos), base.add(pos + n), self.len - pos);
            std::ptr::copy_nonoverlapping(literal.as_ptr(), base.add(pos), n);
        }
        self.len = inserted_len;
        Ok(())
    }

    pub fn replace(&mut self, pos: usize, len: usize, replacement: &Utf8String) -> Result<(), ()> {
        if pos > self.len {
            return Err(());
        }
        let mut effective_len = len;
        if pos + effective_len > self.len {
            effective_len = self.len - pos;
        }
        let replaced_len = self.len + replacement.len - effective_len;
        self.reserve(replaced_len)?;
        unsafe {
            let base = self.data.as_ptr() as *mut u8;
            std::ptr::copy(
                base.add(pos + effective_len),
                base.add(pos + replacement.len),
                self.len - pos - effective_len,
            );
            std::ptr::copy_nonoverlapping(
                replacement.data.as_ptr(),
                base.add(pos),
                replacement.len,
            );
        }
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
        let mut effective_len = len;
        if pos + effective_len > self.len {
            effective_len = self.len - pos;
        }
        let replaced_len = self.len + replacement.len - effective_len;
        self.reserve(replaced_len)?;
        unsafe {
            let base = self.data.as_ptr() as *mut u8;
            std::ptr::copy(
                base.add(pos + effective_len),
                base.add(pos + replacement.len),
                self.len - pos - effective_len,
            );
            std::ptr::copy_nonoverlapping(
                replacement.data.as_ptr(),
                base.add(pos),
                replacement.len,
            );
        }
        self.len = replaced_len;
        Ok(())
    }

    pub fn replace_character(&mut self, pos: usize, len: usize, c: Utf8) -> Result<(), ()> {
        if pos > self.len {
            return Err(());
        }
        // Note: C version doesn't clamp `len` for the character variant.
        // It computes self.len + 1 - len directly. We mirror that.
        let replaced_len = self.len + 1 - len;
        self.reserve(replaced_len)?;
        unsafe {
            let base = self.data.as_ptr() as *mut u8;
            std::ptr::copy(
                base.add(pos + len),
                base.add(pos + 1),
                self.len - pos - len,
            );
            *base.add(pos) = c;
        }
        self.len = replaced_len;
        Ok(())
    }

    pub fn replace_literal(&mut self, pos: usize, len: usize, literal: &[Utf8]) -> Result<(), ()> {
        if pos > self.len {
            return Err(());
        }
        let mut effective_len = len;
        if pos + effective_len > self.len {
            effective_len = self.len - pos;
        }
        let n = literal.len();
        let replaced_len = self.len + n - effective_len;
        self.reserve(replaced_len)?;
        unsafe {
            let base = self.data.as_ptr() as *mut u8;
            std::ptr::copy(
                base.add(pos + effective_len),
                base.add(pos + n),
                self.len - pos - effective_len,
            );
            std::ptr::copy_nonoverlapping(literal.as_ptr(), base.add(pos), n);
        }
        self.len = replaced_len;
        Ok(())
    }

    pub fn erase(&mut self, pos: usize, len: usize) -> Result<(), ()> {
        if pos > self.len {
            return Err(());
        }
        let mut effective_len = len;
        if pos + effective_len > self.len {
            effective_len = self.len - pos;
        }
        unsafe {
            let base = self.data.as_ptr() as *mut u8;
            std::ptr::copy(
                base.add(pos + effective_len),
                base.add(pos),
                self.len - pos - effective_len,
            );
        }
        self.len -= effective_len;
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
        let a_len = self.len;
        let b_len = other.len;
        let min = a_len.min(b_len);
        let a = &self.data[..a_len];
        let b = &other.data[..b_len];
        for i in 0..min {
            if a[i] != b[i] {
                return (a[i] as i32) - (b[i] as i32);
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

    pub fn compare_literal(&self, literal: &[Utf8]) -> i32 {
        let a_len = self.len;
        let b_len = literal.len();
        let min = a_len.min(b_len);
        let a = &self.data[..a_len];
        for i in 0..min {
            if a[i] != literal[i] {
                return (a[i] as i32) - (literal[i] as i32);
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

    pub fn substring(&self, start: usize, end: usize) -> Utf8StringView {
        let mut e = end;
        if e > self.len {
            e = self.len;
        }
        let mut s = start;
        if s > e {
            s = e;
        }
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
        let mut s = start;
        if s > e {
            s = e;
        }
        let mut result = Utf8String::new();
        let len = e - s;
        result.reserve(len)?;
        unsafe {
            std::ptr::copy_nonoverlapping(
                self.data[s..].as_ptr(),
                result.data.as_ptr() as *mut u8,
                len,
            );
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
        let start = if pos == usize::MAX {
            self.len - 1
        } else if pos >= self.len {
            return None;
        } else {
            pos
        };
        let mut i = start as isize;
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
        let a_len = self.len;
        let b_len = other.len;
        let min = a_len.min(b_len);
        for i in 0..min {
            if self.data[i] != other.data[i] {
                return (self.data[i] as i32) - (other.data[i] as i32);
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

    pub fn compare_literal(&self, literal: &[Utf8]) -> i32 {
        let a_len = self.len;
        let b_len = literal.len();
        let min = a_len.min(b_len);
        for i in 0..min {
            if self.data[i] != literal[i] {
                return (self.data[i] as i32) - (literal[i] as i32);
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

    pub fn substring(&self, start: usize, end: usize) -> Utf8StringView<'a> {
        let mut e = end;
        if e > self.len {
            e = self.len;
        }
        let mut s = start;
        if s > e {
            s = e;
        }
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
        let mut s = start;
        if s > e {
            s = e;
        }
        let mut result = Utf8String::new();
        let len = e - s;
        result.reserve(len)?;
        unsafe {
            std::ptr::copy_nonoverlapping(
                self.data[s..].as_ptr(),
                result.data.as_ptr() as *mut u8,
                len,
            );
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
        let start = if pos == usize::MAX {
            self.len - 1
        } else if pos >= self.len {
            return None;
        } else {
            pos
        };
        let mut i = start as isize;
        while i >= 0 {
            if self.data[i as usize] == c {
                return Some(i as usize);
            }
            i -= 1;
        }
        None
    }
}
