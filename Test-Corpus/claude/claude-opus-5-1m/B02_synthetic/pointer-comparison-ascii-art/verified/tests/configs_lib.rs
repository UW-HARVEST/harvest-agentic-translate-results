//! Phase B — valid-path differential tests for the `shape.c` / `scene.c` entry
//! points (rows 1-30 and 53-56 of `CONFIGS.md`).
//!
//! Both shared objects are loaded with `libloading`; nothing is ever called
//! directly in the test crate.  Every case is run against both and all
//! observable effects (return values, struct contents, `stdout`, `stderr`, files)
//! must be byte identical.
//!
//! All rows live in one `#[test]` function on purpose: the capture mechanism
//! redirects the process wide `stdout`/`stderr` file descriptors, which cannot
//! be done from several test threads at once.  The first rows also rely on the
//! *pristine* (never initialised) state of both shared objects, so the order
//! matters.

mod common;

use std::ffi::CString;

use common::*;

const SEED: u64 = 0x5EED_2026;

fn cs(bytes: &[u8]) -> CString {
    CString::new(bytes.to_vec()).unwrap()
}

/// The names/types used all over the place.
const TYPES: [i32; 10] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];

/// `&scene->shapes[i]` - the tests read and write the public struct exactly like a
/// C caller would.
unsafe fn scene_slot(s: *mut SceneT, i: usize) -> *mut *mut ShapeT {
    (std::ptr::addr_of_mut!((*s).shapes) as *mut *mut ShapeT).add(i)
}

