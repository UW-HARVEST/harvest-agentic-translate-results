//! Phase B — valid-path differential tests for the application level entry
//! points exported by `main.c` (rows 31-52 of `CONFIGS.md`).
//!
//! Each case runs one scenario (a list of exported functions plus the `stdin`
//! they consume) in a fresh child process per implementation
//! (`examples/diffharness.rs`, which loads the shared objects with
//! `libloading`), and compares the exit status, the harness results, `stdout`,
//! `stderr` and every file the run created.

mod common;

use common::*;

const SEED: u64 = 0x5EED_2026;

/// Scenarios whose `stdin` runs out while the C code is inside
/// `while (getchar() != '\n');` never terminate - in the C original as well as
/// in the translation.  Those get a short timeout; both sides are then compared
/// including the fact that they were killed and what they had flushed.
const HANG_MS: u64 = 800;

fn names(n: usize) -> Vec<u8> {
    let mut s = Vec::new();
    for i in 0..n {
        s.extend_from_slice(format!("Scene{}\n", i).as_bytes());
    }
    s
}

#[test]
fn configs_app() {
    let apis = load_apis();
    let mut rep = Report::new();

    // --------------------------------------------------------------- row 31
    rep.check(diff_app(&apis, "a-row31-print-menu", &["print_menu"], b""));
    rep.check(diff_app(
        &apis,
        "a-row31-print-menu-twice",
        &["print_menu", "print_menu"],
        b"",
    ));

    // --------------------------------------------------------------- row 32
    rep.check(diff_app(
        &apis,
        "a-row32-view-all-pristine",
        &["view_all_shapes"],
        b"",
    ));
    rep.check(diff_app(
        &apis,
        "a-row32-view-all-init",
        &["shape_manager_init", "view_all_shapes"],
        b"",
    ));
    rep.check(diff_app(
        &apis,
        "a-row32-view-all-cleanup",
        &[
            "shape_manager_init",
            "view_all_shapes",
            "shape_manager_cleanup",
            "view_all_shapes",
        ],
        b"",
    ));

    // --------------------------------------------------------------- row 33
    for (tag, name) in [
        ("empty", &b"\n"[..]),
        ("one", &b"A\n"[..]),
        ("plain", &b"My Scene\n"[..]),
        ("spaced", &b"  spaced  \n"[..]),
        ("tab", &b"tab\there\n"[..]),
        ("percent", &b"100%s%d\n"[..]),
        ("backslash", &b"back\\slash\n"[..]),
        ("high-bit", &[0xff, 0xfe, 0x80, b'x', b'\n'][..]),
        ("62", &[&[b'n'; 62][..], b"\n"].concat()[..]),
        ("63", &[&[b'n'; 63][..], b"\n"].concat()[..]),
        ("64", &[&[b'n'; 64][..], b"\n"].concat()[..]),
        ("200", &[&[b'n'; 200][..], b"\n"].concat()[..]),
    ] {
        rep.check(diff_app(
            &apis,
            &format!("a-row33-create-{}", tag),
            &["create_new_scene", "create_new_scene", "list_all_scenes"],
            name,
        ));
    }

    // --------------------------------------------------------------- row 34
    rep.check(diff_app(
        &apis,
        "a-row34-create-10",
        &[
            "create_new_scene",
            "create_new_scene",
            "create_new_scene",
            "create_new_scene",
            "create_new_scene",
            "create_new_scene",
            "create_new_scene",
            "create_new_scene",
            "create_new_scene",
            "create_new_scene",
            "list_all_scenes",
        ],
        &names(10),
    ));

    // --------------------------------------------------------------- row 35
    for (tag, input, ms) in [
        ("plain", &b"S\n0\n0\n"[..], APP_TIMEOUT_MS),
        ("spaces", &b"S\n 0 \n 3 \n"[..], APP_TIMEOUT_MS),
        ("plus", &b"S\n+0\n+9\n"[..], APP_TIMEOUT_MS),
        ("two-on-line", &b"S\n0 5\n"[..], APP_TIMEOUT_MS),
        ("split-lines", &b"S\n\n\n0\n\n\n1\n"[..], APP_TIMEOUT_MS),
        ("no-newline", &b"S\n0\n0"[..], HANG_MS),
        ("tabs", &b"S\n\t0\t\n\t7\t\n"[..], APP_TIMEOUT_MS),
    ] {
        rep.check(diff_app_full(
            &apis,
            &format!("a-row35-add-{}", tag),
            &[
                "shape_manager_init",
                "create_new_scene",
                "add_shape_to_scene",
                "view_scene",
                "list_all_scenes",
            ],
            &[input, b"0\n"].concat(),
            &[],
            ms,
        ));
    }
    // every shape type through add_shape_to_scene
    for t in 0..10 {
        rep.check(diff_app(
            &apis,
            &format!("a-row35-add-type{}", t),
            &[
                "shape_manager_init",
                "create_new_scene",
                "add_shape_to_scene",
                "view_scene",
            ],
            format!("T{}\n0\n{}\n0\n", t, t).as_bytes(),
        ));
    }

    // --------------------------------------------------------------- row 36
    {
        let mut fns = vec!["shape_manager_init", "create_new_scene"];
        let mut input: Vec<u8> = b"Fifty\n".to_vec();
        for i in 0..52 {
            fns.push("add_shape_to_scene");
            input.extend_from_slice(format!("0\n{}\n", i % 10).as_bytes());
        }
        fns.push("view_scene");
        input.extend_from_slice(b"0\n");
        fns.push("list_all_scenes");
        rep.check(diff_app(&apis, "a-row36-add-52", &fns, &input));
    }

    // --------------------------------------------------------------- row 37
    for (tag, remove_input) in [
        ("first", &b"1\n"[..]),
        ("middle", &b"3\n"[..]),
        ("last", &b"5\n"[..]),
    ] {
        let mut fns = vec!["shape_manager_init", "create_new_scene"];
        let mut input: Vec<u8> = b"Rm\n".to_vec();
        for t in 0..5 {
            fns.push("add_shape_to_scene");
            input.extend_from_slice(format!("0\n{}\n", t).as_bytes());
        }
        fns.push("remove_shape_from_scene");
        input.extend_from_slice(b"0\n");
        input.extend_from_slice(remove_input);
        fns.push("view_scene");
        input.extend_from_slice(b"0\n");
        rep.check(diff_app(
            &apis,
            &format!("a-row37-remove-{}", tag),
            &fns,
            &input,
        ));
    }
    // drain a scene completely
    {
        let mut fns = vec!["shape_manager_init", "create_new_scene"];
        let mut input: Vec<u8> = b"Drain\n".to_vec();
        for t in 0..3 {
            fns.push("add_shape_to_scene");
            input.extend_from_slice(format!("0\n{}\n", t).as_bytes());
        }
        for _ in 0..4 {
            fns.push("remove_shape_from_scene");
            input.extend_from_slice(b"0\n1\n");
        }
        fns.push("view_scene");
        input.extend_from_slice(b"0\n");
        rep.check(diff_app(&apis, "a-row37-drain", &fns, &input));
    }

    // --------------------------------------------------------------- row 38
    rep.check(diff_app(
        &apis,
        "a-row38-view-one",
        &["shape_manager_init", "create_new_scene", "view_scene"],
        b"Only\n0\n",
    ));
    rep.check(diff_app(
        &apis,
        "a-row38-view-last-of-three",
        &[
            "shape_manager_init",
            "create_new_scene",
            "create_new_scene",
            "create_new_scene",
            "add_shape_to_scene",
            "view_scene",
            "view_scene",
        ],
        b"A\nB\nC\n2\n4\n2\n0\n",
    ));

    // --------------------------------------------------------------- row 39
    for n in [1usize, 2, 10] {
        let mut fns: Vec<&str> = Vec::new();
        for _ in 0..n {
            fns.push("create_new_scene");
        }
        fns.push("list_all_scenes");
        rep.check(diff_app(
            &apis,
            &format!("a-row39-list-{}", n),
            &fns,
            &names(n),
        ));
    }
    // with shapes in some of them
    rep.check(diff_app(
        &apis,
        "a-row39-list-with-shapes",
        &[
            "shape_manager_init",
            "create_new_scene",
            "create_new_scene",
            "add_shape_to_scene",
            "add_shape_to_scene",
            "list_all_scenes",
        ],
        b"A\nB\n1\n3\n1\n4\n",
    ));

    // --------------------------------------------------------------- row 40
    for (tag, fname) in [
        ("plain", &b"out.txt\n"[..]),
        ("spaces", &b"with space.txt\n"[..]),
        ("dots", &b"..out..txt\n"[..]),
        ("long", &[&[b'f'; 200][..], b".txt\n"].concat()[..]),
    ] {
        let mut input: Vec<u8> = b"Save\n0\n5\n".to_vec(); // create + add shape 5
        input.extend_from_slice(b"0\n"); // save: scene index
        input.extend_from_slice(fname);
        rep.check(diff_app(
            &apis,
            &format!("a-row40-save-{}", tag),
            &[
                "shape_manager_init",
                "create_new_scene",
                "add_shape_to_scene",
                "save_scene_to_file",
            ],
            &input,
        ));
    }
    // save the last of several scenes, and an empty scene
    rep.check(diff_app(
        &apis,
        "a-row40-save-last",
        &[
            "shape_manager_init",
            "create_new_scene",
            "create_new_scene",
            "save_scene_to_file",
            "save_scene_to_file",
        ],
        b"A\nB\n1\nb.txt\n0\na.txt\n",
    ));

    // --------------------------------------------------------------- row 41
    rep.check(diff_app_files(
        &apis,
        "a-row41-load-twice",
        &[
            "shape_manager_init",
            "load_scene_from_file",
            "load_scene_from_file",
            "list_all_scenes",
            "view_scene",
        ],
        b"scene.dat\nscene.dat\n1\n",
        &[("scene.dat", b"Loaded\n3\n0\n5\n9\n")],
    ));
    rep.check(diff_app_files(
        &apis,
        "a-row41-load-into-slot9",
        &[
            "shape_manager_init",
            "create_new_scene",
            "create_new_scene",
            "create_new_scene",
            "create_new_scene",
            "create_new_scene",
            "create_new_scene",
            "create_new_scene",
            "create_new_scene",
            "create_new_scene",
            "load_scene_from_file",
            "list_all_scenes",
        ],
        &[names(9).as_slice(), b"scene.dat\n"].concat(),
        &[("scene.dat", b"Ninth\n2\n1\n2\n")],
    ));
    // round trip through the application: save then load
    rep.check(diff_app(
        &apis,
        "a-row41-save-then-load",
        &[
            "shape_manager_init",
            "create_new_scene",
            "add_shape_to_scene",
            "add_shape_to_scene",
            "save_scene_to_file",
            "load_scene_from_file",
            "compare_scenes",
            "list_all_scenes",
        ],
        b"Orig\n0\n2\n0\n8\n0\nrt.txt\nrt.txt\n0\n1\n",
    ));

    // --------------------------------------------------------------- row 42
    for (a, b) in [(0, 0), (0, 1), (9, 9), (3, 7), (5, 5)] {
        rep.check(diff_app(
            &apis,
            &format!("a-row42-compare-{}-{}", a, b),
            &["shape_manager_init", "compare_shapes"],
            format!("{}\n{}\n", a, b).as_bytes(),
        ));
    }

    // --------------------------------------------------------------- row 43
    rep.check(diff_app(
        &apis,
        "a-row43-compare-identical",
        &[
            "shape_manager_init",
            "create_new_scene",
            "create_new_scene",
            "add_shape_to_scene",
            "add_shape_to_scene",
            "compare_scenes",
        ],
        b"A\nB\n0\n3\n1\n3\n0\n1\n",
    ));
    rep.check(diff_app(
        &apis,
        "a-row43-compare-permutation",
        &[
            "shape_manager_init",
            "create_new_scene",
            "create_new_scene",
            "add_shape_to_scene",
            "add_shape_to_scene",
            "add_shape_to_scene",
            "add_shape_to_scene",
            "compare_scenes",
            "compare_scenes",
        ],
        b"A\nB\n0\n1\n0\n2\n1\n2\n1\n1\n0\n1\n1\n0\n",
    ));
    rep.check(diff_app(
        &apis,
        "a-row43-compare-empty",
        &[
            "create_new_scene",
            "create_new_scene",
            "compare_scenes",
            "compare_scenes",
        ],
        b"A\nB\n0\n1\n0\n0\n",
    ));
    rep.check(diff_app(
        &apis,
        "a-row43-compare-different",
        &[
            "shape_manager_init",
            "create_new_scene",
            "create_new_scene",
            "add_shape_to_scene",
            "add_shape_to_scene",
            "compare_scenes",
        ],
        b"A\nB\n0\n0\n1\n9\n0\n1\n",
    ));

    // --------------------------------------------------------------- row 44
    for (tag, input) in [
        ("first", &b"A\nB\nC\n0\n"[..]),
        ("middle", &b"A\nB\nC\n1\n"[..]),
        ("last", &b"A\nB\nC\n2\n"[..]),
    ] {
        rep.check(diff_app(
            &apis,
            &format!("a-row44-delete-{}", tag),
            &[
                "create_new_scene",
                "create_new_scene",
                "create_new_scene",
                "delete_scene",
                "list_all_scenes",
            ],
            input,
        ));
    }
    rep.check(diff_app(
        &apis,
        "a-row44-delete-all",
        &[
            "create_new_scene",
            "create_new_scene",
            "create_new_scene",
            "delete_scene",
            "delete_scene",
            "delete_scene",
            "delete_scene",
            "list_all_scenes",
        ],
        b"A\nB\nC\n0\n0\n0\n0\n",
    ));

    // --------------------------------------------------------------- row 45
    for (tag, input) in [
        ("exit", &b"12\n"[..]),
        ("eof", &b""[..]),
        ("view-then-exit", &b"1\n12\n"[..]),
        ("no-trailing-newline", &b"1"[..]),
        ("only-newline", &b"\n12\n"[..]),
    ] {
        rep.check(diff_app(
            &apis,
            &format!("a-row45-main-{}", tag),
            &["main"],
            input,
        ));
    }

    // --------------------------------------------------------------- row 46
    rep.check(diff_app(
        &apis,
        "a-row46-main-session",
        &["main"],
        b"1\n2\nFarm\n3\n0\n1\n3\n0\n0\n3\n0\n2\n5\n0\n6\n7\n0\nfarm.sav\n\
          8\nfarm.sav\n10\n0\n1\n4\n0\n1\n5\n0\n11\n0\n6\n12\n",
    ));
    rep.check(diff_app(
        &apis,
        "a-row46-main-session2",
        &["main"],
        b"2\nA\n2\nB\n3\n0\n0\n3\n1\n0\n9\n0\n0\n9\n1\n2\n10\n0\n1\n\
          4\n0\n1\n5\n0\n5\n1\n6\n11\n1\n6\n12\n",
    ));

    // --------------------------------------------------------------- row 47
    for (tag, input) in [
        ("leading-space", &b"   6\n  +12\n"[..]),
        ("tabs", &b"\t6\n\t12\n"[..]),
        ("trailing-junk", &b"12abc\n"[..]),
        ("plus", &b"+1\n+12\n"[..]),
        ("long-line", &[b"1", &[b'x'; 300][..], b"\n12\n"].concat()[..]),
        ("many-digits", &b"0000000012\n"[..]),
        ("negative", &b"-1\n12\n"[..]),
    ] {
        rep.check(diff_app(
            &apis,
            &format!("a-row47-main-{}", tag),
            &["main"],
            input,
        ));
    }

    // --------------------------------------------------------------- row 48
    {
        let mut input: Vec<u8> = Vec::new();
        for i in 0..12 {
            input.extend_from_slice(format!("2\nS{}\n", i).as_bytes());
        }
        input.extend_from_slice(b"6\n12\n");
        rep.check(diff_app(&apis, "a-row48-main-12-scenes", &["main"], &input));
    }

    // --------------------------------------------------------------- row 49
    {
        let mut input: Vec<u8> = b"2\nBig\n".to_vec();
        for i in 0..52 {
            input.extend_from_slice(format!("3\n0\n{}\n", i % 10).as_bytes());
        }
        input.extend_from_slice(b"5\n0\n12\n");
        rep.check(diff_app(&apis, "a-row49-main-52-shapes", &["main"], &input));
    }

    // --------------------------------------------------------------- row 50
    for (tag, content) in [
        ("valid", &b"Loaded Scene\n3\n0\n5\n9\n"[..]),
        ("crlf", &b"S\r\n2\r\n1\r\n3\r\n"[..]),
        ("invalid-types", &b"S\n4\n0\n99\n-3\n7\n"[..]),
        ("over-50", &[b"Big\n55\n", &b"0\n".repeat(55)[..]].concat()[..]),
        ("long-name", &[&[b'N'; 100][..], b"\n2\n1\n2\n"].concat()[..]),
        ("spaces", &b"S\n  2  \n   1    2   \n"[..]),
        ("no-trailing-nl", &b"S\n2\n1\n2"[..]),
    ] {
        rep.check(diff_app_files(
            &apis,
            &format!("a-row50-main-load-{}", tag),
            &["main"],
            b"8\nscene.dat\n5\n0\n6\n12\n",
            &[("scene.dat", content)],
        ));
    }

    // --------------------------------------------------------------- row 51
    // Randomised sessions: arbitrary menu choices with arbitrary arguments.
    {
        let mut rng = Rng::new(SEED ^ 51);
        for k in 0..64 {
            let mut input: Vec<u8> = Vec::new();
            let steps = 1 + rng.below(8);
            for _ in 0..steps {
                let choice = rng.range_i32(-2, 14);
                input.extend_from_slice(format!("{}\n", choice).as_bytes());
                // plausible arguments for whatever the choice needs
                let args = rng.below(3);
                for _ in 0..args {
                    match rng.below(6) {
                        0 => input.extend_from_slice(b"abc\n"),
                        1 => input.extend_from_slice(b"\n"),
                        2 => input.extend_from_slice(format!("{}\n", rng.range_i32(-3, 12)).as_bytes()),
                        3 => input.extend_from_slice(b"name with space\n"),
                        4 => input.extend_from_slice(b"f.txt\n"),
                        _ => input.extend_from_slice(format!("{}\n", rng.range_i32(0, 9)).as_bytes()),
                    }
                }
            }
            input.extend_from_slice(b"12\n");
            rep.check(diff_app_full(
                &apis,
                &format!("a-row51-rnd-{}", k),
                &["main"],
                &input,
                &[("scene.dat", b"Seed\n2\n1\n2\n")],
                HANG_MS,
            ));
        }
    }

    // --------------------------------------------------------------- row 52
    // Randomised *well formed* sessions.
    {
        let mut rng = Rng::new(SEED ^ 52);
        for k in 0..64 {
            let mut input: Vec<u8> = Vec::new();
            let mut scenes: Vec<usize> = Vec::new();
            let steps = 3 + rng.below(12);
            for _ in 0..steps {
                let op = rng.below(9);
                match op {
                    0 => {
                        // create
                        input.extend_from_slice(b"2\n");
                        input.extend_from_slice(format!("S{}\n", scenes.len()).as_bytes());
                        if scenes.len() < 10 {
                            scenes.push(0);
                        }
                    }
                    1 if !scenes.is_empty() => {
                        // add shape
                        let idx = rng.below(scenes.len());
                        input.extend_from_slice(
                            format!("3\n{}\n{}\n", idx, rng.range_i32(0, 9)).as_bytes(),
                        );
                        scenes[idx] += 1;
                    }
                    2 if !scenes.is_empty() => {
                        // remove shape
                        let idx = rng.below(scenes.len());
                        let n = scenes[idx];
                        let pick = if n == 0 { 1 } else { 1 + rng.below(n) };
                        input.extend_from_slice(format!("4\n{}\n{}\n", idx, pick).as_bytes());
                        if n > 0 {
                            scenes[idx] -= 1;
                        }
                    }
                    3 if !scenes.is_empty() => {
                        let idx = rng.below(scenes.len());
                        input.extend_from_slice(format!("5\n{}\n", idx).as_bytes());
                    }
                    4 => input.extend_from_slice(b"6\n"),
                    5 if !scenes.is_empty() => {
                        let idx = rng.below(scenes.len());
                        input.extend_from_slice(format!("7\n{}\ns{}.txt\n", idx, idx).as_bytes());
                    }
                    6 if scenes.len() >= 2 => {
                        let a = rng.below(scenes.len());
                        let b = rng.below(scenes.len());
                        input.extend_from_slice(format!("10\n{}\n{}\n", a, b).as_bytes());
                    }
                    7 if !scenes.is_empty() => {
                        let idx = rng.below(scenes.len());
                        input.extend_from_slice(format!("11\n{}\n", idx).as_bytes());
                        scenes.remove(idx);
                    }
                    _ => {
                        input.extend_from_slice(
                            format!("9\n{}\n{}\n", rng.range_i32(0, 9), rng.range_i32(0, 9))
                                .as_bytes(),
                        );
                    }
                }
            }
            input.extend_from_slice(b"1\n6\n12\n");
            rep.check(diff_app_full(
                &apis,
                &format!("a-row52-rnd-{}", k),
                &["main"],
                &input,
                &[],
                HANG_MS,
            ));
        }
    }

    rep.finish("CONFIGS.md rows 31-52 (application entry points)");
}
