//! Phase D — symbol parity and ABI/layout parity, asserted inside the test
//! suite so it is re-checked on every run and under every feature combination.

mod common;
use common::*;
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;

fn so_paths() -> (PathBuf, PathBuf) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let cdir = root.join("c_src/build");
    let mut cs: Vec<PathBuf> = std::fs::read_dir(&cdir)
        .expect("c_src/build — build the C library first")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    cs.sort();
    let c = cs.pop().expect("no lib*.so in c_src/build");

    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
    let r = ["release", "debug"]
        .iter()
        .map(|p| base.join(p).join("libgjk_lib.so"))
        .find(|p| p.exists())
        .expect("libgjk_lib.so — run `cargo build --release` first");
    (c, r)
}

/// `nm -D --defined-only` restricted to code/data symbols.
fn defined_symbols(so: &PathBuf) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", so.to_str().unwrap()])
        .output()
        .expect("nm not available");
    assert!(out.status.success(), "nm failed on {}", so.display());
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let f: Vec<&str> = l.split_whitespace().collect();
            if f.len() >= 3 && matches!(f[1], "T" | "t" | "D" | "B" | "R") {
                Some(f[2].to_string())
            } else {
                None
            }
        })
        .collect()
}

fn undefined_symbols(so: &PathBuf) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only", so.to_str().unwrap()])
        .output()
        .expect("nm not available");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .collect()
}

#[test]
fn phase_d_symbol_parity() {
    let (cso, rso) = so_paths();
    let cs = defined_symbols(&cso);
    let rs = defined_symbols(&rso);

    let missing: Vec<&String> = cs.difference(&rs).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing {} symbol(s) exported by the C .so: {:?}",
        missing.len(),
        missing
    );
    assert_eq!(
        cs.len(),
        31,
        "expected 31 exported C symbols, found {}: {:?}",
        cs.len(),
        cs
    );
    eprintln!("phase D: {} C symbols, 0 missing from Rust", cs.len());
}

/// Every symbol must actually be *callable* through `dlsym` from both
/// libraries — a name in `nm` is not proof the export wrapper works.
#[test]
fn phase_d_every_symbol_is_dlsym_resolvable() {
    let (cso, rso) = so_paths();
    let cs = defined_symbols(&cso);
    for so in [&cso, &rso] {
        let lib = unsafe { libloading::Library::new(so).expect("dlopen") };
        for name in &cs {
            let mut bytes = name.clone().into_bytes();
            bytes.push(0);
            let sym: Result<libloading::Symbol<*const ()>, _> = unsafe { lib.get(&bytes) };
            assert!(
                sym.is_ok(),
                "{}: symbol {name} is listed by nm but not resolvable via dlsym",
                so.display()
            );
        }
    }
    // and the harness itself resolved all 31 in both libraries (it panics
    // otherwise), which is the real end-to-end proof
    let _ = libs();
    eprintln!("phase D: all {} symbols dlsym-resolvable in both .so files", cs.len());
}

/// The Rust `.so` must not have picked up any non-libc undefined symbol.
#[test]
fn phase_d_no_unexpected_undefined_symbols() {
    let (_, rso) = so_paths();
    let und = undefined_symbols(&rso);
    let allowed_prefixes = [
        "_ITM_", "_Unwind_", "__cxa_", "__errno", "__gmon_", "__tls_get_addr",
        "__libc_", "__rust_", "_ZN",
    ];
    let libc_names = [
        "abort", "bcmp", "calloc", "close", "dl_iterate_phdr", "free", "fstat64",
        "getcwd", "getenv", "gettid", "lseek64", "malloc", "memcmp", "memcpy",
        "memmove", "memset", "mmap64", "munmap", "open64", "posix_memalign",
        "pthread_key_create", "pthread_key_delete", "pthread_getspecific",
        "pthread_setspecific", "read", "readlink", "realloc", "realpath", "sqrtf",
        "stat64", "statx", "strlen", "syscall", "write", "writev", "sysconf",
        "pthread_mutex_lock", "pthread_mutex_unlock", "pthread_self", "raise",
        "signal", "sigaction", "sigaltstack", "mprotect", "poll", "getrandom",
        "pthread_rwlock_rdlock", "pthread_rwlock_unlock",
    ];
    let mut unexpected = Vec::new();
    for u in &und {
        let bare = u.split('@').next().unwrap_or(u);
        if allowed_prefixes.iter().any(|p| bare.starts_with(p)) {
            continue;
        }
        if libc_names.contains(&bare) {
            continue;
        }
        unexpected.push(u.clone());
    }
    assert!(
        unexpected.is_empty(),
        "Rust .so has unexpected undefined (non-libc) symbols: {unexpected:?}"
    );
    eprintln!(
        "phase D: {} undefined symbols in Rust .so, all libc/unwind",
        und.len()
    );
}

