// Structural differential checks — cover for the Phase B/C blind spot.
//
// WHY THIS FILE EXISTS
//
// C's `bad()` and `good()` print exactly the same bytes ("0\n"): both copy
// `source[10] = {0}` into their `alloca` region and print `data[0]`. So NO
// stdout-differential test can observe which of the two `driver` selected, and
// no differential test can observe whether `bad` really is a distinct
// implementation rather than a forwarder to `good`. Mutation testing confirms
// this directly (scripts/mutation_check.sh): `if (useGood == 0)` (inverted) and
// `fn bad() { good() }` both leave the Phase B/C suite green.
//
// Interposing `bad`/`good` at run time to observe the branch is NOT an option:
// the C `.so` calls them through the PLT (`objdump -R` shows JUMP_SLOT relocs)
// while the Rust `.so` reaches them through its own GOT, so interposition would
// manufacture a difference that is not real.
//
// The branch is therefore pinned STRUCTURALLY: the call graph and the branch
// direction of `driver` are extracted from both `.so`s with objdump and
// required to agree with the C, which compiles to
//
//     driver:  cmpl $0x0,-0x4(%rbp)   # truthiness, NOT a compare against 1
//              je   <bad@plt>         # zero      -> bad
//              call <good@plt>        # non-zero  -> good
//
// Callee resolution handles both direct calls (`call 1030 <good@plt>`) and
// GOT-indirect calls/tail-calls (`call *0x..(%rip) # 4ced8` -> resolved through
// the `R_X86_64_GLOB_DAT` relocation table), which is what makes these checks
// hold for the dev profile AND the release profile.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so() -> PathBuf {
    std::env::var("DRIVER_C_SO")
        .map(PathBuf::from)
        .unwrap_or_else(|_| manifest_dir().join("../c_src/build/libdriver.so"))
}

fn rust_so() -> PathBuf {
    // Deterministic: `cargo test` builds the dev-profile cdylib, so that is the
    // default. Set DRIVER_RUST_SO to test the release cdylib (see
    // scripts/verify_all.sh, which runs the suite against BOTH profiles).
    if let Ok(p) = std::env::var("DRIVER_RUST_SO") {
        return PathBuf::from(p);
    }
    manifest_dir().join("target/debug/libdriver.so")
}

fn objdump(args: &[&str]) -> String {
    let out = Command::new("objdump")
        .args(args)
        .output()
        .expect("objdump (binutils) is required for the structural tests");
    assert!(out.status.success(), "objdump {args:?} failed");
    String::from_utf8_lossy(&out.stdout).to_string()
}

/// address (as written by objdump, no leading zeros) -> symbol name, built from
/// the dynamic relocation table. Covers GOT slots (`GLOB_DAT`) and PLT slots
/// (`JUMP_SLOT`).
fn reloc_map(so: &PathBuf) -> HashMap<String, String> {
    let text = objdump(&["-R", so.to_str().unwrap()]);
    let mut m = HashMap::new();
    for l in text.lines() {
        let f: Vec<&str> = l.split_whitespace().collect();
        if f.len() >= 3 && f[1].starts_with("R_") {
            if let Ok(a) = u64::from_str_radix(f[0], 16) {
                let name = f[2].split('@').next().unwrap_or(f[2]).to_string();
                m.insert(format!("{a:x}"), name);
            }
        }
    }
    m
}

#[derive(Debug, Clone)]
struct Insn {
    addr: u64,
    text: String,
}

/// Disassemble one function and return its instruction stream.
fn body(so: &PathBuf, sym: &str) -> Vec<Insn> {
    let text = objdump(&["-d", &format!("--disassemble={sym}"), so.to_str().unwrap()]);
    assert!(
        text.contains(&format!("<{sym}>:")),
        "objdump produced no body for `{sym}` in {}",
        so.display()
    );
    let mut out = Vec::new();
    let mut inside = false;
    for l in text.lines() {
        if l.contains(&format!("<{sym}>:")) {
            inside = true;
            continue;
        }
        if !inside {
            continue;
        }
        if l.trim().is_empty() {
            break;
        }
        let mut parts = l.split('\t');
        let addr_field = parts.next().unwrap_or("").trim().trim_end_matches(':');
        let Ok(addr) = u64::from_str_radix(addr_field, 16) else { continue };
        let _bytes = parts.next();
        // Continuation lines (long instruction encodings) have no third field.
        if let Some(insn) = parts.next() {
            out.push(Insn { addr, text: insn.trim().to_string() });
        }
    }
    assert!(!out.is_empty(), "no instructions extracted for `{sym}`");
    out
}

