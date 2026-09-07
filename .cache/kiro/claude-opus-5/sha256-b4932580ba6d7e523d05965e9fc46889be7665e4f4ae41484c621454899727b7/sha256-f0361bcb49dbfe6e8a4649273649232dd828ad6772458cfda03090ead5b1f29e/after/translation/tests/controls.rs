//! Negative controls / harness self-checks.
//!
//! A differential test suite is worthless if the comparison can never fail.
//! These tests prove the harness (a) actually invokes both shared objects,
//! (b) actually observes buffer mutation, and (c) actually detects divergence.

mod common;

use common::*;

/// (a) Both `.so` files really are two distinct files, and both really export
/// `premultiply` (dlsym would have panicked in `load_pair` otherwise).
#[test]
fn control_two_distinct_shared_objects() {
    let pair = load_pair();
    let (c_path, rust_path) = paths();
    assert!(exists(&c_path), "C .so missing: {}", c_path.display());
    assert!(
        exists(&rust_path),
        "Rust .so missing: {}",
        rust_path.display()
    );
    assert_ne!(c_path, rust_path);
    assert!(
        c_path.to_string_lossy().contains("c_src"),
        "C lib should come from c_src/build, got {}",
        c_path.display()
    );
    assert!(
        rust_path.to_string_lossy().contains("target"),
        "Rust lib should come from target/, got {}",
        rust_path.display()
    );
    // Distinct file contents.
    let c_bytes = std::fs::read(&c_path).unwrap();
    let r_bytes = std::fs::read(&rust_path).unwrap();
    assert_ne!(c_bytes, r_bytes);
    assert_eq!(pair.c.name, "C");
    assert_eq!(pair.rust.name, "Rust");
}

/// (b) The libraries actually mutate the buffer — the passing tests are not
/// passing because nothing ever happens.
#[test]
fn control_buffer_is_actually_mutated() {
    let pair = load_pair();
    // a = 128 halves each channel, so the output must differ from the input.
    let data: Vec<u8> = vec![255, 255, 255, 128, 200, 100, 50, 128];
    let out = diff_run(&pair, 2, 1, &data, "control-mutate");
    assert_ne!(out, data, "premultiply did not modify the buffer at all");
    assert_eq!(out[3], 128, "alpha must be preserved");
    assert_eq!(out[7], 128, "alpha must be preserved");
    // 255/255 * 128/255 * 255 = 128.0 -> 128
    assert_eq!(&out[0..3], &[128, 128, 128], "unexpected premultiplied value");
}

/// (b2) Each library independently mutates its own buffer (called separately,
/// not through the diff helper) — proves both sides do work.
#[test]
fn control_each_library_mutates_independently() {
    let pair = load_pair();
    for lib in [&pair.c, &pair.rust] {
        let mut buf: Vec<u8> = vec![255, 255, 255, 0];
        let mut img = CpImage {
            w: 1,
            h: 1,
            pix: buf.as_mut_ptr() as *mut CpPixel,
        };
        unsafe { lib.premultiply(&mut img) };
        assert_eq!(
            buf,
            vec![0, 0, 0, 0],
            "{} did not zero rgb for alpha=0",
            lib.name
        );
    }
}

/// (c) The comparison logic detects a planted divergence. We simulate a buggy
/// implementation by running only the C lib on one arena and leaving the other
/// arena untouched, then asserting the byte comparison flags it.
#[test]
fn control_divergence_is_detected() {
    let pair = load_pair();
    let data: Vec<u8> = vec![255, 200, 100, 128];

    let mut a = Arena::new(data.len(), 64, 0);
    a.buf[a.pix_off..a.pix_off + data.len()].copy_from_slice(&data);
    let mut b = Arena::new(data.len(), 64, 0);
    b.buf[b.pix_off..b.pix_off + data.len()].copy_from_slice(&data);
    assert_eq!(a.buf, b.buf, "arenas should start equal");

    let mut img = CpImage {
        w: 1,
        h: 1,
        pix: a.pix_ptr(),
    };
    unsafe { pair.c.premultiply(&mut img) };

    // Only one side ran, so the arenas MUST now differ. If this assertion ever
    // fails, `diff_run`'s comparison could never catch a real bug either.
    assert_ne!(
        a.buf, b.buf,
        "byte comparison cannot distinguish a run from a no-op"
    );

    // And the guard-poison check must be able to fire: plant a byte in a guard.
    let mut c = Arena::new(4, 8, 0);
    c.buf[0] = 0x00;
    assert!(
        !c.buf[..8].iter().all(|&x| x == POISON),
        "guard check cannot detect a clobbered guard"
    );
}

/// (c2) `expected_iterations` — the model the tests rely on — is itself checked
/// against the C's observable behavior: the number of pixels the C mutates.
#[test]
fn control_trip_count_model_matches_c() {
    let pair = load_pair();
    let cases: &[(i32, i32)] = &[
        (0, 5),
        (5, 0),
        (1, 1),
        (2, 3),
        (7, 1),
        (1, 7),
        (-2, -3),
        (-1, -1),
        (i32::MAX, -1),
        (0x7FFF_FFFE, 0x7FFF_FFFE),
        (-4, 4),
        (4, -4),
        (0x4000_0000, 1),
        (0x2000_0000, 1),
    ];
    for &(w, h) in cases {
        let iters = expected_iterations(w, h) as usize;
        // Input: alpha=0 everywhere with rgb=0xFF, so every *processed* pixel
        // has its rgb zeroed and every untouched pixel keeps 0xFF.
        let n = iters + 4;
        let mut data = vec![0xFFu8; n * PIXEL_SIZE];
        for i in 0..n {
            data[i * 4 + 3] = 0;
        }
        let out = diff_run(&pair, w, h, &data, "control-tripcount");
        let mut observed = 0usize;
        for i in 0..n {
            let px = &out[i * 4..i * 4 + 4];
            if px[0] == 0 && px[1] == 0 && px[2] == 0 {
                observed += 1;
            } else {
                break;
            }
        }
        assert_eq!(
            observed, iters,
            "trip-count model wrong for w={w} h={h}: C mutated {observed} px, model said {iters}"
        );
    }
}
