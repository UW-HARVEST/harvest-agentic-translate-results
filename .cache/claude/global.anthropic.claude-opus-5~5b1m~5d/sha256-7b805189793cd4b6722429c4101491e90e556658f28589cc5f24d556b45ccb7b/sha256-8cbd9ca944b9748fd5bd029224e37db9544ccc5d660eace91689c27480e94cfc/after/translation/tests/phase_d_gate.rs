//! Phase D — symbol parity, harness self-validation, and a deep randomized
//! fuzz pass over every entry point at once.

mod common;
use common::*;
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn nm_defined(so: &PathBuf) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", so.to_str().unwrap()])
        .output()
        .expect("nm must be available");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2).map(str::to_string))
        .collect()
}

fn c_so() -> PathBuf {
    let build = workspace_root().join("c_src/build");
    std::fs::read_dir(&build)
        .expect("c_src/build must exist — build the C library first")
        .flatten()
        .map(|e| e.path())
        .find(|p| {
            p.extension().map(|x| x == "so").unwrap_or(false)
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("lib"))
                    .unwrap_or(false)
        })
        .expect("no lib*.so in c_src/build")
}

fn rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("GJK_RUST_SO") {
        return PathBuf::from(p);
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for c in [
        root.join(if cfg!(debug_assertions) {
            "target/debug/libgjk_lib.so"
        } else {
            "target/release/libgjk_lib.so"
        }),
        root.join("target/release/libgjk_lib.so"),
        root.join("target/debug/libgjk_lib.so"),
    ] {
        if c.exists() {
            return c;
        }
    }
    panic!("libgjk_lib.so not found");
}

/// Gate item 1: every symbol the C `.so` exports, the Rust `.so` must export
/// with the exact same name; and the Rust `.so` must have no missing non-libc
/// undefined symbols.
#[test]
fn gate1_symbol_parity() {
    let cs = nm_defined(&c_so());
    let rs = nm_defined(&rust_so());
    assert!(
        cs.len() >= 31,
        "expected >=31 C symbols, found {}: {cs:?}",
        cs.len()
    );
    let missing: Vec<_> = cs.difference(&rs).cloned().collect();
    assert!(
        missing.is_empty(),
        "{} C symbol(s) MISSING from the Rust .so: {missing:?}",
        missing.len()
    );
    // Every documented symbol must be present in both.
    for name in [
        "c2V",
        "c2Mulvs",
        "c2Maxv",
        "c2Minv",
        "c2Clampv",
        "c2Sub",
        "c2Add",
        "c2Dot",
        "c2Det2",
        "c2Len",
        "c2Div",
        "c2Norm",
        "c2Neg",
        "c2Skew",
        "c2CCW90",
        "c2RotIdentity",
        "c2xIdentity",
        "c2Mulrv",
        "c2MulrvT",
        "c2Mulxv",
        "c2BBVerts",
        "c2MakeProxy",
        "c2GJKSimplexMetric",
        "c22",
        "c23",
        "c2D",
        "c2L",
        "c2Support",
        "c2Witness",
        "c2GJK",
        "gjk",
    ] {
        assert!(cs.contains(name), "C .so is missing {name}");
        assert!(rs.contains(name), "Rust .so is missing {name}");
    }
    // No missing non-libc undefined symbols in the Rust .so.
    let out = Command::new("nm")
        .args(["-D", "--undefined-only", rust_so().to_str().unwrap()])
        .output()
        .unwrap();
    let allowed_prefixes = [
        "_ITM_", "_Unwind_", "__cxa_", "__errno", "__gmon_start__", "__tls_get_addr",
    ];
    let libc: BTreeSet<&str> = [
        "abort", "bcmp", "calloc", "close", "dl_iterate_phdr", "free", "fstat", "fstat64",
        "getcwd", "getenv", "gettid", "lseek", "lseek64", "malloc", "memcmp", "memcpy",
        "memmove", "memset", "mmap", "mmap64", "munmap", "open", "open64", "posix_memalign",
        "pthread_key_create", "pthread_key_delete", "pthread_getspecific",
        "pthread_setspecific", "read", "readlink", "realloc", "realpath", "sigaltstack",
        "stat", "stat64", "statx", "strlen", "sysconf", "syscall", "write", "writev",
        "sqrtf", "sqrt", "__libc_start_main",
    ]
    .into_iter()
    .collect();
    let mut unexpected = Vec::new();
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        let Some(sym) = line.split_whitespace().nth(1) else {
            continue;
        };
        let base = sym.split('@').next().unwrap();
        if allowed_prefixes.iter().any(|p| base.starts_with(p)) || libc.contains(base) {
            continue;
        }
        unexpected.push(base.to_string());
    }
    assert!(
        unexpected.is_empty(),
        "Rust .so has unresolved non-libc symbols: {unexpected:?}"
    );
}

