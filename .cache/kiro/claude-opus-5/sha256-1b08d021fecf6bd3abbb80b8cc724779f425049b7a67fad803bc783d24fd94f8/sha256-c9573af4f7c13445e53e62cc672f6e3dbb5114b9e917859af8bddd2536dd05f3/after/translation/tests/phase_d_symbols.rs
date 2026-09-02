//! Phase D — symbol parity, enforced as a test rather than a claim.
//!
//! Shells out to `nm -D` on both shared objects and asserts the exported-symbol
//! sets are equal, and that the Rust `.so` imports no non-libc symbol. This is
//! the same check `SYMBOLS.md` documents, run automatically so it cannot drift.

mod common;
use common::*;

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn c_so() -> PathBuf {
    let build = root().join("c_src/build");
    std::fs::read_dir(&build)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", build.display()))
        .flatten()
        .map(|e| e.path())
        .find(|p| {
            let n = p.file_name().unwrap().to_string_lossy();
            n.starts_with("lib") && n.ends_with(".so")
        })
        .unwrap_or_else(|| panic!("no lib*.so in {}", build.display()))
}

fn rust_so() -> PathBuf {
    let exe = std::env::current_exe().unwrap();
    let profile = exe.parent().unwrap().parent().unwrap();
    let p = profile.join("libspec_ray_lib.so");
    if p.is_file() {
        return p;
    }
    for prof in ["release", "debug"] {
        let q = root()
            .join("translation/target")
            .join(prof)
            .join("libspec_ray_lib.so");
        if q.is_file() {
            return q;
        }
    }
    panic!("libspec_ray_lib.so not found");
}

fn nm(path: &PathBuf, extra: &str) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg(extra)
        .arg(path)
        .output()
        .unwrap_or_else(|e| panic!("nm failed: {e}"));
    assert!(
        out.status.success(),
        "nm {} {} failed: {}",
        extra,
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .filter(|s| !s.is_empty())
        .collect()
}

#[test]
fn d01_exported_symbols_match_exactly() {
    let cs = nm(&c_so(), "--defined-only");
    let rs = nm(&rust_so(), "--defined-only");

    let missing: Vec<_> = cs.difference(&rs).cloned().collect();
    let extra: Vec<_> = rs.difference(&cs).cloned().collect();

    println!("C exports   ({}): {:?}", cs.len(), cs);
    println!("Rust exports({}): {:?}", rs.len(), rs);

    assert!(
        missing.is_empty(),
        "{} symbol(s) exported by the C .so are MISSING from the Rust .so: {missing:?}",
        missing.len()
    );
    assert!(
        extra.is_empty(),
        "the Rust .so exports {} symbol(s) the C .so does not: {extra:?}",
        extra.len()
    );
    assert_eq!(cs.len(), 22, "expected 22 exported symbols, found {}", cs.len());
}

