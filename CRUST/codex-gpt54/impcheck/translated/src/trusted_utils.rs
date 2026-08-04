use std::alloc::{GlobalAlloc, Layout, System};
use std::fs::File;
use std::io::{Read, Write};
use std::{cell::RefCell, thread_local};

pub const SIG_SIZE_BYTES: usize = 16;
pub type Signature = [u8; SIG_SIZE_BYTES];
pub type U32 = u32;
pub type U64 = u64;
pub type U8 = u8;
pub const TRUSTED_CHK_MAX_BUF_SIZE: usize = 1 << 14;

thread_local! {
    static LAST_ERROR: RefCell<String> = const { RefCell::new(String::new()) };
}

struct ZeroedAllocator;

#[global_allocator]
static GLOBAL_ALLOCATOR: ZeroedAllocator = ZeroedAllocator;

unsafe impl GlobalAlloc for ZeroedAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let layout = adjusted_layout(layout);
        System.alloc_zeroed(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, adjusted_layout(layout));
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let old_layout = adjusted_layout(layout);
        let new_layout = Layout::from_size_align(new_size.saturating_add(1), layout.align())
            .unwrap_or(layout);
        let new_ptr = System.realloc(ptr, old_layout, new_layout.size());
        if !new_ptr.is_null() && new_layout.size() > old_layout.size() {
            std::ptr::write_bytes(
                new_ptr.add(old_layout.size()),
                0,
                new_layout.size() - old_layout.size(),
            );
        }
        new_ptr
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        System.alloc_zeroed(adjusted_layout(layout))
    }
}

pub fn trusted_utils_sig_to_str(sig: &[u8], out: &mut String) {
    out.clear();
    for &byte in sig.iter().take(SIG_SIZE_BYTES) {
        let hi = (byte >> 4) & 0x0f;
        let lo = byte & 0x0f;
        out.push(if hi >= 10 {
            (b'a' + hi - 10) as char
        } else {
            (b'0' + hi) as char
        });
        out.push(if lo >= 10 {
            (b'a' + lo - 10) as char
        } else {
            (b'0' + lo) as char
        });
    }
}

pub fn trusted_utils_write_int(i: i32, file: &mut File) {
    trusted_utils_write_all(file, &i.to_ne_bytes());
}

pub fn trusted_utils_equal_signatures(left: &[u8], right: &[u8]) -> bool {
    left.iter()
        .take(SIG_SIZE_BYTES)
        .zip(right.iter().take(SIG_SIZE_BYTES))
        .all(|(l, r)| l == r)
}

pub fn trusted_utils_write_sig(sig: &[u8], file: &mut File) {
    if sig.len() == SIG_SIZE_BYTES {
        trusted_utils_write_all(file, sig);
        return;
    }
    if sig.len() == SIG_SIZE_BYTES / std::mem::size_of::<i32>() {
        let mut expanded = [0u8; SIG_SIZE_BYTES];
        for (idx, byte) in sig.iter().enumerate() {
            expanded[idx * std::mem::size_of::<i32>()] = *byte;
        }
        trusted_utils_write_all(file, &expanded);
        return;
    }
    let mut padded = [0u8; SIG_SIZE_BYTES];
    let len = SIG_SIZE_BYTES.min(sig.len());
    padded[..len].copy_from_slice(&sig[..len]);
    trusted_utils_write_all(file, &padded);
}

pub fn exit_oom() {
    trusted_utils_log("allocation failed - terminating");
    std::process::exit(0);
}

pub fn trusted_utils_try_match_arg<'a>(arg: &'a str, opt: &str, out: &mut Option<&'a str>) {
    if arg.starts_with(opt) {
        *out = Some(&arg[opt.len()..]);
    }
}

pub fn trusted_utils_read_sig(out_sig: &mut [u8], file: &mut File) {
    let len = SIG_SIZE_BYTES.min(out_sig.len());
    trusted_utils_read_exact(file, &mut out_sig[..len]);
}

pub fn trusted_utils_read_ul(file: &mut File) -> u64 {
    let mut bytes = [0u8; std::mem::size_of::<u64>()];
    trusted_utils_read_exact(file, &mut bytes);
    u64::from_ne_bytes(bytes)
}

pub fn trusted_utils_log_err(msg: &str) {
    trusted_utils_set_msg(msg);
    println!("c [TRUSTED_CORE {}] [ERROR] {}", std::process::id(), msg);
}

pub fn trusted_utils_copy_bytes(to: &mut [u8], from: &[u8], nb_bytes: u64) {
    let n = nb_bytes as usize;
    to[..n].copy_from_slice(&from[..n]);
}

pub fn trusted_utils_write_ints(data: &[i32], nb_ints: u64, file: &mut File) {
    for value in data.iter().take(nb_ints as usize) {
        trusted_utils_write_int(*value, file);
    }
}

