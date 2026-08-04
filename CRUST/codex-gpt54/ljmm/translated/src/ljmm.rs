use std::fs::File;
use std::io::Read;
use std::sync::{LazyLock, Mutex, MutexGuard};

const BUFFER_SZ: usize = 8192;
const DUMMY_BLK_SZ: usize = 12;
const ADDR_1G: usize = 0x4000_0000;
const ADDR_2G: usize = 0x8000_0000;

#[derive(Debug, Clone)]
struct LjmmState {
    page_size: usize,
    page_mask: usize,
    addr_upbound: usize,
    addr_lowbound: usize,
    dummy_blk: Vec<u8>,
    map_file: String,
    buffer: Vec<u8>,
    buf_len: usize,
    os_take_care_1g_2g: bool,
    init_succ: bool,
}

impl LjmmState {
    fn new() -> Self {
        let page_size = 4096;
        let addr_lowbound = heap_probe_addr();

        Self {
            page_size,
            page_mask: page_size.saturating_sub(1),
            addr_upbound: ADDR_2G,
            addr_lowbound,
            dummy_blk: vec![0; DUMMY_BLK_SZ],
            map_file: "/proc/self/maps".to_string(),
            buffer: vec![0; BUFFER_SZ],
            buf_len: 0,
            os_take_care_1g_2g: true,
            init_succ: true,
        }
    }
}

static LJMM: LazyLock<Mutex<LjmmState>> = LazyLock::new(|| Mutex::new(LjmmState::new()));

fn lock_state() -> MutexGuard<'static, LjmmState> {
    match LJMM.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn heap_probe_addr() -> usize {
    let probe = Box::new(0_u8);
    (&*probe) as *const u8 as usize
}

fn page_align_addr(addr: usize, page_size: usize, page_mask: usize) -> usize {
    (addr + page_size.saturating_sub(1)) & !page_mask
}

fn parse_addr(addr_str: &[u8]) -> (usize, usize) {
    let mut addr = 0usize;
    let mut consumed = 0usize;

    for &byte in addr_str {
        let digit = match byte {
            b'0'..=b'9' => (byte - b'0') as usize,
            b'a'..=b'f' => 10 + (byte - b'a') as usize,
            b'A'..=b'F' => 10 + (byte - b'A') as usize,
            _ => break,
        };

        addr = addr.saturating_mul(16).saturating_add(digit);
        consumed += 1;
    }

    (addr, consumed)
}

fn read_maps_to_buffer(state: &mut LjmmState) -> std::io::Result<usize> {
    let mut file = File::open(&state.map_file)?;
    let mut len = file.read(&mut state.buffer[..BUFFER_SZ - 1])?;

    if len > 0 && state.buffer[len - 1] != b'\n' {
        state.buffer[len] = b'\n';
        len += 1;
    }

    state.buf_len = len;
    Ok(len)
}

