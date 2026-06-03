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

// Internal helper: allocate a new buffer of the given capacity, leaked to 'static
// so the lifetime can coerce to any lifetime parameter on the struct.
fn allocate_leaked(cap: usize) -> &'static mut [u8] {
    let v: Vec<u8> = vec![0u8; cap];
    Box::leak(v.into_boxed_slice())
}

// Internal helper: get a mutable pointer into the data slice. This is used to
// emulate the C in-place mutation behavior.
unsafe fn data_mut_ptr(data: &[u8]) -> *mut u8 {
    data.as_ptr() as *mut u8
}

// Round up to next power of two, matching the C bit-twiddling implementation.
fn round_up_pow2(mut len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    len -= 1;
    len |= len >> 1;
    len |= len >> 2;
    len |= len >> 4;
    len |= len >> 8;
    len |= len >> 16;
    if core::mem::size_of::<usize>() == 8 {
        len |= len >> 32;
    }
    len + 1
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
    // We leaked the buffer in reserve(); nothing to free here.
    self.data = &[];
    self.len = 0;
    self.cap = 0;
}
pub fn reserve(&mut self, len: usize) -> Result<(), ()> {
    if len <= self.cap {
        return Ok(());
    }
    let new_cap = round_up_pow2(len);
    let new_buf: &'static mut [u8] = allocate_leaked(new_cap);
    // Copy existing data into the new buffer.
    let cur_len = self.len;
    if cur_len > 0 {
        new_buf[..cur_len].copy_from_slice(&self.data[..cur_len]);
    }
    // Coerce &'static mut [u8] -> &'a [u8] (shared, with lifetime 'a)
    let new_slice: &'static [u8] = new_buf;
    self.data = new_slice;
    self.cap = new_cap;
    Ok(())
}
pub fn shrink_to_fit(&mut self) -> Result<(), ()> {
    let new_cap = self.len;
    if new_cap == self.cap {
        return Ok(());
    }
    if new_cap == 0 {
        self.data = &[];
        self.cap = 0;
        return Ok(());
    }
    let new_buf: &'static mut [u8] = allocate_leaked(new_cap);
    new_buf[..new_cap].copy_from_slice(&self.data[..new_cap]);
    let new_slice: &'static [u8] = new_buf;
    self.data = new_slice;
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
    self.reserve(self.len + other.len)?;
    unsafe {
        let dst = data_mut_ptr(self.data).add(self.len);
        core::ptr::copy_nonoverlapping(other.data.as_ptr(), dst, other.len);
    }
    self.len += other.len;
    Ok(())
}
pub fn append_view(&mut self, view: &Utf8StringView) -> Result<(), ()> {
    self.reserve(self.len + view.len)?;
    unsafe {
        let dst = data_mut_ptr(self.data).add(self.len);
        core::ptr::copy_nonoverlapping(view.data.as_ptr(), dst, view.len);
    }
    self.len += view.len;
    Ok(())
}
pub fn append_character(&mut self, c: Utf8) -> Result<(), ()> {
    self.reserve(self.len + 1)?;
    unsafe {
        let dst = data_mut_ptr(self.data).add(self.len);
        *dst = c;
    }
    self.len += 1;
    Ok(())
}
pub fn append_literal(&mut self, literal: &[Utf8]) -> Result<(), ()> {
    let n = literal.len();
    self.reserve(self.len + n)?;
    unsafe {
        let dst = data_mut_ptr(self.data).add(self.len);
        core::ptr::copy_nonoverlapping(literal.as_ptr(), dst, n);
    }
    self.len += n;
    Ok(())
}
pub fn prepend(&mut self, other: &Utf8String) -> Result<(), ()> {
    self.reserve(self.len + other.len)?;
    unsafe {
        let base = data_mut_ptr(self.data);
        core::ptr::copy(base, base.add(other.len), self.len);
        core::ptr::copy_nonoverlapping(other.data.as_ptr(), base, other.len);
    }
    self.len += other.len;
    Ok(())
}
pub fn prepend_view(&mut self, view: &Utf8StringView) -> Result<(), ()> {
    self.reserve(self.len + view.len)?;
    unsafe {
        let base = data_mut_ptr(self.data);
        core::ptr::copy(base, base.add(view.len), self.len);
        core::ptr::copy_nonoverlapping(view.data.as_ptr(), base, view.len);
    }
    self.len += view.len;
    Ok(())
}
pub fn prepend_character(&mut self, c: Utf8) -> Result<(), ()> {
    self.reserve(self.len + 1)?;
    unsafe {
        let base = data_mut_ptr(self.data);
        core::ptr::copy(base, base.add(1), self.len);
        *base = c;
    }
    self.len += 1;
    Ok(())
}
pub fn prepend_literal(&mut self, literal: &[Utf8]) -> Result<(), ()> {
    let n = literal.len();
    self.reserve(self.len + n)?;
    unsafe {
        let base = data_mut_ptr(self.data);
        core::ptr::copy(base, base.add(n), self.len);
        core::ptr::copy_nonoverlapping(literal.as_ptr(), base, n);
    }
    self.len += n;
    Ok(())
}
pub fn insert(&mut self, pos: usize, other: &Utf8String) -> Result<(), ()> {
    if pos > self.len {
        return Err(());
    }
    let inserted_len = self.len + other.len;
    self.reserve(inserted_len)?;
    unsafe {
        let base = data_mut_ptr(self.data);
        core::ptr::copy(base.add(pos), base.add(pos + other.len), self.len - pos);
        core::ptr::copy_nonoverlapping(other.data.as_ptr(), base.add(pos), other.len);
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
        let base = data_mut_ptr(self.data);
        core::ptr::copy(base.add(pos), base.add(pos + view.len), self.len - pos);
        core::ptr::copy_nonoverlapping(view.data.as_ptr(), base.add(pos), view.len);
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
        let base = data_mut_ptr(self.data);
        core::ptr::copy(base.add(pos), base.add(pos + 1), self.len - pos);
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
        let base = data_mut_ptr(self.data);
        core::ptr::copy(base.add(pos), base.add(pos + n), self.len - pos);
        core::ptr::copy_nonoverlapping(literal.as_ptr(), base.add(pos), n);
    }
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
    let replaced_len = self.len + replacement.len - len;
    self.reserve(replaced_len)?;
    unsafe {
        let base = data_mut_ptr(self.data);
        core::ptr::copy(
            base.add(pos + len),
            base.add(pos + replacement.len),
            self.len - pos - len,
        );
        core::ptr::copy_nonoverlapping(
            replacement.data.as_ptr(),
            base.add(pos),
            replacement.len,
        );
    }
    self.len = replaced_len;
    Ok(())
}
pub fn replace_view(&mut self, pos: usize, len: usize, replacement: &Utf8StringView) -> Result<(), ()> {
    if pos > self.len {
        return Err(());
    }
    let len = if pos + len > self.len {
        self.len - pos
    } else {
        len
    };
    let replaced_len = self.len + replacement.len - len;
    self.reserve(replaced_len)?;
    unsafe {
        let base = data_mut_ptr(self.data);
        core::ptr::copy(
            base.add(pos + len),
            base.add(pos + replacement.len),
            self.len - pos - len,
        );
        core::ptr::copy_nonoverlapping(
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
    let replaced_len = self.len + 1 - len;
    self.reserve(replaced_len)?;
    unsafe {
        let base = data_mut_ptr(self.data);
        core::ptr::copy(
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
    let len = if pos + len > self.len {
        self.len - pos
    } else {
        len
    };
    let n = literal.len();
    let replaced_len = self.len + n - len;
    self.reserve(replaced_len)?;
    unsafe {
        let base = data_mut_ptr(self.data);
        core::ptr::copy(
            base.add(pos + len),
            base.add(pos + n),
            self.len - pos - len,
        );
        core::ptr::copy_nonoverlapping(literal.as_ptr(), base.add(pos), n);
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
        let base = data_mut_ptr(self.data);
        core::ptr::copy(
            base.add(pos + len),
            base.add(pos),
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
    Ok(unsafe { core::mem::transmute::<Utf8String<'static>, Utf8String>(transmute_static(result)) })
}
pub fn concat_view(&self, other: &Utf8StringView) -> Result<Utf8String, ()> {
    let mut result = Utf8String::new();
    result.reserve(self.len + other.len)?;
    result.append(self)?;
    result.append_view(other)?;
    Ok(unsafe { core::mem::transmute::<Utf8String<'static>, Utf8String>(transmute_static(result)) })
}
pub fn concat_character(&self, c: Utf8) -> Result<Utf8String, ()> {
    let mut result = Utf8String::new();
    result.reserve(self.len + 1)?;
    result.append(self)?;
    result.append_character(c)?;
    Ok(unsafe { core::mem::transmute::<Utf8String<'static>, Utf8String>(transmute_static(result)) })
}
pub fn concat_literal(&self, literal: &[Utf8]) -> Result<Utf8String, ()> {
    let n = literal.len();
    let mut result = Utf8String::new();
    result.reserve(self.len + n)?;
    result.append(self)?;
    result.append_literal(literal)?;
    Ok(unsafe { core::mem::transmute::<Utf8String<'static>, Utf8String>(transmute_static(result)) })
}
pub fn compare(&self, other: &Utf8String) -> i32 {
    let a_len = self.len;
    let b_len = other.len;
    let n = if a_len < b_len { a_len } else { b_len };
    let result = mem_compare(&self.data[..n], &other.data[..n]);
    if result == 0 {
        if a_len < b_len {
            -1
        } else if a_len > b_len {
            1
        } else {
            0
        }
    } else {
        result
    }
}
pub fn compare_literal(&self, literal: &[Utf8]) -> i32 {
    let a_len = self.len;
    let b_len = literal.len();
    let n = if a_len < b_len { a_len } else { b_len };
    let result = mem_compare(&self.data[..n], &literal[..n]);
    if result == 0 {
        if a_len < b_len {
            -1
        } else if a_len > b_len {
            1
        } else {
            0
        }
    } else {
        result
    }
}
pub fn substring(&self, start: usize, end: usize) -> Utf8StringView {
    let end = if end == usize::MAX || end > self.len {
        self.len
    } else {
        end
    };
    let start = if start > end { end } else { start };
    let slice = &self.data[start..end];
    Utf8StringView {
        data: slice,
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
    unsafe {
        let dst = data_mut_ptr(result.data);
        core::ptr::copy_nonoverlapping(self.data.as_ptr().add(start), dst, len);
    }
    result.len = len;
    Ok(unsafe { core::mem::transmute::<Utf8String<'static>, Utf8String>(transmute_static(result)) })
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
    let mut p = pos;
    if p == usize::MAX {
        if self.len == 0 {
            return None;
        }
        p = self.len - 1;
    } else if p >= self.len {
        return None;
    }
    let mut i = p;
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
    let a_len = self.len;
    let b_len = other.len;
    let n = if a_len < b_len { a_len } else { b_len };
    let result = mem_compare(&self.data[..n], &other.data[..n]);
    if result == 0 {
        if a_len < b_len {
            -1
        } else if a_len > b_len {
            1
        } else {
            0
        }
    } else {
        result
    }
}
pub fn compare_literal(&self, literal: &[Utf8]) -> i32 {
    let a_len = self.len;
    let b_len = literal.len();
    let n = if a_len < b_len { a_len } else { b_len };
    let result = mem_compare(&self.data[..n], &literal[..n]);
    if result == 0 {
        if a_len < b_len {
            -1
        } else if a_len > b_len {
            1
        } else {
            0
        }
    } else {
        result
    }
}
pub fn substring(&self, start: usize, end: usize) -> Utf8StringView<'a> {
    let end = if end == usize::MAX || end > self.len {
        self.len
    } else {
        end
    };
    let start = if start > end { end } else { start };
    let slice = &self.data[start..end];
    Utf8StringView {
        data: slice,
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
    unsafe {
        let dst = data_mut_ptr(result.data);
        core::ptr::copy_nonoverlapping(self.data.as_ptr().add(start), dst, len);
    }
    result.len = len;
    Ok(unsafe { core::mem::transmute::<Utf8String<'static>, Utf8String>(transmute_static(result)) })
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
    let mut p = pos;
    if p == usize::MAX {
        if self.len == 0 {
            return None;
        }
        p = self.len - 1;
    } else if p >= self.len {
        return None;
    }
    let mut i = p;
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

// Mimic strncmp/memcmp returning a tri-state result.
fn mem_compare(a: &[u8], b: &[u8]) -> i32 {
    let n = core::cmp::min(a.len(), b.len());
    for i in 0..n {
        if a[i] != b[i] {
            return (a[i] as i32) - (b[i] as i32);
        }
    }
    0
}

// Helper: convert a Utf8String<'_> into Utf8String<'static>. This is sound here
// because the buffer was allocated via Box::leak (which yields 'static).
fn transmute_static<'a>(s: Utf8String<'a>) -> Utf8String<'static> {
    unsafe { core::mem::transmute::<Utf8String<'a>, Utf8String<'static>>(s) }
}
