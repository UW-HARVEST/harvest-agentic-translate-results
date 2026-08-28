//! Phase D — call-structure / allocator-traffic parity for `matrixsum`.
//!
//! The C `matrixsum` reaches its helpers through the PLT
//! (`R_X86_64_JUMP_SLOT` for `init_array`, `add_element`, `process_flags`,
//! `calculate_matrix_checksum`, `free_array`), so it makes **no direct allocator
//! call at all** — the two `malloc`s, the `realloc` and the two `free`s happen
//! inside `init_array` / `add_element` / `free_array`.
//!
//! Before `#[inline(never)]` was added to the exported functions, LLVM inlined
//! that whole chain into the release `matrixsum` and then SROA'd the
//! `DynamicArray` away, so the release `.so` called `malloc(8)` / `realloc` /
//! `free` *directly* and never made the 24-byte struct allocation at all:
//!
//! | | C | Rust release (before the fix) |
//! |---|---|---|
//! | allocator calls | `malloc(24)`, `malloc(8)`, `realloc`, `free`, `free` | `malloc(8)`, `realloc`, `free` |
//!
//! That is a real behavioural difference: it changes the allocator traffic an
//! interposer sees, it removes one of the two allocation-failure points that can
//! make `matrixsum` return `-1` (`ERRORS.md` E1/E10), and it makes the helpers
//! non-interposable (the C's are, through the PLT).
//!
//! These tests are structural (`objdump`) because the difference is in which
//! calls are made, and `mallinfo2()` accounting cannot see it: the traffic is
//! balanced either way.

mod common;

use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

