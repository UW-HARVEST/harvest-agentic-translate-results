//! Process-level stdout comparison.
//!
//! `c_src/CMakeLists.txt` declares no `add_executable` and `Cargo.toml` declares
//! no `[[bin]]`, so the project ships no driver binary (see SYMBOLS.md). To get
//! the equivalent end-to-end evidence anyway -- a whole process run, with libc's
//! own at-exit flush rather than a harness-forced `fflush`, and stdout being a
//! pipe rather than a regular file -- this test compiles a tiny neutral host
//! program at test time that `dlopen`s whichever `.so` it is handed and calls
//! `driver` for each argument. The SAME host binary is run against the C `.so`
//! and against the Rust `.so`; their stdout must be byte-identical.
//!
//! Nothing in `c_src/` is modified: the host source is written to the crate's
//! own `target/` directory.

use std::path::{Path, PathBuf};
use std::process::Command;

const HOST_C: &str = r#"
#include <dlfcn.h>
#include <stdio.h>
#include <stdlib.h>

int main(int argc, char **argv) {
    if (argc < 2) { fprintf(stderr, "usage: host <so> [ints...]\n"); return 2; }
    void *h = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    if (!h) { fprintf(stderr, "dlopen: %s\n", dlerror()); return 3; }
    void (*d)(int) = (void (*)(int))dlsym(h, "driver");
    if (!d) { fprintf(stderr, "dlsym: %s\n", dlerror()); return 4; }
    for (int i = 2; i < argc; i++) {
        d((int)strtol(argv[i], NULL, 10));
    }
    /* Deliberately no fflush and no dlclose: let libc flush at exit, exactly
       as a real consumer program would. */
    return 0;
}
"#;

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so() -> PathBuf {
    crate_root().join("../c_src/build/libdriver.so")
}

fn rust_sos() -> Vec<PathBuf> {
    ["release", "debug"]
        .iter()
        .map(|p| crate_root().join("target").join(p).join("libdriver.so"))
        .filter(|p| p.exists())
        .collect()
}

fn cc() -> Option<String> {
    for candidate in [std::env::var("CC").unwrap_or_default(), "cc".into(), "gcc".into()] {
        if candidate.is_empty() {
            continue;
        }
        if Command::new(&candidate)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return Some(candidate);
        }
    }
    None
}

/// Compiles the host once and returns its path.
fn host_binary() -> Option<PathBuf> {
    let dir = crate_root().join("target").join("difftest");
    std::fs::create_dir_all(&dir).expect("create target/difftest");
    let src = dir.join("host.c");
    let bin = dir.join("host");
    std::fs::write(&src, HOST_C).expect("write host.c");
    let cc = cc()?;
    let out = Command::new(cc)
        .arg("-O0")
        .arg("-o")
        .arg(&bin)
        .arg(&src)
        .arg("-ldl")
        .output()
        .expect("spawn cc");
    if !out.status.success() {
        eprintln!(
            "skipping subprocess test: could not compile host: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        return None;
    }
    Some(bin)
}

fn run_host(host: &Path, so: &Path, inputs: &[i32]) -> Vec<u8> {
    let mut cmd = Command::new(host);
    cmd.arg(so);
    for x in inputs {
        cmd.arg(x.to_string());
    }
    let out = cmd.output().expect("run host");
    assert!(
        out.status.success(),
        "host exited with {:?} for {}: {}",
        out.status.code(),
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    out.stdout
}

struct Rng(u64);
impl Rng {
    fn next_i32(&mut self) -> i32 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        (z ^ (z >> 31)) as u32 as i32
    }
}

/// Whole-process stdout, byte for byte, C `.so` vs Rust `.so`, with stdout being
/// a pipe (fully buffered) and flushed only by libc at process exit.
#[test]
fn subprocess_stdout_matches_byte_for_byte() {
    assert!(
        c_so().exists(),
        "C .so missing at {} -- build c_src first",
        c_so().display()
    );
    let Some(host) = host_binary() else { return };

    // A batch spanning every interesting class: zero, small +/-, the y == 0
    // preimage, the sign flip, both 2*x overflow edges, the +300 overflow edge,
    // the domain extremes, plus fixed-seed random values.
    let mut inputs: Vec<i32> = vec![
        0,
        1,
        -1,
        150,
        -150,
        -151,
        -149,
        i32::MAX / 2,
        i32::MAX / 2 + 1,
        i32::MIN / 2,
        i32::MIN / 2 - 1,
        ((i32::MAX as i64 - 300) / 2) as i32,
        ((i32::MAX as i64 - 300) / 2) as i32 + 1,
        i32::MIN,
        i32::MIN + 1,
        i32::MAX - 1,
        i32::MAX,
    ];
    let mut rng = Rng(0x5AB9_6041_D1FF_0001u64);
    for _ in 0..400 {
        inputs.push(rng.next_i32());
    }

    let c_out = run_host(&host, &c_so(), &inputs);
    assert!(!c_out.is_empty(), "C host produced no stdout");
    assert_eq!(
        c_out.iter().filter(|b| **b == b'\n').count(),
        inputs.len(),
        "C host printed {} lines for {} inputs",
        c_out.iter().filter(|b| **b == b'\n').count(),
        inputs.len()
    );

    let rust = rust_sos();
    assert!(!rust.is_empty(), "no Rust .so built");
    for so in rust {
        let r_out = run_host(&host, &so, &inputs);
        if r_out != c_out {
            let cl: Vec<&[u8]> = c_out.split(|b| *b == b'\n').collect();
            let rl: Vec<&[u8]> = r_out.split(|b| *b == b'\n').collect();
            for i in 0..cl.len().max(rl.len()) {
                let a = cl.get(i).copied().unwrap_or(b"<missing>");
                let b = rl.get(i).copied().unwrap_or(b"<missing>");
                if a != b {
                    panic!(
                        "subprocess stdout diverged at line {i} (input x = {:?}) for {}:\n  \
                         C    = {:?}\n  Rust = {:?}",
                        inputs.get(i),
                        so.display(),
                        String::from_utf8_lossy(a),
                        String::from_utf8_lossy(b)
                    );
                }
            }
            panic!("subprocess stdout diverged for {}", so.display());
        }
    }
}

/// Same comparison with zero inputs: both hosts must emit nothing at all. A
/// library that printed something at load time (a constructor, a Rust
/// `ctor`-style init) would be caught here and nowhere else.
#[test]
fn subprocess_no_output_on_load() {
    if !c_so().exists() {
        return;
    }
    let Some(host) = host_binary() else { return };
    let c_out = run_host(&host, &c_so(), &[]);
    assert_eq!(
        c_out,
        b"",
        "C .so printed something merely by being loaded: {:?}",
        String::from_utf8_lossy(&c_out)
    );
    for so in rust_sos() {
        let r_out = run_host(&host, &so, &[]);
        assert_eq!(
            r_out, c_out,
            "{} printed {:?} at load time but the C .so printed nothing",
            so.display(),
            String::from_utf8_lossy(&r_out)
        );
    }
}