/// If this instruction transfers control to a *named* function, return the name.
/// Handles direct `call ADDR <name>` / `<name@plt>` and GOT-indirect
/// `call *0x..(%rip) # ADDR <...>` (resolved through the relocation table).
fn callee(i: &Insn, relocs: &HashMap<String, String>) -> Option<String> {
    let t = &i.text;
    if !(t.starts_with("call") || t.starts_with("jmp")) {
        return None;
    }
    // GOT-indirect: trust the `# <hex>` target and the relocation table.
    if let Some(hash) = t.split('#').nth(1) {
        if let Some(tok) = hash.split_whitespace().next() {
            if let Some(name) = relocs.get(tok.trim_start_matches("0x")) {
                return Some(name.clone());
            }
        }
    }
    // Direct: `call 1030 <good@plt>` — a plain symbol name, no `+offset`.
    if let Some(open) = t.find('<') {
        if let Some(close) = t[open..].find('>') {
            let inner = &t[open + 1..open + close];
            if !inner.contains('+') {
                let name = inner.split('@').next().unwrap_or(inner);
                if !name.is_empty() {
                    return Some(name.to_string());
                }
            }
        }
    }
    None
}

/// Names of the interesting library functions this function transfers to.
fn calls_to_library_fns(so: &PathBuf, sym: &str) -> Vec<(u64, String)> {
    let relocs = reloc_map(so);
    const INTERESTING: [&str; 5] = ["bad", "good", "driver", "printIntLine", "printLine"];
    body(so, sym)
        .iter()
        .filter_map(|i| callee(i, &relocs).map(|c| (i.addr, c)))
        .filter(|(_, c)| INTERESTING.contains(&c.as_str()))
        .collect()
}

fn tests_against_zero(insns: &[Insn]) -> bool {
    insns.iter().any(|i| {
        (i.text.starts_with("cmp") && i.text.contains("$0x0,"))
            || (i.text.starts_with("test") && {
                let ops: Vec<&str> = i.text.split_whitespace().nth(1).unwrap_or("").split(',').collect();
                ops.len() == 2 && ops[0] == ops[1]
            })
    })
}

fn nonzero_immediate_compares(insns: &[Insn]) -> Vec<String> {
    insns
        .iter()
        .filter(|i| i.text.starts_with("cmp") && i.text.contains("$0x") && !i.text.contains("$0x0,"))
        .map(|i| i.text.clone())
        .collect()
}

/// `driver` must branch on C truthiness: a test against ZERO, never a compare
/// against a specific non-zero value.
#[test]
fn struct_01_driver_branches_on_truthiness() {
    for (which, so) in [("C", c_so()), ("Rust", rust_so())] {
        let insns = body(&so, "driver");
        assert!(
            tests_against_zero(&insns),
            "{which} `driver` never tests its argument against zero; body: {:?}",
            insns.iter().map(|i| &i.text).collect::<Vec<_>>()
        );
        let bad = nonzero_immediate_compares(&insns);
        assert!(
            bad.is_empty(),
            "{which} `driver` compares the flag against a non-zero immediate {bad:?} \
             — the C is `if (useGood)`, so EVERY non-zero value must select good()"
        );
    }
}

/// `bad` and `good` must be distinct addresses in both `.so`s.
#[test]
fn struct_02_bad_and_good_are_distinct_symbols() {
    for (which, so) in [("C", c_so()), ("Rust", rust_so())] {
        let text = objdump(&["-t", so.to_str().unwrap()]);
        let addr = |name: &str| -> String {
            text.lines()
                .find(|l| l.split_whitespace().last() == Some(name))
                .and_then(|l| l.split_whitespace().next())
                .unwrap_or_else(|| panic!("{which}: symbol `{name}` not in symbol table"))
                .to_string()
        };
        let (b, g) = (addr("bad"), addr("good"));
        assert_ne!(b, g, "{which}: `bad` and `good` were folded onto one address ({b})");
    }
}

