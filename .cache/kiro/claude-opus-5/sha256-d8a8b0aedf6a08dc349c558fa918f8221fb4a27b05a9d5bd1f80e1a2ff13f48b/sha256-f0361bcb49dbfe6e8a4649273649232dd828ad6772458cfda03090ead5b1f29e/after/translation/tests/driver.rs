//! Phase B rows 34-36 + Phase C rows 12-14/16 -- separate-process differential
//! driver (`harness = false`, so this file has its own `main`).
//!
//! ## Why a subprocess is required here
//!
//! `compute_hash` compares raw heap addresses, and `betagamma` folds its result
//! into the return value. So `betagamma(x)` is not a pure function of `x`: it is
//! a function of `x` *and the allocator's state*. Calling the C `.so` and then
//! the Rust `.so` in one process is therefore **not** a valid differential test
//! -- each call mutates the shared glibc tcache, so the second library starts
//! from a different heap than the first and legitimately returns a different
//! value. (Empirically the two libraries produce the same *cycle* of values but
//! at different phases.)
//!
//! The sound comparison is: replay one identical call sequence in two fresh
//! processes, one per `.so`, and diff the stdout byte-for-byte. Both children are
//! the same executable running the same code with the same PRNG seed, so the only
//! difference is which `.so` was loaded.
//!
//! This also satisfies the "if the project builds a binary, compare C and Rust
//! stdout byte-for-byte" requirement: this binary *is* that driver.

mod common;

use common::*;
use std::ffi::c_char;
use std::path::Path;
use std::process::{Command, Stdio};

// ---------------------------------------------------------------------------
// Scenarios
// ---------------------------------------------------------------------------

/// Scenarios whose stdout must match byte-for-byte between the two libraries.
const OUTPUT_SCENARIOS: &[&str] = &[
    "betagamma_seq",
    "betagamma_residues",
    "betagamma_repeat",
    "mixed",
    "lowlevel",
];

/// Scenarios that are expected to terminate abnormally; the two libraries must
/// die the same way (same exit status / signal).
const CRASH_SCENARIOS: &[&str] = &[
    "crash_hash_null_first",
    "crash_hash_null_second",
    "crash_hash_null_both",
    "crash_create_null_name",
];

fn cstr(bytes: &[u8]) -> Vec<c_char> {
    let mut v: Vec<c_char> = bytes.iter().map(|&b| b as c_char).collect();
    v.push(0);
    v
}