/// Harness self-validation (negative control): prove the differential harness
/// really is loading TWO distinct libraries and that its comparison helpers
/// actually fail on a difference. Without this, an "all green" run could mean
/// the harness compares a library against itself.
#[test]
fn gate2_harness_is_a_real_negative_control() {
    let p = api();
    // 1. The two `.so`s are distinct files at distinct load addresses.
    assert_ne!(
        c_so().canonicalize().unwrap(),
        rust_so().canonicalize().unwrap(),
        "the harness must load two different .so files"
    );
    let ca = p.c.c2Dot as usize;
    let ra = p.r.c2Dot as usize;
    assert_ne!(
        ca, ra,
        "c2Dot resolved to the same address in both handles — the harness is \
         comparing one library against itself"
    );
    for (n, a, b) in [
        ("c2GJK", p.c.c2GJK as usize, p.r.c2GJK as usize),
        ("gjk", p.c.gjk as usize, p.r.gjk as usize),
        ("c23", p.c.c23 as usize, p.r.c23 as usize),
        ("c2Witness", p.c.c2Witness as usize, p.r.c2Witness as usize),
    ] {
        assert_ne!(a, b, "{n} resolved to the same address in both handles");
    }

    // 2. The comparison helpers detect a one-bit difference.
    let x = 1.0f32;
    let y = f32::from_bits(x.to_bits() ^ 1);
    assert!(
        std::panic::catch_unwind(|| eq_f32("selfcheck", "", x, y)).is_err(),
        "eq_f32 failed to detect a 1-bit difference"
    );
    // +0.0 vs -0.0 compare equal under `==` but must NOT under eq_f32.
    assert!(
        std::panic::catch_unwind(|| eq_f32("selfcheck", "", 0.0, -0.0)).is_err(),
        "eq_f32 failed to distinguish +0.0 from -0.0"
    );
    // Two NaNs with different payloads must be distinguished.
    assert!(
        std::panic::catch_unwind(|| eq_f32(
            "selfcheck",
            "",
            f32::from_bits(0x7FC0_0001),
            f32::from_bits(0x7FC0_0002)
        ))
        .is_err(),
        "eq_f32 failed to distinguish two NaN payloads"
    );
    // eq_bits must detect a difference in any byte of a big struct.
    let s0 = c2Simplex::default();
    let mut s1 = s0;
    s1.verts[3].iB = 1;
    assert!(
        std::panic::catch_unwind(move || eq_bits("selfcheck", "", &s0, &s1)).is_err(),
        "eq_bits failed to detect a difference in the last simplex slot"
    );
    let c0 = c2GJKCache::default();
    let mut c1 = c0;
    c1.div = f32::from_bits(1);
    assert!(
        std::panic::catch_unwind(move || eq_bits("selfcheck", "", &c0, &c1)).is_err(),
        "eq_bits failed to detect a difference in c2GJKCache::div"
    );
}

