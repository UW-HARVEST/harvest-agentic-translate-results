// tests/ffi_compare.rs
//
// Compares the C reference implementation and the Rust implementation
// through their respective .so files via libloading. Both libraries export
// the same symbol names with the same C ABI, so we load each one, exercise
// the public API, and assert byte-identical results.

mod common;

use common::*;
use std::ffi::CString;
use std::os::raw::c_int;

// ----------------------------------------------------------------
// shape_type_name
// ----------------------------------------------------------------

#[test]
fn shape_type_name_all_values() {
    let _g = common::acquire_lock();
    let c = load_c();
    let r = load_rust();

    for t in -1..=SHAPE_COUNT + 1 {
        let cp = c.shape_type_name(t);
        let rp = r.shape_type_name(t);
        let cs = c_str_to_bytes(cp);
        let rs = c_str_to_bytes(rp);
        assert_eq!(
            cs, rs,
            "shape_type_name({}) C={:?} Rust={:?}",
            t,
            String::from_utf8_lossy(&cs),
            String::from_utf8_lossy(&rs)
        );
    }
}

// ----------------------------------------------------------------
// shape_manager_init / shape_get / shape_manager_cleanup
// ----------------------------------------------------------------

#[test]
fn shape_manager_init_and_get_all_shapes_match() {
    let _g = common::acquire_lock();
    let c = load_c();
    let r = load_rust();
    c.shape_manager_init();
    r.shape_manager_init();

    for t in 0..SHAPE_COUNT {
        let cp = c.shape_get(t);
        let rp = r.shape_get(t);
        assert!(!cp.is_null(), "C shape_get({}) NULL", t);
        assert!(!rp.is_null(), "Rust shape_get({}) NULL", t);
        assert!(
            shapes_content_equal(cp, rp),
            "shape contents differ for type {}",
            t
        );
    }

    // Out-of-range
    assert!(c.shape_get(-1).is_null());
    assert!(r.shape_get(-1).is_null());
    assert!(c.shape_get(SHAPE_COUNT).is_null());
    assert!(r.shape_get(SHAPE_COUNT).is_null());

    c.shape_manager_cleanup();
    r.shape_manager_cleanup();
}

// ----------------------------------------------------------------
// shape_equals
// ----------------------------------------------------------------

#[test]
fn shape_equals_identity_and_distinct() {
    let _g = common::acquire_lock();
    let c = load_c();
    let r = load_rust();
    c.shape_manager_init();
    r.shape_manager_init();

    for t in 0..SHAPE_COUNT {
        let cp = c.shape_get(t);
        let rp = r.shape_get(t);

        // Same pointer -> 1
        assert_eq!(c.shape_equals(cp, cp), 1);
        assert_eq!(r.shape_equals(rp, rp), 1);

        // Different pointer -> 0
        let cp2 = c.shape_get((t + 1) % SHAPE_COUNT);
        let rp2 = r.shape_get((t + 1) % SHAPE_COUNT);
        let c_res = c.shape_equals(cp, cp2);
        let r_res = r.shape_equals(rp, rp2);
        assert_eq!(c_res, r_res);

        // null cases: shape_equals(NULL, NULL) — C returns 1 (NULL == NULL).
        let n1 = c.shape_equals(std::ptr::null(), std::ptr::null());
        let n2 = r.shape_equals(std::ptr::null(), std::ptr::null());
        assert_eq!(n1, n2);
    }

    c.shape_manager_cleanup();
    r.shape_manager_cleanup();
}

// ----------------------------------------------------------------
// scene_create / scene_destroy
// ----------------------------------------------------------------

#[test]
fn scene_create_with_name() {
    let _g = common::acquire_lock();
    let c = load_c();
    let r = load_rust();

    let name = CString::new("MyScene").unwrap();
    let cs = c.scene_create(name.as_ptr());
    let rs = r.scene_create(name.as_ptr());

    assert!(!cs.is_null());
    assert!(!rs.is_null());
    assert_eq!(scene_name_bytes(cs), b"MyScene".to_vec());
    assert_eq!(scene_name_bytes(rs), b"MyScene".to_vec());
    unsafe {
        assert_eq!((*cs).shape_count, 0);
        assert_eq!((*rs).shape_count, 0);
    }

    c.scene_destroy(cs);
    r.scene_destroy(rs);
}