fn run_scenario(api: &Api, scenario: &str) {
    let mut out = String::new();
    match scenario {
        // CONFIGS.md row 34: 600 randomised betagamma calls, all param shapes.
        "betagamma_seq" => {
            let mut rng = Rng::new(SEED ^ 0x34);
            for i in 0..600 {
                let (p1, p2, p3, p4) = (
                    rng.interesting_i32(),
                    rng.interesting_i32(),
                    rng.interesting_i32(),
                    rng.interesting_i32(),
                );
                let v = unsafe { (api.betagamma)(p1, p2, p3, p4) };
                out.push_str(&format!("{i} betagamma({p1},{p2},{p3},{p4}) = {v}\n"));
            }
        }
        // CONFIGS.md rows 25-27, 33: every param1 residue incl. the erroring ones.
        "betagamma_residues" => {
            for p1 in -60i32..=60 {
                for &(p2, p3, p4) in &[
                    (0, 0, 0),
                    (1, 2, 3),
                    (-1, -2, -3),
                    (1000, -1000, 7),
                    (i32::MAX, i32::MIN, 0),
                ] {
                    let v = unsafe { (api.betagamma)(p1, p2, p3, p4) };
                    out.push_str(&format!("betagamma({p1},{p2},{p3},{p4}) = {v}\n"));
                }
            }
            for p1 in [i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1] {
                let v = unsafe { (api.betagamma)(p1, 5, 6, 7) };
                out.push_str(&format!("betagamma({p1},5,6,7) = {v}\n"));
            }
        }
        // CONFIGS.md row 36: the allocator-state cycle, not just one sample.
        "betagamma_repeat" => {
            for p1 in 0..20i32 {
                out.push_str(&format!("p1={p1}:"));
                for _ in 0..8 {
                    let v = unsafe { (api.betagamma)(p1, 1, 2, 3) };
                    out.push_str(&format!(" {v}"));
                }
                out.push('\n');
            }
        }
        // CONFIGS.md row 35: all five entry points interleaved in one workload.
        "mixed" => {
            let mut rng = Rng::new(SEED ^ 0x35);
            let mut live: Vec<*mut MemoryBlock> = Vec::new();
            for step in 0..800 {
                match rng.below(5) {
                    0 => {
                        let len = rng.below(32) as usize;
                        let name: Vec<u8> = (0..len).map(|_| 1 + rng.below(255) as u8).collect();
                        let s = cstr(&name);
                        let id = rng.interesting_i32();
                        let flags = rng.below(256) as u8;
                        let b = unsafe { (api.create_block)(id, s.as_ptr(), flags) };
                        out.push_str(&format!(
                            "{step} create_block -> {:?}\n",
                            defined_bytes(&b, len)
                        ));
                    }
                    1 => {
                        let n = rng.below(24) as usize;
                        let init = rng.interesting_i32();
                        let p = unsafe { (api.allocate_block)(n, init) };
                        match unsafe { read_block(p) } {
                            None => out.push_str(&format!("{step} allocate_block({n},{init}) -> NULL\n")),
                            Some((sz, els)) => {
                                out.push_str(&format!(
                                    "{step} allocate_block({n},{init}) -> size={sz} {els:?}\n"
                                ));
                                live.push(p);
                            }
                        }
                    }
                    2 => {
                        if live.len() >= 2 {
                            let i = rng.below(live.len() as u64) as usize;
                            let j = rng.below(live.len() as u64) as usize;
                            let h = unsafe { (api.compute_hash)(live[i], live[j]) };
                            // Only the *relative* ordering is printed (the hash
                            // itself), never an absolute address.
                            out.push_str(&format!("{step} compute_hash -> {h}\n"));
                        } else {
                            out.push_str(&format!("{step} compute_hash skipped\n"));
                        }
                    }
                    3 => {
                        if let Some(p) = live.pop() {
                            unsafe { (api.free_block)(p) };
                            out.push_str(&format!("{step} free_block\n"));
                        } else {
                            unsafe { (api.free_block)(std::ptr::null_mut()) };
                            out.push_str(&format!("{step} free_block(NULL)\n"));
                        }
                    }
                    _ => {
                        let (p1, p2, p3, p4) = (
                            rng.interesting_i32(),
                            rng.interesting_i32(),
                            rng.interesting_i32(),
                            rng.interesting_i32(),
                        );
                        let v = unsafe { (api.betagamma)(p1, p2, p3, p4) };
                        out.push_str(&format!("{step} betagamma({p1},{p2},{p3},{p4}) = {v}\n"));
                    }
                }
            }
            for p in live.drain(..) {
                unsafe { (api.free_block)(p) };
            }
        }
        // Lowest-level entry points only, driven directly (no betagamma).
        "lowlevel" => {
            let mut rng = Rng::new(SEED ^ 0x36);
            for step in 0..500 {
                let n = rng.below(40) as usize;
                let a = unsafe { (api.allocate_block)(n, rng.interesting_i32()) };
                let b = unsafe { (api.allocate_block)(n, rng.interesting_i32()) };
                let h = unsafe { (api.compute_hash)(a, b) };
                let h2 = unsafe { (api.compute_hash)(b, a) };
                let sa = unsafe { read_block(a) }.unwrap();
                let sb = unsafe { read_block(b) }.unwrap();
                out.push_str(&format!(
                    "{step} n={n} h={h} h2={h2} a={:?} b={:?}\n",
                    sa.1, sb.1
                ));
                unsafe { (api.free_block)(a) };
                unsafe { (api.free_block)(b) };
            }
        }
        // ERRORS.md rows 12-14: compute_hash has no null checks.
        "crash_hash_null_first" => {
            let mut only = MemoryBlock {
                data: 0x1000 as *mut i32,
                size: 1,
            };
            let v = unsafe { (api.compute_hash)(std::ptr::null_mut(), &mut only) };
            out.push_str(&format!("unexpectedly survived: {v}\n"));
        }
        "crash_hash_null_second" => {
            let mut only = MemoryBlock {
                data: 0x1000 as *mut i32,
                size: 1,
            };
            let v = unsafe { (api.compute_hash)(&mut only, std::ptr::null_mut()) };
            out.push_str(&format!("unexpectedly survived: {v}\n"));
        }
        "crash_hash_null_both" => {
            let v = unsafe { (api.compute_hash)(std::ptr::null_mut(), std::ptr::null_mut()) };
            out.push_str(&format!("unexpectedly survived: {v}\n"));
        }
        // ERRORS.md row 16: create_block strcpy's from `name` with no null check.
        "crash_create_null_name" => {
            let b = unsafe { (api.create_block)(1, std::ptr::null(), 0xAA) };
            out.push_str(&format!("unexpectedly survived: {}\n", b.id));
        }
        other => panic!("unknown scenario {other}"),
    }
    print!("{out}");
    use std::io::Write;
    std::io::stdout().flush().unwrap();
}

