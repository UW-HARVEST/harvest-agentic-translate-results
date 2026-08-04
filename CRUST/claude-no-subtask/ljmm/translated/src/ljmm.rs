use std::sync::Mutex;

const BUFFER_SZ: usize = 8192;
const ADDR_2G: usize = 0x80000000;
const ADDR_1G: usize = 0x40000000;

struct Ljmm {
    page_size: usize,
    page_mask: usize,

    addr_upbound: usize,
    addr_lowbound: usize,

    map_file: Option<String>,

    /// it is up to OS to take care the 1G..2G space
    os_take_care_1g_2g: bool,
    init_succ: bool,
}

impl Ljmm {
    const fn new() -> Self {
        Ljmm {
            page_size: 4096,
            page_mask: 4095,
            addr_upbound: ADDR_2G,
            addr_lowbound: 0,
            map_file: None,
            os_take_care_1g_2g: true,
            init_succ: false,
        }
    }
}

fn global() -> &'static Mutex<Ljmm> {
    use std::sync::OnceLock;
    static INSTANCE: OnceLock<Mutex<Ljmm>> = OnceLock::new();
    INSTANCE.get_or_init(|| Mutex::new(Ljmm::new()))
}

/// Initializes the ljmm system.
///
/// # Returns
/// An integer status code.
pub fn ljmm_init() -> i32 {
    let mut g = global().lock().unwrap();
    g.os_take_care_1g_2g = true;
    g.addr_lowbound = 0;
    g.addr_upbound = ADDR_2G;
    // Use a default page size (4096) since we cannot call sysconf in pure Rust
    g.page_size = 4096;
    g.page_mask = g.page_size - 1;
    g.map_file = Some("/proc/self/maps".to_string());
    g.init_succ = true;
    0
}

/// Instructs the OS to take care of the [1G..2G] space.
///
/// # Parameters
/// - `turn_on`: If non-zero, the OS should manage the space.
pub fn ljmm_let_os_take_care_1g_2g(turn_on: i32) {
    let mut g = global().lock().unwrap();
    g.os_take_care_1g_2g = turn_on != 0;
}

/// Sets test parameters for the ljmm system.
///
/// # Parameters
/// - `map_file`: The mapping file name.
/// - `sbrk0`: An address representing the current program break (as a safe usize).
/// - `page_size`: The system's page size.
pub fn ljmm_test_set_test_param(map_file: &str, sbrk0: usize, page_size: i32) {
    let mut g = global().lock().unwrap();
    g.map_file = Some(map_file.to_string());
    g.addr_lowbound = sbrk0;
    // page-size must be a power-of-two
    debug_assert!(page_size > 0 && ((page_size - 1) & page_size) == 0);
    g.page_size = page_size as usize;
    g.page_mask = (page_size as usize).saturating_sub(1);
}

#[allow(dead_code)]
fn page_align_addr(addr: usize) -> usize {
    let g = global().lock().unwrap();
    (addr + g.page_size - 1) & !g.page_mask
}

/// Parse a hexadecimal address string. Stops at the first non-hex character.
/// Returns (number of chars consumed before the terminating char, address value).
#[allow(dead_code)]
fn parse_addr(addr_str: &[u8]) -> (usize, usize) {
    let mut addr: usize = 0;
    let mut i = 0;
    while i < addr_str.len() {
        let c = addr_str[i];
        if c.is_ascii_digit() {
            addr = addr.wrapping_mul(16).wrapping_add((c - b'0') as usize);
            i += 1;
            continue;
        }
        let lc = c | 0x20;
        if (b'a'..=b'f').contains(&lc) {
            addr = addr.wrapping_mul(16).wrapping_add(10 + (lc - b'a') as usize);
            i += 1;
            continue;
        }
        return (i, addr);
    }
    (i, addr)
}

#[allow(dead_code)]
fn read_maps_to_buffer() -> Option<Vec<u8>> {
    let path = {
        let g = global().lock().unwrap();
        g.map_file.clone()?
    };
    let data = std::fs::read(&path).ok()?;
    let take = data.len().min(BUFFER_SZ - 1);
    let mut buf = data[..take].to_vec();
    if buf.is_empty() || *buf.last().unwrap() != b'\n' {
        buf.push(b'\n');
    }
    Some(buf)
}

/// Going through /proc/$PID/maps in an attempt to find an unallocated hole
/// that tightly/best fits the allocation request.
#[allow(dead_code)]
fn find_best_fit(length: usize) -> usize {
    let buffer = match read_maps_to_buffer() {
        Some(b) => b,
        None => return 0,
    };

    let length = page_align_addr(length);

    let (lowbound, upbound, os_take_care, page_mask, page_size) = {
        let g = global().lock().unwrap();
        (
            g.addr_lowbound,
            g.addr_upbound,
            g.os_take_care_1g_2g,
            g.page_mask,
            g.page_size,
        )
    };

    let align = |addr: usize| -> usize { (addr + page_size - 1) & !page_mask };

    let mut best_fit_size: usize = usize::MAX;
    let mut best_fit_start: usize = upbound;
    let mut prev_start: usize = 0;
    let mut prev_size: usize = 0;
    let mut ofst: usize = 0;
    let buf_len = buffer.len();

    while ofst < buf_len {
        // Step 1: lower bound
        let (advance, start_addr) = parse_addr(&buffer[ofst..]);
        if advance > 0 && ofst + advance < buf_len && buffer[ofst + advance] == b'-' {
            ofst += advance + 1;
        } else {
            break;
        }

        // Step 2: upper bound
        let (advance2, mut end_addr) = parse_addr(&buffer[ofst..]);
        if advance2 > 0 && ofst + advance2 < buf_len && buffer[ofst + advance2] == b' ' {
            ofst += advance2 + 1;
            // Skip rest of line
            while ofst < buf_len && buffer[ofst] != b'\n' {
                ofst += 1;
            }
            if ofst < buf_len {
                ofst += 1;
            }
        } else {
            end_addr = if upbound >= start_addr { upbound } else { start_addr };
        }

        end_addr = align(end_addr);

        // Step 3: hole between previous and current
        let hole_start = prev_start + prev_size;
        let hole_size = start_addr.saturating_sub(hole_start);

        if hole_size >= length
            && hole_start >= lowbound
            && hole_start + length <= upbound
            && hole_size < best_fit_size
        {
            best_fit_start = hole_start;
            best_fit_size = hole_size;
            if best_fit_size == length {
                break;
            }
        }

        // Step 4: should we examine next line?
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
        best_fit_start
    } else {
        0
    }
}
