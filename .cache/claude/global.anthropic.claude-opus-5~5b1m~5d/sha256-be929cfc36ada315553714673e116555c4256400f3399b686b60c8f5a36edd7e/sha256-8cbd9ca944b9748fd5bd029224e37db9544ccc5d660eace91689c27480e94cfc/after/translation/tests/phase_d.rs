//! Phase D — symbol parity plus randomized cross-configuration fuzzing.
//!
//! The fuzz test draws a random *configuration* (element size, key size, hash
//! mode, string mode, seed) and then a random operation sequence, and requires
//! the C and the Rust `.so` to stay in lock-step for every observable.

mod common;

use common::driver::*;
use common::*;
use std::ffi::{c_int, c_void, CString};
use std::process::Command;

// --------------------------------------------------------------------------
// symbol parity
// --------------------------------------------------------------------------

fn nm_defined(path: &std::path::Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", path.to_str().unwrap()])
        .output()
        .expect("nm not available");
    assert!(out.status.success(), "nm failed on {}", path.display());
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2).map(|s| s.to_string()))
        .collect();
    v.sort();
    v.dedup();
    v
}

#[test]
fn d_01_symbol_parity() {
    let cs = nm_defined(&c_so_path());
    let rs = nm_defined(&rust_so_path());
    let missing: Vec<&String> = cs.iter().filter(|s| !rs.contains(s)).collect();
    let extra: Vec<&String> = rs.iter().filter(|s| !cs.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but missing from the Rust .so: {missing:?}"
    );
    assert!(
        extra.is_empty(),
        "symbols exported by the Rust .so but not by the C .so: {extra:?}"
    );
    assert_eq!(cs.len(), 16, "unexpected C symbol count: {cs:?}");
}

#[test]
fn d_02_rust_so_has_no_unexpected_undefined_symbols() {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only", rust_so_path().to_str().unwrap()])
        .output()
        .expect("nm");
    let allowed = [
        "realloc", "free", "memcpy", "memmove", "memset", "abort", "__assert_fail",
        "_ITM_deregisterTMCloneTable", "_ITM_registerTMCloneTable", "__cxa_finalize",
        "__gmon_start__", "__tls_get_addr", "bcmp", "memcmp", "__stack_chk_fail",
        "_Unwind_Resume", "_Unwind_Backtrace", "_Unwind_GetIP", "gettimeofday",
        "__errno_location", "sysconf", "getenv", "write", "syscall", "dl_iterate_phdr",
        "_Unwind_GetIPInfo", "_Unwind_FindEnclosingFunction", "_Unwind_GetLanguageSpecificData",
        "_Unwind_GetRegionStart", "_Unwind_GetTextRelBase", "_Unwind_GetDataRelBase",
        "_Unwind_SetIP", "_Unwind_SetGR", "_Unwind_GetCFA", "_Unwind_RaiseException",
        "_Unwind_DeleteException", "posix_memalign", "malloc", "calloc", "realpath",
        "strlen", "poll", "pthread_getattr_np", "pthread_attr_getstack",
        "pthread_attr_destroy", "pthread_self", "sigaltstack", "sigaction", "mmap",
        "munmap", "mprotect", "open64", "close", "read", "readlink", "getcwd",
    ];
    let mut unexpected = Vec::new();
    for l in String::from_utf8_lossy(&out.stdout).lines() {
        if let Some(name) = l.split_whitespace().last() {
            let base = name.split('@').next().unwrap();
            if !allowed.contains(&base) {
                unexpected.push(base.to_string());
            }
        }
    }
    // Anything left must at least not be a stbds_* symbol the library needs.
    let stbds: Vec<&String> = unexpected.iter().filter(|s| s.starts_with("stbds_") || *s == "intput" || *s == "strkey").collect();
    assert!(stbds.is_empty(), "Rust .so has undefined library symbols: {stbds:?}");
}

// --------------------------------------------------------------------------
// randomized cross-configuration fuzz
// --------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
struct Cfg {
    elemsize: usize,
    keysize: usize,
    mode: c_int,
    sh_mode: Option<c_int>,
    seed: usize,
    strings: bool,
}

fn draw_cfg(rng: &mut Rng) -> Cfg {
    let strings = rng.below(2) == 0;
    if strings {
        // string keys always occupy the first 8 bytes (a char *)
        let elemsize = [16usize, 24, 40][rng.below(3)];
        let sh_mode = match rng.below(4) {
            0 => None,      // implicit: hmput_key sets SH_DEFAULT
            1 => Some(1),   // SH_DEFAULT
            2 => Some(2),   // SH_STRDUP
            _ => Some(3),   // SH_ARENA
        };
        // NB: SH_NONE with string keys is covered by CONFIGS row 30; it stores
        // raw bytes rather than a pointer and needs its own key discipline.
        Cfg {
            elemsize,
            keysize: 8,
            mode: [1i32, 2, 1000, i32::MAX][rng.below(4)],
            sh_mode,
            seed: [0usize, 1, 0x31415926, usize::MAX][rng.below(4)],
            strings: true,
        }
    } else {
        let keysize = [1usize, 2, 4, 8, 16][rng.below(5)];
        let elemsize = keysize + [0usize, 1, 4, 8, 24][rng.below(5)];
        Cfg {
            elemsize,
            keysize,
            mode: [0i32, -1, i32::MIN][rng.below(3)],
            sh_mode: if rng.below(4) == 0 { Some(0) } else { None },
            seed: [0usize, 1, 0x31415926, usize::MAX][rng.below(4)],
            strings: false,
        }
    }
}

