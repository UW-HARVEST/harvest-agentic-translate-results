//! Differential test driver (custom harness — see `tests/support/mod.rs`).
//!
//! Loads BOTH the reference C `.so` and the Rust `cdylib` with `libloading` and
//! drives them only through their exported symbols, comparing return values,
//! `cp_error_reason`, the full output allocation (padding included, so
//! out-of-range writes are caught), the exported tables, and — for the inputs
//! that make the library `assert()` or fault — the terminating signal together
//! with the byte-exact stderr text.
//!
//! Usage:
//!   cargo test --test differential            # all tests
//!   cargo test --test differential -- b_dyn   # only ids containing "b_dyn"

mod support;

fn main() {
    // worker mode: run the cases of one test against one library
    if let Ok(which) = std::env::var("PINFLATE_WORKER") {
        support::worker_main(which);
        return;
    }

    let c = support::c_so_path();
    let r = support::rust_so_path();
    println!("C    library : {}", c.display());
    println!("Rust library : {}", r.display());
    println!();

    let mut h = support::Harness::new();

    // Phase D (a): symbol parity, checked before anything else.
    h.check("d_symbol_parity", symbol_parity(&c, &r));

    // Phases B and C.
    for id in support::cases::all_ids() {
        h.run_test(id);
    }

    h.finish();
}

/// `nm -D` on both libraries: every symbol the C `.so` exports must be exported
/// by the Rust `.so` under the exact same name, and the data objects must have
/// the same size.
fn symbol_parity(c: &std::path::Path, r: &std::path::Path) -> Result<(), String> {
    let cs = nm(c)?;
    let rs = nm(r)?;
    let mut missing: Vec<String> = Vec::new();
    let mut wrong: Vec<String> = Vec::new();
    for (name, (kind, size)) in &cs {
        match rs.get(name) {
            None => missing.push(name.clone()),
            Some((rkind, rsize)) => {
                if kind != rkind {
                    wrong.push(format!("{name}: C kind {kind} vs Rust {rkind}"));
                }
                if kind != "T" && size != rsize {
                    wrong.push(format!("{name}: C size {size:#x} vs Rust {rsize:#x}"));
                }
            }
        }
    }
    if missing.is_empty() && wrong.is_empty() {
        println!("     {} exported symbols, all present in the Rust .so", cs.len());
        Ok(())
    } else {
        Err(format!("missing from the Rust .so: {missing:?}; mismatched: {wrong:?}"))
    }
}

fn nm(p: &std::path::Path) -> Result<std::collections::BTreeMap<String, (String, u64)>, String> {
    let out = std::process::Command::new("nm")
        .arg("-D")
        .arg("-S")
        .arg("--defined-only")
        .arg(p)
        .output()
        .map_err(|e| format!("nm: {e}"))?;
    if !out.status.success() {
        return Err(format!("nm failed on {}", p.display()));
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let mut map = std::collections::BTreeMap::new();
    for line in text.lines() {
        let f: Vec<&str> = line.split_whitespace().collect();
        // "addr size kind name" or "addr kind name"
        let (kind, name, size) = match f.len() {
            4 => (f[2], f[3], u64::from_str_radix(f[1], 16).unwrap_or(0)),
            3 => (f[1], f[2], 0u64),
            _ => continue,
        };
        map.insert(name.to_string(), (kind.to_string(), size));
    }
    Ok(map)
}
