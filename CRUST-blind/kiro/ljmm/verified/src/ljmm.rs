use std::fs;
use std::sync::Mutex;

const BUFFER_SZ: usize = 8192;
const ADDR_2G: usize = 0x80000000;
const ADDR_1G: usize = 0x40000000;

struct LjmmState {
    page_size: usize,
    page_mask: usize,
    addr_upbound: usize,
    addr_lowbound: usize,
    map_file: String,
    os_take_care_1g_2g: bool,
    init_succ: bool,
}

impl Default for LjmmState {
    fn default() -> Self {
        Self {
            page_size: 4096,
            page_mask: 4095,
            addr_upbound: ADDR_2G,
            addr_lowbound: 0,
            map_file: String::from("/proc/self/maps"),
            os_take_care_1g_2g: true,
            init_succ: false,
        }
    }
}

static LJMM: Mutex<Option<LjmmState>> = Mutex::new(None);

pub fn page_align_addr(addr: usize, page_size: usize, page_mask: usize) -> usize {
    (addr + page_size - 1) & !page_mask
}

pub fn parse_addr(s: &[u8]) -> (usize, usize) {
    let mut addr: usize = 0;
    let mut i = 0;
    while i < s.len() {
        let c = s[i];
        if c >= b'0' && c <= b'9' {
            addr = addr.wrapping_mul(16).wrapping_add((c - b'0') as usize);
            i += 1;
            continue;
        }
        let cl = c | 0x20;
        if cl >= b'a' && cl <= b'f' {
            addr = addr.wrapping_mul(16).wrapping_add((10 + cl - b'a') as usize);
            i += 1;
            continue;
        }
        break;
    }
    (addr, i)
}

struct MemBlk {
    start: usize,
    size: usize,
}

fn find_best_fit(state: &LjmmState, length: usize) -> usize {
    let buffer = match read_maps_to_buffer(&state.map_file) {
        Some(b) => b,
        None => return 0,
    };

    let length = page_align_addr(length, state.page_size, state.page_mask);
    let lowbound = state.addr_lowbound;
    let upbound = state.addr_upbound;

    let mut best_fit = MemBlk { start: upbound, size: usize::MAX };
    let mut prev_blk = MemBlk { start: 0, size: 0 };
    let mut ofst: usize = 0;
    let buf_len = buffer.len();

    while ofst < buf_len {
        // step 1: parse start address
        let (start_addr, advance) = parse_addr(&buffer[ofst..]);
        if advance > 0 && ofst + advance < buf_len && buffer[ofst + advance] == b'-' {
            ofst += advance + 1;
        } else {
            break;
        }

        // step 2: parse end address
        let (mut end_addr, advance) = parse_addr(&buffer[ofst..]);
        if advance > 0 && ofst + advance < buf_len && buffer[ofst + advance] == b' ' {
            ofst += advance + 1;
            // skip rest of line
            while ofst < buf_len && buffer[ofst] != b'\n' {
                ofst += 1;
            }
            if ofst < buf_len {
                ofst += 1; // skip '\n'
            }
        } else {
            end_addr = if upbound >= start_addr { upbound } else { start_addr };
        }

        end_addr = page_align_addr(end_addr, state.page_size, state.page_mask);

        // step 3: check hole between previous block and current
        let hole_start = prev_blk.start + prev_blk.size;
        let hole_size = start_addr.wrapping_sub(hole_start);

        if hole_size >= length
            && hole_start >= lowbound
            && (hole_start + length) <= upbound
            && hole_size < best_fit.size
        {
            best_fit.start = hole_start;
            best_fit.size = hole_size;
            if best_fit.size == length {
                break;
            }
        }

        // step 4: determine if we need to continue
        if start_addr >= ADDR_1G {
            if !state.os_take_care_1g_2g || start_addr >= upbound {
                break;
            }
        }

        if end_addr >= upbound {
            break;
        }

        prev_blk.start = start_addr;
        prev_blk.size = end_addr - start_addr;
    }

    if best_fit.size != usize::MAX { best_fit.start } else { 0 }
}

fn read_maps_to_buffer(map_file: &str) -> Option<Vec<u8>> {
    let data = fs::read(map_file).ok()?;
    let mut buf: Vec<u8> = if data.len() > BUFFER_SZ - 1 {
        data[..BUFFER_SZ - 1].to_vec()
    } else {
        data
    };
    if buf.is_empty() || *buf.last().unwrap() != b'\n' {
        buf.push(b'\n');
    }
    Some(buf)
}

/// Initializes the ljmm system.
///
/// # Returns
/// An integer status code.
pub fn ljmm_init() -> i32 {
    let mut guard = LJMM.lock().unwrap();
    let state = LjmmState::default();
    let succ = 1_i32;
    *guard = Some(LjmmState { init_succ: true, ..state });
    succ
}

/// Instructs the OS to take care of the [1G..2G] space.
///
/// # Parameters
/// - `turn_on`: If non-zero, the OS should manage the space.
pub fn ljmm_let_os_take_care_1g_2g(turn_on: i32) {
    let mut guard = LJMM.lock().unwrap();
    if let Some(ref mut state) = *guard {
        state.os_take_care_1g_2g = turn_on != 0;
    } else {
        // Initialize with default if not yet initialized
        *guard = Some(LjmmState {
            os_take_care_1g_2g: turn_on != 0,
            ..LjmmState::default()
        });
    }
}

/// Sets test parameters for the ljmm system.
///
/// # Parameters
/// - `map_file`: The mapping file name.
/// - `sbrk0`: An address representing the current program break (as a safe usize).
/// - `page_size`: The system's page size.
pub fn ljmm_test_set_test_param(map_file: &str, sbrk0: usize, page_size: i32) {
    let mut guard = LJMM.lock().unwrap();
    let ps = page_size as usize;
    if let Some(ref mut state) = *guard {
        state.map_file = map_file.to_string();
        state.addr_lowbound = sbrk0;
        state.page_size = ps;
        state.page_mask = ps - 1;
    } else {
        *guard = Some(LjmmState {
            map_file: map_file.to_string(),
            addr_lowbound: sbrk0,
            page_size: ps,
            page_mask: ps - 1,
            ..LjmmState::default()
        });
    }
}

pub fn ljmm_find_best_fit(length: usize) -> usize {
    let guard = LJMM.lock().unwrap();
    if let Some(ref state) = *guard {
        find_best_fit(state, length)
    } else {
        0
    }
}