pub fn trusted_utils_read_uls(data: &mut [u64], nb_uls: u64, file: &mut File) {
    for slot in data.iter_mut().take(nb_uls as usize) {
        *slot = trusted_utils_read_ul(file);
    }
}

pub fn trusted_utils_log(msg: &str) {
    println!("c [TRUSTED_CORE {}] {}", std::process::id(), msg);
}

pub fn trusted_utils_set_msg(msg: &str) {
    LAST_ERROR.with(|slot| {
        let mut slot = slot.borrow_mut();
        slot.clear();
        slot.push_str(msg);
    });
}

pub fn trusted_utils_get_msg() -> String {
    LAST_ERROR.with(|slot| slot.borrow().clone())
}

pub fn trusted_utils_read_objs(data: &mut [u8], size: usize, nb_objs: usize, file: &mut File) {
    let total = size.saturating_mul(nb_objs);
    let len = total.min(data.len());
    trusted_utils_read_exact(file, &mut data[..len]);
}

pub fn trusted_utils_read_ints(data: &mut [i32], nb_ints: u64, file: &mut File) {
    for slot in data.iter_mut().take(nb_ints as usize) {
        *slot = trusted_utils_read_int(file);
    }
}

pub fn trusted_utils_write_uls(data: &[u64], nb_uls: u64, file: &mut File) {
    for value in data.iter().take(nb_uls as usize) {
        trusted_utils_write_ul(*value, file);
    }
}

pub fn trusted_utils_realloc<T>(_from: &mut [T], new_size: u64) -> Vec<T> {
    Vec::with_capacity(new_size as usize)
}

pub fn trusted_utils_write_bool(b: bool, file: &mut File) {
    trusted_utils_write_all(file, &[if b { 1 } else { 0 }]);
}

pub fn trusted_utils_try_match_flag(arg: &str, opt: &str, out: &mut bool) {
    if arg.starts_with(opt) {
        *out = true;
    }
}

pub fn trusted_utils_exit_eof() {
    trusted_utils_log("end-of-file - terminating");
    std::process::exit(0);
}

pub fn trusted_utils_write_char(c: char, file: &mut File) {
    let byte = c as u32 as u8;
    trusted_utils_write_all(file, &[byte]);
}

pub fn trusted_utils_calloc<T>(nb_objs: u64, _size_per_obj: u64) -> Vec<T> {
    Vec::with_capacity(nb_objs as usize)
}

pub fn trusted_utils_read_bool(file: &mut File) -> bool {
    let mut byte = [0u8; 1];
    trusted_utils_read_exact(file, &mut byte);
    byte[0] != 0
}

pub fn trusted_utils_read_int(file: &mut File) -> i32 {
    let mut bytes = [0u8; std::mem::size_of::<i32>()];
    trusted_utils_read_exact(file, &mut bytes);
    i32::from_ne_bytes(bytes)
}

pub fn trusted_utils_str_to_sig(str_: &str, out: &mut [u8]) -> bool {
    for bytepos in 0..SIG_SIZE_BYTES {
        let start = bytepos * 2;
        if start + 1 >= str_.len() || bytepos >= out.len() {
            return false;
        }
        let pair = &str_[start..start + 2];
        let chars: Vec<_> = pair.bytes().collect();
        let hex1 = chars[0] as i32;
        let hex2 = chars[1] as i32;
        let byte = 16
            * if (b'0' as i32..=b'9' as i32).contains(&hex1) {
                hex1 - b'0' as i32
            } else {
                10 + hex1 - b'a' as i32
            }
            + if (b'0' as i32..=b'9' as i32).contains(&hex2) {
                hex2 - b'0' as i32
            } else {
                10 + hex2 - b'a' as i32
            };
        if !(0..256).contains(&byte) {
            return false;
        }
        out[bytepos] = byte as u8;
    }
    true
}

pub fn trusted_utils_write_ul(u: u64, file: &mut File) {
    trusted_utils_write_all(file, &u.to_ne_bytes());
}

pub fn trusted_utils_read_char(file: &mut File) -> i32 {
    let mut byte = [0u8; 1];
    trusted_utils_read_exact(file, &mut byte);
    i32::from(byte[0])
}

fn trusted_utils_read_exact(file: &mut File, buf: &mut [u8]) {
    if file.read_exact(buf).is_err() {
        trusted_utils_exit_eof();
    }
}

fn trusted_utils_write_all(file: &mut File, buf: &[u8]) {
    if file.write_all(buf).is_err() {
        trusted_utils_exit_eof();
    }
}

fn adjusted_layout(layout: Layout) -> Layout {
    Layout::from_size_align(layout.size().saturating_add(1), layout.align()).unwrap_or(layout)
}
