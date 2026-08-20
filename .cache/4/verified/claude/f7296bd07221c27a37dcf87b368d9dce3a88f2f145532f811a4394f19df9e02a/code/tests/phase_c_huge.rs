//! Phase C — `ERRORS.md` rows 23 and 25: the `INT_MAX` guards inside
//! `ensure()`, which are only reachable with a ≥ 2 GiB value, plus the
//! `newsize = INT_MAX` growth branch that only a ≥ 1 GiB value reaches.
//!
//! The 2 GiB payload is allocated **once by the test** and handed to both
//! libraries as a `cJSON_Raw` item whose `valuestring` is borrowed
//! (`cJSON_IsReference` keeps `cJSON_Delete` from freeing it), so peak memory
//! stays around 4 GiB instead of 8.
#![allow(non_snake_case)]

mod common;
use common::*;

use std::ffi::{c_char, c_void};
use std::fmt::Write as _;

const INT_MAX: usize = 2147483647;

fn mem_available_kb() -> u64 {
    std::fs::read_to_string("/proc/meminfo")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("MemAvailable:"))
                .and_then(|l| l.split_whitespace().nth(1).map(|v| v.to_string()))
        })
        .and_then(|v| v.parse().ok())
        .unwrap_or(0)
}

/// Build a `cJSON_Raw` item that borrows `buf` (no copy, no free).
unsafe fn borrowed_raw(api: &Api, buf: *mut u8) -> *mut CJson {
    let it = (api.cJSON_CreateNull)();
    assert!(!it.is_null());
    (*it).type_ = CJSON_RAW | CJSON_IS_REFERENCE;
    (*it).valuestring = buf as *mut c_char;
    it
}

#[test]
fn rows_23_25_ensure_int_max_guards() {
    // 6 GiB head room: the 2 GiB payload plus a 2 GiB print buffer, twice over.
    if mem_available_kb() < 6 * 1024 * 1024 {
        eprintln!(
            "SKIPPED rows 23/25: needs ~6 GiB of free memory, MemAvailable = {} kB",
            mem_available_kb()
        );
        return;
    }

    // 2 GiB + 8 of 'x'
    let mut payload: Vec<u8> = vec![b'x'; INT_MAX + 1 + 8];

    let scenarios: [(&str, usize); 3] = [
        // row 25: raw_length == INT_MAX  =>  needed == INT_MAX + 1 after
        //         `needed += offset + 1`, so the `needed <= INT_MAX` retry fails
        ("row25 strlen=INT_MAX-1", INT_MAX - 1),
        // row 23: raw_length == INT_MAX + 1  =>  the `needed > INT_MAX` guard
        ("row23 strlen=INT_MAX", INT_MAX),
        // valid path: needed > INT_MAX/2 but <= INT_MAX  =>  newsize = INT_MAX
        ("valid newsize=INT_MAX", 1 << 30),
    ];

    for (label, strlen) in scenarios {
        // shape the payload so that strlen(payload) == strlen
        payload[..strlen].fill(b'x');
        payload[strlen] = 0;
        let ptr = payload.as_mut_ptr();

        let (c, r) = libs();
        let mut logs: Vec<String> = Vec::new();
        for api in [c, r] {
            let mut log = String::new();
            unsafe {
                let it = borrowed_raw(api, ptr);
                let printed = (api.cJSON_Print)(it);
                let _ = writeln!(log, "{label}: Print null={}", printed.is_null());
                if !printed.is_null() {
                    let n = libc_strlen(printed);
                    let _ = writeln!(
                        log,
                        "  len={n} first={} last={}",
                        *printed as u8,
                        *printed.add(n - 1) as u8
                    );
                    (api.cJSON_free)(printed as *mut c_void);
                }
                let unformatted = (api.cJSON_PrintUnformatted)(it);
                let _ = writeln!(log, "  PrintUnformatted null={}", unformatted.is_null());
                if !unformatted.is_null() {
                    (api.cJSON_free)(unformatted as *mut c_void);
                }
                // the noalloc sink must refuse it as well
                let mut small = [0u8; 16];
                let _ = writeln!(
                    log,
                    "  PrintPreallocated(16) rc={}",
                    (api.cJSON_PrintPreallocated)(it, small.as_mut_ptr() as *mut c_char, 16, 1)
                );
                let buffered = (api.cJSON_PrintBuffered)(it, 64, 0);
                let _ = writeln!(log, "  PrintBuffered null={}", buffered.is_null());
                if !buffered.is_null() {
                    (api.cJSON_free)(buffered as *mut c_void);
                }
                (*it).type_ = CJSON_NULL;
                (*it).valuestring = std::ptr::null_mut();
                (api.cJSON_Delete)(it);
            }
            logs.push(log);
        }
        assert_eq!(logs[0], logs[1], "{label}: C and Rust differ");
        // sanity: the two guard rows must really refuse to print
        if label.starts_with("row") {
            assert!(
                logs[0].contains("Print null=true"),
                "{label} should have failed: {}",
                logs[0]
            );
        } else {
            assert!(
                logs[0].contains("Print null=false"),
                "{label} should have succeeded: {}",
                logs[0]
            );
        }
    }
}

unsafe fn libc_strlen(p: *const c_char) -> usize {
    extern "C" {
        fn strlen(s: *const c_char) -> usize;
    }
    strlen(p)
}
