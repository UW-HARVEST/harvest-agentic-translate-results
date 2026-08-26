//! A C caller may legally (on x86-64) hand `parse_number` MISALIGNED
//! `cJSON *` / `parse_buffer *` pointers — nothing in `lib.h` or `lib.c`
//! requires natural alignment, and the C compiler just emits ordinary loads and
//! stores. Rust, in contrast, treats a misaligned raw-pointer access as UB and
//! `-C debug-assertions` builds abort on it.
//!
//! Each case runs in a CHILD process so an abort in one implementation does not
//! take the whole test run down; the child's status is then compared.

mod common;

use common::*;
use std::os::unix::process::ExitStatusExt;
use std::process::Command;

const IMPL_ENV: &str = "HARVEST_MISALIGN_IMPL";
const SKEW_ENV: &str = "HARVEST_MISALIGN_SKEW";
const WHAT_ENV: &str = "HARVEST_MISALIGN_WHAT";

/// Child helper: builds `parse_buffer` / `cJSON` at a deliberately skewed
/// address and calls the chosen implementation.
#[test]
#[ignore = "helper: executed in a child process"]
fn misaligned_child() {
    let which = std::env::var(IMPL_ENV).expect("impl");
    let skew: usize = std::env::var(SKEW_ENV).unwrap().parse().unwrap();
    let what = std::env::var(WHAT_ENV).unwrap();
    let f = match which.as_str() {
        "c" => c_parse_number(),
        "rust" => rust_parse_number(),
        o => panic!("bad impl {o}"),
    };

    let mut content = b"-12.5e2".to_vec();
    let clen = content.len();

    // Over-aligned backing storage so we can skew by an arbitrary byte count.
    let mut item_store = vec![0u8; 64];
    let mut buf_store = vec![0u8; 64];
    let base = |v: &mut Vec<u8>| -> usize {
        let p = v.as_mut_ptr() as usize;
        // round up to 16, then add the skew
        (p + 15) & !15usize
    };
    let item_addr = base(&mut item_store) + if what.contains('i') { skew } else { 0 };
    let buf_addr = base(&mut buf_store) + if what.contains('b') { skew } else { 0 };

    let item_ptr = item_addr as *mut CJson;
    let buf_ptr = buf_addr as *mut ParseBuffer;

    unsafe {
        std::ptr::write_unaligned(
            item_ptr,
            CJson {
                type_: POISON_TYPE,
                valueint: POISON_VALUEINT,
                valuedouble: f64::from_bits(POISON_DOUBLE_BITS),
            },
        );
        std::ptr::write_unaligned(
            buf_ptr,
            ParseBuffer {
                content: content.as_mut_ptr(),
                length: clen,
                offset: 0,
                depth: POISON_DEPTH,
            },
        );
        let ret = f(item_ptr, buf_ptr);
        let item = std::ptr::read_unaligned(item_ptr);
        let buf = std::ptr::read_unaligned(buf_ptr);
        println!(
            "ret={ret} type={} valueint={} double_bits={:#x} offset={} depth={:#x}",
            item.type_,
            item.valueint,
            item.valuedouble.to_bits(),
            buf.offset,
            buf.depth
        );
    }
    std::hint::black_box(&content);
}

fn child(which: &str, skew: usize, what: &str) -> (Option<i32>, Option<i32>, String) {
    let exe = std::env::current_exe().unwrap();
    let out = Command::new(exe)
        .args([
            "--exact",
            "misaligned_child",
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(IMPL_ENV, which)
        .env(SKEW_ENV, skew.to_string())
        .env(WHAT_ENV, what)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    // libtest prints "test misaligned_child ... " without a trailing newline,
    // so the child's own output lands on the same line — match anywhere.
    let line = stdout
        .lines()
        .find_map(|l| l.find("ret=").map(|i| l[i..].to_string()))
        .unwrap_or_else(|| "<no result line>".to_string());
    (out.status.code(), out.status.signal(), line)
}

#[test]
fn misaligned_pointers_behave_identically() {
    for what in ["i", "b", "ib"] {
        for skew in [1usize, 2, 3, 4, 5, 6, 7] {
            let c = child("c", skew, what);
            let r = child("rust", skew, what);
            assert_eq!(
                c, r,
                "misaligned {what} by {skew}:\n  C    = {c:?}\n  Rust = {r:?}"
            );
            assert!(
                c.2.starts_with("ret=1"),
                "the C is expected to succeed on misaligned pointers: {c:?}"
            );
        }
    }
}
