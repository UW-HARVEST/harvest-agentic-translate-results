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

impl Utf8String<'_> {
    pub fn new() -> Self {
        Self {
            data: &[],
            len: 0,
            cap: 0,
        }
    }

    pub fn init(&mut self) {
        self.destroy();
        self.data = &[];
        self.len = 0;
        self.cap = 0;
    }

    pub fn destroy(&mut self) {
        if self.cap > 0 {
            let cap = self.cap;
            let ptr = self.data.as_ptr() as *mut u8;
            // Safety: we previously leaked a Box<[u8]> of this exact size from reserve.
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

        // Round up to next power of 2 (mirroring the C bit-twiddling)
        let new_cap = if len <= 1 { 1 } else { len.next_power_of_two() };

        // Allocate new buffer and copy current data over
        let mut new_vec: Vec<u8> = vec![0u8; new_cap];
        if self.len > 0 {
            new_vec[..self.len].copy_from_slice(&self.data[..self.len]);
        }

        // Free old buffer if it was heap allocated
        if self.cap > 0 {
            let old_cap = self.cap;
            let old_ptr = self.data.as_ptr() as *mut u8;
            // Safety: the previous allocation was a leaked Box<[u8]> of cap bytes.
            unsafe {
                let _ = Box::from_raw(std::slice::from_raw_parts_mut(old_ptr, old_cap));
            }
        }

        // Leak the new allocation; we'll free it in destroy/reserve.
        let leaked: &'static mut [u8] = Box::leak(new_vec.into_boxed_slice());
        self.data = leaked;
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
            self.destroy();
            return Ok(());
        }
        let mut new_vec: Vec<u8> = vec![0u8; new_cap];
        new_vec[..self.len].copy_from_slice(&self.data[..self.len]);

        let old_cap = self.cap;
        let old_ptr = self.data.as_ptr() as *mut u8;
        unsafe {
            let _ = Box::from_raw(std::slice::from_raw_parts_mut(old_ptr, old_cap));
        }
        let leaked: &'static mut [u8] = Box::leak(new_vec.into_boxed_slice());
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
        let cur = self.len;
        let new_total = cur + other_len;
        self.reserve(new_total)?;
        let buf = buf_as_mut(self);
        buf[cur..cur + other_len].copy_from_slice(&other.data[..other_len]);
        self.len = new_total;
        Ok(())
    }

    pub fn append_view(&mut self, view: &Utf8StringView) -> Result<(), ()> {
        let v_len = view.len;
        let cur = self.len;
        let new_total = cur + v_len;
        self.reserve(new_total)?;
        let buf = buf_as_mut(self);
        buf[cur..cur + v_len].copy_from_slice(&view.data[..v_len]);
        self.len = new_total;
        Ok(())
    }

    pub fn append_character(&mut self, c: Utf8) -> Result<(), ()> {
        let cur = self.len;
        let new_total = cur + 1;
        self.reserve(new_total)?;
        let buf = buf_as_mut(self);
        buf[cur] = c;
        self.len = new_total;
        Ok(())
    }

    pub fn append_literal(&mut self, literal: &[Utf8]) -> Result<(), ()> {
        let n = literal.len();
        let cur = self.len;
        let new_total = cur + n;
        self.reserve(new_total)?;
        let buf = buf_as_mut(self);
        buf[cur..cur + n].copy_from_slice(literal);
        self.len = new_total;
        Ok(())
    }

    pub fn prepend(&mut self, other: &Utf8String) -> Result<(), ()> {
        let other_len = other.len;
        let new_total = self.len + other_len;
        self.reserve(new_total)?;
        let cur_len = self.len;
        let buf = buf_as_mut(self);
        buf.copy_within(0..cur_len, other_len);
        buf[..other_len].copy_from_slice(&other.data[..other_len]);
        self.len = new_total;
        Ok(())
    }

    pub fn prepend_view(&mut self, view: &Utf8StringView) -> Result<(), ()> {
        let v_len = view.len;
        let new_total = self.len + v_len;
        self.reserve(new_total)?;
        let cur_len = self.len;
        let buf = buf_as_mut(self);
        buf.copy_within(0..cur_len, v_len);
        buf[..v_len].copy_from_slice(&view.data[..v_len]);
        self.len = new_total;
        Ok(())
    }

    pub fn prepend_character(&mut self, c: Utf8) -> Result<(), ()> {
        let new_total = self.len + 1;
        self.reserve(new_total)?;
        let cur_len = self.len;
        let buf = buf_as_mut(self);
        buf.copy_within(0..cur_len, 1);
        buf[0] = c;
        self.len = new_total;
        Ok(())
    }

    pub fn prepend_literal(&mut self, literal: &[Utf8]) -> Result<(), ()> {
        let n = literal.len();
        let new_total = self.len + n;
        self.reserve(new_total)?;
        let cur_len = self.len;
        let buf = buf_as_mut(self);
        buf.copy_within(0..cur_len, n);
        buf[..n].copy_from_slice(literal);
        self.len = new_total;
        Ok(())
    }

    pub fn insert(&mut self, pos: usize, other: &Utf8String) -> Result<(), ()> {
        if pos > self.len {
            return Err(());
        }
        let other_len = other.len;
        let inserted_len = self.len + other_len;
        self.reserve(inserted_len)?;
        let cur_len = self.len;
        let buf = buf_as_mut(self);
        buf.copy_within(pos..cur_len, pos + other_len);
        buf[pos..pos + other_len].copy_from_slice(&other.data[..other_len]);
        self.len = inserted_len;
        Ok(())
    }

    pub fn insert_view(&mut self, pos: usize, view: &Utf8StringView) -> Result<(), ()> {
        if pos > self.len {
            return Err(());
        }
        let v_len = view.len;
        let inserted_len = self.len + v_len;
        self.reserve(inserted_len)?;
        let cur_len = self.len;
        let buf = buf_as_mut(self);
        buf.copy_within(pos..cur_len, pos + v_len);
        buf[pos..pos + v_len].copy_from_slice(&view.data[..v_len]);
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
        let buf = buf_as_mut(self);
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
        let buf = buf_as_mut(self);
        buf.copy_within(pos..cur_len, pos + n);
        buf[pos..pos + n].copy_from_slice(literal);
        self.len = inserted_len;
        Ok(())
    }

    pub fn replace(&mut self, pos: usize, len: usize, replacement: &Utf8String) -> Result<(), ()> {
        if pos > self.len {
            return Err(());
        }
        let len = if pos + len > self.len {
            self.len - pos
        } else {
            len
        };
        let r_len = replacement.len;
        let replaced_len = self.len + r_len - len;
        self.reserve(replaced_len)?;
        let cur_len = self.len;
        let buf = buf_as_mut(self);
        // memmove from pos+len .. cur_len  to  pos+r_len
        buf.copy_within(pos + len..cur_len, pos + r_len);
        buf[pos..pos + r_len].copy_from_slice(&replacement.data[..r_len]);
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
        let len = if pos + len > self.len {
            self.len - pos
        } else {
            len
        };
        let r_len = replacement.len;
        let replaced_len = self.len + r_len - len;
        self.reserve(replaced_len)?;
        let cur_len = self.len;
        let buf = buf_as_mut(self);
        buf.copy_within(pos + len..cur_len, pos + r_len);
        buf[pos..pos + r_len].copy_from_slice(&replacement.data[..r_len]);
        self.len = replaced_len;
        Ok(())
    }

    pub fn replace_character(&mut self, pos: usize, len: usize, c: Utf8) -> Result<(), ()> {
        if pos > self.len {
            return Err(());
        }
        // Note: C does NOT clamp len here for replace_character
        let replaced_len = self.len + 1 - len;
        self.reserve(replaced_len)?;
        let cur_len = self.len;
        let buf = buf_as_mut(self);
        buf.copy_within(pos + len..cur_len, pos + 1);
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
        let len = if pos + len > self.len {
            self.len - pos
        } else {
            len
        };
        let n = literal.len();
        let replaced_len = self.len + n - len;
        self.reserve(replaced_len)?;
        let cur_len = self.len;
        let buf = buf_as_mut(self);
        buf.copy_within(pos + len..cur_len, pos + n);
        buf[pos..pos + n].copy_from_slice(literal);
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
        let cur_len = self.len;
        let buf = buf_as_mut(self);
        buf.copy_within(pos + len..cur_len, pos);
        self.len -= len;
        Ok(())
    }

    pub fn concat(&self, other: &Utf8String) -> Result<Utf8String, ()> {
        let mut result = Utf8String::new();
        result.reserve(self.len + other.len)?;
        result.append_self_and(self)?;
        result.append(other)?;
        Ok(result)
    }

    pub fn concat_view(&self, other: &Utf8StringView) -> Result<Utf8String, ()> {
        let mut result = Utf8String::new();
        result.reserve(self.len + other.len)?;
        result.append_self_and(self)?;
        result.append_view(other)?;
        Ok(result)
    }

    pub fn concat_character(&self, c: Utf8) -> Result<Utf8String, ()> {
        let mut result = Utf8String::new();
        result.reserve(self.len + 1)?;
        result.append_self_and(self)?;
        result.append_character(c)?;
        Ok(result)
    }

    pub fn concat_literal(&self, literal: &[Utf8]) -> Result<Utf8String, ()> {
        let mut result = Utf8String::new();
        result.reserve(self.len + literal.len())?;
        result.append_self_and(self)?;
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
        let end = if end == usize::MAX || end > self.len {
            self.len
        } else {
            end
        };
        let start = if start > end { end } else { start };
        Utf8StringView {
            data: &self.data[start..end],
            len: end - start,
        }
    }

    pub fn substring_copy(&self, start: usize, end: usize) -> Result<Utf8String, ()> {
        let end = if end == usize::MAX || end > self.len {
            self.len
        } else {
            end
        };
        let start = if start > end { end } else { start };
        let mut result = Utf8String::new();
        let len = end - start;
        result.reserve(len)?;
        if len > 0 {
            let buf = buf_as_mut(&mut result);
            buf[..len].copy_from_slice(&self.data[start..end]);
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
        let pos = if pos == usize::MAX {
            if self.len == 0 {
                return None;
            }
            self.len - 1
        } else if pos >= self.len {
            return None;
        } else {
            pos
        };
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

impl Utf8String<'_> {
    // Internal helper: append from another Utf8String, used inside concat
    // (we cannot use append because we already have an exclusive borrow of `result`).
    fn append_self_and(&mut self, other: &Utf8String) -> Result<(), ()> {
        self.append(other)
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
        let end = if end == usize::MAX || end > self.len {
            self.len
        } else {
            end
        };
        let start = if start > end { end } else { start };
        Utf8StringView {
            data: &self.data[start..end],
            len: end - start,
        }
    }

    pub fn substring_copy(&self, start: usize, end: usize) -> Result<Utf8String, ()> {
        let end = if end == usize::MAX || end > self.len {
            self.len
        } else {
            end
        };
        let start = if start > end { end } else { start };
        let mut result = Utf8String::new();
        let len = end - start;
        result.reserve(len)?;
        if len > 0 {
            let buf = buf_as_mut(&mut result);
            buf[..len].copy_from_slice(&self.data[start..end]);
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
        let pos = if pos == usize::MAX {
            if self.len == 0 {
                return None;
            }
            self.len - 1
        } else if pos >= self.len {
            return None;
        } else {
            pos
        };
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

// Returns a mutable view over the heap allocation backing `string.data`.
//
// Safety: relies on the invariant that whenever cap > 0, `data` points at a
// `Box<[u8]>` allocation (created in reserve via Box::leak) of exactly `cap`
// bytes, which we own. The borrow checker on `&mut Utf8String` ensures the
// caller has unique access.
fn buf_as_mut<'b>(string: &'b mut Utf8String<'_>) -> &'b mut [u8] {
    if string.cap == 0 {
        return &mut [];
    }
    let ptr = string.data.as_ptr() as *mut u8;
    unsafe { std::slice::from_raw_parts_mut(ptr, string.cap) }
}

fn compare_bytes(a: &[u8], b: &[u8]) -> i32 {
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