#[test]
fn configs_lib() {
    let apis = load_apis();
    let mut rep = Report::new();

    // ---------------------------------------------------------------- row 1
    // Must run first: it observes the pristine `shapes[]` array.
    rep.check(diff_case(&apis, "row01-init-get", &|api, ctx| unsafe {
        for t in -1..=10 {
            let p = (api.shape_get)(t);
            let tag = ctx.tag(p as *const _);
            ctx.line(format!("pre-init shape_get({}) = {}", t, tag));
        }
        (api.shape_manager_init)();
        for t in TYPES {
            let p = (api.shape_get)(t);
            let tag = ctx.tag(p as *const _);
            let again = (api.shape_get)(t);
            let tag2 = ctx.tag(again as *const _);
            ctx.line(format!("shape_get({}) = {} / {}", t, tag, tag2));
        }
    }));

    // ---------------------------------------------------------------- row 2
    rep.check(diff_case(&apis, "row02-shape-fields", &|api, ctx| unsafe {
        (api.shape_manager_init)();
        for t in TYPES {
            let p = (api.shape_get)(t);
            ctx.line(format!("shape_get({}):", t));
            ctx.dump_shape(p);
        }
    }));

    // ---------------------------------------------------------------- row 3
    rep.check(diff_case(&apis, "row03-shape-print", &|api, ctx| unsafe {
        (api.shape_manager_init)();
        for t in TYPES {
            ctx.line(format!("shape_print({}):", t));
            (api.shape_print)((api.shape_get)(t));
        }
    }));

    // ---------------------------------------------------------------- row 4
    rep.check(diff_case(&apis, "row04-shape-print-rnd", &|api, ctx| unsafe {
        (api.shape_manager_init)();
        let mut rng = Rng::new(SEED ^ 4);
        for k in 0..64 {
            let t = rng.range_i32(0, 9);
            ctx.line(format!("iter {} type {}", k, t));
            (api.shape_print)((api.shape_get)(t));
            (api.shape_print)((api.shape_get)(t)); // same shape twice in a row
        }
    }));

    // ---------------------------------------------------------------- row 5
    rep.check(diff_case(&apis, "row05-type-name", &|api, ctx| unsafe {
        for t in TYPES {
            let s = (api.shape_type_name)(t);
            let text = ctx.c_str(s);
            ctx.line(format!("shape_type_name({}) = \"{}\"", t, text));
        }
    }));

    // ---------------------------------------------------------------- row 6
    rep.check(diff_case(&apis, "row06-shape-equals", &|api, ctx| unsafe {
        (api.shape_manager_init)();
        for a in TYPES {
            for b in TYPES {
                let r = (api.shape_equals)((api.shape_get)(a), (api.shape_get)(b));
                ctx.line(format!("shape_equals({},{}) = {}", a, b, r));
            }
        }
    }));

    // ---------------------------------------------------------------- row 7
    rep.check(diff_case(&apis, "row07-init-cleanup-init", &|api, ctx| unsafe {
        (api.shape_manager_init)();
        for t in TYPES {
            let tag = ctx.tag((api.shape_get)(t) as *const _);
            ctx.line(format!("first init: shape_get({}) = {}", t, tag));
        }
        (api.shape_manager_cleanup)();
        for t in TYPES {
            let tag = ctx.tag((api.shape_get)(t) as *const _);
            ctx.line(format!("after cleanup: shape_get({}) = {}", t, tag));
        }
        // Whether malloc hands out the same addresses again is an allocator
        // property, not a translation property.
        ctx.forget_ptrs();
        (api.shape_manager_init)();
        for t in TYPES {
            let p = (api.shape_get)(t);
            let tag = ctx.tag(p as *const _);
            ctx.line(format!("second init: shape_get({}) = {}", t, tag));
            ctx.dump_shape(p);
            (api.shape_print)(p);
        }
    }));

    // ---------------------------------------------------------------- row 8
    rep.check(diff_case(&apis, "row08-scene-create-null", &|api, ctx| unsafe {
        let s = (api.scene_create)(std::ptr::null());
        ctx.line("scene_create(NULL):");
        ctx.dump_scene(s);
        (api.scene_print)(s);
        (api.scene_list_shapes)(s);
    }));

    // ---------------------------------------------------------------- row 9
    rep.check(diff_case(&apis, "row09-scene-create-names", &|api, ctx| unsafe {
        for len in [0usize, 1, 2, 30, 62, 63, 64, 65, 100, 200] {
            let mut name: Vec<u8> = Vec::new();
            for i in 0..len {
                name.push(b'a' + (i % 26) as u8);
            }
            let c = cs(&name);
            let s = (api.scene_create)(c.as_ptr());
            ctx.line(format!("scene_create(len {}):", len));
            ctx.dump_scene(s);
            ctx.dump_scene_name_raw(s);
            (api.scene_print)(s);
        }
    }));

    // --------------------------------------------------------------- row 10
    rep.check(diff_case(&apis, "row10-scene-create-rnd", &|api, ctx| unsafe {
        let mut rng = Rng::new(SEED ^ 10);
        for k in 0..64 {
            let len = rng.below(201);
            let mut name: Vec<u8> = Vec::with_capacity(len);
            for _ in 0..len {
                // any non-NUL byte, biased towards the interesting ones
                let b = match rng.below(8) {
                    0 => b'%',
                    1 => b'\\',
                    2 => b'"',
                    3 => b'\t',
                    4 => b' ',
                    5 => 0x80 | (rng.byte() & 0x7f),
                    _ => rng.byte(),
                };
                name.push(if b == 0 { b'.' } else { b });
            }
            let c = cs(&name);
            let s = (api.scene_create)(c.as_ptr());
            ctx.line(format!("iter {} len {}:", k, len));
            ctx.dump_scene(s);
            ctx.dump_scene_name_raw(s);
            (api.scene_print)(s);
            (api.scene_list_shapes)(s);
        }
    }));

    // --------------------------------------------------------------- row 11
    rep.check(diff_case(&apis, "row11-add-shape", &|api, ctx| unsafe {
        (api.shape_manager_init)();
        let name = cs(b"Add");
        let s = (api.scene_create)(name.as_ptr());
        ctx.dump_scene(s);
        for t in 0..5 {
            let r = (api.scene_add_shape)(s, (api.shape_get)(t));
            ctx.line(format!("scene_add_shape(type {}) = {}", t, r));
            ctx.dump_scene(s);
        }
        (api.scene_print)(s);
        (api.scene_list_shapes)(s);
    }));

    // --------------------------------------------------------------- row 12
    rep.check(diff_case(&apis, "row12-add-50", &|api, ctx| unsafe {
        (api.shape_manager_init)();
        let name = cs(b"Full");
        let s = (api.scene_create)(name.as_ptr());
        for i in 0..50 {
            let r = (api.scene_add_shape)(s, (api.shape_get)(i % 10));
            ctx.line(format!("add {} = {}", i, r));
        }
        ctx.dump_scene(s);
        (api.scene_list_shapes)(s);
    }));

    // --------------------------------------------------------------- row 13
    rep.check(diff_case(&apis, "row13-add-duplicates", &|api, ctx| unsafe {
        (api.shape_manager_init)();
        let name = cs(b"Dup");
        let s = (api.scene_create)(name.as_ptr());
        for _ in 0..5 {
            let r = (api.scene_add_shape)(s, (api.shape_get)(3));
            ctx.line(format!("add Sun = {}", r));
        }
        ctx.dump_scene(s);
        (api.scene_print)(s);
        (api.scene_list_shapes)(s);
    }));

    // --------------------------------------------------------------- row 14
    rep.check(diff_case(&apis, "row14-remove-first", &|api, ctx| unsafe {
        (api.shape_manager_init)();
        let name = cs(b"Rm");
        let s = (api.scene_create)(name.as_ptr());
        for t in 0..3 {
            (api.scene_add_shape)(s, (api.shape_get)(t));
        }
        ctx.dump_scene(s);
        let r = (api.scene_remove_shape)(s, 0);
        ctx.line(format!("scene_remove_shape(0) = {}", r));
        ctx.dump_scene(s);
        (api.scene_list_shapes)(s);
    }));

    // --------------------------------------------------------------- row 15
    rep.check(diff_case(&apis, "row15-remove-mid-last", &|api, ctx| unsafe {
        (api.shape_manager_init)();
        let name = cs(b"Rm2");
        let s = (api.scene_create)(name.as_ptr());
        for t in 0..5 {
            (api.scene_add_shape)(s, (api.shape_get)(t));
        }
        ctx.dump_scene(s);
        let r = (api.scene_remove_shape)(s, 2);
        ctx.line(format!("remove(2) = {}", r));
        ctx.dump_scene(s);
        let last = (*s).shape_count - 1;
        let r = (api.scene_remove_shape)(s, last);
        ctx.line(format!("remove(last {}) = {}", last, r));
        ctx.dump_scene(s);
        while (*s).shape_count > 0 {
            let r = (api.scene_remove_shape)(s, 0);
            ctx.line(format!("drain remove(0) = {}", r));
            ctx.dump_scene(s);
        }
        (api.scene_print)(s);
    }));

    // --------------------------------------------------------------- row 16
    rep.check(diff_case(&apis, "row16-remove-rnd", &|api, ctx| unsafe {
        (api.shape_manager_init)();
        let mut rng = Rng::new(SEED ^ 16);
        for k in 0..32 {
            let name = cs(format!("rnd{}", k).as_bytes());
            let s = (api.scene_create)(name.as_ptr());
            let n = 1 + rng.below(50);
            for _ in 0..n {
                let t = rng.range_i32(0, 9);
                (api.scene_add_shape)(s, (api.shape_get)(t));
            }
            ctx.line(format!("iter {} filled {}", k, n));
            ctx.dump_scene(s);
            while (*s).shape_count > 0 {
                let idx = rng.below((*s).shape_count as usize) as i32;
                let r = (api.scene_remove_shape)(s, idx);
                ctx.line(format!("remove({}) = {}", idx, r));
                ctx.dump_scene(s);
            }
        }
    }));

    // --------------------------------------------------------------- row 17
    rep.check(diff_case(&apis, "row17-scene-print-counts", &|api, ctx| unsafe {
        (api.shape_manager_init)();
        for n in [0usize, 1, 3, 50] {
            let name = cs(format!("Scene{}", n).as_bytes());
            let s = (api.scene_create)(name.as_ptr());
            for i in 0..n {
                (api.scene_add_shape)(s, (api.shape_get)((i % 10) as i32));
            }
            ctx.line(format!("scene_print with {} shapes:", n));
            (api.scene_print)(s);
        }
    }));

    // --------------------------------------------------------------- row 18
    rep.check(diff_case(&apis, "row18-scene-print-names", &|api, ctx| unsafe {
        (api.shape_manager_init)();
        let names: [&[u8]; 6] = [
            b"100%% sure",
            b"back\\slash",
            b"  spaced  ",
            b"tab\there",
            &[0xff, 0xfe, b'x', 0x80],
            b"%s%d%n",
        ];
        for name in names {
            let c = cs(name);
            let s = (api.scene_create)(c.as_ptr());
            (api.scene_add_shape)(s, (api.shape_get)(0));
            (api.scene_add_shape)(s, (api.shape_get)(9));
            ctx.line(format!("name = \"{}\"", escape(name)));
            ctx.dump_scene(s);
            (api.scene_print)(s);
            (api.scene_list_shapes)(s);
        }
    }));

    // --------------------------------------------------------------- row 19
    rep.check(diff_case(&apis, "row19-list-shapes", &|api, ctx| unsafe {
        (api.shape_manager_init)();
        for n in [0usize, 1, 3, 50] {
            let name = cs(format!("L{}", n).as_bytes());
            let s = (api.scene_create)(name.as_ptr());
            for i in 0..n {
                (api.scene_add_shape)(s, (api.shape_get)((i % 10) as i32));
            }
            ctx.line(format!("scene_list_shapes with {} shapes:", n));
            (api.scene_list_shapes)(s);
        }
    }));

    // --------------------------------------------------------------- row 20
    rep.check(diff_case(&apis, "row20-equals-permutation", &|api, ctx| unsafe {
        (api.shape_manager_init)();
        let a = cs(b"A");
        let b = cs(b"B");
        let s1 = (api.scene_create)(a.as_ptr());
        let s2 = (api.scene_create)(b.as_ptr());
        for t in [0, 3, 7, 9] {
            (api.scene_add_shape)(s1, (api.shape_get)(t));
        }
        for t in [9, 0, 7, 3] {
            (api.scene_add_shape)(s2, (api.shape_get)(t));
        }
        ctx.dump_scene(s1);
        ctx.dump_scene(s2);
        ctx.line(format!("equals(s1,s2) = {}", (api.scene_equals)(s1, s2)));
        ctx.line(format!("equals(s2,s1) = {}", (api.scene_equals)(s2, s1)));
        ctx.line(format!("equals(s1,s1) = {}", (api.scene_equals)(s1, s1)));
    }));

    // --------------------------------------------------------------- row 21
    rep.check(diff_case(&apis, "row21-equals-variants", &|api, ctx| unsafe {
        (api.shape_manager_init)();
        let mk = |api: &Api, ctx: &mut Ctx, tag: &str, types: &[i32]| {
            let n = cs(tag.as_bytes());
            let s = (api.scene_create)(n.as_ptr());
            for &t in types {
                (api.scene_add_shape)(s, (api.shape_get)(t));
            }
            ctx.dump_scene(s);
            s
        };
        let cases: [(&str, &[i32], &[i32]); 6] = [
            ("dup-vs-distinct", &[1, 1, 2], &[1, 2, 2]),
            ("dup-vs-same-dup", &[1, 1, 2], &[1, 1, 2]),
            ("subset", &[1, 2], &[1, 2, 3]),
            ("superset", &[1, 2, 3], &[1, 2]),
            ("disjoint", &[0, 1, 2], &[3, 4, 5]),
            ("same-single", &[5], &[5]),
        ];
        for (tag, l, r) in cases {
            ctx.line(format!("case {}", tag));
            let s1 = mk(api, ctx, tag, l);
            let s2 = mk(api, ctx, tag, r);
            ctx.line(format!("  equals(l,r) = {}", (api.scene_equals)(s1, s2)));
            ctx.line(format!("  equals(r,l) = {}", (api.scene_equals)(s2, s1)));
        }
    }));

    // --------------------------------------------------------------- row 22
    rep.check(diff_case(&apis, "row22-equals-rnd", &|api, ctx| unsafe {
        (api.shape_manager_init)();
        let mut rng = Rng::new(SEED ^ 22);
        for k in 0..48 {
            let n1 = rng.below(8);
            let n2 = if rng.below(2) == 0 { n1 } else { rng.below(8) };
            let name = cs(format!("p{}", k).as_bytes());
            let s1 = (api.scene_create)(name.as_ptr());
            let s2 = (api.scene_create)(name.as_ptr());
            for _ in 0..n1 {
                (api.scene_add_shape)(s1, (api.shape_get)(rng.range_i32(0, 9)));
            }
            for _ in 0..n2 {
                (api.scene_add_shape)(s2, (api.shape_get)(rng.range_i32(0, 9)));
            }
            ctx.line(format!("iter {}", k));
            ctx.dump_scene(s1);
            ctx.dump_scene(s2);
            ctx.line(format!("  equals(1,2) = {}", (api.scene_equals)(s1, s2)));
            ctx.line(format!("  equals(2,1) = {}", (api.scene_equals)(s2, s1)));
        }
    }));

    // --------------------------------------------------------------- row 23
    rep.check(diff_case(&apis, "row23-equals-empty", &|api, ctx| unsafe {
        let a = cs(b"e1");
        let b = cs(b"e2");
        let s1 = (api.scene_create)(a.as_ptr());
        let s2 = (api.scene_create)(b.as_ptr());
        ctx.line(format!("equals(empty,empty) = {}", (api.scene_equals)(s1, s2)));
        ctx.line(format!("equals(s1,s1) = {}", (api.scene_equals)(s1, s1)));
    }));

    // --------------------------------------------------------------- row 24
    rep.check(diff_case(&apis, "row24-save-counts", &|api, ctx| unsafe {
        (api.shape_manager_init)();
        for n in [0usize, 1, 3, 50] {
            let name = cs(format!("Save{}", n).as_bytes());
            let s = (api.scene_create)(name.as_ptr());
            for i in 0..n {
                (api.scene_add_shape)(s, (api.shape_get)((i % 10) as i32));
            }
            let path = ctx.path(&format!("save{}.txt", n));
            let r = (api.scene_save)(s, path.as_ptr());
            ctx.line(format!("scene_save({} shapes) = {}", n, r));
        }
    }));

    // --------------------------------------------------------------- row 25
    rep.check(diff_case(&apis, "row25-save-names", &|api, ctx| unsafe {
        (api.shape_manager_init)();
        let names: [&[u8]; 4] = [
            b"x",
            b"a name with spaces",
            b"012345678901234567890123456789012345678901234567890123456789012345678901234567890",
            b"%d percent",
        ];
        for (i, name) in names.iter().enumerate() {
            let c = cs(name);
            let s = (api.scene_create)(c.as_ptr());
            (api.scene_add_shape)(s, (api.shape_get)(2));
            (api.scene_add_shape)(s, (api.shape_get)(8));
            let path = ctx.path(&format!("n{}.txt", i));
            let r = (api.scene_save)(s, path.as_ptr());
            ctx.line(format!("save name {} = {}", i, r));
            // also with a file name that contains spaces
            let path2 = ctx.path(&format!("with space {}.txt", i));
            let r2 = (api.scene_save)(s, path2.as_ptr());
            ctx.line(format!("save spaced name {} = {}", i, r2));
        }
    }));

    // --------------------------------------------------------------- row 26
    rep.check(diff_case(&apis, "row26-round-trip", &|api, ctx| unsafe {
        (api.shape_manager_init)();
        for n in [0usize, 1, 3, 50] {
            let name = cs(format!("Trip{}", n).as_bytes());
            let s = (api.scene_create)(name.as_ptr());
            for i in 0..n {
                (api.scene_add_shape)(s, (api.shape_get)((i % 10) as i32));
            }
            let path = ctx.path(&format!("t{}.txt", n));
            ctx.line(format!("save = {}", (api.scene_save)(s, path.as_ptr())));
            let loaded = (api.scene_load)(path.as_ptr());
            ctx.line(format!("loaded {} shapes:", n));
            ctx.dump_scene(loaded);
            (api.scene_print)(loaded);
            ctx.line(format!(
                "equals(orig,loaded) = {}",
                (api.scene_equals)(s, loaded)
            ));
        }
    }));

    // --------------------------------------------------------------- row 27
    rep.check(diff_case(&apis, "row27-load-file-shapes", &|api, ctx| unsafe {
        (api.shape_manager_init)();
        let files: [(&str, &[u8]); 10] = [
            ("plain", b"Name\n2\n0\n1\n"),
            ("crlf", b"Name\r\n2\r\n0\r\n1\r\n"),
            ("no-trailing-nl", b"Name\n2\n0\n1"),
            ("blank-lines", b"Name\n\n2\n\n0\n\n1\n\n"),
            ("spaces", b"Name\n  2  \n   0    1   \n"),
            ("junk-after", b"Name\n1\n5\ngarbage\nmore\n"),
            ("tabs", b"Name\n\t2\t\n\t3\t4\t\n"),
            ("count-zero", b"Empty\n0\n"),
            ("plus-signs", b"Name\n+2\n+3\n+4\n"),
            ("all-types", b"All\n10\n0\n1\n2\n3\n4\n5\n6\n7\n8\n9\n"),
        ];
        for (tag, content) in files {
            let fname = format!("{}.dat", tag);
            ctx.write_file(&fname, content);
            let path = ctx.path(&fname);
            ctx.line(format!("load {}: {}", tag, escape(content)));
            let s = (api.scene_load)(path.as_ptr());
            ctx.dump_scene(s);
            (api.scene_print)(s);
            (api.scene_list_shapes)(s);
        }
    }));

    // --------------------------------------------------------------- row 28
    rep.check(diff_case(&apis, "row28-load-long-name", &|api, ctx| unsafe {
        (api.shape_manager_init)();
        for len in [62usize, 63, 64, 65, 100, 200] {
            let mut content: Vec<u8> = vec![b'N'; len];
            content.extend_from_slice(b"\n2\n1\n2\n");
            let fname = format!("long{}.dat", len);
            ctx.write_file(&fname, &content);
            let path = ctx.path(&fname);
            ctx.line(format!("name line of {} bytes:", len));
            let s = (api.scene_load)(path.as_ptr());
            ctx.dump_scene(s);
            if !s.is_null() {
                ctx.dump_scene_name_raw(s);
            }
        }
    }));

    // --------------------------------------------------------------- row 29
    rep.check(diff_case(&apis, "row29-load-rnd", &|api, ctx| unsafe {
        (api.shape_manager_init)();
        let mut rng = Rng::new(SEED ^ 29);
        for k in 0..48 {
            let mut content: Vec<u8> = Vec::new();
            let name_len = rng.below(70);
            for i in 0..name_len {
                content.push(b'A' + (i % 26) as u8);
            }
            content.push(b'\n');
            let count = rng.range_i32(-3, 60);
            content.extend_from_slice(format!("{}", count).as_bytes());
            content.push(b'\n');
            let lines = rng.below(62);
            for _ in 0..lines {
                let t = match rng.below(6) {
                    0 => rng.range_i32(-100, -1),
                    1 => rng.range_i32(10, 100),
                    _ => rng.range_i32(0, 9),
                };
                content.extend_from_slice(format!("{}", t).as_bytes());
                match rng.below(4) {
                    0 => content.extend_from_slice(b"\r\n"),
                    1 => content.extend_from_slice(b" \n"),
                    2 => content.extend_from_slice(b"\n\n"),
                    _ => content.push(b'\n'),
                }
            }
            let fname = format!("r{}.dat", k);
            ctx.write_file(&fname, &content);
            let path = ctx.path(&fname);
            ctx.line(format!("iter {} file = \"{}\"", k, escape(&content)));
            let s = (api.scene_load)(path.as_ptr());
            ctx.dump_scene(s);
            (api.scene_print)(s);
        }
    }));

    // --------------------------------------------------------------- row 30
    rep.check(diff_case(&apis, "row30-destroy", &|api, ctx| unsafe {
        (api.shape_manager_init)();
        let n1 = cs(b"empty");
        let s1 = (api.scene_create)(n1.as_ptr());
        ctx.dump_scene(s1);
        (api.scene_destroy)(s1);
        ctx.line("destroyed empty scene");
        ctx.forget_ptrs();

        let n2 = cs(b"filled");
        let s2 = (api.scene_create)(n2.as_ptr());
        for t in 0..4 {
            (api.scene_add_shape)(s2, (api.shape_get)(t));
        }
        ctx.dump_scene(s2);
        (api.scene_destroy)(s2);
        ctx.line("destroyed filled scene");
        ctx.forget_ptrs();

        // the singletons must still be intact
        for t in TYPES {
            (api.shape_print)((api.shape_get)(t));
        }
    }));

    // --------------------------------------------------------------- row 53
    // The public headers expose both structs, so an external caller may write to
    // their fields.  Every function must therefore read the *struct*, not some
    // state cached beside it.
    rep.check(diff_case(&apis, "row53-caller-mutates-scene", &|api, ctx| unsafe {
        (api.shape_manager_init)();
        let name = cs(b"Mutated");
        let s = (api.scene_create)(name.as_ptr());
        for t in 0..5 {
            (api.scene_add_shape)(s, (api.shape_get)(t));
        }
        // shrink the scene behind the library's back
        (*s).shape_count = 2;
        ctx.dump_scene(s);
        (api.scene_print)(s);
        (api.scene_list_shapes)(s);
        let path = ctx.path("shrunk.txt");
        ctx.line(format!("save = {}", (api.scene_save)(s, path.as_ptr())));

        // rewrite the name in place (shorter, longer, non-UTF-8)
        let raw = std::ptr::addr_of_mut!((*s).name) as *mut u8;
        for (i, b) in b"Zed\0".iter().enumerate() {
            *raw.add(i) = *b;
        }
        (api.scene_print)(s);
        for (i, b) in [0xffu8, 0xfe, b'%', b's', 0].iter().enumerate() {
            *raw.add(i) = *b;
        }
        (api.scene_print)(s);
        (api.scene_list_shapes)(s);
        let path2 = ctx.path("renamed.txt");
        ctx.line(format!("save = {}", (api.scene_save)(s, path2.as_ptr())));

        // reorder the shapes array by hand and compare with a fresh scene
        *scene_slot(s, 0) = (api.shape_get)(9);
        *scene_slot(s, 1) = (api.shape_get)(8);
        ctx.dump_scene(s);
        let other = cs(b"Other");
        let s2 = (api.scene_create)(other.as_ptr());
        (api.scene_add_shape)(s2, (api.shape_get)(8));
        (api.scene_add_shape)(s2, (api.shape_get)(9));
        ctx.line(format!("equals = {}", (api.scene_equals)(s, s2)));
        (api.scene_print)(s);
    }));

    // --------------------------------------------------------------- row 54
    rep.check(diff_case(&apis, "row54-caller-mutates-shape", &|api, ctx| unsafe {
        (api.shape_manager_init)();
        let sh = (api.shape_get)(0); // Tree, height 7
        for h in [0i32, 1, 3, 7] {
            (*sh).height = h;
            ctx.line(format!("height = {}", h));
            (api.shape_print)(sh);
            ctx.dump_shape(sh);
        }
        (*sh).height = 7;
        // rename the singleton
        let raw = std::ptr::addr_of_mut!((*sh).name) as *mut u8;
        for (i, b) in b"Zed\0".iter().enumerate() {
            *raw.add(i) = *b;
        }
        (api.shape_print)(sh);
        // change its type: scene_save writes shape->type
        (*sh).type_ = 7;
        let name = cs(b"TypeChanged");
        let s = (api.scene_create)(name.as_ptr());
        (api.scene_add_shape)(s, sh);
        let path = ctx.path("typed.txt");
        ctx.line(format!("save = {}", (api.scene_save)(s, path.as_ptr())));
        (api.scene_list_shapes)(s);
        // patch one art row (the buffer is 80 bytes wide)
        let row = (std::ptr::addr_of_mut!((*sh).art) as *mut u8).add(2 * MAX_SHAPE_WIDTH);
        for (i, b) in b"patched row\0".iter().enumerate() {
            *row.add(i) = *b;
        }
        (api.shape_print)(sh);
        ctx.dump_shape(sh);
        // restore the singleton so later cases are unaffected
        (api.shape_manager_init)();
    }));

    // --------------------------------------------------------------- row 55
    rep.check(diff_case(&apis, "row55-save-overwrites", &|api, ctx| unsafe {
        (api.shape_manager_init)();
        let a = cs(b"First scene with a long name");
        let s1 = (api.scene_create)(a.as_ptr());
        for t in 0..6 {
            (api.scene_add_shape)(s1, (api.shape_get)(t));
        }
        let path = ctx.path("same.txt");
        ctx.line(format!("save 1 = {}", (api.scene_save)(s1, path.as_ptr())));
        let b = cs(b"2nd");
        let s2 = (api.scene_create)(b.as_ptr());
        ctx.line(format!("save 2 = {}", (api.scene_save)(s2, path.as_ptr())));
        // the file must have been truncated, not appended to
        let loaded = (api.scene_load)(path.as_ptr());
        ctx.dump_scene(loaded);
    }));

    // --------------------------------------------------------------- row 56
    rep.check(diff_case(&apis, "row56-load-non-utf8-name", &|api, ctx| unsafe {
        (api.shape_manager_init)();
        let content: Vec<u8> = [
            &[0xff, 0xfe, 0x80, b'%', b's', b' ', 0x01, 0x7f][..],
            b"\n2\n3\n4\n",
        ]
        .concat();
        ctx.write_file("weird.dat", &content);
        let path = ctx.path("weird.dat");
        let s = (api.scene_load)(path.as_ptr());
        ctx.dump_scene(s);
        ctx.dump_scene_name_raw(s);
        (api.scene_print)(s);
        (api.scene_list_shapes)(s);
        let out = ctx.path("weird-out.dat");
        ctx.line(format!("save = {}", (api.scene_save)(s, out.as_ptr())));
    }));

    rep.finish("CONFIGS.md rows 1-30 + 53-56 (library entry points)");
}
