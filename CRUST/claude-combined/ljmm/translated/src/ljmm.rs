use std::sync::Mutex;

#[allow(dead_code)]
const BUFFER_SZ: usize = 8192;
const ADDR_2G: usize = 0x80000000;
#[allow(dead_code)]
const ADDR_1G: usize = 0x40000000;

struct LjmmState {
    page_size: usize,
    page_mask: usize,

    addr_upbound: usize,
    addr_lowbound: usize,

    map_file: Option<String>,

    /// it is up to OS to take care the 1G..2G space
    os_take_care_1g_2g: bool,
    init_succ: bool,
}

impl LjmmState {
    const fn new() -> Self {
        Self {
            page_size: 0,
            page_mask: 0,
            addr_upbound: 0,
            addr_lowbound: 0,
            map_file: None,
            os_take_care_1g_2g: true,
            init_succ: false,
        }
    }
}

static LJMM: Mutex<LjmmState> = Mutex::new(LjmmState::new());

/// Initializes the ljmm system.
///
/// # Returns
/// An integer status code.
pub fn ljmm_init() -> i32 {
    let mut state = match LJMM.lock() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    // Mirror the C constructor: STRESS_TEST is not set, so default is true (1).
    state.os_take_care_1g_2g = true;
    state.addr_lowbound = 0;
    state.addr_upbound = ADDR_2G;

    // Use a sane default page size (4096 is typical on Linux/x86-64).
    let page_size: usize = 4096;
    state.page_size = page_size;
    state.page_mask = page_size - 1;

    state.map_file = Some(String::from("/proc/self/maps"));
    state.init_succ = true;

    0
}

/// Instructs the OS to take care of the [1G..2G] space.
///
/// # Parameters
/// - `turn_on`: If non-zero, the OS should manage the space.
pub fn ljmm_let_os_take_care_1g_2g(turn_on: i32) {
    if let Ok(mut state) = LJMM.lock() {
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
    if let Ok(mut state) = LJMM.lock() {
        state.map_file = Some(map_file.to_string());
        state.addr_lowbound = sbrk0;

        // page-size must be a power-of-two
        debug_assert!(page_size > 0 && ((page_size - 1) & page_size) == 0);
        let ps = page_size as usize;
        state.page_size = ps;
        state.page_mask = ps.wrapping_sub(1);
    }
}

// ---------------------------------------------------------------------------
// Helpers that mirror the original C code. They are not directly required by
// the current tests, but are kept here so the module's behavior is faithful
// to the original implementation.
// ---------------------------------------------------------------------------

#[allow(dead_code)]
fn page_align_addr(addr: usize, page_size: usize, page_mask: usize) -> usize {
    (addr + page_size - 1) & !page_mask
}

/// Parse a hex address from `bytes`, stopping at the first non-hex character.
/// Returns `(address, num_bytes_consumed)`.
#[allow(dead_code)]
fn parse_addr(bytes: &[u8]) -> (usize, usize) {
    let mut addr: usize = 0;
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if c.is_ascii_digit() {
            addr = addr * 16 + (c - b'0') as usize;
        } else {
            let lc = c | 0x20;
            if (b'a'..=b'f').contains(&lc) {
                addr = addr * 16 + 10 + (lc - b'a') as usize;
            } else {
                break;
            }
        }
        i += 1;
    }
    (addr, i)
}

/// Find a best-fit hole in the address space described by the maps file
/// previously set via `ljmm_test_set_test_param`. Returns `Some(addr)` on
/// success, or `None` if no fit was found.
#[allow(dead_code)]
fn find_best_fit(length: usize) -> Option<usize> {
    let state = LJMM.lock().ok()?;
    let map_file = state.map_file.as_ref()?.clone();
    let page_size = state.page_size;
    let page_mask = state.page_mask;
    let lowbound = state.addr_lowbound;
    let upbound = state.addr_upbound;
    let os_take_care = state.os_take_care_1g_2g;
    drop(state);

    let buffer = std::fs::read(&map_file).ok()?;
    if buffer.is_empty() {
        return None;
    }

    // Ensure trailing newline like the C code does.
    let mut buffer = buffer;
    if *buffer.last().unwrap() != b'\n' {
        buffer.push(b'\n');
    }

    let length = page_align_addr(length, page_size, page_mask);
    let mut best_fit_size: usize = usize::MAX;
    let mut best_fit_start: usize = upbound;
    let mut prev_start: usize = 0;
    let mut prev_size: usize = 0;
    let mut ofst: usize = 0;
    let buf_len = buffer.len();

    while ofst < buf_len {
        // step 1: parse start of address range
        let (start_addr, advance) = parse_addr(&buffer[ofst..]);
        if advance == 0 || ofst + advance >= buf_len || buffer[ofst + advance] != b'-' {
            break;
        }
        ofst += advance + 1;

        // step 2: parse end of address range
        let (mut end_addr, advance) = parse_addr(&buffer[ofst..]);
        let have_end = advance > 0
            && ofst + advance < buf_len
            && buffer[ofst + advance] == b' ';
        if have_end {
            ofst += advance + 1;
            // skip the rest of the line
            while ofst < buf_len && buffer[ofst] != b'\n' {
                ofst += 1;
            }
            if ofst < buf_len {
                ofst += 1; // skip the '\n'
            }
        } else {
            end_addr = if upbound >= start_addr { upbound } else { start_addr };
        }

        end_addr = page_align_addr(end_addr, page_size, page_mask);

        // step 3: examine the hole between previous and current blocks
        let hole_start = prev_start + prev_size;
        let hole_size = start_addr.saturating_sub(hole_start);

        if hole_size >= length
            && hole_start >= lowbound
            && hole_start.checked_add(length).map(|v| v <= upbound).unwrap_or(false)
            && hole_size < best_fit_size
        {
            best_fit_start = hole_start;
            best_fit_size = hole_size;
            if best_fit_size == length {
                break;
            }
        }

        // step 4: termination conditions
        if start_addr >= ADDR_1G {
            if !os_take_care || start_addr >= upbound {
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
        Some(best_fit_start)
    } else {
        None
    }
}