// ---------------------------------------------------------------------------
// Orchestration
// ---------------------------------------------------------------------------

struct Outcome {
    stdout: Vec<u8>,
    status: String,
}

fn spawn_child(exe: &Path, lib: &Path, scenario: &str) -> Outcome {
    let out = Command::new(exe)
        .arg("--child")
        .arg(lib)
        .arg(scenario)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn driver child");
    Outcome {
        stdout: out.stdout,
        status: describe(&out.status),
    }
}

fn describe(st: &std::process::ExitStatus) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(sig) = st.signal() {
            return format!("signal:{sig}");
        }
    }
    match st.code() {
        Some(c) => format!("exit:{c}"),
        None => "unknown".to_string(),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // Child mode: `--child <lib.so> <scenario>`
    if args.len() >= 4 && args[1] == "--child" {
        let api = Api::load(Path::new(&args[2]));
        run_scenario(&api, &args[3]);
        return;
    }

    // Ignore libtest-style flags cargo may pass through (e.g. --nocapture).
    let exe = std::env::current_exe().expect("current_exe");
    let c_lib = c_so_path();
    let rust_lib = rust_so_path();
    println!("C   .so: {}", c_lib.display());
    println!("Rust.so: {}", rust_lib.display());

    let mut failures = Vec::new();

    for &scenario in OUTPUT_SCENARIOS {
        let c = spawn_child(&exe, &c_lib, scenario);
        let r = spawn_child(&exe, &rust_lib, scenario);

        if c.status != "exit:0" {
            failures.push(format!("{scenario}: C child terminated {}", c.status));
            continue;
        }
        if c.status != r.status {
            failures.push(format!(
                "{scenario}: status mismatch C={} Rust={}",
                c.status, r.status
            ));
            continue;
        }
        if c.stdout != r.stdout {
            let cl: Vec<&[u8]> = c.stdout.split(|&b| b == b'\n').collect();
            let rl: Vec<&[u8]> = r.stdout.split(|&b| b == b'\n').collect();
            let mut diffs = Vec::new();
            for i in 0..cl.len().max(rl.len()) {
                let a = cl.get(i).copied().unwrap_or(b"<missing>");
                let b = rl.get(i).copied().unwrap_or(b"<missing>");
                if a != b {
                    diffs.push(format!(
                        "  line {}:\n    C   : {}\n    Rust: {}",
                        i + 1,
                        String::from_utf8_lossy(a),
                        String::from_utf8_lossy(b)
                    ));
                }
            }
            let shown = diffs.len();
            diffs.truncate(12);
            failures.push(format!(
                "{scenario}: stdout differs ({} bytes vs {} bytes, {shown} differing lines)\n{}",
                c.stdout.len(),
                r.stdout.len(),
                diffs.join("\n")
            ));
            continue;
        }
        println!(
            "ok  {scenario:<20} {} bytes of stdout identical",
            c.stdout.len()
        );
    }

    for &scenario in CRASH_SCENARIOS {
        let c = spawn_child(&exe, &c_lib, scenario);
        let r = spawn_child(&exe, &rust_lib, scenario);
        if c.status != r.status {
            failures.push(format!(
                "{scenario}: termination mismatch C={} Rust={} (C stdout {:?}, Rust stdout {:?})",
                c.status,
                r.status,
                String::from_utf8_lossy(&c.stdout),
                String::from_utf8_lossy(&r.stdout)
            ));
            continue;
        }
        if c.status == "exit:0" {
            failures.push(format!(
                "{scenario}: expected abnormal termination, both exited 0"
            ));
            continue;
        }
        println!("ok  {scenario:<24} both terminated with {}", c.status);
    }

    if !failures.is_empty() {
        eprintln!("\n=== {} DRIVER FAILURE(S) ===", failures.len());
        for f in &failures {
            eprintln!("{f}");
        }
        std::process::exit(1);
    }
    println!(
        "\nall {} output scenarios + {} crash scenarios matched",
        OUTPUT_SCENARIOS.len(),
        CRASH_SCENARIOS.len()
    );
}
