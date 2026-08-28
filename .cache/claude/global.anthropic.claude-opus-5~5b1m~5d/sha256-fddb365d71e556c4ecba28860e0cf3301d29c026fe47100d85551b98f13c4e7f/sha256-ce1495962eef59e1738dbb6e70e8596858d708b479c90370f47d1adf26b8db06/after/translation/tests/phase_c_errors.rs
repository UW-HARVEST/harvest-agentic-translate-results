// Phase C — error-path differential tests, one test per row of ERRORS.md.
//
// Each test constructs the exact invalid input / failure condition, calls BOTH
// implementations through their `.so` exports, and asserts they return the same
// sentinel AND print the same bytes. Where the C source pins the answer down
// (e.g. "returns -1", "returns 0 and prints nothing") the absolute value is
// asserted too, so a harness that silently captured nothing cannot pass.

mod common;

use common::*;
use std::ffi::c_char;
use std::path::{Path, PathBuf};
use std::process::Command;

// ===========================================================================
// E1 / E3 / E23 — forced allocation failure.
//
// `malloc` failing is unreachable through the public ABI for the 24-byte
// ProcessState, so these rows are driven for real with an LD_PRELOAD shim that
// fails exactly one malloc of a chosen size, inside a child process that
// dlopen()s one implementation. See tests/fixtures/oom_{preload,driver}.c.
// ===========================================================================
fn oom_tools() -> (PathBuf, PathBuf) {
    let out = manifest_dir().join("target");
    let fixtures = manifest_dir().join("tests/fixtures");
    let preload = out.join("oom_preload.so");
    let driver = out.join("oom_driver");

    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".to_string());
    let s1 = Command::new(&cc)
        .args(["-shared", "-fPIC", "-O1", "-o"])
        .arg(&preload)
        .arg(fixtures.join("oom_preload.c"))
        .status()
        .unwrap_or_else(|e| panic!("cannot run {cc}: {e}"));
    assert!(s1.success(), "failed to build oom_preload.so");

    let s2 = Command::new(&cc)
        .args(["-O1", "-o"])
        .arg(&driver)
        .arg(fixtures.join("oom_driver.c"))
        .arg("-ldl")
        .status()
        .expect("run cc");
    assert!(s2.success(), "failed to build oom_driver");

    (preload, driver)
}

