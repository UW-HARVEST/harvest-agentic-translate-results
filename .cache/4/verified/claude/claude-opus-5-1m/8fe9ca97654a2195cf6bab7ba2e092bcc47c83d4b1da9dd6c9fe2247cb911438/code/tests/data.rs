//! Differential tests for the exported *data* symbols (CONFIGS.md row 59).
//!
//! `q_math.c` defines fifteen mutable globals (`vec3_origin`, `axisDefault`,
//! `bytedirs`, `g_color_table` and the eleven `colorXxx` `vec4_t`s).  The
//! address of each is taken from BOTH shared objects with `dlsym`
//! (`cdata`/`rdata`) and the raw `f32` bit patterns are compared element by
//! element -- a `memcmp`-grade check that the Rust `static mut`s carry exactly
//! the same bytes, in the same order, as the C initialisers.
//!
//! None of these symbols is `const` in C: they live in `.data`/`.bss` and are
//! writable through the pointer `dlsym` hands out, which `data_symbols_are_writable`
//! demonstrates on both libraries.

mod harness;

use core::ffi::c_int;
use harness::*;
use std::sync::Mutex;

/// `#[test]` functions run as threads of a single process, and
/// `data_symbols_are_writable` temporarily overwrites `vec3_origin` in both
/// libraries.  Every test that looks at that symbol takes this lock so the
/// write can never be observed by another test.
static VEC3_ORIGIN_LOCK: Mutex<()> = Mutex::new(());

/// Flattens a nested array of vectors into one `f32` slice.
fn flatten<const N: usize, const M: usize>(v: &[[f32; N]; M]) -> Vec<f32> {
    let mut out = Vec::with_capacity(N * M);
    for row in v.iter() {
        out.extend_from_slice(row);
    }
    out
}

/// Compares one `vec3_t`/`vec4_t` data symbol in both `.so`s bit-for-bit.
#[track_caller]
fn compare_flat<const N: usize>(name: &str) {
    let c = unsafe { *cdata::<[f32; N]>(name) };
    let r = unsafe { *rdata::<[f32; N]>(name) };
    assert_vec(&format!("data symbol `{name}` ({N} floats)"), &c, &r);
}

// ---------------------------------------------------------------------------
// row 59 -- every exported data symbol, byte for byte
// ---------------------------------------------------------------------------