fn find_best_fit(state: &mut LjmmState, length: usize) -> Option<usize> {
    if read_maps_to_buffer(state).is_err() {
        return None;
    }

    if state.buf_len == 0 {
        return None;
    }

    let length = page_align_addr(length, state.page_size, state.page_mask);
    let buffer = &state.buffer[..state.buf_len];
    let lowbound = state.addr_lowbound;
    let upbound = state.addr_upbound;

    let mut best_fit_start = upbound;
    let mut best_fit_size = usize::MAX;
    let mut prev_blk_start = 0usize;
    let mut prev_blk_size = 0usize;
    let mut ofst = 0usize;

    while ofst < buffer.len() {
        let (start_addr, advance) = parse_addr(&buffer[ofst..]);
        if advance == 0 || ofst + advance >= buffer.len() || buffer[ofst + advance] != b'-' {
            break;
        }
        ofst += advance + 1;

        let (mut end_addr, advance) = parse_addr(&buffer[ofst..]);
        if advance > 0 && ofst + advance < buffer.len() && buffer[ofst + advance] == b' ' {
            ofst += advance + 1;
            while ofst < buffer.len() && buffer[ofst] != b'\n' {
                ofst += 1;
            }
            if ofst < buffer.len() {
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

        let hole_start = prev_blk_start.saturating_add(prev_blk_size);
        let hole_size = start_addr.saturating_sub(hole_start);
        let fits_upbound = hole_start
            .checked_add(length)
            .is_some_and(|end| end <= upbound);

        if hole_size >= length
            && hole_start >= lowbound
            && fits_upbound
            && hole_size < best_fit_size
        {
            best_fit_start = hole_start;
            best_fit_size = hole_size;

            if best_fit_size == length {
                break;
            }
        }

        if start_addr >= ADDR_1G && (!state.os_take_care_1g_2g || start_addr >= upbound) {
            break;
        }

        if end_addr >= upbound {
            break;
        }

        prev_blk_start = start_addr;
        prev_blk_size = end_addr.saturating_sub(start_addr);
    }

    (best_fit_size != usize::MAX).then_some(best_fit_start)
}

/// Initializes the ljmm system.
///
/// # Returns
/// An integer status code.
pub fn ljmm_init() -> i32 {
    let mut state = lock_state();
    *state = LjmmState::new();
    let _ = state.dummy_blk.len();

    // Mirror the C implementation closely enough to validate setup and buffer parsing.
    let _ = find_best_fit(&mut state, 1);

    i32::from(state.init_succ)
}
/// Instructs the OS to take care of the [1G..2G] space.
///
/// # Parameters
/// - `turn_on`: If non-zero, the OS should manage the space.
pub fn ljmm_let_os_take_care_1g_2g(turn_on: i32) {
    let mut state = lock_state();
    state.os_take_care_1g_2g = turn_on != 0;
}
/// Sets test parameters for the ljmm system.
///
/// # Parameters
/// - `map_file`: The mapping file name.
/// - `sbrk0`: An address representing the current program break (as a safe usize).
/// - `page_size`: The system's page size.
pub fn ljmm_test_set_test_param(map_file: &str, sbrk0: usize, page_size: i32) {
    let mut state = lock_state();
    state.map_file = map_file.to_string();
    state.addr_lowbound = sbrk0;

    if let Ok(page_size) = usize::try_from(page_size) {
        debug_assert!(page_size != 0 && page_size.is_power_of_two());
        if page_size != 0 && page_size.is_power_of_two() {
            state.page_size = page_size;
            state.page_mask = page_size - 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{find_best_fit, LjmmState};

    fn state_for(path: &str, lowbound: usize, os_take_care_1g_2g: bool) -> LjmmState {
        let mut state = LjmmState::new();
        state.map_file = path.to_string();
        state.addr_lowbound = lowbound;
        state.os_take_care_1g_2g = os_take_care_1g_2g;
        state.page_size = 4096;
        state.page_mask = 4095;
        state
    }

    #[test]
    fn best_fit_matches_fixture_001_case_1() {
        let mut state = state_for("src/bin/test_input/input_001_001.txt", 0, false);
        assert_eq!(find_best_fit(&mut state, 32 * 1024 - 1), Some(0x418000));
    }

    #[test]
    fn best_fit_matches_fixture_001_case_2() {
        let mut state = state_for("src/bin/test_input/input_001_001.txt", 0x619000, false);
        assert_eq!(find_best_fit(&mut state, 32 * 1024 - 100), Some(0x619000));
    }

    #[test]
    fn best_fit_matches_fixture_002_case_4() {
        let mut state = state_for("src/bin/test_input/input_001_002.txt", 0x619000, false);
        assert_eq!(find_best_fit(&mut state, 32 * 1024 - 10), Some(0x619000));
    }

    #[test]
    fn best_fit_handles_incomplete_last_range() {
        let mut state = state_for("src/bin/test_input/input_001_003.txt", 0x619000, false);
        assert_eq!(find_best_fit(&mut state, 32 * 1024 - 10), None);
    }

    #[test]
    fn best_fit_matches_fixture_004_case_6() {
        let mut state = state_for("src/bin/test_input/input_001_004.txt", 0x619000, false);
        assert_eq!(find_best_fit(&mut state, 32 * 1024), Some(0x3ffff000));
    }
}
