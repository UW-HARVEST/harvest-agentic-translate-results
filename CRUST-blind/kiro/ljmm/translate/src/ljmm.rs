use std::sync::Mutex;
use std::fs;

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

fn page_align_addr(addr: usize, page_size: usize, page_mask: usize) -> usize {
    (addr + page_size - 1) & !page_mask
}

fn parse_addr(bytes: &[u8]) -> (usize, usize) {
    let mut addr: usize = 0;
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
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

/// Finds the best-fit hole in the address space for the given length.
/// Returns the start address of the best-fit hole, or 0 if none found.
pub fn find_best_fit(length: usize) -> usize {
    let guard = LJMM.lock().unwrap();
    let state = match guard.as_ref() {
        Some(s) => s,
        None => return 0,
    };

    let content = match fs::read(&state.map_file) {
        Ok(c) => c,
        Err(_) => return 0,
    };

    // Mimic C: read at most BUFFER_SZ - 1 bytes, ensure trailing newline
    let mut buffer: Vec<u8> = content.into_iter().take(BUFFER_SZ - 1).collect();
    if buffer.is_empty() || *buffer.last().unwrap() != b'\n' {
        buffer.push(b'\n');
    }
    let buf_len = buffer.len();

    let length = page_align_addr(length, state.page_size, state.page_mask);
    let lowbound = state.addr_lowbound;
    let upbound = state.addr_upbound;

    let mut best_fit_start = upbound;
    let mut best_fit_size: usize = usize::MAX;
    let mut prev_start: usize = 0;
    let mut prev_size: usize = 0;
    let mut ofst: usize = 0;

    while ofst < buf_len {
        // step 1: parse start address
        let (start_addr, advance) = parse_addr(&buffer[ofst..]);
        if advance > 0 && ofst + advance < buf_len && buffer[ofst + advance] == b'-' {
            ofst += advance + 1;
        } else {
            break;
        }

        // step 2: parse end address
        let (mut end_addr, advance2) = parse_addr(&buffer[ofst..]);
        if advance2 > 0 && ofst + advance2 < buf_len && buffer[ofst + advance2] == b' ' {
            ofst += advance2 + 1;
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
        let hole_start = prev_start.wrapping_add(prev_size);
        let hole_size = start_addr.wrapping_sub(hole_start);

        if hole_size >= length
            && hole_start >= lowbound
            && (hole_start + length) <= upbound
            && hole_size < best_fit_size
        {
            best_fit_start = hole_start;
            best_fit_size = hole_size;
            if best_fit_size == length {
                break;
            }
        }

        // step 4: check if we should continue
        if start_addr >= ADDR_1G {
            if !state.os_take_care_1g_2g || start_addr >= upbound {
                break;
            }
        }

        if end_addr >= upbound {
            break;
        }

        prev_start = start_addr;
        prev_size = end_addr - start_addr;
    }

    if best_fit_size != usize::MAX {
        best_fit_start
    } else {
        0
    }
}

/// Initializes the ljmm system.
///
/// # Returns
/// An integer status code.
pub fn ljmm_init() -> i32 {
    let mut guard = LJMM.lock().unwrap();
    let mut state = LjmmState::default();
    state.init_succ = true;
    *guard = Some(state);
    1
}

/// Instructs the OS to take care of the [1G..2G] space.
///
/// # Parameters
/// - `turn_on`: If non-zero, the OS should manage the space.
pub fn ljmm_let_os_take_care_1g_2g(turn_on: i32) {
    let mut guard = LJMM.lock().unwrap();
    if let Some(ref mut state) = *guard {
        state.os_take_care_1g_2g = turn_on != 0;
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
    if let Some(ref mut state) = *guard {
        state.map_file = map_file.to_string();
        state.addr_lowbound = sbrk0;
        state.page_size = page_size as usize;
        state.page_mask = (page_size as usize) - 1;
    }
}
