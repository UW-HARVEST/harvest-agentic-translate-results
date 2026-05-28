// Tests for scene_* functions, comparing C and Rust implementations.

#[path = "common.rs"]
mod common;

use common::*;
use std::ffi::c_int;

#[test]
fn scene_create_and_destroy() {
    let c = ApiSyms::load(&c_lib_path());
    let r = ApiSyms::load(&rust_lib_path());

    unsafe {
        // Test with a name
        let name = cstring("My Scene");
        let cs = (c.scene_create)(name.as_ptr());
        let rs = (r.scene_create)(name.as_ptr());
        assert!(!cs.is_null());
        assert!(!rs.is_null());
        let cname = buf_to_string(&(*cs).name);
        let rname = buf_to_string(&(*rs).name);
        assert_eq!(cname, "My Scene");
        assert_eq!(rname, "My Scene");
        assert_eq!((*cs).shape_count, 0);
        assert_eq!((*rs).shape_count, 0);

        (c.scene_destroy)(cs);
        (r.scene_destroy)(rs);

        // Test with NULL name -> "Untitled Scene"
        let cs2 = (c.scene_create)(std::ptr::null());
        let rs2 = (r.scene_create)(std::ptr::null());
        let cn2 = buf_to_string(&(*cs2).name);
        let rn2 = buf_to_string(&(*rs2).name);
        assert_eq!(cn2, "Untitled Scene");
        assert_eq!(rn2, "Untitled Scene");

        (c.scene_destroy)(cs2);
        (r.scene_destroy)(rs2);

        // NULL destroy is safe
        (c.scene_destroy)(std::ptr::null_mut());
        (r.scene_destroy)(std::ptr::null_mut());
    }
}

#[test]
fn scene_create_long_name_truncated() {
    let c = ApiSyms::load(&c_lib_path());
    let r = ApiSyms::load(&rust_lib_path());

    unsafe {
        let long = "A".repeat(100);
        let cn = cstring(&long);
        let cs = (c.scene_create)(cn.as_ptr());
        let rs = (r.scene_create)(cn.as_ptr());
        let cname = buf_to_string(&(*cs).name);
        let rname = buf_to_string(&(*rs).name);
        assert_eq!(cname.len(), MAX_SCENE_NAME - 1);
        assert_eq!(rname.len(), MAX_SCENE_NAME - 1);
        assert_eq!(cname, rname);
        (c.scene_destroy)(cs);
        (r.scene_destroy)(rs);
    }
}

#[test]
fn scene_add_remove_shape() {
    let _g = common::serialize();
    let c = ApiSyms::load(&c_lib_path());
    let r = ApiSyms::load(&rust_lib_path());

    unsafe {
        (c.shape_manager_init)();
        (r.shape_manager_init)();

        let n = cstring("S");
        let cs = (c.scene_create)(n.as_ptr());
        let rs = (r.scene_create)(n.as_ptr());

        // Add 3 shapes (Tree, House, Sun)
        let order = [0, 2, 3];
        for &i in &order {
            let csh = (c.shape_get)(i);
            let rsh = (r.shape_get)(i);
            assert_eq!((c.scene_add_shape)(cs, csh), 0);
            assert_eq!((r.scene_add_shape)(rs, rsh), 0);
        }
        assert_eq!((*cs).shape_count, 3);
        assert_eq!((*rs).shape_count, 3);

        // Add to NULL scene -> -1
        assert_eq!((c.scene_add_shape)(std::ptr::null_mut(), (c.shape_get)(0)), -1);
        assert_eq!((r.scene_add_shape)(std::ptr::null_mut(), (r.shape_get)(0)), -1);
        // Add NULL shape -> -1
        assert_eq!((c.scene_add_shape)(cs, std::ptr::null_mut()), -1);
        assert_eq!((r.scene_add_shape)(rs, std::ptr::null_mut()), -1);

        // Remove middle (index 1 -> House)
        assert_eq!((c.scene_remove_shape)(cs, 1), 0);
        assert_eq!((r.scene_remove_shape)(rs, 1), 0);
        assert_eq!((*cs).shape_count, 2);
        assert_eq!((*rs).shape_count, 2);

        // Verify remaining shape types match between C and Rust
        for i in 0..(*cs).shape_count as usize {
            let cst = (*(*cs).shapes[i]).shape_type;
            let rst = (*(*rs).shapes[i]).shape_type;
            assert_eq!(cst, rst, "shape_type[{}]", i);
        }

        // Out-of-range removal -> -1
        assert_eq!((c.scene_remove_shape)(cs, -1), -1);
        assert_eq!((r.scene_remove_shape)(rs, -1), -1);
        assert_eq!((c.scene_remove_shape)(cs, 99), -1);
        assert_eq!((r.scene_remove_shape)(rs, 99), -1);
        // NULL scene removal -> -1
        assert_eq!((c.scene_remove_shape)(std::ptr::null_mut(), 0), -1);
        assert_eq!((r.scene_remove_shape)(std::ptr::null_mut(), 0), -1);

        (c.scene_destroy)(cs);
        (r.scene_destroy)(rs);

        (c.shape_manager_cleanup)();
        (r.shape_manager_cleanup)();
    }
}