fn objdump(args: &[&str], so: &Path) -> String {
    let out = Command::new("objdump")
        .args(args)
        .arg(so)
        .output()
        .unwrap_or_else(|e| panic!("failed to run objdump on {}: {e}", so.display()));
    assert!(
        out.status.success(),
        "objdump failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).to_string()
}

fn strip_suffix(name: &str) -> String {
    name.split('@').next().unwrap_or(name).to_string()
}

/// address -> symbol name, from the dynamic relocation table (GOT/PLT slots).
fn reloc_map(so: &Path) -> BTreeMap<u64, String> {
    let text = objdump(&["-R"], so);
    let mut map = BTreeMap::new();
    for line in text.lines() {
        let f: Vec<&str> = line.split_whitespace().collect();
        if f.len() == 3 && f[1].starts_with("R_") {
            if let Ok(addr) = u64::from_str_radix(f[0], 16) {
                map.insert(addr, strip_suffix(f[2]));
            }
        }
    }
    map
}

/// The set of symbol names a dynamic relocation resolves to.
fn reloc_targets(so: &Path) -> Vec<String> {
    reloc_map(so).into_values().collect()
}

/// Every call target inside function `func`, resolved through the GOT/PLT where
/// possible. Register-indirect calls come back as `"<indirect>"`.
fn call_targets(so: &Path, func: &str) -> Vec<String> {
    let text = objdump(&["-d", "--no-show-raw-insn"], so);
    let got = reloc_map(so);

    let header = format!("<{func}>:");
    let mut inside = false;
    let mut out = Vec::new();
    for line in text.lines() {
        if line.contains(&header) {
            inside = true;
            continue;
        }
        if inside {
            if line.trim().is_empty() {
                break;
            }
            if !line.contains("call") {
                continue;
            }
            // (a) GOT-indirect: `call *0x3bb06(%rip)        # 4dc18 <...>`
            if line.contains("*0x") && line.contains("(%rip)") {
                let addr = line
                    .split('#')
                    .nth(1)
                    .and_then(|s| s.split_whitespace().next())
                    .and_then(|s| u64::from_str_radix(s.trim_start_matches("0x"), 16).ok());
                match addr.and_then(|a| got.get(&a)) {
                    Some(name) => out.push(name.clone()),
                    None => out.push("<indirect>".to_string()),
                }
            }
            // (b) register-indirect: `call *%rbp`
            else if line.contains("call   *%") || line.contains("call *%") {
                out.push("<indirect>".to_string());
            }
            // (c) direct: `call   1070 <init_array@plt>`
            else if let Some(rest) = line.split('<').nth(1) {
                let name = rest.split('>').next().unwrap_or("");
                let name = name.split('+').next().unwrap_or(name);
                out.push(strip_suffix(name));
            }
        }
    }
    assert!(
        !out.is_empty(),
        "found no call instructions inside `{func}` of {} — did the disassembly \
         format change?",
        so.display()
    );
    out
}

const ALLOCATORS: [&str; 3] = ["malloc", "realloc", "free"];
const HELPERS: [&str; 5] = [
    "init_array",
    "add_element",
    "process_flags",
    "calculate_matrix_checksum",
    "free_array",
];

#[test]
fn t1_matrixsum_makes_no_direct_allocator_calls_in_either() {
    let p = common::load();

    for (name, so) in [("C", &p.c.path), ("Rust", &p.rs.path)] {
        let calls = call_targets(so, "matrixsum");
        let direct: Vec<&String> = calls
            .iter()
            .filter(|c| ALLOCATORS.contains(&c.as_str()))
            .collect();
        assert!(
            direct.is_empty(),
            "{name} `matrixsum` calls the allocator directly ({direct:?}). The C \
             delegates ALL allocation to init_array/add_element/free_array, so a \
             direct malloc/realloc/free here means the helper chain was inlined and \
             the 24-byte DynamicArray allocation was optimised away.\n\
             all calls: {calls:?}"
        );
    }
}

#[test]
fn t2_matrixsum_reaches_its_helpers_through_real_calls() {
    let p = common::load();
    let c_calls = call_targets(&p.c.path, "matrixsum");
    let rs_calls = call_targets(&p.rs.path, "matrixsum");

    // The C makes 8 internal calls: init_array, add_element x4, process_flags,
    // calculate_matrix_checksum, free_array.
    let c_internal = c_calls.iter().filter(|c| HELPERS.contains(&c.as_str())).count();
    assert_eq!(
        c_internal, 8,
        "expected 8 helper calls in the C `matrixsum`, found {c_internal}: {c_calls:?}"
    );

    // The Rust must make at least as many calls (some resolve as <indirect>
    // because the callee address is hoisted into a register first).
    assert!(
        rs_calls.len() >= c_internal,
        "Rust `matrixsum` makes only {} call(s) but the C makes {c_internal} helper \
         calls — the helper chain was inlined away.\nRust calls: {rs_calls:?}",
        rs_calls.len()
    );
}

#[test]
fn t3_helpers_are_interposable_in_both() {
    let p = common::load();
    // The C's helpers all have PLT slots (R_X86_64_JUMP_SLOT), so an LD_PRELOAD
    // or RTLD_GLOBAL definition can replace them even for the library's own
    // internal calls. The Rust must expose the same interposition surface
    // (R_X86_64_GLOB_DAT GOT slots).
    let c_targets = reloc_targets(&p.c.path);
    let rs_targets = reloc_targets(&p.rs.path);
    for helper in HELPERS {
        assert!(
            c_targets.iter().any(|t| t == helper),
            "the C .so has no dynamic relocation for `{helper}` — update this test"
        );
        assert!(
            rs_targets.iter().any(|t| t == helper),
            "the Rust .so resolves `{helper}` statically: its internal calls are NOT \
             interposable, unlike the C's PLT calls. Add `#[inline(never)]`."
        );
    }
    // `matrix` is referenced through the GOT in both.
    assert!(c_targets.iter().any(|t| t == "matrix"));
    assert!(rs_targets.iter().any(|t| t == "matrix"));
}

#[test]
fn t4_add_element_calls_expand_array_rather_than_inlining_realloc() {
    let p = common::load();
    for (name, so) in [("C", &p.c.path), ("Rust", &p.rs.path)] {
        let calls = call_targets(so, "add_element");
        assert!(
            calls.iter().any(|c| c == "expand_array") || calls.iter().any(|c| c == "<indirect>"),
            "{name} `add_element` does not call `expand_array`: {calls:?}"
        );
        let direct: Vec<&String> = calls
            .iter()
            .filter(|c| ALLOCATORS.contains(&c.as_str()))
            .collect();
        assert!(
            direct.is_empty(),
            "{name} `add_element` calls the allocator directly ({direct:?}); the C \
             goes through `expand_array`"
        );
    }
}
