//! Differential tests for the `driver` *executable*.
//!
//! The crate contains two translations of the same C program:
//!
//!  * `src/capi/` — the C ABI shared library surface (covered by
//!    `tests/{configs,errors}_{lib,app}.rs`), and
//!  * `src/main.rs` + `src/cio.rs` + `src/scene.rs` + `src/shape.rs` — the safe,
//!    idiomatic translation that the `driver` binary is built from.
//!
//! This file drives the second one exactly like a user does: it runs the C
//! executable (built from the unmodified `c_src`) and the Rust executable with
//! the same `stdin`, the same working directory contents and the same
//! environment, and compares `stdout`, `stderr`, the exit status and every file
//! that was created.
//!
//! Heap pointer values printed with `%p` are normalised to first-use ids (they
//! can never be equal by construction), exactly like `dev_tests/compare.py`.

mod common;

use std::path::PathBuf;

use common::*;

const SEED: u64 = 0x5EED_2026;
const TIMEOUT_MS: u64 = 2000;

/// Builds the C reference *executable* from the unmodified `c_src` (the same
/// source list as the CMake target).
fn c_bin_path() -> PathBuf {
    let root = crate_dir();
    let out_dir = root.join("c_build");
    let bin = out_dir.join("driver_c");
    let sources = [
        root.join("c_src/src/main.c"),
        root.join("c_src/src/scene.c"),
        root.join("c_src/src/shape.c"),
        root.join("c_src/include/scene.h"),
        root.join("c_src/include/shape.h"),
    ];
    let newest = sources
        .iter()
        .map(|p| std::fs::metadata(p).and_then(|m| m.modified()).unwrap())
        .max()
        .unwrap();
    let fresh = std::fs::metadata(&bin)
        .and_then(|m| m.modified())
        .map(|t| t >= newest)
        .unwrap_or(false);
    if fresh {
        return bin;
    }
    std::fs::create_dir_all(&out_dir).unwrap();
    let tmp = out_dir.join(format!("driver_c.{}", std::process::id()));
    let status = std::process::Command::new("gcc")
        .args(["-O0", "-g", "-I"])
        .arg(root.join("c_src/include"))
        .arg(root.join("c_src/src/main.c"))
        .arg(root.join("c_src/src/scene.c"))
        .arg(root.join("c_src/src/shape.c"))
        .arg("-o")
        .arg(&tmp)
        .status()
        .expect("gcc");
    assert!(status.success(), "gcc failed to build the C executable");
    std::fs::rename(&tmp, &bin).unwrap();
    bin
}

fn run_bin(bin: &std::path::Path, case: &str, stdin: &[u8], files: &[(&str, &[u8])]) -> String {
    let dir = fresh_dir(case);
    for (name, content) in files {
        std::fs::write(dir.join(name), content).unwrap();
    }
    let stdin_path = dir.join(".stdin");
    std::fs::write(&stdin_path, stdin).unwrap();
    let out_path = dir.join(".stdout");
    let err_path = dir.join(".stderr");

    let mut child = std::process::Command::new(bin)
        .current_dir(&dir)
        .stdin(std::fs::File::open(&stdin_path).unwrap())
        .stdout(std::fs::File::create(&out_path).unwrap())
        .stderr(std::fs::File::create(&err_path).unwrap())
        .spawn()
        .expect("spawn");

    let start = std::time::Instant::now();
    let status = loop {
        match child.try_wait().unwrap() {
            Some(s) => break Some(s),
            None => {
                if start.elapsed().as_millis() as u64 > TIMEOUT_MS {
                    let _ = child.kill();
                    let _ = child.wait();
                    break None;
                }
                std::thread::sleep(std::time::Duration::from_millis(2));
            }
        }
    };

    let out = std::fs::read(&out_path).unwrap_or_default();
    let err = std::fs::read(&err_path).unwrap_or_default();
    let status_text = match status {
        None => "TIMEOUT (killed)".to_string(),
        Some(s) => match s.code() {
            Some(c) => format!("exit {}", c),
            None => format!(
                "signal {:?}",
                std::os::unix::process::ExitStatusExt::signal(&s)
            ),
        },
    };

    let norm = |b: &[u8]| normalise_ptrs(&normalise_dir(b, &dir));
    let mut text = String::new();
    text.push_str(&format!("--- status ---\n{}\n", status_text));
    text.push_str("--- stdout ---\n");
    text.push_str(&numbered(&norm(&out)));
    text.push_str("--- stderr ---\n");
    text.push_str(&numbered(&norm(&err)));
    text.push_str("--- files ---\n");
    let mut entries: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    entries.sort();
    for p in entries {
        let name = p.file_name().unwrap().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        text.push_str(&format!("{}:\n", name));
        text.push_str(&numbered(&norm(&std::fs::read(&p).unwrap_or_default())));
    }
    let _ = std::fs::remove_dir_all(&dir);
    text
}

fn diff_bin(
    c_bin: &std::path::Path,
    r_bin: &std::path::Path,
    case: &str,
    stdin: &[u8],
    files: &[(&str, &[u8])],
) -> Result<(), String> {
    let c = run_bin(c_bin, case, stdin, files);
    let r = run_bin(r_bin, case, stdin, files);
    if std::env::var_os("DIFF_DUMP").is_some() {
        eprintln!("===== binary case {} (C transcript) =====\n{}", case, c);
    }
    if c == r {
        return Ok(());
    }
    let mut msg = format!("binary case `{}` diverges (stdin={:?}):\n", case, escape(stdin));
    let cl: Vec<&str> = c.lines().collect();
    let rl: Vec<&str> = r.lines().collect();
    for i in 0..cl.len().max(rl.len()) {
        let a = cl.get(i).copied().unwrap_or("<missing>");
        let b = rl.get(i).copied().unwrap_or("<missing>");
        if a != b {
            msg.push_str(&format!(
                "  line {}:\n    C   : {}\n    RUST: {}\n",
                i + 1,
                a,
                b
            ));
        }
    }
    Err(msg)
}