/// The C struct layouts the FFI depends on, asserted against the harness
/// mirrors. A mismatch here would silently corrupt every other test.
#[test]
fn phase_d_struct_layout_parity() {
    use std::mem::{align_of, size_of};
    macro_rules! chk {
        ($t:ty, $sz:expr, $al:expr) => {
            assert_eq!(size_of::<$t>(), $sz, concat!("size_of ", stringify!($t)));
            assert_eq!(align_of::<$t>(), $al, concat!("align_of ", stringify!($t)));
        };
    }
    chk!(c2v, 8, 4);
    chk!(c2r, 8, 4);
    chk!(c2x, 16, 4);
    chk!(c2Circle, 12, 4);
    chk!(c2AABB, 16, 4);
    chk!(c2Capsule, 20, 4);
    chk!(c2GJKCache, 36, 4);
    chk!(c2Proxy, 72, 4);
    chk!(c2sv, 36, 4);
    chk!(c2Simplex, 152, 4);

    // `c2Simplex` models `c2sv a, b, c, d` as an array; div/count must follow it
    let s = c2Simplex::default();
    let base = &s as *const c2Simplex as usize;
    assert_eq!(&s.verts[0] as *const c2sv as usize - base, 0, "verts offset");
    assert_eq!(&s.verts[1] as *const c2sv as usize - base, 36, "b offset");
    assert_eq!(&s.verts[2] as *const c2sv as usize - base, 72, "c offset");
    assert_eq!(&s.verts[3] as *const c2sv as usize - base, 108, "d offset");
    assert_eq!(&s.div as *const f32 as usize - base, 144, "div offset");
    assert_eq!(
        &s.count as *const std::ffi::c_int as usize - base,
        148,
        "count offset"
    );

    // `c2GJKCache` field offsets, which the forged-cache tests rely on
    let k = c2GJKCache::default();
    let kb = &k as *const c2GJKCache as usize;
    assert_eq!(&k.metric as *const f32 as usize - kb, 0);
    assert_eq!(&k.count as *const std::ffi::c_int as usize - kb, 4);
    assert_eq!(&k.iA as *const _ as usize - kb, 8);
    assert_eq!(&k.iB as *const _ as usize - kb, 20);
    assert_eq!(&k.div as *const f32 as usize - kb, 32);
}

/// The C source spells `FLT_MAX` and `FLT_EPSILON` out as decimal literals;
/// verify the Rust constants land on exactly the same `f32` bit patterns by
/// observing behaviour that depends on `FLT_EPSILON` at the boundary.
#[test]
fn phase_d_float_constant_parity() {
    let (c, r) = (&libs().c, &libs().r);
    assert_eq!(
        3.402_823_466_385_288_598_117_041_834_845_169_25e38f32.to_bits(),
        f32::MAX.to_bits(),
        "FLT_MAX literal"
    );
    assert_eq!(
        1.192_092_895_507_812_5e-7f32.to_bits(),
        1.192_092_9e-7f32.to_bits(),
        "FLT_EPSILON literal"
    );
    // Drive `dist > FLT_EPSILON` from both sides of the threshold through
    // circle-circle (radius 0) so the comparison is the only thing that varies.
    let eps = 1.192_092_895_507_812_5e-7f32;
    for sep in [
        0.0f32,
        f32::from_bits(1),
        eps * 0.5,
        f32::from_bits(eps.to_bits() - 1),
        eps,
        f32::from_bits(eps.to_bits() + 1),
        eps * 2.0,
    ] {
        let a = ShapeBuf::circle(c2Circle {
            p: c2v { x: 0.0, y: 0.0 },
            r: 0.0,
        });
        let b = ShapeBuf::circle(c2Circle {
            p: c2v { x: sep, y: 0.0 },
            r: 0.0,
        });
        for ur in [1i32, 0] {
            let co = call_gjk(c, &a, None, &b, None, ur, true, true, None);
            let ro = call_gjk(r, &a, None, &b, None, ur, true, true, None);
            assert_gjk_eq(&format!("phase D FLT_EPSILON boundary sep={sep:?} ur={ur}"), &co, &ro);
        }
    }
    // And `d0 = FLT_MAX` in the loop: the first `d1 > d0` test must never fire,
    // so a single-iteration result is always produced for a sane pair.
    let a = ShapeBuf::aabb(c2AABB {
        min: c2v { x: -1.0, y: -1.0 },
        max: c2v { x: 1.0, y: 1.0 },
    });
    let b = ShapeBuf::aabb(c2AABB {
        min: c2v { x: 1e30, y: 1e30 },
        max: c2v { x: 2e30, y: 2e30 },
    });
    let co = call_gjk(c, &a, None, &b, None, 1, true, true, None);
    let ro = call_gjk(r, &a, None, &b, None, 1, true, true, None);
    assert_gjk_eq("phase D FLT_MAX d0 seed", &co, &ro);
}

/// The project builds no executable, so there is no stdout to compare; assert
/// that explicitly so the claim is checked rather than assumed.
#[test]
fn phase_d_no_driver_binary_exists() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let cmake = std::fs::read_to_string(root.join("c_src/CMakeLists.txt")).expect("CMakeLists.txt");
    assert!(
        !cmake.contains("add_executable"),
        "c_src now builds an executable; the stdout comparison must be implemented"
    );
    assert!(
        !PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/main.rs").exists(),
        "translation/src/main.rs appeared; the stdout comparison must be implemented"
    );
    let manifest =
        std::fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"))
            .unwrap();
    assert!(!manifest.contains("[[bin]]"), "a [[bin]] target appeared");
    // Also assert the feature surface is still empty, i.e. one configuration.
    assert!(
        !manifest.contains("[features]"),
        "Cargo.toml gained a [features] table; Phases B-C must be re-run per combination"
    );
}