#[test]
fn scene_create_with_null_name() {
    let _g = common::acquire_lock();
    let c = load_c();
    let r = load_rust();

    let cs = c.scene_create(std::ptr::null());
    let rs = r.scene_create(std::ptr::null());
    assert_eq!(scene_name_bytes(cs), b"Untitled Scene".to_vec());
    assert_eq!(scene_name_bytes(rs), b"Untitled Scene".to_vec());

    c.scene_destroy(cs);
    r.scene_destroy(rs);
}

#[test]
fn scene_create_truncates_long_name() {
    let _g = common::acquire_lock();
    let c = load_c();
    let r = load_rust();
    let long = "A".repeat(120);
    let name = CString::new(long).unwrap();
    let cs = c.scene_create(name.as_ptr());
    let rs = r.scene_create(name.as_ptr());

    let cn = scene_name_bytes(cs);
    let rn = scene_name_bytes(rs);
    assert_eq!(cn.len(), MAX_SCENE_NAME - 1);
    assert_eq!(cn, rn);

    c.scene_destroy(cs);
    r.scene_destroy(rs);
}

// ----------------------------------------------------------------
// scene_add_shape / scene_remove_shape
// ----------------------------------------------------------------

#[test]
fn scene_add_and_remove_shapes() {
    let _g = common::acquire_lock();
    let c = load_c();
    let r = load_rust();
    c.shape_manager_init();
    r.shape_manager_init();

    let name = CString::new("Test").unwrap();
    let cs = c.scene_create(name.as_ptr());
    let rs = r.scene_create(name.as_ptr());

    // Add 3 shapes
    for t in [0, 3, 7] {
        let csh = c.shape_get(t);
        let rsh = r.shape_get(t);
        assert_eq!(c.scene_add_shape(cs, csh), 0);
        assert_eq!(r.scene_add_shape(rs, rsh), 0);
    }
    unsafe {
        assert_eq!((*cs).shape_count, 3);
        assert_eq!((*rs).shape_count, 3);
    }

    // Add to NULL scene
    assert_eq!(c.scene_add_shape(std::ptr::null_mut(), c.shape_get(0)), -1);
    assert_eq!(r.scene_add_shape(std::ptr::null_mut(), r.shape_get(0)), -1);

    // Add NULL shape
    assert_eq!(c.scene_add_shape(cs, std::ptr::null_mut()), -1);
    assert_eq!(r.scene_add_shape(rs, std::ptr::null_mut()), -1);

    // Remove middle shape
    assert_eq!(c.scene_remove_shape(cs, 1), 0);
    assert_eq!(r.scene_remove_shape(rs, 1), 0);
    unsafe {
        assert_eq!((*cs).shape_count, 2);
        assert_eq!((*rs).shape_count, 2);
    }

    // Out of range
    assert_eq!(c.scene_remove_shape(cs, 10), -1);
    assert_eq!(r.scene_remove_shape(rs, 10), -1);
    assert_eq!(c.scene_remove_shape(cs, -1), -1);
    assert_eq!(r.scene_remove_shape(rs, -1), -1);

    // Null scene
    assert_eq!(c.scene_remove_shape(std::ptr::null_mut(), 0), -1);
    assert_eq!(r.scene_remove_shape(std::ptr::null_mut(), 0), -1);

    c.scene_destroy(cs);
    r.scene_destroy(rs);
    c.shape_manager_cleanup();
    r.shape_manager_cleanup();
}

#[test]
fn scene_add_shape_until_full() {
    let _g = common::acquire_lock();
    let c = load_c();
    let r = load_rust();
    c.shape_manager_init();
    r.shape_manager_init();

    let name = CString::new("Full").unwrap();
    let cs = c.scene_create(name.as_ptr());
    let rs = r.scene_create(name.as_ptr());

    let csh = c.shape_get(0);
    let rsh = r.shape_get(0);
    for _ in 0..MAX_SHAPES_IN_SCENE {
        assert_eq!(c.scene_add_shape(cs, csh), 0);
        assert_eq!(r.scene_add_shape(rs, rsh), 0);
    }
    // Next add should fail (scene full)
    assert_eq!(c.scene_add_shape(cs, csh), -1);
    assert_eq!(r.scene_add_shape(rs, rsh), -1);

    c.scene_destroy(cs);
    r.scene_destroy(rs);
    c.shape_manager_cleanup();
    r.shape_manager_cleanup();
}