/// BRANCH DIRECTION — the check that makes the inverted-`driver` mutant fail.
///
/// Zero must reach `bad`, non-zero must reach `good`, in BOTH implementations.
/// The conditional jump's sense (`je` = taken when zero, `jne` = taken when
/// non-zero) decides which side of the branch is the zero path.
#[test]
fn struct_03_driver_maps_zero_to_bad_and_nonzero_to_good() {
    for (which, so) in [("C", c_so()), ("Rust", rust_so())] {
        let insns = body(&so, "driver");
        let relocs = reloc_map(&so);

        // The single conditional jump on the zero test.
        let (idx, cond) = insns
            .iter()
            .enumerate()
            .find_map(|(n, i)| {
                let m = i.text.split_whitespace().next().unwrap_or("");
                (m == "je" || m == "jne" || m == "jz" || m == "jnz").then(|| (n, i.clone()))
            })
            .unwrap_or_else(|| {
                panic!(
                    "{which} `driver`: no je/jne found; body: {:?}",
                    insns.iter().map(|i| &i.text).collect::<Vec<_>>()
                )
            });
        let mnemonic = cond.text.split_whitespace().next().unwrap().to_string();
        let target = u64::from_str_radix(
            cond.text
                .split_whitespace()
                .nth(1)
                .unwrap_or("")
                .trim_start_matches("0x"),
            16,
        )
        .unwrap_or_else(|_| panic!("{which}: cannot parse jump target from `{}`", cond.text));

        // First bad/good reached on the fall-through path (stop at the
        // unconditional jmp that ends the block).
        let mut fallthrough: Option<String> = None;
        for i in &insns[idx + 1..] {
            if let Some(c) = callee(i, &relocs) {
                if c == "bad" || c == "good" {
                    fallthrough = Some(c);
                    break;
                }
            }
            if i.text.starts_with("jmp ") && !i.text.contains('*') {
                break; // end of the fall-through block
            }
        }

        // First bad/good reached at/after the branch target.
        let mut taken: Option<String> = None;
        for i in insns.iter().filter(|i| i.addr >= target) {
            if let Some(c) = callee(i, &relocs) {
                if c == "bad" || c == "good" {
                    taken = Some(c);
                    break;
                }
            }
        }

        let dump = || {
            insns
                .iter()
                .map(|i| format!("{:x}: {}", i.addr, i.text))
                .collect::<Vec<_>>()
        };
        let ft = fallthrough
            .unwrap_or_else(|| panic!("{which}: no bad/good on fall-through path; {:?}", dump()));
        let tk = taken
            .unwrap_or_else(|| panic!("{which}: no bad/good on taken path; {:?}", dump()));

        // je/jz taken => argument was zero. jne/jnz taken => non-zero.
        let (zero_path, nonzero_path) = if mnemonic == "je" || mnemonic == "jz" {
            (tk.as_str(), ft.as_str())
        } else {
            (ft.as_str(), tk.as_str())
        };

        assert_eq!(
            zero_path, "bad",
            "{which} `driver`: useGood == 0 must reach bad(), reaches {zero_path}(); {:?}",
            dump()
        );
        assert_eq!(
            nonzero_path, "good",
            "{which} `driver`: useGood != 0 must reach good(), reaches {nonzero_path}(); {:?}",
            dump()
        );
    }
}

/// CALL GRAPH — the check that makes the "bad forwards to good" mutant fail.
///
/// In the C, `bad` and `good` are independent leaf functions: neither calls the
/// other, and neither calls `driver`. A Rust `bad` that delegated to `good`
/// would be output-identical yet structurally wrong.
#[test]
fn struct_04_bad_and_good_do_not_call_each_other() {
    for (which, so) in [("C", c_so()), ("Rust", rust_so())] {
        for (owner, forbidden) in [("bad", "good"), ("good", "bad")] {
            let calls = calls_to_library_fns(&so, owner);
            let names: Vec<&str> = calls.iter().map(|(_, c)| c.as_str()).collect();
            assert!(
                !names.contains(&forbidden),
                "{which}: `{owner}` transfers control to `{forbidden}` (calls: {names:?}); \
                 in the C they are independent implementations"
            );
            assert!(
                !names.contains(&"driver"),
                "{which}: `{owner}` calls `driver` (calls: {names:?})"
            );
        }
    }
}

// NOT structurally checked, deliberately:
//
//   * `printLine`'s NULL guard — it is fully OBSERVABLE (err_01 kills the
//     "drop the guard" mutant), so a structural check would only add
//     brittleness. In a dev-profile build Rust lowers the check to an
//     out-of-line `core::ptr::const_ptr::is_null` call, so the guard is not
//     visible as a `cmp $0x0` inside `printLine`'s own body at all.
//   * the loop bound `10` and the printed index `data[0]` inside `bad`/`good`
//     — `source[10] = {0}` is all zeros and only `data[0]` is printed, so every
//     index and every loop bound >= 1 yields the same "0". These are
//     unobservable through the C ABI *by construction* (no consumer of the
//     original C library can distinguish them either), and the instruction
//     shapes of a C `alloca` vs. a Rust stack array differ legitimately, so
//     pinning them would produce false failures. Recorded as EXPECTED
//     SURVIVORS in scripts/mutation_check.sh.