include!("common/binary_cases.rs");

#[test]
fn binary_diff() {
    let c_bin = c_bin_path();
    let r_bin = PathBuf::from(env!("CARGO_BIN_EXE_driver"));
    let mut rep = Report::new();

    // The curated scenarios (mechanically generated from dev_tests/compare.py,
    // see tests/binary_cases.rs).
    for (name, stdin, files) in curated_cases() {
        rep.check(diff_bin(&c_bin, &r_bin, &name, &stdin, &files));
    }

    // Randomised sessions: arbitrary menu choices with arbitrary arguments.
    let mut rng = Rng::new(SEED ^ 0xB1);
    for k in 0..48 {
        let mut input: Vec<u8> = Vec::new();
        let steps = 1 + rng.below(10);
        for _ in 0..steps {
            input.extend_from_slice(format!("{}\n", rng.range_i32(-2, 14)).as_bytes());
            for _ in 0..rng.below(3) {
                match rng.below(7) {
                    0 => input.extend_from_slice(b"abc\n"),
                    1 => input.extend_from_slice(b"\n"),
                    2 => input
                        .extend_from_slice(format!("{}\n", rng.range_i32(-3, 12)).as_bytes()),
                    3 => input.extend_from_slice(b"name with space\n"),
                    4 => input.extend_from_slice(b"f.txt\n"),
                    5 => input.extend_from_slice(b"scene.dat\n"),
                    _ => input.extend_from_slice(format!("{}\n", rng.range_i32(0, 9)).as_bytes()),
                }
            }
        }
        input.extend_from_slice(b"12\n");
        rep.check(diff_bin(
            &c_bin,
            &r_bin,
            &format!("bin-rnd-{}", k),
            &input,
            &[("scene.dat", b"Seed\n2\n1\n2\n")],
        ));
    }

    // Randomised *well formed* sessions.
    let mut rng = Rng::new(SEED ^ 0xB2);
    for k in 0..48 {
        let mut input: Vec<u8> = Vec::new();
        let mut scenes: Vec<usize> = Vec::new();
        let steps = 3 + rng.below(14);
        for _ in 0..steps {
            match rng.below(9) {
                0 => {
                    input.extend_from_slice(b"2\n");
                    input.extend_from_slice(format!("S{}\n", scenes.len()).as_bytes());
                    if scenes.len() < 10 {
                        scenes.push(0);
                    }
                }
                1 if !scenes.is_empty() => {
                    let idx = rng.below(scenes.len());
                    input.extend_from_slice(
                        format!("3\n{}\n{}\n", idx, rng.range_i32(0, 9)).as_bytes(),
                    );
                    scenes[idx] += 1;
                }
                2 if !scenes.is_empty() => {
                    let idx = rng.below(scenes.len());
                    let n = scenes[idx];
                    let pick = if n == 0 { 1 } else { 1 + rng.below(n) };
                    input.extend_from_slice(format!("4\n{}\n{}\n", idx, pick).as_bytes());
                    if n > 0 {
                        scenes[idx] -= 1;
                    }
                }
                3 if !scenes.is_empty() => {
                    let idx = rng.below(scenes.len());
                    input.extend_from_slice(format!("5\n{}\n", idx).as_bytes());
                }
                4 => input.extend_from_slice(b"6\n"),
                5 if !scenes.is_empty() => {
                    let idx = rng.below(scenes.len());
                    input.extend_from_slice(format!("7\n{}\ns{}.txt\n", idx, idx).as_bytes());
                }
                6 => input.extend_from_slice(b"8\nscene.dat\n"),
                7 if !scenes.is_empty() => {
                    let idx = rng.below(scenes.len());
                    input.extend_from_slice(format!("11\n{}\n", idx).as_bytes());
                    scenes.remove(idx);
                }
                _ => {
                    input.extend_from_slice(
                        format!("10\n{}\n{}\n", rng.below(3), rng.below(3)).as_bytes(),
                    );
                }
            }
        }
        input.extend_from_slice(b"1\n6\n12\n");
        rep.check(diff_bin(
            &c_bin,
            &r_bin,
            &format!("bin-rnd-wf-{}", k),
            &input,
            &[("scene.dat", b"Seed\n3\n0\n5\n9\n")],
        ));
    }

    // Randomised scene files fed through menu entry 8 (load).
    let mut rng = Rng::new(SEED ^ 0xB3);
    for k in 0..32 {
        let mut content: Vec<u8> = Vec::new();
        for i in 0..rng.below(70) {
            content.push(b'A' + (i % 26) as u8);
        }
        content.push(b'\n');
        content.extend_from_slice(format!("{}\n", rng.range_i32(-3, 60)).as_bytes());
        for _ in 0..rng.below(60) {
            let t = match rng.below(6) {
                0 => rng.range_i32(-100, -1),
                1 => rng.range_i32(10, 100),
                _ => rng.range_i32(0, 9),
            };
            content.extend_from_slice(format!("{}", t).as_bytes());
            match rng.below(4) {
                0 => content.extend_from_slice(b"\r\n"),
                1 => content.extend_from_slice(b" \n"),
                2 => content.extend_from_slice(b"\n\n"),
                _ => content.push(b'\n'),
            }
        }
        rep.check(diff_bin(
            &c_bin,
            &r_bin,
            &format!("bin-rnd-load-{}", k),
            b"8\nscene.dat\n5\n0\n6\n12\n",
            &[("scene.dat", &content)],
        ));
    }

    rep.finish("driver executable (idiomatic translation)");
}