// ----------------------------------------------------------------
// scene_equals
// ----------------------------------------------------------------

#[test]
fn scene_equals_basic() {
    let _g = common::acquire_lock();
    let c = load_c();
    let r = load_rust();
    c.shape_manager_init();
    r.shape_manager_init();

    let n1 = CString::new("S1").unwrap();
    let n2 = CString::new("S2").unwrap();
    let c1 = c.scene_create(n1.as_ptr());
    let c2 = c.scene_create(n2.as_ptr());
    let r1 = r.scene_create(n1.as_ptr());
    let r2 = r.scene_create(n2.as_ptr());

    // Empty + empty -> equal
    assert_eq!(c.scene_equals(c1, c2), 1);
    assert_eq!(r.scene_equals(r1, r2), 1);

    // Add same shapes to each in same order
    for t in [0, 5, 8] {
        c.scene_add_shape(c1, c.shape_get(t));
        c.scene_add_shape(c2, c.shape_get(t));
        r.scene_add_shape(r1, r.shape_get(t));
        r.scene_add_shape(r2, r.shape_get(t));
    }
    assert_eq!(c.scene_equals(c1, c2), 1);
    assert_eq!(r.scene_equals(r1, r2), 1);

    // Add different shape to c2/r2
    c.scene_add_shape(c2, c.shape_get(1));
    r.scene_add_shape(r2, r.shape_get(1));
    assert_eq!(c.scene_equals(c1, c2), 0);
    assert_eq!(r.scene_equals(r1, r2), 0);

    // null scenes
    assert_eq!(c.scene_equals(std::ptr::null(), c1), 0);
    assert_eq!(r.scene_equals(std::ptr::null(), r1), 0);
    assert_eq!(c.scene_equals(c1, std::ptr::null()), 0);
    assert_eq!(r.scene_equals(r1, std::ptr::null()), 0);

    c.scene_destroy(c1);
    c.scene_destroy(c2);
    r.scene_destroy(r1);
    r.scene_destroy(r2);
    c.shape_manager_cleanup();
    r.shape_manager_cleanup();
}

#[test]
fn scene_equals_with_reordering() {
    // Same shapes added in different orders should still compare equal
    // (1:1 correspondence semantics in C).
    let c = load_c();
    let r = load_rust();
    c.shape_manager_init();
    r.shape_manager_init();

    let n = CString::new("X").unwrap();
    let c1 = c.scene_create(n.as_ptr());
    let c2 = c.scene_create(n.as_ptr());
    let r1 = r.scene_create(n.as_ptr());
    let r2 = r.scene_create(n.as_ptr());

    let order1 = [2, 4, 6];
    let order2 = [6, 2, 4];
    for t in order1 {
        c.scene_add_shape(c1, c.shape_get(t));
        r.scene_add_shape(r1, r.shape_get(t));
    }
    for t in order2 {
        c.scene_add_shape(c2, c.shape_get(t));
        r.scene_add_shape(r2, r.shape_get(t));
    }
    assert_eq!(c.scene_equals(c1, c2), 1);
    assert_eq!(r.scene_equals(r1, r2), 1);

    c.scene_destroy(c1);
    c.scene_destroy(c2);
    r.scene_destroy(r1);
    r.scene_destroy(r2);
    c.shape_manager_cleanup();
    r.shape_manager_cleanup();
}

// ----------------------------------------------------------------
// scene_save / scene_load - round trip
// ----------------------------------------------------------------

