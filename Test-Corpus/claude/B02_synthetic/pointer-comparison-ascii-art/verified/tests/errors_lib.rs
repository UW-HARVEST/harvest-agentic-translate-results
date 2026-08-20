//! Phase C — error-path differential tests for `shape.c` / `scene.c`
//! (rows 1-38 and the generic boundary rows 72-81 of `ERRORS.md`).
//!
//! Both shared objects are loaded with `libloading`; every case constructs one
//! exact invalid input and asserts that both return the same sentinel and print
//! the same diagnostics.

mod common;

use std::ffi::CString;

use common::*;

fn cs(bytes: &[u8]) -> CString {
    CString::new(bytes.to_vec()).unwrap()
}

/// Values with no valid `shape_type_t` variant (a C enum accepts any `int`).
const BAD_TYPES: [i32; 9] = [
    i32::MIN,
    -1000,
    -2,
    -1,
    10,
    11,
    12,
    1000,
    i32::MAX,
];

#[test]
fn errors_lib() {
    let apis = load_apis();
    let mut rep = Report::new();

    // ------------------------------------------------------------- row 4
    // Must run first: it needs the pristine (never initialised) `shapes[]`.
    rep.check(diff_case(&apis, "e-row04-get-before-init", &|api, ctx| unsafe {
        for t in 0..SHAPE_COUNT {
            let p = (api.shape_get)(t);
            let tag = ctx.tag(p as *const _);
            ctx.line(format!("shape_get({}) before init = {}", t, tag));
        }
        // and printing what it returned
        (api.shape_print)((api.shape_get)(0));
    }));

    // ------------------------------------------------------------- row 77
    rep.check(diff_case(&apis, "e-row77-cleanup-without-init", &|api, ctx| unsafe {
        (api.shape_manager_cleanup)();
        (api.shape_manager_cleanup)();
        for t in 0..SHAPE_COUNT {
            let tag = ctx.tag((api.shape_get)(t) as *const _);
            ctx.line(format!("after double cleanup: shape_get({}) = {}", t, tag));
        }
        (api.shape_print)((api.shape_get)(3));
    }));

    // ------------------------------------------------------------ rows 2,3,72
    rep.check(diff_case(&apis, "e-row02-03-72-shape-get-range", &|api, ctx| unsafe {
        (api.shape_manager_init)();
        for t in BAD_TYPES {
            let p = (api.shape_get)(t);
            let tag = ctx.tag(p as *const _);
            ctx.line(format!("shape_get({}) = {}", t, tag));
            (api.shape_print)(p);
        }
    }));

    // ------------------------------------------------------------ rows 9,72
    rep.check(diff_case(&apis, "e-row09-type-name-range", &|api, ctx| unsafe {
        for t in BAD_TYPES {
            let s = (api.shape_type_name)(t);
            let text = ctx.c_str(s);
            ctx.line(format!("shape_type_name({}) = \"{}\"", t, text));
        }
    }));

    // ------------------------------------------------------------- row 5
    rep.check(diff_case(&apis, "e-row05-get-after-cleanup", &|api, ctx| unsafe {
        (api.shape_manager_init)();
        (api.shape_manager_cleanup)();
        ctx.forget_ptrs();
        for t in 0..SHAPE_COUNT {
            let tag = ctx.tag((api.shape_get)(t) as *const _);
            ctx.line(format!("shape_get({}) after cleanup = {}", t, tag));
        }
    }));

    // ------------------------------------------------------------- row 6
    rep.check(diff_case(&apis, "e-row06-shape-print-null", &|api, ctx| unsafe {
        ctx.line("shape_print(NULL):");
        (api.shape_print)(std::ptr::null());
        (api.shape_print)(std::ptr::null());
    }));

    // ----------------------------------------------------------- rows 7,8
    rep.check(diff_case(&apis, "e-row07-08-shape-equals-null", &|api, ctx| unsafe {
        (api.shape_manager_init)();
        let a = (api.shape_get)(0);
        let b = (api.shape_get)(1);
        let n = std::ptr::null::<ShapeT>();
        ctx.line(format!("equals(NULL,NULL) = {}", (api.shape_equals)(n, n)));
        ctx.line(format!("equals(a,NULL)   = {}", (api.shape_equals)(a, n)));
        ctx.line(format!("equals(NULL,a)   = {}", (api.shape_equals)(n, a)));
        ctx.line(format!("equals(a,b)      = {}", (api.shape_equals)(a, b)));
        ctx.line(format!("equals(a,a)      = {}", (api.shape_equals)(a, a)));
    }));

    // ------------------------------------------------------------ row 78
    rep.check(diff_case(&apis, "e-row78-double-init", &|api, ctx| unsafe {
        (api.shape_manager_init)();
        let first = (api.shape_get)(0);
        let tag_first = ctx.tag(first as *const _);
        ctx.line(format!("first init shape_get(0) = {}", tag_first));
        (api.shape_manager_init)(); // leaks the first set, exactly like the C code
        let second = (api.shape_get)(0);
        ctx.line(format!(
            "equals(first, second) = {}",
            (api.shape_equals)(first, second)
        ));
        ctx.forget_ptrs();
        for t in 0..SHAPE_COUNT {
            let p = (api.shape_get)(t);
            let tag = ctx.tag(p as *const _);
            ctx.line(format!("second init shape_get({}) = {}", t, tag));
            ctx.dump_shape(p);
        }
    }));

    // ----------------------------------------------------------- rows 11,74
    rep.check(diff_case(&apis, "e-row11-74-create-null-empty", &|api, ctx| unsafe {
        let s0 = (api.scene_create)(std::ptr::null());
        ctx.line("scene_create(NULL):");
        ctx.dump_scene(s0);
        let empty = cs(b"");
        let s1 = (api.scene_create)(empty.as_ptr());
        ctx.line("scene_create(\"\"):");
        ctx.dump_scene(s1);
        ctx.dump_scene_name_raw(s1);
        (api.scene_print)(s1);
        (api.scene_list_shapes)(s1);
    }));

    // ----------------------------------------------------------- rows 12,75
    rep.check(diff_case(&apis, "e-row12-75-create-too-long", &|api, ctx| unsafe {
        for len in [63usize, 64, 65, 200, 1000] {
            let name: Vec<u8> = (0..len).map(|i| b'a' + (i % 26) as u8).collect();
            let c = cs(&name);
            let s = (api.scene_create)(c.as_ptr());
            ctx.line(format!("scene_create({} bytes):", len));
            ctx.dump_scene(s);
            ctx.dump_scene_name_raw(s);
        }
    }));

    // ------------------------------------------------------------ row 13
    rep.check(diff_case(&apis, "e-row13-destroy-null", &|api, ctx| unsafe {
        (api.scene_destroy)(std::ptr::null_mut());
        (api.scene_destroy)(std::ptr::null_mut());
        ctx.line("scene_destroy(NULL) twice: no crash");
    }));

    // --------------------------------------------------------- rows 14,15,16
    rep.check(diff_case(&apis, "e-row14-16-add-null", &|api, ctx| unsafe {
        (api.shape_manager_init)();
        let name = cs(b"S");
        let s = (api.scene_create)(name.as_ptr());
        let shape = (api.shape_get)(0);
        ctx.line(format!(
            "add(NULL, shape) = {}",
            (api.scene_add_shape)(std::ptr::null_mut(), shape)
        ));
        ctx.line(format!(
            "add(scene, NULL) = {}",
            (api.scene_add_shape)(s, std::ptr::null_mut())
        ));
        ctx.line(format!(
            "add(NULL, NULL)  = {}",
            (api.scene_add_shape)(std::ptr::null_mut(), std::ptr::null_mut())
        ));
        // the scene must be untouched
        ctx.dump_scene(s);
        // shape_get of an invalid type yields NULL -> add fails
        ctx.line(format!(
            "add(scene, shape_get(99)) = {}",
            (api.scene_add_shape)(s, (api.shape_get)(99))
        ));
        ctx.dump_scene(s);
    }));

    // ---------------------------------------------------------- rows 17,76
    rep.check(diff_case(&apis, "e-row17-76-scene-full", &|api, ctx| unsafe {
        (api.shape_manager_init)();
        let name = cs(b"Full");
        let s = (api.scene_create)(name.as_ptr());
        for i in 0..55 {
            let r = (api.scene_add_shape)(s, (api.shape_get)(i % 10));
            ctx.line(format!("add #{} = {}", i, r));
        }
        ctx.dump_scene(s);
        (api.scene_list_shapes)(s);
    }));

    // --------------------------------------------------------- rows 18,19,20
    rep.check(diff_case(&apis, "e-row18-20-remove-range", &|api, ctx| unsafe {
        (api.shape_manager_init)();
        let name = cs(b"R");
        let s = (api.scene_create)(name.as_ptr());
        ctx.line(format!(
            "remove(NULL,0) = {}",
            (api.scene_remove_shape)(std::ptr::null_mut(), 0)
        ));
        // empty scene: every index is out of range
        for idx in [i32::MIN, -1, 0, 1, i32::MAX] {
            ctx.line(format!(
                "empty remove({}) = {}",
                idx,
                (api.scene_remove_shape)(s, idx)
            ));
        }
        for t in 0..3 {
            (api.scene_add_shape)(s, (api.shape_get)(t));
        }
        for idx in [i32::MIN, -2, -1, 3, 4, i32::MAX] {
            ctx.line(format!(
                "3-shape remove({}) = {}",
                idx,
                (api.scene_remove_shape)(s, idx)
            ));
        }
        ctx.dump_scene(s);
    }));

    // ------------------------------------------------------------ row 21
    rep.check(diff_case(&apis, "e-row21-scene-print-null", &|api, ctx| unsafe {
        (api.scene_print)(std::ptr::null());
        ctx.line("scene_print(NULL) done");
    }));

    // --------------------------------------------------------- rows 22,23,24
    rep.check(diff_case(&apis, "e-row22-24-equals-null", &|api, ctx| unsafe {
        let name = cs(b"E");
        let s = (api.scene_create)(name.as_ptr());
        let n = std::ptr::null::<SceneT>();
        ctx.line(format!("equals(NULL,NULL) = {}", (api.scene_equals)(n, n)));
        ctx.line(format!("equals(s,NULL)    = {}", (api.scene_equals)(s, n)));
        ctx.line(format!("equals(NULL,s)    = {}", (api.scene_equals)(n, s)));
    }));

    // ---------------------------------------------------------- rows 25,26
    rep.check(diff_case(&apis, "e-row25-26-equals-mismatch", &|api, ctx| unsafe {
        (api.shape_manager_init)();
        let a = cs(b"A");
        let s1 = (api.scene_create)(a.as_ptr());
        let s2 = (api.scene_create)(a.as_ptr());
        (api.scene_add_shape)(s1, (api.shape_get)(0));
        (api.scene_add_shape)(s1, (api.shape_get)(1));
        (api.scene_add_shape)(s2, (api.shape_get)(0));
        ctx.line(format!("different counts = {}", (api.scene_equals)(s1, s2)));
        (api.scene_add_shape)(s2, (api.shape_get)(2));
        ctx.dump_scene(s1);
        ctx.dump_scene(s2);
        ctx.line(format!("no partner = {}", (api.scene_equals)(s1, s2)));
        ctx.line(format!("no partner (rev) = {}", (api.scene_equals)(s2, s1)));
    }));

    // ---------------------------------------------------------- rows 27,28
    rep.check(diff_case(&apis, "e-row27-28-save-null", &|api, ctx| unsafe {
        (api.shape_manager_init)();
        let name = cs(b"S");
        let s = (api.scene_create)(name.as_ptr());
        let path = ctx.path("out.txt");
        ctx.line(format!(
            "save(NULL, path) = {}",
            (api.scene_save)(std::ptr::null(), path.as_ptr())
        ));
        ctx.line(format!(
            "save(scene, NULL) = {}",
            (api.scene_save)(s, std::ptr::null())
        ));
        ctx.line(format!(
            "save(NULL, NULL) = {}",
            (api.scene_save)(std::ptr::null(), std::ptr::null())
        ));
    }));

    // ------------------------------------------------------------ row 29
    rep.check(diff_case(&apis, "e-row29-save-open-fail", &|api, ctx| unsafe {
        (api.shape_manager_init)();
        let name = cs(b"S");
        let s = (api.scene_create)(name.as_ptr());
        (api.scene_add_shape)(s, (api.shape_get)(4));

        let empty = cs(b"");
        ctx.line(format!(
            "save(\"\") = {}",
            (api.scene_save)(s, empty.as_ptr())
        ));

        let missing = cs(b"/nonexistent_dir_xyz/sub/out.txt");
        ctx.line(format!(
            "save(missing dir) = {}",
            (api.scene_save)(s, missing.as_ptr())
        ));

        // an existing directory
        let dir = ctx.path(".");
        ctx.line(format!(
            "save(directory) = {}",
            (api.scene_save)(s, dir.as_ptr())
        ));

        // 300 byte file name component -> ENAMETOOLONG
        let long: Vec<u8> = vec![b'x'; 300];
        let long_path = ctx.path(&String::from_utf8(long).unwrap());
        ctx.line(format!(
            "save(300 byte name) = {}",
            (api.scene_save)(s, long_path.as_ptr())
        ));
    }));

    // ---------------------------------------------------------- rows 30,31
    rep.check(diff_case(&apis, "e-row30-31-load-open-fail", &|api, ctx| unsafe {
        (api.shape_manager_init)();
        let n = (api.scene_load)(std::ptr::null());
        let tag = ctx.tag(n as *const _);
        ctx.line(format!("load(NULL) = {}", tag));

        let missing = ctx.path("does-not-exist.dat");
        let s = (api.scene_load)(missing.as_ptr());
        let tag = ctx.tag(s as *const _);
        ctx.line(format!("load(missing) = {}", tag));

        let empty = cs(b"");
        let s = (api.scene_load)(empty.as_ptr());
        let tag = ctx.tag(s as *const _);
        ctx.line(format!("load(\"\") = {}", tag));

        let dir = ctx.path(".");
        let s = (api.scene_load)(dir.as_ptr());
        let tag = ctx.tag(s as *const _);
        ctx.line(format!("load(directory) = {}", tag));
    }));

    // ------------------------------------------------- rows 32,33,34,35,36,37
    rep.check(diff_case(&apis, "e-row32-37-load-bad-content", &|api, ctx| unsafe {
        (api.shape_manager_init)();
        let files: [(&str, &[u8]); 24] = [
            ("empty", b""),
            ("newline-only", b"\n"),
            ("name-only", b"Name\n"),
            ("name-no-nl", b"Name"),
            ("count-not-numeric", b"Name\nxyz\n"),
            ("count-empty-line", b"Name\n\n"),
            ("count-partial", b"Name\n1abc\n"),
            ("fewer-types", b"Name\n5\n1\n2\n"),
            ("type-not-numeric", b"Name\n2\n1\nzz\n"),
            ("type-negative", b"Name\n3\n-3\n1\n-1\n"),
            ("type-too-big", b"Name\n3\n99\n1\n10\n"),
            ("count-negative", b"Name\n-5\n"),
            ("count-negative-types", b"Name\n-1\n1\n2\n"),
            ("count-51", b"Name\n51\n"),
            ("count-huge", b"Name\n99999999999999999999\n1\n"),
            ("only-numbers", b"7\n2\n1\n2\n"),
            ("count-int-max", b"Name\n2147483647\n1\n2\n"),
            ("count-int-max-bare", b"Name\n2147483647\n"),
            ("count-int-min", b"Name\n-2147483648\n1\n"),
            ("leading-zeros", b"Name\n0002\n0001\n0009\n"),
            ("hex-like", b"Name\n0x2\n1\n"),
            ("type-hex-like", b"Name\n2\n0x1\n2\n"),
            ("nul-in-name", b"Na\x00me\n1\n2\n"),
            ("crlf-only", b"Name\r\n\r\n"),
        ];
        for (tag, content) in files {
            let fname = format!("{}.dat", tag);
            ctx.write_file(&fname, content);
            let path = ctx.path(&fname);
            ctx.line(format!("--- {} = \"{}\"", tag, escape(content)));
            let s = (api.scene_load)(path.as_ptr());
            ctx.dump_scene(s);
            if !s.is_null() {
                (api.scene_print)(s);
            }
        }
    }));

    // ------------------------------------------------------------ row 36
    rep.check(diff_case(&apis, "e-row36-load-over-50", &|api, ctx| unsafe {
        (api.shape_manager_init)();
        for count in [50usize, 51, 55, 60] {
            let mut content = format!("Big{}\n{}\n", count, count).into_bytes();
            for i in 0..count {
                content.extend_from_slice(format!("{}\n", i % 10).as_bytes());
            }
            let fname = format!("big{}.dat", count);
            ctx.write_file(&fname, &content);
            let path = ctx.path(&fname);
            ctx.line(format!("--- count {}", count));
            let s = (api.scene_load)(path.as_ptr());
            ctx.dump_scene(s);
        }
    }));

    // ------------------------------------------------------------ row 38
    rep.check(diff_case(&apis, "e-row38-list-null", &|api, ctx| unsafe {
        (api.scene_list_shapes)(std::ptr::null());
        ctx.line("scene_list_shapes(NULL) done");
    }));

    // ------------------------------------------------------------ row 74
    rep.check(diff_case(&apis, "e-row74-zero-length", &|api, ctx| unsafe {
        (api.shape_manager_init)();
        let empty = cs(b"");
        let s = (api.scene_create)(empty.as_ptr());
        ctx.dump_scene(s);
        ctx.line(format!(
            "save(\"\") = {}",
            (api.scene_save)(s, empty.as_ptr())
        ));
        let l = (api.scene_load)(empty.as_ptr());
        let tag = ctx.tag(l as *const _);
        ctx.line(format!("load(\"\") = {}", tag));
    }));

    // ------------------------------------------------------- rows 79,80,81
    rep.check(diff_case(&apis, "e-row79-81-permissions-and-paths", &|api, ctx| unsafe {
        (api.shape_manager_init)();
        let name = cs(b"P");
        let s = (api.scene_create)(name.as_ptr());
        (api.scene_add_shape)(s, (api.shape_get)(1));

        // row 79: an existing file without write permission
        ctx.write_file("ro.txt", b"keep me\n");
        let ro = ctx.dir.join("ro.txt");
        let mut perms = std::fs::metadata(&ro).unwrap().permissions();
        std::os::unix::fs::PermissionsExt::set_mode(&mut perms, 0o444);
        std::fs::set_permissions(&ro, perms).unwrap();
        let ro_c = ctx.path("ro.txt");
        ctx.line(format!(
            "save(read-only file) = {}",
            (api.scene_save)(s, ro_c.as_ptr())
        ));

        // row 80: an existing file without read permission
        ctx.write_file("noread.dat", b"N\n1\n1\n");
        let nr = ctx.dir.join("noread.dat");
        let mut perms = std::fs::metadata(&nr).unwrap().permissions();
        std::os::unix::fs::PermissionsExt::set_mode(&mut perms, 0o000);
        std::fs::set_permissions(&nr, perms).unwrap();
        let nr_c = ctx.path("noread.dat");
        let loaded = (api.scene_load)(nr_c.as_ptr());
        let tag = ctx.tag(loaded as *const _);
        ctx.line(format!("load(unreadable) = {}", tag));

        // row 81: a 300 byte file name component (ENAMETOOLONG)
        let long: String = "l".repeat(300);
        let long_c = ctx.path(&long);
        let loaded = (api.scene_load)(long_c.as_ptr());
        let tag = ctx.tag(loaded as *const _);
        ctx.line(format!("load(300 byte name) = {}", tag));
        ctx.line(format!(
            "save(300 byte name) = {}",
            (api.scene_save)(s, long_c.as_ptr())
        ));

        // make the files readable again so the transcript can include them
        for f in ["ro.txt", "noread.dat"] {
            let p = ctx.dir.join(f);
            let mut perms = std::fs::metadata(&p).unwrap().permissions();
            std::os::unix::fs::PermissionsExt::set_mode(&mut perms, 0o644);
            std::fs::set_permissions(&p, perms).unwrap();
        }
    }));

    rep.finish("ERRORS.md rows 1-38 + 72-81 (library entry points)");
}
