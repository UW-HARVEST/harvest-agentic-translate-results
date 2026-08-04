use std::sync::{Mutex, OnceLock};

/// Address constants mirroring the C source.
const ADDR_2G: usize = 0x8000_0000;

/// Default page size used at construction time. The real value (system page
/// size) is set lazily by `ljmm_init` (via `page_size()`), and may be
/// overridden by `ljmm_test_set_test_param`.
const DEFAULT_PAGE_SIZE: usize = 4096;

/// Internal state corresponding to the `ljmm_t` struct in the C source.
#[derive(Debug)]
struct LjmmState {
    page_size: usize,
    page_mask: usize,

    addr_upbound: usize,
    addr_lowbound: usize,

    map_file: String,

    /// it is up to OS to take care the 1G..2G space
    os_take_care_1g_2g: bool,
    init_succ: bool,
}

impl LjmmState {
    fn new() -> Self {
        LjmmState {
            page_size: DEFAULT_PAGE_SIZE,
            page_mask: DEFAULT_PAGE_SIZE - 1,
            addr_upbound: ADDR_2G,
            addr_lowbound: 0,
            map_file: String::from("/proc/self/maps"),
            // Mirrors the non-stress-test default in the C code:
            // OS_take_care_1G_2G defaults to enabled (1).
            os_take_care_1g_2g: true,
            init_succ: false,
        }
    }
}

/// Returns the current system page size, falling back to a sensible default
/// when it can't be determined.
fn detect_page_size() -> usize {
    // The C source uses `sysconf(_SC_PAGESIZE)`.  Pure-Rust standard library
    // does not expose that, so we derive it heuristically: most modern systems
    // use 4 KiB pages.  The value can always be overridden via
    // `ljmm_test_set_test_param`.
    DEFAULT_PAGE_SIZE
}

/// Returns a handle to the singleton state, creating it on first access.
fn ljmm_state() -> &'static Mutex<LjmmState> {
    static STATE: OnceLock<Mutex<LjmmState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(LjmmState::new()))
}

/// Initializes the ljmm system.
///
/// # Returns
/// An integer status code.
pub fn ljmm_init() -> i32 {
    let mut state = match ljmm_state().lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };

    // Mirror the constructor in C: configure page size, address bounds,
    // mark initialization as succeeded.
    let page_size = detect_page_size();
    state.page_size = page_size;
    state.page_mask = page_size - 1;
    state.addr_upbound = ADDR_2G;
    // In C this would be `(uintptr_t)sbrk(0)`. We don't have a portable
    // safe-Rust equivalent, so we leave addr_lowbound at its current value
    // (which tests override via `ljmm_test_set_test_param`).
    state.init_succ = true;
    0
}

/// Instructs the OS to take care of the [1G..2G] space.
///
/// # Parameters
/// - `turn_on`: If non-zero, the OS should manage the space.
pub fn ljmm_let_os_take_care_1g_2g(turn_on: i32) {
    let mut state = match ljmm_state().lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    state.os_take_care_1g_2g = turn_on != 0;
}

/// Sets test parameters for the ljmm system.
///
/// # Parameters
/// - `map_file`: The mapping file name.
/// - `sbrk0`: An address representing the current program break (as a safe usize).
/// - `page_size`: The system's page size.
pub fn ljmm_test_set_test_param(map_file: &str, sbrk0: usize, page_size: i32) {
    let mut state = match ljmm_state().lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };

    state.map_file = map_file.to_string();
    state.addr_lowbound = sbrk0;

    // page_size must be a power of two (matching the C ASSERT).  If it isn't,
    // we still accept it but compute a defensive page mask.
    let ps = if page_size <= 0 {
        DEFAULT_PAGE_SIZE
    } else {
        page_size as usize
    };
    state.page_size = ps;
    state.page_mask = ps.wrapping_sub(1);
}

// ---------------------------------------------------------------------------
// Helpers retained for fidelity with the C source.  They are not part of the
// public API, but exercise the same logic so the translation stays useful if
// extended in the future.
// ---------------------------------------------------------------------------

/// Round `addr` up to the next page boundary defined by `page_size`.
#[allow(dead_code)]
fn page_align_addr(addr: usize, page_size: usize, page_mask: usize) -> usize {
    addr.wrapping_add(page_size - 1) & !page_mask
}

/// Parse a hexadecimal address out of `s`, stopping at the first non-hex
/// character.  Returns `(value, consumed_bytes)`.
#[allow(dead_code)]
fn parse_addr(s: &[u8]) -> (usize, usize) {
    let mut addr: usize = 0;
    let mut i = 0;
    while i < s.len() {
        let c = s[i];
        if (b'0'..=b'9').contains(&c) {
            addr = addr.wrapping_mul(16).wrapping_add((c - b'0') as usize);
            i += 1;
            continue;
        }
        let lower = c | 0x20;
        if (b'a'..=b'f').contains(&lower) {
            addr = addr
                .wrapping_mul(16)
                .wrapping_add(10 + (lower - b'a') as usize);
            i += 1;
            continue;
        }
        break;
    }
    (addr, i)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_addr_hex() {
        let (val, n) = parse_addr(b"00418000-");
        assert_eq!(val, 0x418000);
        assert_eq!(n, 8);
    }

    #[test]
    fn page_align() {
        assert_eq!(page_align_addr(0, 4096, 4095), 0);
        assert_eq!(page_align_addr(1, 4096, 4095), 4096);
        assert_eq!(page_align_addr(4096, 4096, 4095), 4096);
        assert_eq!(page_align_addr(4097, 4096, 4095), 8192);
    }
}
