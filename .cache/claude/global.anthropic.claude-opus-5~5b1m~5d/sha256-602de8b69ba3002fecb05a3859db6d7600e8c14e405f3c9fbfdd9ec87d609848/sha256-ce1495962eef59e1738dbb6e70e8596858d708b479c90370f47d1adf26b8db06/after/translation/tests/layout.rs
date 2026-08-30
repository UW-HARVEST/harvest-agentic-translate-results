// CONFIGS.md row C24 — ABI/layout parity for `struct ListNode`.
//
// `struct ListNode` is header-only, so it emits no dynamic symbol, but its layout
// is still part of the surface the FFI boundary depends on. This compiles a
// throwaway C probe against the REAL c_src header and compares its
// sizeof/alignof/offsetof against Rust's.

mod common;

use std::path::PathBuf;
use std::process::Command;

use common::ListNode;

#[test]
fn c24_listnode_layout_matches_c() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let include_dir = repo_root.join("c_src/include");

    let tmp = std::env::temp_dir().join(format!("listnode_probe_{}", std::process::id()));
    std::fs::create_dir_all(&tmp).expect("create probe dir");
    let src = tmp.join("probe.c");
    let bin = tmp.join("probe");

    std::fs::write(
        &src,
        r#"#include <stddef.h>
#include <stdio.h>
#include "simplestruct.h"
int main(void) {
    printf("%zu %zu %zu %zu\n",
           sizeof(struct ListNode),
           _Alignof(struct ListNode),
           offsetof(struct ListNode, value),
           offsetof(struct ListNode, next));
    return 0;
}
"#,
    )
    .expect("write probe.c");

    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".to_string());
    let status = Command::new(&cc)
        .arg("-std=c11")
        .arg("-I")
        .arg(&include_dir)
        .arg("-o")
        .arg(&bin)
        .arg(&src)
        .status();

    let status = match status {
        Ok(s) => s,
        Err(e) => {
            eprintln!("skipping C24: cannot run {cc}: {e}");
            return;
        }
    };
    assert!(status.success(), "C24: probe failed to compile");

    let out = Command::new(&bin).output().expect("run probe");
    assert!(out.status.success(), "C24: probe failed to run");
    let text = String::from_utf8(out.stdout).expect("probe output is utf8");
    let nums: Vec<usize> = text
        .split_whitespace()
        .map(|t| t.parse().expect("probe printed a number"))
        .collect();
    assert_eq!(nums.len(), 4, "C24: unexpected probe output {text:?}");
    let (c_size, c_align, c_off_value, c_off_next) = (nums[0], nums[1], nums[2], nums[3]);

    assert_eq!(
        c_size,
        std::mem::size_of::<ListNode>(),
        "C24: sizeof(struct ListNode) mismatch"
    );
    assert_eq!(
        c_align,
        std::mem::align_of::<ListNode>(),
        "C24: alignof(struct ListNode) mismatch"
    );
    assert_eq!(
        c_off_value,
        std::mem::offset_of!(ListNode, value),
        "C24: offsetof(value) mismatch"
    );
    assert_eq!(
        c_off_next,
        std::mem::offset_of!(ListNode, next),
        "C24: offsetof(next) mismatch"
    );

    let _ = std::fs::remove_dir_all(&tmp);
}

/// Phase D — symbol parity asserted from inside the test suite, so a regression
/// in the exported surface fails `cargo test` rather than only a manual `nm`.
#[test]
fn symbol_parity_c_so_vs_rust_so() {
    let c_so = common::c_so_path();
    let rust_so = common::rust_so_path();
    assert!(c_so.exists(), "build the C .so first: {}", c_so.display());
    assert!(
        rust_so.exists(),
        "build the Rust .so first: {}",
        rust_so.display()
    );

    let defined = |p: &PathBuf| -> Vec<String> {
        let out = Command::new("nm")
            .args(["-D", "--defined-only"])
            .arg(p)
            .output()
            .expect("run nm");
        assert!(out.status.success(), "nm failed on {}", p.display());
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|l| l.split_whitespace().nth(2).map(str::to_string))
            .collect()
    };

    let c_syms = defined(&c_so);
    let rust_syms = defined(&rust_so);

    assert!(
        c_syms.contains(&"smallestValue".to_string()),
        "C .so must export smallestValue, got {c_syms:?}"
    );

    let missing: Vec<&String> = c_syms.iter().filter(|s| !rust_syms.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing symbols exported by the C .so: {missing:?}\n\
         C: {c_syms:?}\nRust: {rust_syms:?}"
    );
}
