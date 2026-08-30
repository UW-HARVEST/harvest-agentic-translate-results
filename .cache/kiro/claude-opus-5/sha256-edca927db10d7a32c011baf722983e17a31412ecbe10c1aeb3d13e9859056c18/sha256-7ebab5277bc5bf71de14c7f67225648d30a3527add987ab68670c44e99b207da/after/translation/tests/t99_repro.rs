//! Manual reproducer, driven by environment variables:
//!
//! ```text
//! PINFLATE_HEX=<hex bytes> PINFLATE_OFFSET=0 PINFLATE_OUT=64 \
//!   cargo test --test t99_repro -- --ignored --nocapture
//! ```
//!
//! Runs the case against both shared objects with the child's stderr left
//! attached, so the exact assertion that fires is visible.

mod harness;

use harness::{run_raw, Impl};

fn hex(s: &str) -> Vec<u8> {
    let s: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    (0..s.len() / 2)
        .map(|i| u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).unwrap())
        .collect()
}

#[test]
#[ignore = "manual reproducer; needs PINFLATE_HEX"]
fn repro() {
    std::env::set_var("PINFLATE_TEST_VERBOSE", "1");
    let input = hex(&std::env::var("PINFLATE_HEX").expect("PINFLATE_HEX"));
    let offset: usize = std::env::var("PINFLATE_OFFSET")
        .unwrap_or_else(|_| "0".into())
        .parse()
        .unwrap();
    let out_bytes: usize = std::env::var("PINFLATE_OUT")
        .unwrap_or_else(|_| "64".into())
        .parse()
        .unwrap();
    let in_bytes: i32 = std::env::var("PINFLATE_IN")
        .map(|v| v.parse().unwrap())
        .unwrap_or(input.len() as i32);

    for (path, label) in [
        (harness::c_so_path(), "C"),
        (harness::rust_so_path(), "Rust"),
    ] {
        let imp = Impl::load(&path, label);
        eprintln!("--- {label} ---");
        let o = run_raw(&imp, &input, offset, out_bytes, in_bytes);
        eprintln!("{label}: {}", o.summary());
    }
}