struct ChildRun {
    code: Option<i32>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

fn run_oom(
    preload: &Path,
    driver: &Path,
    so: &Path,
    scenario: &str,
    fail_size: usize,
    params: &[i32],
) -> ChildRun {
    let mut cmd = Command::new(driver);
    cmd.env("LD_PRELOAD", preload)
        .arg(so)
        .arg(scenario)
        .arg(fail_size.to_string());
    for p in params {
        cmd.arg(p.to_string());
    }
    let out = cmd.output().expect("run oom_driver");
    ChildRun {
        code: out.status.code(),
        stdout: out.stdout,
        stderr: out.stderr,
    }
}

#[track_caller]
fn diff_oom(ctx: &str, scenario: &str, fail_size: usize, params: &[i32]) -> Vec<u8> {
    let (preload, driver) = oom_tools();
    let c_so = c_so_path().canonicalize().expect("canonicalize C .so");
    let r_so = rust_so_path().canonicalize().expect("canonicalize Rust .so");

    let c = run_oom(&preload, &driver, &c_so, scenario, fail_size, params);
    let r = run_oom(&preload, &driver, &r_so, scenario, fail_size, params);

    assert_eq!(
        c.code, r.code,
        "[{ctx}] exit-code divergence (C={:?} RUST={:?})",
        c.code, r.code
    );
    assert!(
        c.stderr.is_empty() && r.stderr.is_empty(),
        "[{ctx}] driver reported an error\n  C stderr: {}\n  RUST stderr: {}",
        show(&c.stderr),
        show(&r.stderr)
    );
    assert_eq!(
        c.stdout,
        r.stdout,
        "[{ctx}] stdout divergence\n  C    = \"{}\"\n  RUST = \"{}\"",
        show(&c.stdout),
        show(&r.stdout)
    );
    assert_eq!(c.code, Some(0), "[{ctx}] driver exited non-zero");
    c.stdout
}

/// E1 — `malloc(sizeof(ProcessState))` returns NULL (`lib.c:60`).
#[test]
fn err_e1_state_malloc_failure() {
    for init in [0i32, 12345, -1, i32::MIN, i32::MAX] {
        for cap in [0i32, 1, 16, 128, 4096] {
            let out = diff_oom(
                &format!("E1 init={init} cap={cap}"),
                "create_state",
                STATE_SIZE,
                &[init, cap],
            );
            let text = String::from_utf8_lossy(&out).to_string();
            assert!(
                text.contains("Error: Failed to allocate memory for state\n"),
                "E1 did not take the state-allocation-failure branch: {text:?}"
            );
            assert!(text.contains("create_state -> NULL\n"), "E1: {text:?}");
            assert!(text.contains("oom_fired = 1\n"), "E1 shim never fired: {text:?}");
            assert!(
                !text.contains("Error: Failed to allocate buffer"),
                "E1 took the wrong branch: {text:?}"
            );
        }
    }
}

/// E3 — the *buffer* `malloc` fails for an otherwise perfectly valid capacity
/// (`lib.c:78`): prints, `free(state)`, returns NULL.
#[test]
fn err_e3_buffer_malloc_failure_forced() {
    for cap in [1i32, 15, 16, 17, 128, 4096, 65536] {
        for init in [0i32, 12345, -7, i32::MIN] {
            let out = diff_oom(
                &format!("E3 cap={cap} init={init}"),
                "create_state",
                cap as usize,
                &[init, cap],
            );
            let text = String::from_utf8_lossy(&out).to_string();
            assert!(
                text.contains("Error: Failed to allocate buffer\n"),
                "E3 did not take the buffer-allocation-failure branch: {text:?}"
            );
            assert!(text.contains("create_state -> NULL\n"), "E3: {text:?}");
            assert!(text.contains("oom_fired = 1\n"), "E3 shim never fired: {text:?}");
            assert!(
                !text.contains("Error: Failed to allocate memory for state"),
                "E3 took the wrong branch: {text:?}"
            );
        }
    }
    // control: nothing is armed that can match, so the happy path must run and
    // still agree between the two implementations.
    let out = diff_oom("E3 control", "create_state", 999_331, &[12345, 128]);
    let text = String::from_utf8_lossy(&out).to_string();
    assert!(text.contains("create_state -> NON-NULL\n"), "{text:?}");
    assert!(text.contains("oom_fired = 0\n"), "{text:?}");
    assert!(text.contains("buf = \"State:12345:Mode:3\"\n"), "{text:?}");
}

/// E23 — `confusion` returns -1 when `create_state` fails (`lib.c:188`),
/// for both underlying failure sites.
#[test]
fn err_e23_confusion_create_state_failure() {
    for (tag, fail_size) in [("state", STATE_SIZE), ("buffer", 128usize)] {
        for params in [
            [0i32, 0, 0, 0],
            [7, 8, 9, 1],
            [i32::MIN, i32::MIN, i32::MIN, i32::MIN],
            [i32::MAX, i32::MAX, i32::MAX, i32::MAX],
            [1078530011, 42, -5, 3],
        ] {
            let out = diff_oom(
                &format!("E23 {tag} {params:?}"),
                "confusion",
                fail_size,
                &params,
            );
            let text = String::from_utf8_lossy(&out).to_string();
            assert!(text.contains("confusion -> -1\n"), "E23 {tag}: {text:?}");
            assert!(text.contains("oom_fired = 1\n"), "E23 {tag}: {text:?}");
            // The four Debug lines are emitted *before* the failure.
            for (i, p) in params.iter().enumerate() {
                assert!(
                    text.contains(&format!("Debug: param{} = {p}\n", i + 1)),
                    "E23 {tag} missing Debug line {}: {text:?}",
                    i + 1
                );
            }
        }
    }
}

/// Allocator-behaviour parity: the two implementations must perform the *same*
/// malloc/free calls for the same work. Catches leaks, double frees and
/// differently-sized allocations, none of which the return-value/stdout
/// comparison can see.
#[test]
fn alloc_trace_parity() {
    let (preload, driver) = oom_tools();
    let c_so = c_so_path().canonicalize().unwrap();
    let r_so = rust_so_path().canonicalize().unwrap();

    let mut rng = Rng::new(0xA110C);
    let mut cases: Vec<[i32; 4]> = vec![
        [0, 128, 0, 0],
        [-987654321, 128, 47, 1],
        [1078530011, 1, 9, 2],
        [i32::MIN, 16, -7, 3],
        [i32::MAX, 4096, 3, 4],
        [12345, 0, 0, -1],
    ];
    for _ in 0..24 {
        cases.push([
            rng.interesting_i32(),
            (rng.below(300) as i32) - 4,
            rng.interesting_i32(),
            rng.interesting_i32(),
        ]);
    }

    for params in cases {
        let c = run_oom(&preload, &driver, &c_so, "alloc_trace", 999_331, &params);
        let r = run_oom(&preload, &driver, &r_so, "alloc_trace", 999_331, &params);
        assert!(c.stderr.is_empty() && r.stderr.is_empty());
        assert_eq!(c.code, r.code, "alloc_trace {params:?} exit code");
        assert_eq!(
            c.stdout,
            r.stdout,
            "alloc_trace {params:?} divergence\n  C    = \"{}\"\n  RUST = \"{}\"",
            show(&c.stdout),
            show(&r.stdout)
        );
        // Every allocation must be released again in both.
        for line in String::from_utf8_lossy(&c.stdout).lines() {
            if let Some(rest) = line.split("mallocs=").nth(1) {
                let m: u64 = rest.split_whitespace().next().unwrap().parse().unwrap();
                let f: u64 = line
                    .split("frees=")
                    .nth(1)
                    .unwrap()
                    .split_whitespace()
                    .next()
                    .unwrap()
                    .parse()
                    .unwrap();
                assert_eq!(m, f, "leak/double-free in {line:?} for {params:?}");
            }
        }
    }
}

// ===========================================================================
// E2 — capacity < 0: malloc((size_t)(int64)capacity) is enormous -> NULL
// ===========================================================================
#[test]
fn err_e2_negative_capacity() {
    let mut caps: Vec<i32> = vec![-1, -2, -8, -16, -127, -128, -1000, -65536, i32::MIN, i32::MIN + 1];
    let mut rng = Rng::new(0xE2E2);
    for _ in 0..60 {
        caps.push(-(1 + (rng.below(2_000_000_000) as i32)));
    }
    for cap in caps {
        let init = 12345;
        let out = diff_and_get(&format!("E2 cap={cap}"), &move |lib, log| unsafe {
            let s = (lib.create_state)(init, cap);
            log.push(format!("create_state -> null={}", s.is_null()));
            if !s.is_null() {
                log_state_no_buffer(log, "unexpected", s);
                (lib.destroy_state)(s);
            }
        });
        assert_eq!(out.log, vec!["create_state -> null=true".to_string()]);
        assert_eq!(
            out.stdout, b"Error: Failed to allocate buffer\n",
            "E2 cap={cap}: unexpected stdout {:?}",
            show(&out.stdout)
        );
    }
}

// ===========================================================================
// E4 — capacity == 0: malloc(0) succeeds, snprintf writes nothing.
//
// The buffer bytes are left indeterminate by the C, so only the defined part of
// the state is compared.
// ===========================================================================
#[test]
fn err_e4_zero_capacity() {
    let mut rng = Rng::new(0xE4);
    for i in 0..80 {
        let init = rng.interesting_i32();
        let out = diff_and_get(&format!("E4 #{i} init={init}"), &move |lib, log| unsafe {
            let s = (lib.create_state)(init, 0);
            log.push(format!("null={}", s.is_null()));
            if !s.is_null() {
                log_state_no_buffer(log, "state", s);
                (lib.destroy_state)(s);
            }
        });
        assert_eq!(out.log[0], "null=false", "E4: glibc malloc(0) returns non-NULL");
        assert!(out.log[1].contains("flags=0x00007b05"), "E4: {:?}", out.log[1]);
        assert!(out.log[1].contains("capacity=0"), "E4: {:?}", out.log[1]);
        assert!(out.stdout.is_empty(), "E4 printed {:?}", show(&out.stdout));
    }
}

// ===========================================================================
// E5 — capacity too small for "State:%d:Mode:%d": snprintf truncates
// ===========================================================================
#[test]
fn err_e5_truncating_capacity() {
    let mut rng = Rng::new(0xE5);
    for cap in 1..=19i32 {
        for i in 0..20 {
            let init = rng.interesting_i32();
            let out = diff_and_get(
                &format!("E5 cap={cap} #{i} init={init}"),
                &move |lib, log| unsafe {
                    let s = (lib.create_state)(init, cap);
                    log_state(log, "state", s);
                    (lib.destroy_state)(s);
                },
            );
            // snprintf always NUL-terminates, so at most capacity-1 chars.
            let snap = &out.log[0];
            assert!(snap.contains(&format!("capacity={cap}")), "{snap}");
            let quoted = snap.split("buf=Some(\"").nth(1).unwrap();
            let content = &quoted[..quoted.rfind("\")").unwrap()];
            assert!(
                content.len() <= (cap - 1) as usize,
                "E5 cap={cap}: {content:?} longer than capacity-1"
            );
            let full = format!("State:{init}:Mode:3");
            assert!(
                full.starts_with(content),
                "E5 cap={cap}: {content:?} is not a prefix of {full:?}"
            );
        }
    }
}

// ===========================================================================
// E6 — destroy_state(NULL) is a silent no-op
// ===========================================================================
#[test]
fn err_e6_destroy_null() {
    let out = diff_and_get("E6", &|lib, log| unsafe {
        for _ in 0..5 {
            (lib.destroy_state)(std::ptr::null_mut());
        }
        log.push("survived".into());
    });
    assert_eq!(out.log, vec!["survived".to_string()]);
    assert!(out.stdout.is_empty(), "E6 printed {:?}", show(&out.stdout));
}

// ===========================================================================
// E7 — destroy_state on a state whose buffer field is NULL (lib.c:92)
// ===========================================================================
#[test]
fn err_e7_destroy_state_with_null_buffer() {
    let mut rng = Rng::new(0xE7);
    for i in 0..60 {
        let init = rng.interesting_i32();
        let out = diff_and_get(&format!("E7 #{i}"), &move |lib, log| unsafe {
            let s = (lib.create_state)(init, 128);
            null_out_buffer(s);
            log_state_no_buffer(log, "before_destroy", s);
            (lib.destroy_state)(s); // must free only the struct
            log.push("survived".into());
        });
        assert!(out.log[0].contains("buf_null=true"), "{:?}", out.log[0]);
        assert_eq!(out.log[1], "survived");
        assert!(out.stdout.is_empty(), "E7 printed {:?}", show(&out.stdout));
    }
}

// ===========================================================================
// E8 — process_buffer(NULL, target) -> -1 (lib.c:100 first disjunct)
// ===========================================================================
#[test]
fn err_e8_process_buffer_null_state() {
    for t in -128i32..=127 {
        let out = diff_and_get(&format!("E8 t={t}"), &move |lib, log| unsafe {
            log.push(format!(
                "pb -> {}",
                (lib.process_buffer)(std::ptr::null_mut(), t as c_char)
            ));
        });
        assert_eq!(out.log, vec!["pb -> -1".to_string()]);
        assert_eq!(out.stdout, b"Error: Null pointer in process_buffer\n");
    }
}

// ===========================================================================
// E9 — process_buffer(state with NULL buffer) -> -1 (second disjunct)
// ===========================================================================
#[test]
fn err_e9_process_buffer_null_buffer() {
    let mut rng = Rng::new(0xE9);
    for i in 0..80 {
        let init = rng.interesting_i32();
        let t = rng.next_u32() as u8 as i8;
        let out = diff_and_get(&format!("E9 #{i} t={t}"), &move |lib, log| unsafe {
            let s = (lib.create_state)(init, 128);
            null_out_buffer(s);
            log.push(format!("pb -> {}", (lib.process_buffer)(s, t as c_char)));
            log.push(format!("pb2 -> {}", (lib.process_buffer)(s, 0)));
            (lib.destroy_state)(s);
        });
        assert_eq!(out.log, vec!["pb -> -1".to_string(), "pb2 -> -1".to_string()]);
        assert_eq!(
            out.stdout,
            b"Error: Null pointer in process_buffer\nError: Null pointer in process_buffer\n"
        );
    }
}

// ===========================================================================
// E10 — strlen(buffer) == 0: the while loop never runs (lib.c:109)
// ===========================================================================
#[test]
fn err_e10_process_buffer_empty_string() {
    let mut rng = Rng::new(0xEA);
    for i in 0..60 {
        let init = rng.interesting_i32();
        let out = diff_and_get(&format!("E10 #{i} init={init}"), &move |lib, log| unsafe {
            // capacity 1 => snprintf writes just the NUL => strlen == 0
            let s = (lib.create_state)(init, 1);
            log_state(log, "state", s);
            for t in [b'0' as i8, b'S' as i8, 0, -1, 127] {
                log.push(format!("pb({t}) -> {}", (lib.process_buffer)(s, t as c_char)));
            }
            (lib.destroy_state)(s);
        });
        assert!(out.log[0].contains("buf=Some(\"\")"), "{:?}", out.log[0]);
        for k in 1..6 {
            assert!(out.log[k].ends_with("-> 0"), "E10: {:?}", out.log[k]);
        }
        assert!(out.stdout.is_empty(), "E10 printed {:?}", show(&out.stdout));
    }
}

// ===========================================================================
// E11 — memchr returns NULL: break with the count so far (lib.c:112)
// ===========================================================================
#[test]
fn err_e11_process_buffer_no_match() {
    let mut rng = Rng::new(0xEB);
    for i in 0..120 {
        // "State:<digits>:Mode:3" never contains any of these
        let absent: [i8; 8] = [
            b'z' as i8, b'Q' as i8, b' ' as i8, b'~' as i8, b'!' as i8, b'\t' as i8,
            b'/' as i8, b'@' as i8,
        ];
        let init = rng.interesting_i32();
        let out = diff_and_get(&format!("E11 #{i} init={init}"), &move |lib, log| unsafe {
            let s = (lib.create_state)(init, 128);
            for t in absent {
                log.push(format!("pb({t}) -> {}", (lib.process_buffer)(s, t as c_char)));
            }
            (lib.destroy_state)(s);
        });
        for l in &out.log {
            assert!(l.ends_with("-> 0"), "E11: {l}");
        }
        assert!(out.stdout.is_empty(), "E11 printed {:?}", show(&out.stdout));
    }
}

// ===========================================================================
// E12 — target == '\0' is outside remaining = strlen(buf)
// ===========================================================================
#[test]
fn err_e12_process_buffer_nul_target() {
    let mut rng = Rng::new(0xEC);
    for i in 0..80 {
        let init = rng.interesting_i32();
        let cap = 1 + rng.below(140) as i32;
        let out = diff_and_get(
            &format!("E12 #{i} init={init} cap={cap}"),
            &move |lib, log| unsafe {
                let s = (lib.create_state)(init, cap);
                log.push(format!("pb(0) -> {}", (lib.process_buffer)(s, 0)));
                (lib.destroy_state)(s);
            },
        );
        assert_eq!(out.log, vec!["pb(0) -> 0".to_string()]);
        assert!(out.stdout.is_empty(), "E12 printed {:?}", show(&out.stdout));
    }
}

// ===========================================================================
// E13 — target with the sign bit set; memchr compares (unsigned char)c
// ===========================================================================
#[test]
fn err_e13_process_buffer_negative_target() {
    let mut rng = Rng::new(0xED);
    for t in -128i8..=-1 {
        let init = rng.interesting_i32();
        let out = diff_and_get(&format!("E13 t={t}"), &move |lib, log| unsafe {
            let s = (lib.create_state)(init, 128);
            log.push(format!("pb -> {}", (lib.process_buffer)(s, t as c_char)));
            (lib.destroy_state)(s);
        });
        assert_eq!(out.log, vec!["pb -> 0".to_string()], "E13 t={t}");
        assert!(out.stdout.is_empty());
    }
}

// ===========================================================================
// E14 — update_flags(NULL, param) returns silently (lib.c:127)
// ===========================================================================
#[test]
fn err_e14_update_flags_null_state() {
    let mut params: Vec<i32> = BOUNDARY_I32.to_vec();
    let mut rng = Rng::new(0xEE);
    for _ in 0..40 {
        params.push(rng.next_i32());
    }
    for p in params {
        let out = diff_and_get(&format!("E14 param={p}"), &move |lib, log| unsafe {
            (lib.update_flags)(std::ptr::null_mut(), p);
            log.push("returned".into());
        });
        assert_eq!(out.log, vec!["returned".to_string()]);
        assert!(
            out.stdout.is_empty(),
            "E14 must print nothing, printed {:?}",
            show(&out.stdout)
        );
    }
}

// ===========================================================================
// E15 — the 5-bit counter wraps: (counter + 1) & 0x1F (lib.c:131)
// ===========================================================================
#[test]
fn err_e15_counter_wrap() {
    for param in [0i32, 1, 7, 63, -1, i32::MIN, i32::MAX] {
        let out = diff_and_get(&format!("E15 param={param}"), &move |lib, log| unsafe {
            let s = (lib.create_state)(0, 128);
            for _ in 0..70 {
                (lib.update_flags)(s, param);
                log.push(format!("counter={}", snapshot(s).counter()));
            }
            (lib.destroy_state)(s);
        });
        // 1,2,...,31,0,1,...  -- never leaves 0..31
        for (k, l) in out.log.iter().enumerate() {
            let expected = ((k as u32) + 1) % 32;
            assert_eq!(l, &format!("counter={expected}"), "E15 step {k}");
        }
    }
}

// ===========================================================================
// E16 / E17 — mode is masked to 3 bits; param >> 3 is an arithmetic shift
// ===========================================================================
#[test]
fn err_e16_mode_mask() {
    let mut rng = Rng::new(0xF0);
    let mut params: Vec<i32> = (0..64).collect();
    params.extend([i32::MAX, i32::MAX - 1, 0x7FFF_FFF8, 0x0FFF_FFFF, 1 << 30]);
    for _ in 0..80 {
        params.push(rng.next_u32() as i32 & i32::MAX);
    }
    for p in params {
        let out = diff_and_get(&format!("E16 param={p}"), &move |lib, log| unsafe {
            let s = (lib.create_state)(0, 128);
            (lib.update_flags)(s, p);
            let snap = snapshot(s);
            log.push(format!("mode={} flags=0x{:08x}", snap.mode(), snap.flags));
            (lib.destroy_state)(s);
        });
        let expected_mode = ((p >> 3) & 0x7) as u32;
        assert!(
            out.log[0].starts_with(&format!("mode={expected_mode} ")),
            "E16 param={p}: {:?} (expected mode {expected_mode})",
            out.log[0]
        );
    }
}

#[test]
fn err_e17_negative_param_arithmetic_shift() {
    let mut params: Vec<i32> = (-64..0).collect();
    params.extend([i32::MIN, i32::MIN + 1, -1, -8, -9, -0x4000_0000]);
    for p in params {
        let out = diff_and_get(&format!("E17 param={p}"), &move |lib, log| unsafe {
            let s = (lib.create_state)(0, 128);
            (lib.update_flags)(s, p);
            let snap = snapshot(s);
            log.push(format!(
                "f1={} f2={} f3={} mode={}",
                snap.flag1(),
                snap.flag2(),
                snap.flag3(),
                snap.mode()
            ));
            (lib.destroy_state)(s);
        });
        // GCC emits `sar`, i.e. an arithmetic shift, for `param >> 3`.
        let expected = format!(
            "f1={} f2={} f3={} mode={}",
            p & 1,
            (p & 2) >> 1,
            (p & 4) >> 2,
            (p >> 3) & 0x7
        );
        assert_eq!(out.log[0], expected, "E17 param={p}");
    }
}

// ===========================================================================
// E18 — confuse_types(NULL, op) -> 0 with no output (lib.c:144)
// ===========================================================================
#[test]
fn err_e18_confuse_types_null_state() {
    let mut ops: Vec<i32> = vec![0, 1, 2, 3, 4, 5, -1, -2, -3, i32::MIN, i32::MAX];
    let mut rng = Rng::new(0xF2);
    for _ in 0..60 {
        ops.push(rng.next_i32());
    }
    for op in ops {
        let out = diff_and_get(&format!("E18 op={op}"), &move |lib, log| unsafe {
            log.push(format!(
                "ct -> {}",
                (lib.confuse_types)(std::ptr::null_mut(), op)
            ));
        });
        assert_eq!(out.log, vec!["ct -> 0".to_string()], "E18 op={op}");
        assert!(
            out.stdout.is_empty(),
            "E18 op={op} printed {:?}",
            show(&out.stdout)
        );
    }
}

// ===========================================================================
// E19 — `operation` outside 0..3: the switch has no default, so the function
// prints nothing and returns 0. C enums accept any int, so these ARE valid
// inputs across the FFI boundary.
// ===========================================================================
#[test]
fn err_e19_confuse_types_out_of_range_operation() {
    let mut ops: Vec<i32> = vec![
        4,
        5,
        6,
        7,
        8,
        100,
        -1,
        -2,
        -3,
        -4,
        -100,
        i32::MAX,
        i32::MAX - 1,
        i32::MIN,
        i32::MIN + 1,
        0x8000_0000u32 as i32,
        1 << 30,
    ];
    let mut rng = Rng::new(0xF3);
    while ops.len() < 200 {
        let v = rng.next_i32();
        if !(0..=3).contains(&v) {
            ops.push(v);
        }
    }
    for op in ops {
        let init = 1078530011;
        let out = diff_and_get(&format!("E19 op={op}"), &move |lib, log| unsafe {
            let s = (lib.create_state)(init, 128);
            let before = snapshot(s);
            log.push(format!("ct -> {}", (lib.confuse_types)(s, op)));
            let after = snapshot(s);
            log.push(format!(
                "unchanged={} data=0x{:08x}",
                before.data == after.data && before.flags == after.flags,
                after.data
            ));
            (lib.destroy_state)(s);
        });
        assert_eq!(out.log[0], "ct -> 0", "E19 op={op}");
        assert_eq!(
            out.log[1], "unchanged=true data=0x40490fdb",
            "E19 op={op} mutated the state"
        );
        assert!(
            out.stdout.is_empty(),
            "E19 op={op} must print nothing, printed {:?}",
            show(&out.stdout)
        );
    }
}

// ===========================================================================
// E20 — operation 2 clamps with & 0xFF (lib.c:163)
// ===========================================================================
#[test]
fn err_e20_confuse_types_uint_mask() {
    let mut inits: Vec<i32> = BOUNDARY_I32.to_vec();
    inits.extend(FLOAT_BITS);
    let mut rng = Rng::new(0xF4);
    for _ in 0..300 {
        inits.push(rng.next_i32());
    }
    for init in inits {
        let out = diff_and_get(&format!("E20 init={init}"), &move |lib, log| unsafe {
            let s = (lib.create_state)(init, 128);
            log.push(format!("ct2 -> {}", (lib.confuse_types)(s, 2)));
            (lib.destroy_state)(s);
        });
        let expected = (init as u32 & 0xFF) as i32;
        assert_eq!(out.log[0], format!("ct2 -> {expected}"), "E20 init={init}");
        assert_eq!(
            out.stdout,
            format!("Read as uint: {}\n", init as u32).into_bytes(),
            "E20 init={init}"
        );
    }
}

// ===========================================================================
// E21 — operation 1 with float_val * 100 NaN / Inf / out of int32 range
// -> x86-64 cvttss2si "integer indefinite" == INT_MIN
// ===========================================================================
#[test]
fn err_e21_confuse_types_float_out_of_range() {
    // Bit patterns whose float value times 100 is NaN, +/-Inf, or |x| >= 2^31.
    let mut inits: Vec<i32> = vec![
        0x7F80_0000u32 as i32, // +Inf
        0xFF80_0000u32 as i32, // -Inf
        0x7FC0_0000u32 as i32, // qNaN
        0xFFC0_0000u32 as i32, // -qNaN
        0x7F80_0001u32 as i32, // sNaN
        0x7F7F_FFFFu32 as i32, // FLT_MAX
        0xFF7F_FFFFu32 as i32, // -FLT_MAX
        0x4F00_0000u32 as i32, // 2^31 exactly
        0x4CBE_BC20u32 as i32, // 1e8
        0xCCBE_BC20u32 as i32, // -1e8
    ];
    let mut rng = Rng::new(0xF5);
    // plus randomly-chosen huge-magnitude patterns
    while inits.len() < 200 {
        let e = 0x50 + (rng.below(0x2E) as u32); // exponent byte -> very large
        let v = (e << 24) | (rng.next_u32() & 0x00FF_FFFF);
        inits.push(v as i32);
        inits.push((v | 0x8000_0000) as i32);
    }
    for init in inits {
        let f = f32::from_bits(init as u32);
        let prod = f * 100.0f32;
        let out_of_range = prod.is_nan() || !(-2147483648.0f32..2147483648.0f32).contains(&prod.trunc());
        let out = diff_and_get(
            &format!("E21 init=0x{:08x} f={f} prod={prod}", init as u32),
            &move |lib, log| unsafe {
                let s = (lib.create_state)(init, 128);
                log.push(format!("ct1 -> {}", (lib.confuse_types)(s, 1)));
                (lib.destroy_state)(s);
            },
        );
        if out_of_range {
            assert_eq!(
                out.log[0],
                format!("ct1 -> {}", i32::MIN),
                "E21 init=0x{:08x}: cvttss2si must yield INT_MIN",
                init as u32
            );
        }
    }
}

// ===========================================================================
// E22 — operation 3 sums two *signed* chars (lib.c:170)
// ===========================================================================
#[test]
fn err_e22_confuse_types_signed_bytes() {
    for b0 in [0u32, 1, 0x7F, 0x80, 0x81, 0xFE, 0xFF] {
        for b1 in [0u32, 1, 0x7F, 0x80, 0x81, 0xFE, 0xFF] {
            for hi in [0x0000u32, 0x8080, 0xFFFF, 0x7F7F] {
                let init = (b0 | (b1 << 8) | (hi << 16)) as i32;
                let out = diff_and_get(
                    &format!("E22 b0={b0:#04x} b1={b1:#04x} hi={hi:#06x}"),
                    &move |lib, log| unsafe {
                        let s = (lib.create_state)(init, 128);
                        log.push(format!("ct3 -> {}", (lib.confuse_types)(s, 3)));
                        (lib.destroy_state)(s);
                    },
                );
                let expected = (b0 as u8 as i8 as i32) + (b1 as u8 as i8 as i32);
                assert_eq!(out.log[0], format!("ct3 -> {expected}"));
                let bytes = (init as u32).to_le_bytes();
                assert_eq!(
                    out.stdout,
                    format!(
                        "Read as bytes: [{}, {}, {}, {}]\n",
                        bytes[0] as i8, bytes[1] as i8, bytes[2] as i8, bytes[3] as i8
                    )
                    .into_bytes()
                );
            }
        }
    }
}

// ===========================================================================
// E24 — confusion with param3 < 0: '0' + negative remainder
// ===========================================================================
#[test]
fn err_e24_confusion_negative_param3() {
    let mut rng = Rng::new(0xF7);
    let mut p3s: Vec<i32> = vec![-1, -2, -3, -9, -10, -11, -19, -99, i32::MIN, i32::MIN + 1];
    for _ in 0..120 {
        p3s.push(-(1 + rng.below(2_000_000_000) as i32));
    }
    for p3 in p3s {
        let (p1, p2, p4) = (1078530011, 42, 2);
        diff(&format!("E24 p3={p3}"), &move |lib, log| unsafe {
            log.push(format!("confusion -> {}", (lib.confusion)(p1, p2, p3, p4)));
        });
        // the search char really is below '0' unless p3 % 10 == 0
        let ch = (b'0' as i32) + (p3 % 10);
        assert!((39..=57).contains(&ch), "E24 p3={p3} -> char {ch}");
    }
}

// ===========================================================================
// E25 — confusion with param4 < 0: operation is negative -> fall-through
// ===========================================================================
#[test]
fn err_e25_confusion_negative_param4() {
    let mut rng = Rng::new(0xF8);
    let mut p4s: Vec<i32> = vec![-1, -2, -3, -4, -5, -7, -8, i32::MIN, i32::MIN + 1, i32::MIN + 3];
    for _ in 0..120 {
        p4s.push(-(1 + rng.below(2_000_000_000) as i32));
    }
    for p4 in p4s {
        let out = diff_and_get(&format!("E25 p4={p4}"), &move |lib, log| unsafe {
            log.push(format!("confusion -> {}", (lib.confusion)(7, 0, 0, p4)));
        });
        let text = String::from_utf8_lossy(&out.stdout).to_string();
        if p4 % 4 != 0 {
            // negative operation: no `Set as int` / `Read as ...` line at all
            for marker in ["Set as int", "Read as float", "Read as uint", "Read as bytes"] {
                assert!(
                    !text.contains(marker),
                    "E25 p4={p4} unexpectedly ran case {marker}: {text:?}"
                );
            }
        }
    }
}

// ===========================================================================
// E26 — the additions in `confusion` are plain signed `int` adds.
//
// The largest value confuse_types can return is bounded by the f32 grid: the
// biggest float below 2^31 is 2147483520 == INT_MAX - 127, and the other three
// addends (found*10 <= 100, counter*5 == 5, mode*3 <= 21) cannot make up the
// remaining 127. So `result` cannot actually overflow through `confusion`.
// This test drives the *reachable extreme* -- both ends of the range -- and
// additionally checks the wrapping addition directly through the low-level API.
// ===========================================================================
#[test]
fn err_e26_confusion_result_overflow() {
    // 1. Bit patterns whose float*100 lands just below INT_MAX.
    let mut extreme: Vec<i32> = Vec::new();
    for bits in 0x4BA3_D000u32..0x4BA3_E000 {
        let prod = f32::from_bits(bits) * 100.0f32;
        if prod.is_finite() && (2_147_000_000.0..2_147_483_648.0).contains(&prod) {
            extreme.push(bits as i32);
        }
    }
    assert!(
        !extreme.is_empty(),
        "E26: could not find a near-INT_MAX float pattern"
    );
    let mut rng = Rng::new(0xF9);
    for _ in 0..40 {
        let p1 = *rng.pick(&extreme);
        for p2 in [0i32, 63, 56, -1] {
            for p3 in [0i32, 1, 3, 7, 9, -9] {
                diff(
                    &format!("E26a p1=0x{:08x} p2={p2} p3={p3}", p1 as u32),
                    &move |lib, log| unsafe {
                        log.push(format!("confusion -> {}", (lib.confusion)(p1, p2, p3, 1)));
                    },
                );
            }
        }
    }

    // 2. The other end: confuse_types returns INT_MIN, then positive addends.
    for p1 in [0x4F00_0000u32 as i32, 0x7F80_0000u32 as i32, 0xFFC0_0000u32 as i32] {
        for p2 in [0i32, 63, -1] {
            for p3 in [0i32, 3, -3] {
                let out = diff_and_get(
                    &format!("E26b p1=0x{:08x} p2={p2} p3={p3}", p1 as u32),
                    &move |lib, log| unsafe {
                        log.push(format!("confusion -> {}", (lib.confusion)(p1, p2, p3, 1)));
                    },
                );
                let text = String::from_utf8_lossy(&out.stdout).to_string();
                assert!(text.contains("Read as float:"), "{text:?}");
            }
        }
    }

    // 3. Directly through the low-level API, where counter can reach 31, so the
    //    positive addends are as large as the library can ever make them.
    for p1 in extreme.iter().take(8).copied() {
        diff(
            &format!("E26c p1=0x{:08x}", p1 as u32),
            &move |lib, log| unsafe {
                let s = (lib.create_state)(p1, 128);
                for _ in 0..31 {
                    (lib.update_flags)(s, 63);
                }
                let snap = snapshot(s);
                let found = (lib.process_buffer)(s, b'0' as c_char);
                let cr = (lib.confuse_types)(s, 1);
                let result = found
                    .wrapping_mul(10)
                    .wrapping_add(cr)
                    .wrapping_add((snap.counter() as i32).wrapping_mul(5))
                    .wrapping_add((snap.mode() as i32).wrapping_mul(3));
                log.push(format!(
                    "counter={} mode={} found={found} cr={cr} result={result}",
                    snap.counter(),
                    snap.mode()
                ));
                (lib.destroy_state)(s);
            },
        );
    }
}

// ===========================================================================
// Generic FFI-boundary cases that every C API has
// ===========================================================================

/// NULL pointer into every pointer-taking entry point, with extreme scalars.
#[test]
fn generic_null_pointer_every_entry_point() {
    let scalars: [i32; 9] = [0, 1, -1, 2, 3, 4, -4, i32::MAX, i32::MIN];
    for v in scalars {
        let out = diff_and_get(&format!("null-all v={v}"), &move |lib, log| unsafe {
            let n = std::ptr::null_mut();
            (lib.destroy_state)(n);
            log.push(format!("pb -> {}", (lib.process_buffer)(n, v as i8 as c_char)));
            (lib.update_flags)(n, v);
            log.push(format!("ct -> {}", (lib.confuse_types)(n, v)));
        });
        assert_eq!(out.log, vec!["pb -> -1".to_string(), "ct -> 0".to_string()]);
        assert_eq!(out.stdout, b"Error: Null pointer in process_buffer\n");
    }
}

/// Zero and oversized lengths for the only length-like parameter (`capacity`).
#[test]
fn generic_zero_and_oversized_lengths() {
    // 0 and negative are deterministic; see E2/E4.
    for cap in [0i32, -1, i32::MIN] {
        diff(&format!("len cap={cap}"), &move |lib, log| unsafe {
            let s = (lib.create_state)(999, cap);
            log.push(format!("null={}", s.is_null()));
            if !s.is_null() {
                log_state_no_buffer(log, "s", s);
                (lib.destroy_state)(s);
            }
        });
    }
    // The largest `int` capacity the ABI can express.
    for cap in [i32::MAX, i32::MAX - 1, 0x7FFF_FF00] {
        diff(&format!("len cap={cap}"), &move |lib, log| unsafe {
            let s = (lib.create_state)(-424242, cap);
            log.push(format!("null={}", s.is_null()));
            if !s.is_null() {
                let snap = snapshot(s);
                log.push(format!(
                    "flags=0x{:08x} cap={} buf={:?}",
                    snap.flags, snap.capacity, snap.buffer
                ));
                (lib.destroy_state)(s);
            }
        });
    }
}

/// One step past every documented valid range.
#[test]
fn generic_one_past_valid_ranges() {
    // confuse_types operations: 0..3 valid, so -1 and 4 are one step past.
    for op in [-1i32, 4] {
        let out = diff_and_get(&format!("past op={op}"), &move |lib, log| unsafe {
            let s = (lib.create_state)(1078530011, 128);
            log.push(format!("ct -> {}", (lib.confuse_types)(s, op)));
            (lib.destroy_state)(s);
        });
        assert_eq!(out.log, vec!["ct -> 0".to_string()]);
        assert!(out.stdout.is_empty());
    }
    // mode is a 3-bit field: param>>3 == 8 is one past its range.
    for param in [8i32 << 3, (7i32 << 3) + 7, -1] {
        diff(&format!("past param={param}"), &move |lib, log| unsafe {
            let s = (lib.create_state)(0, 128);
            (lib.update_flags)(s, param);
            log.push(format!("mode={}", snapshot(s).mode()));
            (lib.destroy_state)(s);
        });
    }
    // counter is a 5-bit field: the 32nd increment is one past its range (E15).
    // capacity: 0 and 1 straddle "can hold anything at all".
    for cap in [0i32, 1, 2] {
        diff(&format!("past cap={cap}"), &move |lib, log| unsafe {
            let s = (lib.create_state)(7, cap);
            log_state_no_buffer(log, "s", s);
            (lib.destroy_state)(s);
        });
    }
    // process_buffer target: full domain plus the values just outside `char`
    // that a C caller can still pass (they are truncated by the ABI).
    for t in [-129i32, -128, -1, 0, 1, 127, 128, 255, 256] {
        diff(&format!("past target={t}"), &move |lib, log| unsafe {
            let s = (lib.create_state)(1234567890, 128);
            log.push(format!("pb -> {}", (lib.process_buffer)(s, t as i8 as c_char)));
            (lib.destroy_state)(s);
        });
    }
}

/// Out-of-range "enum" values swept broadly — the class of bug happy-path
/// tests miss. `operation` is an `int` in C, so every one of these is legal.
#[test]
fn generic_out_of_range_enum_sweep() {
    for op in -600i32..=600 {
        let out = diff_and_get(&format!("enum op={op}"), &move |lib, log| unsafe {
            let s = (lib.create_state)(0x4B18_9680u32 as i32, 128);
            log.push(format!("ct -> {}", (lib.confuse_types)(s, op)));
            (lib.destroy_state)(s);
        });
        if !(0..=3).contains(&op) {
            assert_eq!(out.log, vec!["ct -> 0".to_string()], "enum op={op}");
            assert!(out.stdout.is_empty(), "enum op={op}");
        }
    }
    // and the extremes of the int domain
    let mut rng = Rng::new(0xFB);
    for _ in 0..200 {
        let op = match rng.below(3) {
            0 => i32::MIN + rng.below(50) as i32,
            1 => i32::MAX - rng.below(50) as i32,
            _ => rng.next_i32(),
        };
        diff(&format!("enum-x op={op}"), &move |lib, log| unsafe {
            let s = (lib.create_state)(0x4B18_9680u32 as i32, 128);
            log.push(format!("ct -> {}", (lib.confuse_types)(s, op)));
            (lib.destroy_state)(s);
        });
    }
}