#[test]
fn scene_equals_matches() {
    let _g = common::serialize();
    let c = ApiSyms::load(&c_lib_path());
    let r = ApiSyms::load(&rust_lib_path());

    unsafe {
        (c.shape_manager_init)();
        (r.shape_manager_init)();

        // 1. Empty scenes
        let n = cstring("a");
        let cs1 = (c.scene_create)(n.as_ptr());
        let cs2 = (c.scene_create)(n.as_ptr());
        let rs1 = (r.scene_create)(n.as_ptr());
        let rs2 = (r.scene_create)(n.as_ptr());
        assert_eq!((c.scene_equals)(cs1, cs2), 1);
        assert_eq!((r.scene_equals)(rs1, rs2), 1);

        // 2. Different counts
        (c.scene_add_shape)(cs1, (c.shape_get)(0));
        (r.scene_add_shape)(rs1, (r.shape_get)(0));
        assert_eq!((c.scene_equals)(cs1, cs2), 0);
        assert_eq!((r.scene_equals)(rs1, rs2), 0);

        // 3. Same set, different order -> equal
        (c.scene_add_shape)(cs1, (c.shape_get)(1));
        (r.scene_add_shape)(rs1, (r.shape_get)(1));
        (c.scene_add_shape)(cs2, (c.shape_get)(1));
        (r.scene_add_shape)(rs2, (r.shape_get)(1));
        (c.scene_add_shape)(cs2, (c.shape_get)(0));
        (r.scene_add_shape)(rs2, (r.shape_get)(0));
        assert_eq!((c.scene_equals)(cs1, cs2), 1);
        assert_eq!((r.scene_equals)(rs1, rs2), 1);

        // 4. Replace one with a different shape -> not equal
        (c.scene_remove_shape)(cs2, 0);
        (r.scene_remove_shape)(rs2, 0);
        (c.scene_add_shape)(cs2, (c.shape_get)(2));
        (r.scene_add_shape)(rs2, (r.shape_get)(2));
        assert_eq!((c.scene_equals)(cs1, cs2), 0);
        assert_eq!((r.scene_equals)(rs1, rs2), 0);

        // 5. Null scenes
        assert_eq!((c.scene_equals)(std::ptr::null(), cs1), 0);
        assert_eq!((r.scene_equals)(std::ptr::null(), rs1), 0);
        assert_eq!((c.scene_equals)(cs1, std::ptr::null()), 0);
        assert_eq!((r.scene_equals)(rs1, std::ptr::null()), 0);

        (c.scene_destroy)(cs1);
        (c.scene_destroy)(cs2);
        (r.scene_destroy)(rs1);
        (r.scene_destroy)(rs2);

        (c.shape_manager_cleanup)();
        (r.shape_manager_cleanup)();
    }
}

#[test]
fn scene_save_and_load() {
    let _g = common::serialize();
    let c = ApiSyms::load(&c_lib_path());
    let r = ApiSyms::load(&rust_lib_path());

    unsafe {
        (c.shape_manager_init)();
        (r.shape_manager_init)();

        let dir = tempfile::tempdir().unwrap();
        let cpath = dir.path().join("c_scene.txt");
        let rpath = dir.path().join("r_scene.txt");
        let cpath_s = cstring(cpath.to_str().unwrap());
        let rpath_s = cstring(rpath.to_str().unwrap());

        let n = cstring("Sample");
        let cs = (c.scene_create)(n.as_ptr());
        let rs = (r.scene_create)(n.as_ptr());

        for i in [3i32, 0, 7, 9] {
            (c.scene_add_shape)(cs, (c.shape_get)(i));
            (r.scene_add_shape)(rs, (r.shape_get)(i));
        }
        let cret = (c.scene_save)(cs, cpath_s.as_ptr());
        let rret = (r.scene_save)(rs, rpath_s.as_ptr());
        assert_eq!(cret, 0);
        assert_eq!(rret, 0);

        let c_data = std::fs::read(&cpath).unwrap();
        let r_data = std::fs::read(&rpath).unwrap();
        assert_eq!(c_data, r_data, "saved file bytes differ");

        // Now reload using each API
        let c_loaded = (c.scene_load)(cpath_s.as_ptr());
        let r_loaded = (r.scene_load)(cpath_s.as_ptr());
        assert!(!c_loaded.is_null());
        assert!(!r_loaded.is_null());
        let cname = buf_to_string(&(*c_loaded).name);
        let rname = buf_to_string(&(*r_loaded).name);
        assert_eq!(cname, "Sample");
        assert_eq!(rname, "Sample");
        assert_eq!((*c_loaded).shape_count, 4);
        assert_eq!((*r_loaded).shape_count, 4);
        for i in 0..4 {
            let ct = (*(*c_loaded).shapes[i]).shape_type;
            let rt = (*(*r_loaded).shapes[i]).shape_type;
            assert_eq!(ct, rt);
        }

        // Save/load with NULL parameters
        assert_eq!((c.scene_save)(std::ptr::null(), cpath_s.as_ptr()), -1);
        assert_eq!((r.scene_save)(std::ptr::null(), cpath_s.as_ptr()), -1);
        assert_eq!((c.scene_save)(cs, std::ptr::null()), -1);
        assert_eq!((r.scene_save)(rs, std::ptr::null()), -1);
        assert!((c.scene_load)(std::ptr::null()).is_null());
        assert!((r.scene_load)(std::ptr::null()).is_null());

        // Loading a non-existent file
        let bad = cstring("/tmp/nonexistent_scene_test_file_xyzw.txt");
        assert!((c.scene_load)(bad.as_ptr()).is_null());
        assert!((r.scene_load)(bad.as_ptr()).is_null());

        (c.scene_destroy)(cs);
        (c.scene_destroy)(c_loaded);
        (r.scene_destroy)(rs);
        (r.scene_destroy)(r_loaded);

        (c.shape_manager_cleanup)();
        (r.shape_manager_cleanup)();
    }
}

