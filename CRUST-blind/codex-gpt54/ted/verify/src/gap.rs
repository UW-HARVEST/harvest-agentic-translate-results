pub const MEM_ERROR: i32 = 128;
#[derive(Clone)]
pub struct GapBuffer {
pub buffer: Vec<char>,
pub str_len: usize,
pub gap_len: usize,
pub gap_loc: usize,
}
impl GapBuffer {
pub fn create(capacity: usize) -> Self {
Self {
buffer: vec!['\0'; capacity],
str_len: 0,
gap_len: capacity,
gap_loc: 0,
}
}
pub fn destroy(self) {
drop(self);
}
pub fn insert_char(&mut self, ch: char) -> i32 {
if self.gap_len <= 1 {
let current_cap = self.gap_len + self.str_len;
let new_capacity = current_cap.saturating_mul(2).max(current_cap + 1).max(1);
if resize_buffer(self, new_capacity).is_err() {
return MEM_ERROR;
}
}

if self.gap_loc >= self.buffer.len() {
return MEM_ERROR;
}

self.buffer[self.gap_loc] = ch;
self.str_len += 1;
self.gap_loc += 1;
self.gap_len = self.gap_len.saturating_sub(1);
0
}
pub fn backspace(&mut self) {
if self.gap_loc > 0 {
self.gap_loc -= 1;
self.gap_len += 1;
self.str_len -= 1;
}
}
pub fn move_gap(&mut self, location: usize) -> i32 {
let location = location.min(self.str_len);
let capacity = self.gap_len + self.str_len;
let mut new_buffer = vec!['\0'; capacity];

if location < self.gap_loc {
new_buffer[..location].copy_from_slice(&self.buffer[..location]);

let moved_len = self.gap_loc - location;
let dst_start = location + self.gap_len;
let src_start = location;
let src_end = src_start + moved_len;
new_buffer[dst_start..dst_start + moved_len]
.copy_from_slice(&self.buffer[src_start..src_end]);

let suffix_len = self.str_len - self.gap_loc;
let old_suffix_start = self.gap_loc + self.gap_len;
let new_suffix_start = dst_start + moved_len;
new_buffer[new_suffix_start..new_suffix_start + suffix_len]
.copy_from_slice(&self.buffer[old_suffix_start..old_suffix_start + suffix_len]);
} else {
new_buffer[..self.gap_loc].copy_from_slice(&self.buffer[..self.gap_loc]);

let middle_len = location - self.gap_loc;
let old_middle_start = self.gap_loc + self.gap_len;
new_buffer[self.gap_loc..self.gap_loc + middle_len]
.copy_from_slice(&self.buffer[old_middle_start..old_middle_start + middle_len]);

let suffix_len = self.str_len - location;
let old_suffix_start = old_middle_start + middle_len;
let new_suffix_start = location + self.gap_len;
new_buffer[new_suffix_start..new_suffix_start + suffix_len]
.copy_from_slice(&self.buffer[old_suffix_start..old_suffix_start + suffix_len]);
}

self.buffer = new_buffer;
self.gap_loc = location;
0
}
pub fn get_string(&self) -> String {
let mut out = String::with_capacity(self.str_len);
for ch in &self.buffer[..self.gap_loc] {
out.push(*ch);
}
for ch in &self.buffer[self.gap_loc + self.gap_len..self.gap_loc + self.gap_len + (self.str_len - self.gap_loc)] {
out.push(*ch);
}
out
}
pub fn split(&self) -> Self {
let capacity = self.gap_len + self.str_len;
let second_half_len = self.str_len.saturating_sub(self.gap_loc);
let mut new_gap_buffer = Self::create(capacity);

let dst_start = capacity - second_half_len;
let src_start = self.gap_loc + self.gap_len;
new_gap_buffer.buffer[dst_start..dst_start + second_half_len]
.copy_from_slice(&self.buffer[src_start..src_start + second_half_len]);
new_gap_buffer.str_len = second_half_len;
new_gap_buffer.gap_loc = 0;
new_gap_buffer.gap_len = capacity - second_half_len;
new_gap_buffer
}
pub fn create_from_string(s: &str, gap_len: usize) -> Self {
if s.is_empty() {
return Self::create(gap_len);
}

let chars: Vec<char> = s.chars().collect();
let s_len = chars.len();
let capacity = s_len + gap_len;
let mut new_buffer = Self::create(capacity);
new_buffer.buffer[..s_len].copy_from_slice(&chars);
new_buffer.str_len = s_len;
new_buffer.gap_loc = s_len;
new_buffer.gap_len = gap_len;
new_buffer
}
pub fn char_at(&self, i: usize) -> char {
if self.str_len == 0 || i >= self.str_len {
return '\0';
}

if i < self.gap_loc {
self.buffer[i]
} else {
self.buffer[i + self.gap_len]
}
}
}

fn resize_buffer(instance: &mut GapBuffer, new_capacity: usize) -> Result<(), ()> {
let buffer_size = instance.str_len + instance.gap_len;
let new_capacity = new_capacity.max(buffer_size);
let gap_size = instance.gap_len + (new_capacity - buffer_size);
let mut new_buffer = vec!['\0'; new_capacity];

new_buffer[..instance.gap_loc].copy_from_slice(&instance.buffer[..instance.gap_loc]);

let suffix_len = instance.str_len - instance.gap_loc;
let new_suffix_start = instance.gap_loc + gap_size;
let old_suffix_start = instance.gap_loc + instance.gap_len;
new_buffer[new_suffix_start..new_suffix_start + suffix_len]
.copy_from_slice(&instance.buffer[old_suffix_start..old_suffix_start + suffix_len]);

instance.buffer = new_buffer;
instance.gap_len = gap_size;
Ok(())
}