#[test]
fn d_03_fuzz_all_configurations() {
    let (c, r) = both();
    let mut rng = Rng::new(0xF0_0DF0_0D);
    for round in 0..1200 {
        let cfg = draw_cfg(&mut rng);
        if std::env::var("FUZZ_TRACE").is_ok() { eprintln!("ROUND {round} {cfg:?}"); }
        unsafe {
            seed_both(&c, &r, cfg.seed);
            let kind = if cfg.strings { KeyKind::Pointer } else { KeyKind::Inline };
            let label = format!("fuzz round {round} {cfg:?}");
            let mut p = match cfg.sh_mode {
                Some(m) => Pair::shmode(&c, &r, cfg.elemsize, cfg.keysize, cfg.mode, m, kind, &label),
                None => Pair::empty(&c, &r, cfg.elemsize, cfg.keysize, cfg.mode, kind, &label),
            };
            if rng.below(3) == 0 {
                p.put_default();
            }

            let nops = 40 + rng.below(160);
            let mut bin_keys: Vec<Vec<u8>> = Vec::new();
            // A stable key pool that outlives the whole round: SH_DEFAULT stores
            // the caller's pointer, so keys must never be dropped while live.
            let pool: Vec<CString> = (0..24).map(|_| rng.ascii_len(0, 12)).collect();
            let mut live: Vec<usize> = Vec::new();

            for op in 0..nops {
                let choice = rng.below(10);
                if std::env::var("FUZZ_TRACE2").is_ok() { eprintln!("  op {op} choice {choice}"); }
                if cfg.strings {
                    match choice {
                        0..=5 => {
                            let i = rng.below(pool.len());
                            let val = rng.bytes(cfg.elemsize - 8);
                            p.put_str(&pool[i], &val);
                            if !live.contains(&i) {
                                live.push(i);
                            }
                        }
                        6..=8 => {
                            let i = rng.below(pool.len());
                            p.get_str(&pool[i]);
                        }
                        _ => {
                            // hmdel with mode != 1 can legitimately abort in the C
                            // (ERRORS row 44), so restrict deletes to mode == 1.
                            if cfg.mode == 1 {
                                let i = if live.is_empty() {
                                    rng.below(pool.len())
                                } else {
                                    let j = rng.below(live.len());
                                    live.remove(j)
                                };
                                p.del_str(&pool[i]);
                            }
                        }
                    }
                } else {
                    match choice {
                        0..=5 => {
                            let k = rng.bytes(cfg.keysize);
                            let val = rng.bytes(cfg.elemsize - cfg.keysize);
                            p.put_bin(&k, &val);
                            bin_keys.push(k);
                        }
                        6..=8 => {
                            let k = if !bin_keys.is_empty() && rng.below(2) == 0 {
                                bin_keys[rng.below(bin_keys.len())].clone()
                            } else {
                                rng.bytes(cfg.keysize)
                            };
                            p.get_bin(&k);
                        }
                        _ => {
                            let k = if !bin_keys.is_empty() && rng.below(2) == 0 {
                                bin_keys.remove(rng.below(bin_keys.len()))
                            } else {
                                rng.bytes(cfg.keysize)
                            };
                            p.del_bin(&k, 0);
                        }
                    }
                }
                if op % 7 == 0 {
                    p.check(&format!("op {op}"));
                }
            }
            p.check("end of round");
            p.free();
        }
    }
}

#[test]
fn d_04_fuzz_hashes_and_arena() {
    let (c, r) = both();
    let mut rng = Rng::new(0xA11CE);
    unsafe {
        for _ in 0..4000 {
            let len = rng.below(200);
            let mut buf = rng.bytes(len.max(1));
            let seed = rng.next_u64() as usize;
            assert_eq!(
                (c.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, seed),
                (r.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, seed),
                "hash_bytes len={len} seed={seed:#x}"
            );
            let s = rng.high_bytes_string(len);
            assert_eq!(
                (c.hash_string)(s.as_ptr() as *mut i8, seed),
                (r.hash_string)(s.as_ptr() as *mut i8, seed),
                "hash_string len={len} seed={seed:#x}"
            );
        }
        // arena fuzz
        for _ in 0..40 {
            let mut ca = StringArena { storage: std::ptr::null_mut(), remaining: 0, block: 0, mode: 0 };
            let mut ra = ca;
            for step in 0..200 {
                let len = match rng.below(10) {
                    0 => 600 + rng.below(4000),
                    1 => 0,
                    _ => rng.below(64),
                };
                let s = rng.ascii(len);
                let pc = (c.stralloc)(&mut ca as *mut _ as *mut c_void, s.as_ptr() as *mut i8);
                let pr = (r.stralloc)(&mut ra as *mut _ as *mut c_void, s.as_ptr() as *mut i8);
                assert_eq!(cstr(pc), cstr(pr), "arena fuzz step {step} content");
                assert_same("arena fuzz", &snapshot_arena(&ca), &snapshot_arena(&ra));
            }
            (c.strreset)(&mut ca as *mut _ as *mut c_void);
            (r.strreset)(&mut ra as *mut _ as *mut c_void);
            assert_same("arena fuzz reset", &snapshot_arena(&ca), &snapshot_arena(&ra));
        }
    }
}

#[test]
fn d_05_fuzz_strkey_and_intput() {
    let (c, r) = both();
    let mut rng = Rng::new(0xBEEF);
    unsafe {
        for _ in 0..5000 {
            let n = rng.next_u32() as c_int;
            assert_eq!(cstr((c.strkey)(n)), cstr((r.strkey)(n)), "strkey({n})");
        }
        for _ in 0..300 {
            let n = rng.next_u32() as c_int;
            if n == 9 || n == 11 {
                continue;
            }
            let seed = rng.next_u64() as usize;
            seed_both(&c, &r, seed);
            (c.intput)(n);
            (r.intput)(n);
        }
    }
}