#[test]
fn scene_save_empty_scene() {
    let _g = common::serialize();
    let c = ApiSyms::load(&c_lib_path());
    let r = ApiSyms::load(&rust_lib_path());

    unsafe {
        (c.shape_manager_init)();
        (r.shape_manager_init)();

        let dir = tempfile::tempdir().unwrap();
        let cpath = dir.path().join("ec.txt");
        let rpath = dir.path().join("er.txt");
        let cps = cstring(cpath.to_str().unwrap());
        let rps = cstring(rpath.to_str().unwrap());

        let n = cstring("Empty");
        let cs = (c.scene_create)(n.as_ptr());
        let rs = (r.scene_create)(n.as_ptr());

        assert_eq!((c.scene_save)(cs, cps.as_ptr()), 0);
        assert_eq!((r.scene_save)(rs, rps.as_ptr()), 0);

        let c_data = std::fs::read(&cpath).unwrap();
        let r_data = std::fs::read(&rpath).unwrap();
        assert_eq!(c_data, r_data);

        (c.scene_destroy)(cs);
        (r.scene_destroy)(rs);
        (c.shape_manager_cleanup)();
        (r.shape_manager_cleanup)();
    }
}

#[test]
fn scene_add_max_shapes() {
    let _g = common::serialize();
    let c = ApiSyms::load(&c_lib_path());
    let r = ApiSyms::load(&rust_lib_path());

    unsafe {
        (c.shape_manager_init)();
        (r.shape_manager_init)();

        let n = cstring("Max");
        let cs = (c.scene_create)(n.as_ptr());
        let rs = (r.scene_create)(n.as_ptr());

        for _ in 0..MAX_SHAPES_IN_SCENE {
            assert_eq!((c.scene_add_shape)(cs, (c.shape_get)(0)), 0);
            assert_eq!((r.scene_add_shape)(rs, (r.shape_get)(0)), 0);
        }
        // 51st should fail
        assert_eq!((c.scene_add_shape)(cs, (c.shape_get)(0)), -1);
        assert_eq!((r.scene_add_shape)(rs, (r.shape_get)(0)), -1);

        (c.scene_destroy)(cs);
        (r.scene_destroy)(rs);

        (c.shape_manager_cleanup)();
        (r.shape_manager_cleanup)();
    }
}

#[test]
fn scene_remove_shifts_correctly() {
    let _g = common::serialize();
    let c = ApiSyms::load(&c_lib_path());
    let r = ApiSyms::load(&rust_lib_path());

    unsafe {
        (c.shape_manager_init)();
        (r.shape_manager_init)();

        let n = cstring("Sh");
        let cs = (c.scene_create)(n.as_ptr());
        let rs = (r.scene_create)(n.as_ptr());

        for i in 0..5 {
            (c.scene_add_shape)(cs, (c.shape_get)(i as c_int));
            (r.scene_add_shape)(rs, (r.shape_get)(i as c_int));
        }
        // Remove first
        (c.scene_remove_shape)(cs, 0);
        (r.scene_remove_shape)(rs, 0);
        // Check remaining types are 1,2,3,4
        for i in 0..4 {
            let ct = (*(*cs).shapes[i]).shape_type;
            let rt = (*(*rs).shapes[i]).shape_type;
            assert_eq!(ct, rt);
            assert_eq!(ct, i as c_int + 1);
        }
        // Remove last (index 3)
        (c.scene_remove_shape)(cs, 3);
        (r.scene_remove_shape)(rs, 3);
        for i in 0..3 {
            let ct = (*(*cs).shapes[i]).shape_type;
            let rt = (*(*rs).shapes[i]).shape_type;
            assert_eq!(ct, rt);
            assert_eq!(ct, i as c_int + 1);
        }

        (c.scene_destroy)(cs);
        (r.scene_destroy)(rs);

        (c.shape_manager_cleanup)();
        (r.shape_manager_cleanup)();
    }
}