/// A deep randomized pass that hammers every entry point in one loop with a
/// fixed seed — the catch-all for interactions the per-row tests partition.
#[test]
fn gate3_deep_fuzz_all_entry_points() {
    let p = api();
    let mut rng = Rng::new(0xD00D);

    for i in 0..60_000usize {
        // --- leaf helpers, wild values ---
        let (a, b) = (rng.wild_vec(), rng.wild_vec());
        let sc = rng.wild_f32();
        let ctx = format!("fuzz i={i}");
        eq_v("f c2V", &ctx, unsafe { (p.c.c2V)(a.x, a.y) }, unsafe {
            (p.r.c2V)(a.x, a.y)
        });
        eq_v("f c2Add", &ctx, unsafe { (p.c.c2Add)(a, b) }, unsafe {
            (p.r.c2Add)(a, b)
        });
        eq_v("f c2Sub", &ctx, unsafe { (p.c.c2Sub)(a, b) }, unsafe {
            (p.r.c2Sub)(a, b)
        });
        eq_v("f c2Mulvs", &ctx, unsafe { (p.c.c2Mulvs)(a, sc) }, unsafe {
            (p.r.c2Mulvs)(a, sc)
        });
        eq_v("f c2Div", &ctx, unsafe { (p.c.c2Div)(a, sc) }, unsafe {
            (p.r.c2Div)(a, sc)
        });
        eq_v("f c2Norm", &ctx, unsafe { (p.c.c2Norm)(a) }, unsafe {
            (p.r.c2Norm)(a)
        });
        eq_v("f c2Neg", &ctx, unsafe { (p.c.c2Neg)(a) }, unsafe {
            (p.r.c2Neg)(a)
        });
        eq_v("f c2Skew", &ctx, unsafe { (p.c.c2Skew)(a) }, unsafe {
            (p.r.c2Skew)(a)
        });
        eq_v("f c2CCW90", &ctx, unsafe { (p.c.c2CCW90)(a) }, unsafe {
            (p.r.c2CCW90)(a)
        });
        eq_v("f c2Maxv", &ctx, unsafe { (p.c.c2Maxv)(a, b) }, unsafe {
            (p.r.c2Maxv)(a, b)
        });
        eq_v("f c2Minv", &ctx, unsafe { (p.c.c2Minv)(a, b) }, unsafe {
            (p.r.c2Minv)(a, b)
        });
        let hi = rng.wild_vec();
        eq_v(
            "f c2Clampv",
            &ctx,
            unsafe { (p.c.c2Clampv)(a, b, hi) },
            unsafe { (p.r.c2Clampv)(a, b, hi) },
        );
        eq_f32("f c2Dot", &ctx, unsafe { (p.c.c2Dot)(a, b) }, unsafe {
            (p.r.c2Dot)(a, b)
        });
        eq_f32("f c2Det2", &ctx, unsafe { (p.c.c2Det2)(a, b) }, unsafe {
            (p.r.c2Det2)(a, b)
        });
        eq_f32("f c2Len", &ctx, unsafe { (p.c.c2Len)(a) }, unsafe {
            (p.r.c2Len)(a)
        });
        let r = rng.rot_wild();
        eq_v("f c2Mulrv", &ctx, unsafe { (p.c.c2Mulrv)(r, a) }, unsafe {
            (p.r.c2Mulrv)(r, a)
        });
        eq_v("f c2MulrvT", &ctx, unsafe { (p.c.c2MulrvT)(r, a) }, unsafe {
            (p.r.c2MulrvT)(r, a)
        });
        let x = rng.xform_wild();
        eq_v("f c2Mulxv", &ctx, unsafe { (p.c.c2Mulxv)(x, a) }, unsafe {
            (p.r.c2Mulxv)(x, a)
        });

        // --- c2BBVerts ---
        {
            let bb = c2AABB { min: a, max: b };
            let (mut cb, mut rb) = ([ZV; 4], [ZV; 4]);
            let (mut c1, mut r1) = (bb, bb);
            unsafe {
                (p.c.c2BBVerts)(cb.as_mut_ptr(), &mut c1);
                (p.r.c2BBVerts)(rb.as_mut_ptr(), &mut r1);
            }
            eq_bits("f c2BBVerts", &ctx, &cb, &rb);
        }

        // --- c2Support at a random count ---
        {
            let n = 1 + rng.below(8);
            let mut verts = vec![ZV; n];
            for v in verts.iter_mut() {
                *v = if rng.bool() { rng.vec() } else { rng.wild_vec() };
            }
            let d = if rng.bool() { rng.vec() } else { rng.wild_vec() };
            let cn = n as i32;
            eq_i32(
                "f c2Support",
                &ctx,
                unsafe { (p.c.c2Support)(verts.as_ptr(), cn, d) },
                unsafe { (p.r.c2Support)(verts.as_ptr(), cn, d) },
            );
        }

        // --- simplex functions at a random (possibly invalid) count ---
        {
            let count = match rng.below(8) {
                0 => 0,
                1 => 4,
                2 => -1,
                3 => i32::MAX,
                4 => i32::MIN,
                n => (n - 4) as i32, // 1, 2, 3
            };
            let wild = rng.bool();
            let s = rand_simplex(&mut rng, count, wild);
            diff_metric(&ctx, &s);
            diff_c2D(&ctx, &s);
            diff_c2L(&ctx, &s);
            diff_witness(&ctx, &s);
            diff_c22(&ctx, &s);
            diff_c23(&ctx, &s);
        }

        // --- c2GJK with fully random options (a fraction of iterations, since
        //     it is the most expensive call) ---
        if i % 4 == 0 {
            let ka = KINDS[rng.below(3)];
            let kb = KINDS[rng.below(3)];
            let mk = |rng: &mut Rng, k: i32| -> Shape {
                if rng.below(6) == 0 {
                    // wild
                    match k {
                        C2_TYPE_CIRCLE => Shape::Circle(c2Circle {
                            p: rng.wild_vec(),
                            r: rng.wild_f32(),
                        }),
                        C2_TYPE_AABB => Shape::Aabb(c2AABB {
                            min: rng.wild_vec(),
                            max: rng.wild_vec(),
                        }),
                        _ => Shape::Capsule(c2Capsule {
                            a: rng.wild_vec(),
                            b: rng.wild_vec(),
                            r: rng.wild_f32(),
                        }),
                    }
                } else {
                    let at = c2v::new(rng.uniform(-8.0, 8.0), rng.uniform(-8.0, 8.0));
                    let sc = rng.uniform(0.0, 3.0);
                    rand_shape(rng, k, at, sc)
                }
            };
            let sa = mk(&mut rng, ka);
            let sb = mk(&mut rng, kb);
            let vc = |k: i32| match k {
                C2_TYPE_CIRCLE => 1usize,
                C2_TYPE_AABB => 4,
                _ => 2,
            };
            let cache = if rng.bool() {
                let cnt = 1 + rng.below(3) as i32;
                let mut c = c2GJKCache {
                    metric: if rng.below(4) == 0 {
                        rng.wild_f32()
                    } else {
                        rng.uniform(-1.0e9, 1.0e9)
                    },
                    count: if rng.below(8) == 0 { 0 } else { cnt },
                    iA: [0; 3],
                    iB: [0; 3],
                    div: if rng.below(4) == 0 {
                        rng.wild_f32()
                    } else {
                        rng.uniform(0.0, 5.0)
                    },
                };
                for k in 0..3 {
                    c.iA[k] = rng.below(vc(ka)) as i32;
                    c.iB[k] = rng.below(vc(kb)) as i32;
                }
                Some(c)
            } else {
                None
            };
            let o = GjkOpts {
                use_radius: if rng.bool() { 1 } else { 0 },
                ax: if rng.below(3) == 0 {
                    Some(rng.xform_wild())
                } else {
                    None
                },
                bx: if rng.below(3) == 0 {
                    Some(rng.xform_wild())
                } else {
                    None
                },
                want_a: rng.below(5) != 0,
                want_b: rng.below(5) != 0,
                want_iters: rng.below(5) != 0,
                cache,
            };
            diff_gjk(&ctx, &sa, &sb, &o);
        }

        // --- gjk wrapper ---
        {
            let mut f = [0.0f32; 9];
            for k in 0..9 {
                f[k] = if rng.below(6) == 0 {
                    rng.wild_f32()
                } else {
                    rng.coord()
                };
            }
            let rv = [0i8, 1, -1, 2, 0x7f, -128][rng.below(6)];
            diff_gjk_wrap(&ctx, rv, rng.below(6) != 0, rng.below(6) != 0, f);
        }
    }
}
