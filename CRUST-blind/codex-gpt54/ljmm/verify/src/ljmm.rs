use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

const BUFFER_SZ: usize = 8192;
const ADDR_1G: usize = 0x4000_0000;
const ADDR_2G: usize = 0x8000_0000;

#[derive(Clone, Debug)]
struct LjmmState {
    page_size: usize,
    page_mask: usize,
    addr_upbound: usize,
    addr_lowbound: usize,
    map_file: PathBuf,
    buffer: String,
    buf_len: usize,
    os_take_care_1g_2g: bool,
    init_succ: bool,
}

impl Default for LjmmState {
    fn default() -> Self {
        let page_size = page_size::get();
        Self {
            page_size,
            page_mask: page_size.saturating_sub(1),
            addr_upbound: ADDR_2G,
            addr_lowbound: detect_heap_end().unwrap_or(0),
            map_file: PathBuf::from("/proc/self/maps"),
            buffer: String::with_capacity(BUFFER_SZ),
            buf_len: 0,
            os_take_care_1g_2g: true,
            init_succ: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct MemBlk {
    start: usize,
    size: usize,
}

fn state() -> &'static Mutex<LjmmState> {
    static STATE: OnceLock<Mutex<LjmmState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(LjmmState::default()))
}

fn with_state<R>(f: impl FnOnce(&mut LjmmState) -> R) -> R {
    match state().lock() {
        Ok(mut guard) => f(&mut guard),
        Err(poisoned) => {
            let mut guard = poisoned.into_inner();
            f(&mut guard)
        }
    }
}

fn detect_heap_end() -> Option<usize> {
    let contents = std::fs::read_to_string("/proc/self/maps").ok()?;
    for line in contents.lines() {
        if !line.ends_with("[heap]") {
            continue;
        }

        let (range, _) = line.split_once(' ')?;
        let (_, end) = range.split_once('-')?;
        return usize::from_str_radix(end, 16).ok();
    }

    None
}

fn page_align_addr(addr: usize, page_size: usize, page_mask: usize) -> usize {
    addr.wrapping_add(page_size.saturating_sub(1)) & !page_mask
}

fn parse_addr(addr_str: &str) -> (usize, usize) {
    let mut addr = 0usize;
    let mut consumed = 0usize;

    for byte in addr_str.bytes() {
        let digit = match byte {
            b'0'..=b'9' => Some((byte - b'0') as usize),
            b'a'..=b'f' => Some((byte - b'a' + 10) as usize),
            b'A'..=b'F' => Some((byte - b'A' + 10) as usize),
            _ => None,
        };

        match digit {
            Some(d) => {
                addr = addr.wrapping_mul(16).wrapping_add(d);
                consumed += 1;
            }
            None => break,
        }
    }

    (addr, consumed)
}

fn read_maps_to_buffer(state: &mut LjmmState) -> Option<usize> {
    let mut file = File::open(&state.map_file).ok()?;
    let mut raw = vec![0u8; BUFFER_SZ - 1];
    let len = file.read(&mut raw).ok()?;
    raw.truncate(len);

    if raw.last().copied() != Some(b'\n') {
        raw.push(b'\n');
    }

    state.buffer = String::from_utf8_lossy(&raw).into_owned();
    state.buf_len = state.buffer.len();
    Some(state.buf_len)
}

fn find_best_fit(state: &mut LjmmState, length: usize) -> usize {
    if read_maps_to_buffer(state).is_none() {
        return 0;
    }

    let length = page_align_addr(length, state.page_size, state.page_mask);
    let lowbound = state.addr_lowbound;
    let upbound = state.addr_upbound;
    let buffer = state.buffer.as_bytes();
    let buf_len = state.buf_len;

    let mut best_fit = MemBlk {
        start: upbound,
        size: usize::MAX,
    };
    let mut prev_blk = MemBlk::default();
    let mut ofst = 0usize;

    while ofst < buf_len {
        let remaining = &state.buffer[ofst..];
        let (start_addr, advance) = parse_addr(remaining);
        if advance != 0 && buffer.get(ofst + advance) == Some(&b'-') {
            ofst += advance + 1;
        } else {
            break;
        }

        let remaining = &state.buffer[ofst..];
        let (mut end_addr, advance) = parse_addr(remaining);
        if advance != 0 && buffer.get(ofst + advance) == Some(&b' ') {
            ofst += advance + 1;
            while ofst < buf_len && buffer[ofst] != b'\n' {
                ofst += 1;
            }
            if ofst < buf_len {
                ofst += 1;
            }
        } else {
            end_addr = if upbound >= start_addr {
                upbound
            } else {
                start_addr
            };
        }

        end_addr = page_align_addr(end_addr, state.page_size, state.page_mask);

        let hole_start = prev_blk.start.wrapping_add(prev_blk.size);
        let hole_size = start_addr.wrapping_sub(hole_start);
        let fits_bounds = hole_start >= lowbound
            && hole_start
                .checked_add(length)
                .is_some_and(|end| end <= upbound);

        if hole_size >= length && fits_bounds && hole_size < best_fit.size {
            best_fit = MemBlk {
                start: hole_start,
                size: hole_size,
            };

            if best_fit.size == length {
                break;
            }
        }

        if start_addr >= ADDR_1G && (!state.os_take_care_1g_2g || start_addr >= upbound) {
            break;
        }

        if end_addr >= upbound {
            break;
        }

        prev_blk = MemBlk {
            start: start_addr,
            size: end_addr.wrapping_sub(start_addr),
        };
    }

    if best_fit.size != usize::MAX {
        best_fit.start
    } else {
        0
    }
}

/// Initializes the ljmm system.
///
/// # Returns
/// An integer status code.
pub fn ljmm_init() -> i32 {
    with_state(|state| {
        let page_size = page_size::get();

        state.page_size = page_size;
        state.page_mask = page_size.saturating_sub(1);
        state.addr_lowbound = detect_heap_end().unwrap_or(state.addr_lowbound);
        state.addr_upbound = ADDR_2G;
        state.map_file = PathBuf::from("/proc/self/maps");
        state.buffer.clear();
        state.buf_len = 0;
        state.os_take_care_1g_2g = true;
        state.init_succ = true;

        let _ = find_best_fit(state, 1);
        i32::from(state.init_succ)
    })
}
/// Instructs the OS to take care of the [1G..2G] space.
///
/// # Parameters
/// - `turn_on`: If non-zero, the OS should manage the space.
pub fn ljmm_let_os_take_care_1g_2g(turn_on: i32) {
    with_state(|state| {
        state.os_take_care_1g_2g = turn_on != 0;
    });
}
/// Sets test parameters for the ljmm system.
///
/// # Parameters
/// - `map_file`: The mapping file name.
/// - `sbrk0`: An address representing the current program break (as a safe usize).
/// - `page_size`: The system's page size.
pub fn ljmm_test_set_test_param(map_file: &str, sbrk0: usize, page_size: i32) {
    with_state(|state| {
        state.map_file = PathBuf::from(map_file);
        state.addr_lowbound = sbrk0;

        if page_size > 0 {
            let page_size = page_size as usize;
            if page_size.is_power_of_two() {
                state.page_size = page_size;
                state.page_mask = page_size - 1;
            }
        }
    });
}