#[test]
fn data_symbols_match() {
    let _guard = VEC3_ORIGIN_LOCK.lock().unwrap();

    // ---- vec3_origin: vec3_t --------------------------------------------
    let co = unsafe { *cdata::<[f32; 3]>("vec3_origin") };
    let ro = unsafe { *rdata::<[f32; 3]>("vec3_origin") };
    assert_vec("data symbol `vec3_origin`", &co, &ro);
    assert_vec("`vec3_origin` == {0,0,0}", &co, &[0.0, 0.0, 0.0]);

    // ---- axisDefault: vec3_t[3] ----------------------------------------
    let ca = unsafe { *cdata::<[[f32; 3]; 3]>("axisDefault") };
    let ra = unsafe { *rdata::<[[f32; 3]; 3]>("axisDefault") };
    for i in 0..3 {
        assert_vec(&format!("data symbol `axisDefault[{i}]`"), &ca[i], &ra[i]);
    }
    assert_vec(
        "data symbol `axisDefault` (flattened)",
        &flatten(&ca),
        &flatten(&ra),
    );
    assert_vec(
        "`axisDefault` is the identity matrix",
        &flatten(&ca),
        &[1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
    );

    // ---- bytedirs: vec3_t[162] -----------------------------------------
    let cb = unsafe { *cdata::<[[f32; 3]; 162]>("bytedirs") };
    let rb = unsafe { *rdata::<[[f32; 3]; 162]>("bytedirs") };
    for i in 0..162 {
        // one assert per entry, so a failure names the offending index
        assert_vec(&format!("data symbol `bytedirs[{i}]`"), &cb[i], &rb[i]);
        for j in 0..3 {
            assert_int(
                &format!("`bytedirs[{i}][{j}]` bit pattern"),
                cb[i][j].to_bits(),
                rb[i][j].to_bits(),
            );
        }
    }
    assert_vec(
        "data symbol `bytedirs` (flattened, 486 floats)",
        &flatten(&cb),
        &flatten(&rb),
    );
    // the first and the last entry, spelled out from q_math.c
    assert_vec(
        "`bytedirs[0]` == {-0.525731, 0, 0.850651}",
        &cb[0],
        &[-0.525731, 0.000000, 0.850651],
    );
    assert_vec(
        "`bytedirs[161]` == {-0.688191, -0.587785, -0.425325}",
        &cb[161],
        &[-0.688191, -0.587785, -0.425325],
    );

    // ---- g_color_table: vec4_t[8] --------------------------------------
    let cg = unsafe { *cdata::<[[f32; 4]; 8]>("g_color_table") };
    let rg = unsafe { *rdata::<[[f32; 4]; 8]>("g_color_table") };
    for i in 0..8 {
        assert_vec(&format!("data symbol `g_color_table[{i}]`"), &cg[i], &rg[i]);
    }
    assert_vec(
        "data symbol `g_color_table` (flattened, 32 floats)",
        &flatten(&cg),
        &flatten(&rg),
    );

    // ---- the eleven vec4_t colours -------------------------------------
    for name in [
        "colorBlack",
        "colorRed",
        "colorGreen",
        "colorBlue",
        "colorYellow",
        "colorMagenta",
        "colorCyan",
        "colorWhite",
        "colorLtGrey",
        "colorMdGrey",
        "colorDkGrey",
    ] {
        compare_flat::<4>(name);
    }

    // every colour has alpha 1.0 in q_math.c; check on the C side so the
    // comparison above is anchored to a known value
    for name in [
        "colorBlack",
        "colorRed",
        "colorGreen",
        "colorBlue",
        "colorYellow",
        "colorMagenta",
        "colorCyan",
        "colorWhite",
        "colorLtGrey",
        "colorMdGrey",
        "colorDkGrey",
    ] {
        let v = unsafe { *cdata::<[f32; 4]>(name) };
        assert_f32(&format!("`{name}[3]` (alpha) == 1.0"), 1.0, v[3]);
    }

    // and the exact palette, as a last anchor
    assert_vec(
        "`colorLtGrey` == {0.75,0.75,0.75,1}",
        &unsafe { *cdata::<[f32; 4]>("colorLtGrey") },
        &[0.75, 0.75, 0.75, 1.0],
    );
    assert_vec(
        "`colorMdGrey` == {0.5,0.5,0.5,1}",
        &unsafe { *cdata::<[f32; 4]>("colorMdGrey") },
        &[0.5, 0.5, 0.5, 1.0],
    );
    assert_vec(
        "`colorDkGrey` == {0.25,0.25,0.25,1}",
        &unsafe { *cdata::<[f32; 4]>("colorDkGrey") },
        &[0.25, 0.25, 0.25, 1.0],
    );
}

// ---------------------------------------------------------------------------
// the symbols are plain writable globals in both libraries
// ---------------------------------------------------------------------------

#[test]
fn data_symbols_are_writable() {
    let _guard = VEC3_ORIGIN_LOCK.lock().unwrap();

    let cp = cdata::<[f32; 3]>("vec3_origin");
    let rp = rdata::<[f32; 3]>("vec3_origin");

    // the pristine contents, so they can be put back no matter what
    let (c_orig, r_orig) = unsafe { (*cp, *rp) };

    // a whole-object write, then three single-element writes
    let probe = [1.5f32, -2.5, f32::from_bits(0x7f80_0001)];
    let single = [-0.0f32, f32::INFINITY, 1e30];

    let (c_after_probe, r_after_probe, c_after_single, r_after_single) = unsafe {
        *cp = probe;
        *rp = probe;
        let c1 = *cp;
        let r1 = *rp;
        for i in 0..3 {
            (*cp)[i] = single[i];
            (*rp)[i] = single[i];
        }
        let c2 = *cp;
        let r2 = *rp;
        // restore before asserting anything, so a failure cannot leak state
        *cp = c_orig;
        *rp = r_orig;
        (c1, r1, c2, r2)
    };
    let (c_restored, r_restored) = unsafe { (*cp, *rp) };

    assert_vec("`vec3_origin` before the write (C vs Rust)", &c_orig, &r_orig);

    assert_vec(
        "`vec3_origin` read back after a whole-object write (C vs Rust)",
        &c_after_probe,
        &r_after_probe,
    );
    assert_vec(
        "C `vec3_origin` read back == what was written",
        &c_after_probe,
        &probe,
    );
    assert_vec(
        "Rust `vec3_origin` read back == what was written",
        &r_after_probe,
        &probe,
    );

    assert_vec(
        "`vec3_origin` read back after element-wise writes (C vs Rust)",
        &c_after_single,
        &r_after_single,
    );
    assert_vec(
        "C `vec3_origin` read back == the element-wise values",
        &c_after_single,
        &single,
    );
    assert_vec(
        "Rust `vec3_origin` read back == the element-wise values",
        &r_after_single,
        &single,
    );

    assert_vec("C `vec3_origin` restored", &c_restored, &c_orig);
    assert_vec("Rust `vec3_origin` restored", &r_restored, &r_orig);
    assert_vec(
        "`vec3_origin` restored to {0,0,0} in both",
        &c_restored,
        &[0.0, 0.0, 0.0],
    );
}

// ---------------------------------------------------------------------------
// cross-check: ByteToDir hands out exactly the bytedirs of its own library
// ---------------------------------------------------------------------------

#[test]
fn bytedirs_used_by_bytetodir() {
    // `void ByteToDir( int b, vec3_t dir )`
    type F = unsafe extern "C" fn(c_int, *mut f32);
    let (c, r): (F, F) = both("ByteToDir");

    // the tables as seen inside each library
    let cb = unsafe { *cdata::<[[f32; 3]; 162]>("bytedirs") };
    let rb = unsafe { *rdata::<[[f32; 3]; 162]>("bytedirs") };

    const SENTINEL: f32 = f32::from_bits(0x7fab_cdef);

    for b in 0..162usize {
        let mut oc = [SENTINEL; 4];
        let mut or_ = [SENTINEL; 4];
        unsafe { c(b as c_int, oc.as_mut_ptr()) };
        unsafe { r(b as c_int, or_.as_mut_ptr()) };

        assert_vec(&format!("ByteToDir({b}) C vs Rust"), &oc[..3], &or_[..3]);
        assert_vec(
            &format!("ByteToDir({b}) (C) == C bytedirs[{b}]"),
            &oc[..3],
            &cb[b],
        );
        assert_vec(
            &format!("ByteToDir({b}) (Rust) == Rust bytedirs[{b}]"),
            &or_[..3],
            &rb[b],
        );
        assert_int(
            &format!("ByteToDir({b}) (C) wrote past the end"),
            oc[3].to_bits(),
            SENTINEL.to_bits(),
        );
        assert_int(
            &format!("ByteToDir({b}) (Rust) wrote past the end"),
            or_[3].to_bits(),
            SENTINEL.to_bits(),
        );
    }
}
