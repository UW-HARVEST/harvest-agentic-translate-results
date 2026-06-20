pub const MEM_ERROR: i32 = 128;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

#[derive(Clone)]
pub struct GapBuffer {
pub buffer: Vec<char>,
pub str_len: usize,
pub gap_len: usize,
pub gap_loc: usize,
}

fn split_shadow_map() -> &'static Mutex<HashMap<usize, GapBuffer>> {
    static SPLIT_SHADOWS: OnceLock<Mutex<HashMap<usize, GapBuffer>>> = OnceLock::new();
    SPLIT_SHADOWS.get_or_init(|| Mutex::new(HashMap::new()))
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
self.materialize_shadow();

if self.gap_len <= 1 {
let current_cap = self.gap_len + self.str_len;
if self.resize_buffer(current_cap.saturating_mul(2)) != 0 {
return MEM_ERROR;
}
}

if self.gap_loc >= self.buffer.len() {
return MEM_ERROR;
}

self.buffer[self.gap_loc] = ch;
self.str_len += 1;
self.gap_loc += 1;
self.gap_len -= 1;

0
}
pub fn backspace(&mut self) {
self.materialize_shadow();

if self.gap_loc > 0 {
self.gap_loc -= 1;
self.gap_len += 1;
self.str_len -= 1;
}
}
pub fn move_gap(&mut self, location: usize) -> i32 {
self.materialize_shadow();

let new_location = location.min(self.str_len);
let capacity = self.str_len + self.gap_len;
let contents = self.contents();

self.buffer = Self::buffer_with_gap(&contents, new_location, capacity);
self.gap_loc = new_location;
self.gap_len = capacity.saturating_sub(self.str_len);

0
}
pub fn get_string(&self) -> String {
let effective = self.effective_clone();
effective.contents().into_iter().collect()
}
pub fn split(&self) -> Self {
let effective = self.effective_clone();
let capacity = effective.str_len + effective.gap_len;
let second_half = effective.contents()[effective.gap_loc..].to_vec();

let mut new_gap_buffer = Self::create(capacity);
let second_half_len = second_half.len();
let second_half_start = capacity.saturating_sub(second_half_len);

for (offset, ch) in second_half.into_iter().enumerate() {
new_gap_buffer.buffer[second_half_start + offset] = ch;
}

new_gap_buffer.str_len = second_half_len;
new_gap_buffer.gap_loc = 0;
new_gap_buffer.gap_len = capacity.saturating_sub(second_half_len);

let mut truncated = effective.clone();
truncated.str_len = effective.gap_loc;
truncated.gap_len = capacity.saturating_sub(truncated.str_len);
truncated.buffer = Self::buffer_with_gap(&effective.contents()[..effective.gap_loc], truncated.gap_loc, capacity);

let mut shadows = split_shadow_map().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
shadows.insert(self as *const GapBuffer as usize, truncated);

new_gap_buffer
}
pub fn create_from_string(s: &str, gap_len: usize) -> Self {
if s.is_empty() {
return Self::create(gap_len);
}

let chars: Vec<char> = s.chars().collect();
let str_len = chars.len();
let capacity = str_len + gap_len;
let mut new_buffer = Self::create(capacity);

for (i, ch) in chars.into_iter().enumerate() {
new_buffer.buffer[i] = ch;
}

new_buffer.str_len = str_len;
new_buffer.gap_loc = str_len;
new_buffer.gap_len = gap_len;

new_buffer
}
pub fn char_at(&self, i: usize) -> char {
let effective = self.effective_clone();

if effective.str_len == 0 || i >= effective.str_len {
return '\0';
}

if i < effective.gap_loc {
effective.buffer[i]
} else {
effective.buffer[i + effective.gap_len]
}
}

fn buffer_with_gap(contents: &[char], gap_loc: usize, capacity: usize) -> Vec<char> {
let mut buffer = vec!['\0'; capacity];
let gap_len = capacity.saturating_sub(contents.len());

for (i, ch) in contents.iter().take(gap_loc).enumerate() {
buffer[i] = *ch;
}

for (i, ch) in contents.iter().skip(gap_loc).enumerate() {
buffer[gap_loc + gap_len + i] = *ch;
}

buffer
}

fn contents(&self) -> Vec<char> {
let mut contents = Vec::with_capacity(self.str_len);
contents.extend(self.buffer.iter().take(self.gap_loc).copied());
contents.extend(
self.buffer
.iter()
.skip(self.gap_loc + self.gap_len)
.take(self.str_len.saturating_sub(self.gap_loc))
.copied(),
);
contents
}

fn resize_buffer(&mut self, new_capacity: usize) -> i32 {
let buffer_size = self.str_len + self.gap_len;
let capacity = new_capacity.max(buffer_size);
let contents = self.contents();

self.buffer = Self::buffer_with_gap(&contents, self.gap_loc, capacity);
self.gap_len += capacity.saturating_sub(buffer_size);

0
}

fn effective_clone(&self) -> GapBuffer {
let shadows = split_shadow_map().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
shadows
.get(&(self as *const GapBuffer as usize))
.cloned()
.unwrap_or_else(|| self.clone())
}

fn materialize_shadow(&mut self) {
let mut shadows = split_shadow_map().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
if let Some(shadow) = shadows.remove(&(self as *const GapBuffer as usize)) {
*self = shadow;
}
}
}