#[test]
fn scene_save_then_load_round_trip() {
    let _g = common::acquire_lock();
    let c = load_c();
    let r = load_rust();
    c.shape_manager_init();
    r.shape_manager_init();

    let name = CString::new("RoundTrip").unwrap();
    let cs = c.scene_create(name.as_ptr());
    let rs = r.scene_create(name.as_ptr());
    for t in [3, 1, 4, 1, 5, 9] {
        c.scene_add_shape(cs, c.shape_get(t));
        r.scene_add_shape(rs, r.shape_get(t));
    }

    let tmpdir = std::env::temp_dir();
    let cfile = tmpdir.join("scene_c.txt");
    let rfile = tmpdir.join("scene_r.txt");
    let cf_c = CString::new(cfile.to_str().unwrap()).unwrap();
    let rf_c = CString::new(rfile.to_str().unwrap()).unwrap();

    assert_eq!(c.scene_save(cs, cf_c.as_ptr()), 0);
    assert_eq!(r.scene_save(rs, rf_c.as_ptr()), 0);

    // Files should match byte-for-byte
    let cf_bytes = std::fs::read(&cfile).unwrap();
    let rf_bytes = std::fs::read(&rfile).unwrap();
    assert_eq!(
        cf_bytes,
        rf_bytes,
        "saved files differ:\nC:   {:?}\nRust: {:?}",
        String::from_utf8_lossy(&cf_bytes),
        String::from_utf8_lossy(&rf_bytes)
    );

    // Load back
    let cl = c.scene_load(cf_c.as_ptr());
    let rl = r.scene_load(rf_c.as_ptr());
    assert!(!cl.is_null());
    assert!(!rl.is_null());
    unsafe {
        assert_eq!((*cl).shape_count, 6);
        assert_eq!((*rl).shape_count, 6);
    }
    assert_eq!(scene_name_bytes(cl), b"RoundTrip".to_vec());
    assert_eq!(scene_name_bytes(rl), b"RoundTrip".to_vec());

    // Loaded scene should equal saved scene
    assert_eq!(c.scene_equals(cs, cl), 1);
    assert_eq!(r.scene_equals(rs, rl), 1);

    c.scene_destroy(cs);
    c.scene_destroy(cl);
    r.scene_destroy(rs);
    r.scene_destroy(rl);
    let _ = std::fs::remove_file(&cfile);
    let _ = std::fs::remove_file(&rfile);
    c.shape_manager_cleanup();
    r.shape_manager_cleanup();
}

#[test]
fn scene_load_nonexistent() {
    let _g = common::acquire_lock();
    let c = load_c();
    let r = load_rust();
    let bad = CString::new("/no/such/file/path/xyz123.txt").unwrap();
    let cs = c.scene_load(bad.as_ptr());
    let rs = r.scene_load(bad.as_ptr());
    assert!(cs.is_null());
    assert!(rs.is_null());
}

#[test]
fn scene_save_null_args() {
    let _g = common::acquire_lock();
    let c = load_c();
    let r = load_rust();
    let bad = CString::new("/tmp/scene_save_null.txt").unwrap();
    assert_eq!(c.scene_save(std::ptr::null(), bad.as_ptr()), -1);
    assert_eq!(r.scene_save(std::ptr::null(), bad.as_ptr()), -1);
}

#[test]
fn scene_load_null_arg() {
    let _g = common::acquire_lock();
    let c = load_c();
    let r = load_rust();
    assert!(c.scene_load(std::ptr::null()).is_null());
    assert!(r.scene_load(std::ptr::null()).is_null());
}

// ----------------------------------------------------------------
// Save file content format (deeper inspection)
// ----------------------------------------------------------------

#[test]
fn scene_save_format_is_identical() {
    let _g = common::acquire_lock();
    let c = load_c();
    let r = load_rust();
    c.shape_manager_init();
    r.shape_manager_init();

    // Test multiple cases
    let cases: &[(&str, &[c_int])] = &[
        ("Empty", &[]),
        ("OneShape", &[2]),
        ("ManyShapes", &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]),
    ];
    for (name, shapes) in cases {
        let cn = CString::new(*name).unwrap();
        let cs = c.scene_create(cn.as_ptr());
        let rs = r.scene_create(cn.as_ptr());
        for &t in *shapes {
            c.scene_add_shape(cs, c.shape_get(t));
            r.scene_add_shape(rs, r.shape_get(t));
        }
        let tmp = std::env::temp_dir();
        let cfile = tmp.join(format!("scene_save_{}_c.txt", name));
        let rfile = tmp.join(format!("scene_save_{}_r.txt", name));
        let cf = CString::new(cfile.to_str().unwrap()).unwrap();
        let rf = CString::new(rfile.to_str().unwrap()).unwrap();
        assert_eq!(c.scene_save(cs, cf.as_ptr()), 0);
        assert_eq!(r.scene_save(rs, rf.as_ptr()), 0);

        let cb = std::fs::read(&cfile).unwrap();
        let rb = std::fs::read(&rfile).unwrap();
        assert_eq!(cb, rb, "save format differs for case {}", name);
        let _ = std::fs::remove_file(&cfile);
        let _ = std::fs::remove_file(&rfile);

        c.scene_destroy(cs);
        r.scene_destroy(rs);
    }

    c.shape_manager_cleanup();
    r.shape_manager_cleanup();
}