#[test]
fn d02_no_stubs_every_symbol_is_callable() {
    // A symbol present in `nm -D` proves nothing about behaviour, so confirm
    // every one is reachable AND produces a value that matches the C. (The
    // per-function differential rows do the heavy lifting; this is the
    // "nothing here is an `unimplemented!()`" tripwire.)
    let (c, r) = pair();
    let mut d = Diff::new("D2: every exported symbol is a real implementation");
    let a = c2v { x: 3.0, y: -4.0 };
    let b = c2v { x: -1.5, y: 2.25 };

    d.v_bits(|| "c2V".into(), unsafe { (c.c2V)(3.0, -4.0) }, unsafe {
        (r.c2V)(3.0, -4.0)
    });
    d.f32_bits(|| "c2Dot".into(), unsafe { (c.c2Dot)(a, b) }, unsafe {
        (r.c2Dot)(a, b)
    });
    d.f32_bits(|| "c2Len".into(), unsafe { (c.c2Len)(a) }, unsafe { (r.c2Len)(a) });
    d.v_bits(|| "c2Add".into(), unsafe { (c.c2Add)(a, b) }, unsafe { (r.c2Add)(a, b) });
    d.v_bits(|| "c2Sub".into(), unsafe { (c.c2Sub)(a, b) }, unsafe { (r.c2Sub)(a, b) });
    d.v_bits(|| "c2Mulvs".into(), unsafe { (c.c2Mulvs)(a, 2.5) }, unsafe {
        (r.c2Mulvs)(a, 2.5)
    });
    d.v_bits(|| "c2Div".into(), unsafe { (c.c2Div)(a, 2.5) }, unsafe {
        (r.c2Div)(a, 2.5)
    });
    d.v_bits(|| "c2Norm".into(), unsafe { (c.c2Norm)(a) }, unsafe { (r.c2Norm)(a) });
    d.v_bits(|| "c2Minv".into(), unsafe { (c.c2Minv)(a, b) }, unsafe {
        (r.c2Minv)(a, b)
    });
    d.v_bits(|| "c2Maxv".into(), unsafe { (c.c2Maxv)(a, b) }, unsafe {
        (r.c2Maxv)(a, b)
    });
    d.v_bits(|| "c2Skew".into(), unsafe { (c.c2Skew)(a) }, unsafe { (r.c2Skew)(a) });
    d.v_bits(|| "c2Absv".into(), unsafe { (c.c2Absv)(a) }, unsafe { (r.c2Absv)(a) });
    d.v_bits(|| "c2CCW90".into(), unsafe { (c.c2CCW90)(a) }, unsafe { (r.c2CCW90)(a) });
    let m = c2m { x: a, y: b };
    d.v_bits(|| "c2MulmvT".into(), unsafe { (c.c2MulmvT)(m, a) }, unsafe {
        (r.c2MulmvT)(m, a)
    });
    let bx = c2AABB {
        min: c2v { x: -1.0, y: -1.0 },
        max: c2v { x: 1.0, y: 1.0 },
    };
    let bx2 = c2AABB {
        min: c2v { x: 0.5, y: 0.5 },
        max: c2v { x: 3.0, y: 3.0 },
    };
    d.eq(
        || "c2AABBtoAABB".into(),
        unsafe { (c.c2AABBtoAABB)(bx, bx2) },
        unsafe { (r.c2AABBtoAABB)(bx, bx2) },
    );
    d.eq(
        || "c2AABBtoPoint".into(),
        unsafe { (c.c2AABBtoPoint)(bx, c2v { x: 0.0, y: 0.0 }) },
        unsafe { (r.c2AABBtoPoint)(bx, c2v { x: 0.0, y: 0.0 }) },
    );
    let cir = c2Circle {
        p: c2v { x: 0.0, y: 0.0 },
        r: 2.0,
    };
    d.eq(
        || "c2CircleToPoint".into(),
        unsafe { (c.c2CircleToPoint)(cir, c2v { x: 1.0, y: 1.0 }) },
        unsafe { (r.c2CircleToPoint)(cir, c2v { x: 1.0, y: 1.0 }) },
    );
    let ray = c2Ray {
        p: c2v { x: -10.0, y: 0.0 },
        d: c2v { x: 1.0, y: 0.0 },
        t: 100.0,
    };
    cmp_ray_circle(&mut d, c, r, ray, cir);
    cmp_ray_aabb(&mut d, c, r, ray, bx);
    cmp_ray_capsule(
        &mut d,
        c,
        r,
        ray,
        c2Capsule {
            a: c2v { x: 0.0, y: -2.0 },
            b: c2v { x: 0.0, y: 2.0 },
            r: 1.0,
        },
    );
    cmp_cast_ray(&mut d, c, r, ray, as_bytes(&cir), C2_TYPE_CIRCLE, "smoke");
    cmp_spec_ray(
        &mut d,
        c,
        r,
        c2v { x: 0.0, y: 0.0 },
        c2v { x: 0.0, y: 0.0 },
        2.0,
        c2v { x: -10.0, y: 0.0 },
    );
    // 22 symbols; `v_bits` contributes 2 comparisons (x and y), the scalar and
    // int helpers 1 each, and the five composite drivers 1 each.
    assert!(
        d.checked() >= 22,
        "only {} comparisons -- not every symbol was exercised",
        d.checked()
    );
    d.finish();
}

#[test]
fn d03_rust_so_imports_only_libc() {
    let undef = nm(&rust_so(), "--undefined-only");
    // Everything the Rust cdylib may legitimately import: glibc, libgcc's
    // unwinder, and the standard weak ELF hooks.
    let allowed_exact = [
        "_ITM_deregisterTMCloneTable",
        "_ITM_registerTMCloneTable",
        "__gmon_start__",
    ];
    let offenders: Vec<_> = undef
        .iter()
        .filter(|s| {
            let base = s.split('@').next().unwrap_or(s);
            !(allowed_exact.contains(&base)
                || s.contains("@GLIBC")
                || s.contains("@GCC")
                || s.contains("@GLIBCXX")
                || base.starts_with("_Unwind_"))
        })
        .cloned()
        .collect();
    println!("Rust undefined ({}): {:?}", undef.len(), undef);
    assert!(
        offenders.is_empty(),
        "the Rust .so has unresolved non-libc imports: {offenders:?}"
    );
}
