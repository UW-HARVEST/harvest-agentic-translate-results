//! Phase C — error-path differential tests for the application level entry
//! points exported by `main.c` (rows 39-71 of `ERRORS.md`, plus the generic FFI
//! boundary rows 72-78 as far as they apply to `main.c`).

mod common;

use common::*;

/// Scenarios that end up in `while (getchar() != '\n');` with `stdin` at EOF
/// never terminate (in the C original as well as in the translation): they are
/// run with a short timeout and compared including their killed status and the
/// bytes they had flushed.
const HANG_MS: u64 = 800;

fn repeat(what: &str, n: usize) -> Vec<&str> {
    let mut v = Vec::new();
    for _ in 0..n {
        v.push(what);
    }
    v
}

#[test]
fn errors_app() {
    let apis = load_apis();
    let mut rep = Report::new();

    // --------------------------------------------------------------- row 39
    {
        let mut fns = repeat("create_new_scene", 11);
        fns.push("list_all_scenes");
        let mut input = Vec::new();
        for i in 0..11 {
            input.extend_from_slice(format!("S{}\n", i).as_bytes());
        }
        rep.check(diff_app(&apis, "e-row39-max-scenes", &fns, &input));
    }

    // --------------------------------------------------------------- row 40
    rep.check(diff_app(
        &apis,
        "e-row40-create-eof",
        &["create_new_scene", "list_all_scenes"],
        b"",
    ));
    rep.check(diff_app(
        &apis,
        "e-row40-create-eof-after-one",
        &["create_new_scene", "create_new_scene", "list_all_scenes"],
        b"A\n",
    ));

    // --------------------------------------------------------------- row 42
    rep.check(diff_app(
        &apis,
        "e-row42-add-no-scenes",
        &["shape_manager_init", "add_shape_to_scene"],
        b"0\n0\n",
    ));

    // ------------------------------------------------------------ rows 43,46
    for (tag, input, ms) in [
        ("scanf1-abc", &b"S\nabc\n0\n"[..], APP_TIMEOUT_MS),
        ("scanf1-empty-line", &b"S\n\n\n\nx\n"[..], APP_TIMEOUT_MS),
        ("scanf2-xyz", &b"S\n0\nxyz\n"[..], APP_TIMEOUT_MS),
        ("scanf1-eof", &b"S\nabc"[..], HANG_MS),
        ("scanf2-eof", &b"S\n0\nzz"[..], HANG_MS),
        ("scanf1-only-sign", &b"S\n-\n0\n"[..], APP_TIMEOUT_MS),
        ("scanf1-dot", &b"S\n.5\n0\n"[..], APP_TIMEOUT_MS),
    ] {
        rep.check(diff_app_full(
            &apis,
            &format!("e-row43-46-add-{}", tag),
            &["shape_manager_init", "create_new_scene", "add_shape_to_scene"],
            input,
            &[],
            ms,
        ));
    }

    // ------------------------------------------------------------ rows 44,45
    for (tag, idx) in [
        ("neg1", "-1"),
        ("neg-big", "-2147483648"),
        ("one-past", "1"),
        ("big", "999"),
        ("int-max", "2147483647"),
        ("huge", "99999999999999999999"),
    ] {
        rep.check(diff_app(
            &apis,
            &format!("e-row44-45-add-bad-scene-{}", tag),
            &["shape_manager_init", "create_new_scene", "add_shape_to_scene"],
            format!("S\n{}\n0\n", idx).as_bytes(),
        ));
    }

    // --------------------------------------------------------------- row 47
    for (tag, t) in [
        ("neg1", "-1"),
        ("ten", "10"),
        ("eleven", "11"),
        ("99", "99"),
        ("int-min", "-2147483648"),
        ("int-max", "2147483647"),
        ("huge", "4294967308"),
    ] {
        rep.check(diff_app(
            &apis,
            &format!("e-row47-add-bad-type-{}", tag),
            &[
                "shape_manager_init",
                "create_new_scene",
                "add_shape_to_scene",
                "view_scene",
            ],
            format!("S\n0\n{}\n0\n", t).as_bytes(),
        ));
    }
    // a valid type but the shape manager was never initialised
    rep.check(diff_app(
        &apis,
        "e-row47-add-without-init",
        &["create_new_scene", "add_shape_to_scene", "view_scene"],
        b"S\n0\n3\n0\n",
    ));

    // --------------------------------------------------------------- row 48
    {
        let mut fns = vec!["shape_manager_init", "create_new_scene"];
        let mut input: Vec<u8> = b"Full\n".to_vec();
        for i in 0..52 {
            fns.push("add_shape_to_scene");
            input.extend_from_slice(format!("0\n{}\n", i % 10).as_bytes());
        }
        rep.check(diff_app(&apis, "e-row48-add-to-full", &fns, &input));
    }

    // --------------------------------------------------------------- row 49
    rep.check(diff_app(
        &apis,
        "e-row49-remove-no-scenes",
        &["remove_shape_from_scene"],
        b"0\n1\n",
    ));

    // ------------------------------------------------------------ rows 50,52
    for (tag, input, ms) in [
        ("scanf1", &b"S\nabc\n1\n"[..], APP_TIMEOUT_MS),
        ("bad-index", &b"S\n7\n"[..], APP_TIMEOUT_MS),
        ("neg-index", &b"S\n-1\n"[..], APP_TIMEOUT_MS),
        ("scanf1-eof", &b"S\nabc"[..], HANG_MS),
    ] {
        rep.check(diff_app_full(
            &apis,
            &format!("e-row50-52-remove-{}", tag),
            &[
                "shape_manager_init",
                "create_new_scene",
                "remove_shape_from_scene",
            ],
            input,
            &[],
            ms,
        ));
    }
    // second scanf fails (the scene has shapes)
    rep.check(diff_app_full(
        &apis,
        "e-row52-remove-scanf2",
        &[
            "shape_manager_init",
            "create_new_scene",
            "add_shape_to_scene",
            "remove_shape_from_scene",
        ],
        b"S\n0\n2\n0\nzz\n",
        &[],
        APP_TIMEOUT_MS,
    ));
    rep.check(diff_app_full(
        &apis,
        "e-row52-remove-scanf2-eof",
        &[
            "shape_manager_init",
            "create_new_scene",
            "add_shape_to_scene",
            "remove_shape_from_scene",
        ],
        b"S\n0\n2\n0\nzz",
        &[],
        HANG_MS,
    ));

    // --------------------------------------------------------------- row 51
    rep.check(diff_app(
        &apis,
        "e-row51-remove-empty-scene",
        &[
            "shape_manager_init",
            "create_new_scene",
            "remove_shape_from_scene",
        ],
        b"S\n0\n",
    ));

    // --------------------------------------------------------------- row 53
    for (tag, pick) in [
        ("zero", "0"),
        ("neg", "-1"),
        ("too-big", "4"),
        ("int-min", "-2147483648"),
        ("int-max", "2147483647"),
    ] {
        rep.check(diff_app(
            &apis,
            &format!("e-row53-remove-bad-index-{}", tag),
            &[
                "shape_manager_init",
                "create_new_scene",
                "add_shape_to_scene",
                "add_shape_to_scene",
                "remove_shape_from_scene",
                "view_scene",
            ],
            format!("S\n0\n1\n0\n2\n0\n{}\n0\n", pick).as_bytes(),
        ));
    }

    // --------------------------------------------------------------- row 54
    rep.check(diff_app(&apis, "e-row54-view-no-scenes", &["view_scene"], b"0\n"));
    for (tag, input, ms) in [
        ("scanf", &b"S\nabc\n"[..], APP_TIMEOUT_MS),
        ("scanf-eof", &b"S\nabc"[..], HANG_MS),
        ("bad-index", &b"S\n3\n"[..], APP_TIMEOUT_MS),
        ("neg-index", &b"S\n-7\n"[..], APP_TIMEOUT_MS),
    ] {
        rep.check(diff_app_full(
            &apis,
            &format!("e-row54-view-{}", tag),
            &["create_new_scene", "view_scene"],
            input,
            &[],
            ms,
        ));
    }

    // --------------------------------------------------------------- row 55
    rep.check(diff_app(
        &apis,
        "e-row55-list-no-scenes",
        &["list_all_scenes", "list_all_scenes"],
        b"",
    ));

    // ------------------------------------------------------------ rows 56,57
    rep.check(diff_app(
        &apis,
        "e-row56-save-no-scenes",
        &["save_scene_to_file"],
        b"0\nx.txt\n",
    ));
    for (tag, input, ms) in [
        ("scanf", &b"S\nabc\nx.txt\n"[..], APP_TIMEOUT_MS),
        ("scanf-eof", &b"S\nabc"[..], HANG_MS),
        ("bad-index", &b"S\n9\nx.txt\n"[..], APP_TIMEOUT_MS),
        ("filename-eof", &b"S\n0\n"[..], APP_TIMEOUT_MS),
        ("empty-filename", &b"S\n0\n\n"[..], APP_TIMEOUT_MS),
        ("dir-filename", &b"S\n0\n.\n"[..], APP_TIMEOUT_MS),
        (
            "missing-dir",
            &b"S\n0\n/nonexistent_dir_xyz/a/b.txt\n"[..],
            APP_TIMEOUT_MS,
        ),
    ] {
        rep.check(diff_app_full(
            &apis,
            &format!("e-row56-57-save-{}", tag),
            &["shape_manager_init", "create_new_scene", "save_scene_to_file"],
            input,
            &[],
            ms,
        ));
    }

    // ------------------------------------------------------- rows 58,59,60
    {
        let mut fns = repeat("create_new_scene", 10);
        fns.push("load_scene_from_file");
        fns.push("list_all_scenes");
        let mut input = Vec::new();
        for i in 0..10 {
            input.extend_from_slice(format!("S{}\n", i).as_bytes());
        }
        input.extend_from_slice(b"scene.dat\n");
        rep.check(diff_app_files(
            &apis,
            "e-row58-load-max-scenes",
            &fns,
            &input,
            &[("scene.dat", b"L\n1\n0\n")],
        ));
    }
    rep.check(diff_app(
        &apis,
        "e-row59-load-filename-eof",
        &["load_scene_from_file", "list_all_scenes"],
        b"",
    ));
    for (tag, input) in [
        ("missing", &b"nope.dat\n"[..]),
        ("empty-name", &b"\n"[..]),
        ("directory", &b".\n"[..]),
    ] {
        rep.check(diff_app(
            &apis,
            &format!("e-row60-load-fail-{}", tag),
            &["shape_manager_init", "load_scene_from_file", "list_all_scenes"],
            input,
        ));
    }
    for (tag, content) in [
        ("empty-file", &b""[..]),
        ("name-only", &b"Only Name\n"[..]),
        ("bad-count", &b"Name\nxyz\n"[..]),
        ("short", &b"S\n5\n1\n2\n"[..]),
    ] {
        rep.check(diff_app_files(
            &apis,
            &format!("e-row60-load-bad-{}", tag),
            &["shape_manager_init", "load_scene_from_file", "list_all_scenes"],
            b"scene.dat\n",
            &[("scene.dat", content)],
        ));
    }

    // --------------------------------------------------------------- row 61
    for (tag, input, ms) in [
        ("first-abc", &b"abc\n0\n"[..], APP_TIMEOUT_MS),
        ("second-xyz", &b"0\nxyz\n"[..], APP_TIMEOUT_MS),
        ("first-eof", &b"abc"[..], HANG_MS),
        ("second-eof", &b"0\nxyz"[..], HANG_MS),
        ("both-empty-lines", &b"\n\n\n\n\nq\n"[..], APP_TIMEOUT_MS),
    ] {
        rep.check(diff_app_full(
            &apis,
            &format!("e-row61-compare-shapes-{}", tag),
            &["shape_manager_init", "compare_shapes"],
            input,
            &[],
            ms,
        ));
    }

    // --------------------------------------------------------------- row 62
    for (tag, a, b) in [
        ("neg-first", "-1", "0"),
        ("neg-second", "0", "-1"),
        ("ten-first", "10", "1"),
        ("ten-second", "1", "10"),
        ("both-bad", "99", "-99"),
        ("int-min", "-2147483648", "0"),
        ("int-max", "0", "2147483647"),
    ] {
        rep.check(diff_app(
            &apis,
            &format!("e-row62-compare-shapes-range-{}", tag),
            &["shape_manager_init", "compare_shapes"],
            format!("{}\n{}\n", a, b).as_bytes(),
        ));
    }

    // --------------------------------------------------------------- row 63
    rep.check(diff_app(
        &apis,
        "e-row63-compare-scenes-none",
        &["compare_scenes"],
        b"0\n1\n",
    ));
    rep.check(diff_app(
        &apis,
        "e-row63-compare-scenes-one",
        &["create_new_scene", "compare_scenes"],
        b"A\n0\n0\n",
    ));

    // ------------------------------------------------------------ rows 64,65
    for (tag, input, ms) in [
        ("scanf1", &b"A\nB\nabc\n0\n"[..], APP_TIMEOUT_MS),
        ("scanf2", &b"A\nB\n0\nzz\n"[..], APP_TIMEOUT_MS),
        ("scanf1-eof", &b"A\nB\nabc"[..], HANG_MS),
        ("scanf2-eof", &b"A\nB\n0\nzz"[..], HANG_MS),
        ("idx1-bad", &b"A\nB\n5\n0\n"[..], APP_TIMEOUT_MS),
        ("idx2-bad", &b"A\nB\n0\n-1\n"[..], APP_TIMEOUT_MS),
        ("both-bad", &b"A\nB\n99\n-99\n"[..], APP_TIMEOUT_MS),
    ] {
        rep.check(diff_app_full(
            &apis,
            &format!("e-row64-65-compare-scenes-{}", tag),
            &["create_new_scene", "create_new_scene", "compare_scenes"],
            input,
            &[],
            ms,
        ));
    }

    // ------------------------------------------------------- rows 66,67,68
    rep.check(diff_app(
        &apis,
        "e-row66-delete-none",
        &["delete_scene"],
        b"0\n",
    ));
    for (tag, input, ms) in [
        ("scanf", &b"A\nabc\n"[..], APP_TIMEOUT_MS),
        ("scanf-eof", &b"A\nabc"[..], HANG_MS),
        ("neg", &b"A\n-1\n"[..], APP_TIMEOUT_MS),
        ("one-past", &b"A\n1\n"[..], APP_TIMEOUT_MS),
        ("int-min", &b"A\n-2147483648\n"[..], APP_TIMEOUT_MS),
    ] {
        rep.check(diff_app_full(
            &apis,
            &format!("e-row67-68-delete-{}", tag),
            &["create_new_scene", "delete_scene", "list_all_scenes"],
            input,
            &[],
            ms,
        ));
    }

    // --------------------------------------------------------------- row 69
    rep.check(diff_app(&apis, "e-row69-main-eof", &["main"], b""));
    rep.check(diff_app(&apis, "e-row69-main-eof-mid", &["main"], b"1\n"));
    rep.check(diff_app(&apis, "e-row69-main-partial-line", &["main"], b"6"));

    // --------------------------------------------------------------- row 70
    for (tag, input) in [
        ("abc", &b"abc\n12\n"[..]),
        ("empty-line", &b"\n12\n"[..]),
        ("spaces", &b"   \n12\n"[..]),
        ("x1", &b"x1\n12\n"[..]),
        ("sign-only", &b"-\n12\n"[..]),
        ("plus-only", &b"+\n12\n"[..]),
        ("dot", &b".\n12\n"[..]),
        ("tab-only", &b"\t\n12\n"[..]),
    ] {
        rep.check(diff_app(
            &apis,
            &format!("e-row70-main-invalid-input-{}", tag),
            &["main"],
            input,
        ));
    }

    // --------------------------------------------------------------- row 71
    for (tag, input) in [
        ("zero", &b"0\n12\n"[..]),
        ("thirteen", &b"13\n12\n"[..]),
        ("neg5", &b"-5\n12\n"[..]),
        ("99", &b"99\n12\n"[..]),
        ("int-max", &b"2147483647\n12\n"[..]),
        ("overflow", &b"2147483648\n12\n"[..]),
        ("overflow2", &b"4294967308\n12\n"[..]),
        ("neg-overflow", &b"-99999999999999999999\n12\n"[..]),
        ("all", &b"0\n13\n99\n-5\n2147483648\n99999999999999999999\n12\n"[..]),
    ] {
        rep.check(diff_app(
            &apis,
            &format!("e-row71-main-invalid-choice-{}", tag),
            &["main"],
            input,
        ));
    }

    // ----------------------------------------------- generic boundary cases
    // A hanging scenario that has flushed more than one stdio buffer (4096
    // bytes) before it blocks: this compares the *flushed prefix* byte for byte,
    // i.e. the buffering behaviour itself.
    rep.check(diff_app_full(
        &apis,
        "e-generic-flush-boundary",
        &[
            "shape_manager_init",
            "create_new_scene",
            "view_all_shapes",
            "view_all_shapes",
            "view_all_shapes",
            "view_all_shapes",
            "view_all_shapes",
            "view_all_shapes",
            "view_all_shapes",
            "view_all_shapes",
            "view_scene",
        ],
        b"S\nabc",
        &[],
        HANG_MS,
    ));
    // 300 byte input lines (the buffers are 64 / 256 bytes)
    rep.check(diff_app(
        &apis,
        "e-generic-long-name-line",
        &["create_new_scene", "create_new_scene", "list_all_scenes"],
        &[&[b'z'; 300][..], b"\n"].concat(),
    ));
    rep.check(diff_app(
        &apis,
        "e-generic-long-filename",
        &["shape_manager_init", "create_new_scene", "save_scene_to_file"],
        &[b"S\n0\n", &[b'y'; 300][..], b"\n"].concat(),
    ));
    rep.check(diff_app(
        &apis,
        "e-generic-long-menu-line",
        &["main"],
        &[b"1", &[b'x'; 300][..], b"\n12\n"].concat(),
    ));
    // ------------------------------------------------------ rows 1, 10 and 41
    // The allocation-failure branches.  `tests/support/failmalloc.c` is
    // LD_PRELOADed into the harness child and fails allocations of one exact
    // size, so `malloc(sizeof(shape_t))` (2444) and `malloc(sizeof(scene_t))`
    // (472) can be made to fail without disturbing anything else.
    {
        let preload = failmalloc_path().to_string_lossy().to_string();
        // row 1: the very first shape allocation fails -> stderr + exit(1)
        for after in [0, 1, 5, 9] {
            rep.check(diff_app_env(
                &apis,
                &format!("e-row01-shape-alloc-fails-after-{}", after),
                &["shape_manager_init", "view_all_shapes"],
                b"",
                &[],
                &[
                    ("LD_PRELOAD", preload.clone()),
                    ("FAILMALLOC_SIZE", "2444".to_string()),
                    ("FAILMALLOC_AFTER", after.to_string()),
                ],
                APP_TIMEOUT_MS,
            ));
        }
        // rows 10 + 41: scene_create returns NULL -> "Error creating scene"
        rep.check(diff_app_env(
            &apis,
            "e-row10-41-scene-alloc-fails",
            &["create_new_scene", "create_new_scene", "list_all_scenes"],
            b"A\nB\n",
            &[],
            &[
                ("LD_PRELOAD", preload.clone()),
                ("FAILMALLOC_SIZE", "472".to_string()),
                ("FAILMALLOC_AFTER", "0".to_string()),
            ],
            APP_TIMEOUT_MS,
        ));
        // the second scene fails to allocate
        rep.check(diff_app_env(
            &apis,
            "e-row10-41-scene-alloc-fails-second",
            &["create_new_scene", "create_new_scene", "list_all_scenes"],
            b"A\nB\n",
            &[],
            &[
                ("LD_PRELOAD", preload.clone()),
                ("FAILMALLOC_SIZE", "472".to_string()),
                ("FAILMALLOC_AFTER", "1".to_string()),
            ],
            APP_TIMEOUT_MS,
        ));
        // row 10 through scene_load: fopen and fgets succeed, scene_create fails
        rep.check(diff_app_env(
            &apis,
            "e-row10-load-scene-alloc-fails",
            &[
                "shape_manager_init",
                "load_scene_from_file",
                "list_all_scenes",
            ],
            b"scene.dat\n",
            &[("scene.dat", b"L\n2\n1\n2\n")],
            &[
                ("LD_PRELOAD", preload.clone()),
                ("FAILMALLOC_SIZE", "472".to_string()),
                ("FAILMALLOC_AFTER", "0".to_string()),
            ],
            APP_TIMEOUT_MS,
        ));
        // and through `main`, where the failure happens mid session
        rep.check(diff_app_env(
            &apis,
            "e-row01-main-shape-alloc-fails",
            &["main"],
            b"1\n12\n",
            &[],
            &[
                ("LD_PRELOAD", preload.clone()),
                ("FAILMALLOC_SIZE", "2444".to_string()),
                ("FAILMALLOC_AFTER", "3".to_string()),
            ],
            APP_TIMEOUT_MS,
        ));
        rep.check(diff_app_env(
            &apis,
            "e-row41-main-scene-alloc-fails",
            &["main"],
            b"2\nA\n2\nB\n6\n12\n",
            &[],
            &[
                ("LD_PRELOAD", preload.clone()),
                ("FAILMALLOC_SIZE", "472".to_string()),
                ("FAILMALLOC_AFTER", "1".to_string()),
            ],
            APP_TIMEOUT_MS,
        ));
        // sanity check: with the interposer loaded but disarmed everything works
        rep.check(diff_app_env(
            &apis,
            "e-row01-failmalloc-disarmed",
            &["shape_manager_init", "view_all_shapes"],
            b"",
            &[],
            &[("LD_PRELOAD", preload.clone())],
            APP_TIMEOUT_MS,
        ));
    }

    // `compare_shapes` validates the *type* but not the pointer `shape_get`
    // returned, so calling it before `shape_manager_init` makes the C code
    // dereference `NULL` (`printf("%s", s1->name)`) and die with `SIGSEGV`.  The
    // translation must reproduce that instead of "helpfully" checking for NULL:
    // the compared transcript contains the killing signal and the (empty,
    // unflushed) output.
    rep.check(diff_app(
        &apis,
        "e-generic-compare-shapes-without-init",
        &["compare_shapes"],
        b"0\n1\n",
    ));
    rep.check(diff_app(
        &apis,
        "e-generic-compare-shapes-after-cleanup",
        &["shape_manager_init", "shape_manager_cleanup", "compare_shapes"],
        b"5\n5\n",
    ));

    // `shape_manager_cleanup` while a scene still references the singletons.
    //
    // NOTE: only the functions that do *not* dereference the (now dangling)
    // `shape_t *` are called afterwards.  `view_scene` would make the C code
    // read `free`d memory (`shape_print(scene->shapes[i])`), i.e. undefined
    // behaviour whose observable output is whatever glibc's allocator happens to
    // have written into the recycled chunk - not a property a translation can or
    // should reproduce.
    rep.check(diff_app(
        &apis,
        "e-generic-cleanup-then-list",
        &[
            "shape_manager_init",
            "create_new_scene",
            "add_shape_to_scene",
            "shape_manager_cleanup",
            "list_all_scenes",
            "shape_manager_init",
            "list_all_scenes",
            "add_shape_to_scene",
        ],
        b"S\n0\n1\n0\n4\n",
    ));

    rep.finish("ERRORS.md rows 39-71 + generic boundaries (application level)");
}
