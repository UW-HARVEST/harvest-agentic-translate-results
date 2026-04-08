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

fn alloc_slice(cap: usize) -> &'static mut [Utf8] {
    let v = vec![0u8; cap];
    Box::leak(v.into_boxed_slice())
}

fn compare_bytes(a: &[Utf8], a_len: usize, b: &[Utf8], b_len: usize) -> i32 {
    let min_len = a_len.min(b_len);
    for i in 0..min_len {
        if a[i] != b[i] {
            return if (a[i] as i8) < (b[i] as i8) { -1 } else { 1 };
        }
    }
    if a_len < b_len { -1 } else if a_len > b_len { 1 } else { 0 }
}

fn get_data_mut(data: &[Utf8], cap: usize) -> &mut [Utf8] {
    let ptr = data.as_ptr() as *mut Utf8;
    unsafe { std::slice::from_raw_parts_mut(ptr, cap) }
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
        unsafe { drop(Box::from_raw(std::slice::from_raw_parts_mut(ptr, self.cap))); }
    }
    self.data = &[];
    self.len = 0;
    self.cap = 0;
}
pub fn reserve(&mut self, len: usize) -> Result<(), ()> {
    if len <= self.cap { return Ok(()); }
    let new_cap = round_up_pow2(len);
    let new_buf = alloc_slice(new_cap);
    new_buf[..self.len].copy_from_slice(&self.data[..self.len]);
    if self.cap > 0 {
        let ptr = self.data.as_ptr() as *mut Utf8;
        unsafe { drop(Box::from_raw(std::slice::from_raw_parts_mut(ptr, self.cap))); }
    }
    self.data = unsafe { &*(new_buf as *const [Utf8]) };
    self.cap = new_cap;
    Ok(())
}
pub fn shrink_to_fit(&mut self) -> Result<(), ()> {
    if self.cap == 0 { return Ok(()); }
    let new_cap = self.len;
    if new_cap == 0 {
        let ptr = self.data.as_ptr() as *mut Utf8;
        unsafe { drop(Box::from_raw(std::slice::from_raw_parts_mut(ptr, self.cap))); }
        self.data = &[];
        self.cap = 0;
        return Ok(());
    }
    let new_buf = alloc_slice(new_cap);
    new_buf[..new_cap].copy_from_slice(&self.data[..new_cap]);
    let ptr = self.data.as_ptr() as *mut Utf8;
    unsafe { drop(Box::from_raw(std::slice::from_raw_parts_mut(ptr, self.cap))); }
    self.data = unsafe { &*(new_buf as *const [Utf8]) };
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
    let other_data: Vec<u8> = other.data[..other_len].to_vec();
    self.reserve(self.len + other_len)?;
    let start = self.len;
    let d = get_data_mut(self.data, self.cap);
    d[start..start + other_len].copy_from_slice(&other_data);
    self.len += other_len;
    Ok(())
}
pub fn append_view(&mut self, view: &Utf8StringView) -> Result<(), ()> {
    let vlen = view.len;
    let vdata: Vec<u8> = view.data[..vlen].to_vec();
    self.reserve(self.len + vlen)?;
    let start = self.len;
    let d = get_data_mut(self.data, self.cap);
    d[start..start + vlen].copy_from_slice(&vdata);
    self.len += vlen;
    Ok(())
}
pub fn append_character(&mut self, c: Utf8) -> Result<(), ()> {
    self.reserve(self.len + 1)?;
    let start = self.len;
    get_data_mut(self.data, self.cap)[start] = c;
    self.len += 1;
    Ok(())
}
pub fn append_literal(&mut self, literal: &[Utf8]) -> Result<(), ()> {
    let n = literal.len();
    let lit: Vec<u8> = literal.to_vec();
    self.reserve(self.len + n)?;
    let start = self.len;
    let d = get_data_mut(self.data, self.cap);
    d[start..start + n].copy_from_slice(&lit);
    self.len += n;
    Ok(())
}
pub fn prepend(&mut self, other: &Utf8String) -> Result<(), ()> {
    let n = other.len;
    let src: Vec<u8> = other.data[..n].to_vec();
    self.reserve(self.len + n)?;
    let old_len = self.len;
    let d = get_data_mut(self.data, self.cap);
    d.copy_within(0..old_len, n);
    d[..n].copy_from_slice(&src);
    self.len += n;
    Ok(())
}
pub fn prepend_view(&mut self, view: &Utf8StringView) -> Result<(), ()> {
    let n = view.len;
    let src: Vec<u8> = view.data[..n].to_vec();
    self.reserve(self.len + n)?;
    let old_len = self.len;
    let d = get_data_mut(self.data, self.cap);
    d.copy_within(0..old_len, n);
    d[..n].copy_from_slice(&src);
    self.len += n;
    Ok(())
}
pub fn prepend_character(&mut self, c: Utf8) -> Result<(), ()> {
    self.reserve(self.len + 1)?;
    let old_len = self.len;
    let d = get_data_mut(self.data, self.cap);
    d.copy_within(0..old_len, 1);
    d[0] = c;
    self.len += 1;
    Ok(())
}
pub fn prepend_literal(&mut self, literal: &[Utf8]) -> Result<(), ()> {
    let n = literal.len();
    let src: Vec<u8> = literal.to_vec();
    self.reserve(self.len + n)?;
    let old_len = self.len;
    let d = get_data_mut(self.data, self.cap);
    d.copy_within(0..old_len, n);
    d[..n].copy_from_slice(&src);
    self.len += n;
    Ok(())
}
pub fn insert(&mut self, pos: usize, other: &Utf8String) -> Result<(), ()> {
    if pos > self.len { return Err(()); }
    let n = other.len;
    let src: Vec<u8> = other.data[..n].to_vec();
    let old_len = self.len;
    self.reserve(old_len + n)?;
    let d = get_data_mut(self.data, self.cap);
    d.copy_within(pos..old_len, pos + n);
    d[pos..pos + n].copy_from_slice(&src);
    self.len = old_len + n;
    Ok(())
}
pub fn insert_view(&mut self, pos: usize, view: &Utf8StringView) -> Result<(), ()> {
    if pos > self.len { return Err(()); }
    let n = view.len;
    let src: Vec<u8> = view.data[..n].to_vec();
    let old_len = self.len;
    self.reserve(old_len + n)?;
    let d = get_data_mut(self.data, self.cap);
    d.copy_within(pos..old_len, pos + n);
    d[pos..pos + n].copy_from_slice(&src);
    self.len = old_len + n;
    Ok(())
}
pub fn insert_character(&mut self, pos: usize, c: Utf8) -> Result<(), ()> {
    if pos > self.len { return Err(()); }
    let old_len = self.len;
    self.reserve(old_len + 1)?;
    let d = get_data_mut(self.data, self.cap);
    d.copy_within(pos..old_len, pos + 1);
    d[pos] = c;
    self.len = old_len + 1;
    Ok(())
}
pub fn insert_literal(&mut self, pos: usize, literal: &[Utf8]) -> Result<(), ()> {
    if pos > self.len { return Err(()); }
    let n = literal.len();
    let src: Vec<u8> = literal.to_vec();
    let old_len = self.len;
    self.reserve(old_len + n)?;
    let d = get_data_mut(self.data, self.cap);
    d.copy_within(pos..old_len, pos + n);
    d[pos..pos + n].copy_from_slice(&src);
    self.len = old_len + n;
    Ok(())
}
pub fn replace(&mut self, pos: usize, mut len: usize, replacement: &Utf8String) -> Result<(), ()> {
    if pos > self.len { return Err(()); }
    if pos + len > self.len { len = self.len - pos; }
    let rlen = replacement.len;
    let src: Vec<u8> = replacement.data[..rlen].to_vec();
    let new_len = self.len + rlen - len;
    let old_len = self.len;
    self.reserve(new_len)?;
    let d = get_data_mut(self.data, self.cap);
    d.copy_within(pos + len..old_len, pos + rlen);
    d[pos..pos + rlen].copy_from_slice(&src);
    self.len = new_len;
    Ok(())
}
pub fn replace_view(&mut self, pos: usize, mut len: usize, replacement: &Utf8StringView) -> Result<(), ()> {
    if pos > self.len { return Err(()); }
    if pos + len > self.len { len = self.len - pos; }
    let rlen = replacement.len;
    let src: Vec<u8> = replacement.data[..rlen].to_vec();
    let new_len = self.len + rlen - len;
    let old_len = self.len;
    self.reserve(new_len)?;
    let d = get_data_mut(self.data, self.cap);
    d.copy_within(pos + len..old_len, pos + rlen);
    d[pos..pos + rlen].copy_from_slice(&src);
    self.len = new_len;
    Ok(())
}
pub fn replace_character(&mut self, pos: usize, len: usize, c: Utf8) -> Result<(), ()> {
    if pos > self.len { return Err(()); }
    let new_len = self.len + 1 - len;
    let old_len = self.len;
    self.reserve(new_len)?;
    let d = get_data_mut(self.data, self.cap);
    d.copy_within(pos + len..old_len, pos + 1);
    d[pos] = c;
    self.len = new_len;
    Ok(())
}
pub fn replace_literal(&mut self, pos: usize, mut len: usize, literal: &[Utf8]) -> Result<(), ()> {
    if pos > self.len { return Err(()); }
    if pos + len > self.len { len = self.len - pos; }
    let n = literal.len();
    let src: Vec<u8> = literal.to_vec();
    let new_len = self.len + n - len;
    let old_len = self.len;
    self.reserve(new_len)?;
    let d = get_data_mut(self.data, self.cap);
    d.copy_within(pos + len..old_len, pos + n);
    d[pos..pos + n].copy_from_slice(&src);
    self.len = new_len;
    Ok(())
}
pub fn erase(&mut self, pos: usize, mut len: usize) -> Result<(), ()> {
    if pos > self.len { return Err(()); }
    if pos + len > self.len { len = self.len - pos; }
    let old_len = self.len;
    let d = get_data_mut(self.data, self.cap.max(old_len));
    d.copy_within(pos + len..old_len, pos);
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
    compare_bytes(self.data, self.len, other.data, other.len)
}
pub fn compare_literal(&self, literal: &[Utf8]) -> i32 {
    compare_bytes(self.data, self.len, literal, literal.len())
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
    let len = e - s;
    let mut result = Utf8String::new();
    result.reserve(len)?;
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
    compare_bytes(self.data, self.len, other.data, other.len)
}
pub fn compare_literal(&self, literal: &[Utf8]) -> i32 {
    compare_bytes(self.data, self.len, literal, literal.len())
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
    let len = e - s;
    let mut result = Utf8String::new();
    result.reserve(len)?;
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
