//! Differential tests: the C program in `../c_src` is the ground truth and the
//! Rust binary produced by this crate must behave identically.
//!
//! Both are executed as subprocesses; stdout, stderr, the exit status and the
//! files left in the working directory are compared for every input.

mod harness;

use harness::{case, check_all, Case};

fn rep(s: &str, n: usize) -> String {
    s.repeat(n)
}

fn cat(parts: &[&[u8]]) -> Vec<u8> {
    let mut v = Vec::new();
    for p in parts {
        v.extend_from_slice(p);
    }
    v
}

// --- input builders -------------------------------------------------------
//
// The menu handlers consume a different number of lines each (`fgets` for
// names and filenames, `scanf` + a getchar drain for numbers), so building the
// byte stream with these helpers keeps a multi-step session correct by
// construction.

/// 1. View all available shapes
const SHAPES: &str = "1\n";
/// 2. Create new scene
fn create(name: &str) -> String {
    format!("2\n{name}\n")
}
/// 3. Add shape to scene
fn add(scene: i32, shape: i32) -> String {
    format!("3\n{scene}\n{shape}\n")
}
/// 4. Remove shape from scene (1-based shape number)
fn remove(scene: i32, shape_no: i32) -> String {
    format!("4\n{scene}\n{shape_no}\n")
}
/// 5. View scene
fn view(scene: i32) -> String {
    format!("5\n{scene}\n")
}
/// 6. List all scenes
const LIST: &str = "6\n";
/// 7. Save scene
fn save(scene: i32, file: &str) -> String {
    format!("7\n{scene}\n{file}\n")
}
/// 8. Load scene
fn load(file: &str) -> String {
    format!("8\n{file}\n")
}
/// 9. Compare two shapes
fn cmp_shapes(a: i32, b: i32) -> String {
    format!("9\n{a}\n{b}\n")
}
/// 10. Compare two scenes
fn cmp_scenes(a: i32, b: i32) -> String {
    format!("10\n{a}\n{b}\n")
}
/// 11. Delete scene
fn del(scene: i32) -> String {
    format!("11\n{scene}\n")
}
/// 12. Exit
const EXIT: &str = "12\n";

/// A scene file as `scene_save` writes it: name line, count line, shape types.
fn scene_file(name: &str, types: &[i32]) -> Vec<u8> {
    let mut s = format!("{name}\n{}\n", types.len());
    for t in types {
        s.push_str(&format!("{t}\n"));
    }
    s.into_bytes()
}

fn join(parts: &[String]) -> String {
    parts.concat()
}

// ---------------------------------------------------------------------------
// main(): the menu loop.  fgets(input, 256, stdin) + sscanf(input, "%d")
// ---------------------------------------------------------------------------

#[test]
fn menu_loop_and_choice_parsing() {
    let cases: Vec<Case> = vec![
        // fgets returns NULL straight away -> the loop breaks, exit 0.
        case("empty_stdin", ""),
        // A blank line is a matching failure for sscanf -> "Invalid input".
        case("blank_line_then_eof", "\n"),
        case("non_numeric", "x\n12\n"),
        case("only_sign", "-\n12\n"),
        case("choice_zero", "0\n12\n"),
        case("choice_13", "13\n12\n"),
        case("choice_negative", "-1\n12\n"),
        case("choice_int_min", "-2147483648\n12\n"),
        case("exit_immediately", "12\n"),
        case("exit_without_trailing_newline", "12"),
        case("digits_then_text", "12abc\n"),
        case("surrounding_spaces", "  12  \n"),
        case("tab_prefix", "\t12\n"),
        case("crlf_line_ending", "12\r\n"),
        // sscanf stops at the NUL, so nothing is converted.
        case("nul_before_digits", cat(&[b"\x0012\n", b"12\n"])),
        case("plus_sign", "+6\n12\n"),
        // "0x3" converts as 0 -> "Invalid choice".
        case("hex_looking", "0x3\n12\n"),
        // %d overflows: glibc clamps to LONG_MAX and truncates to int (-1).
        case("overflow_to_minus_one", "99999999999999999999\n12\n"),
        // Fits in long, truncated to int: 4294967298 -> 2 (create scene).
        case("truncates_to_two", "4294967298\nName\n6\n12\n"),
        // A line longer than the 255-byte fgets buffer is split in two reads.
        case("line_longer_than_buffer", format!("{}\n12\n", rep("1", 300))),
        case("line_exactly_255_digits", format!("{}\n12\n", rep("1", 255))),
        case("many_invalid_choices", format!("{}12\n", rep("0\n", 12))),
        // Every menu entry, in order.
        case(
            "all_menu_entries",
            join(&[
                SHAPES.into(),
                create("S"),
                add(0, 0),
                remove(0, 1),
                view(0),
                LIST.into(),
                save(0, "save.txt"),
                load("save.txt"),
                cmp_shapes(0, 0),
                cmp_scenes(0, 1),
                del(0),
                EXIT.into(),
            ]),
        ),
    ];
    check_all("menu", cases);
}

// ---------------------------------------------------------------------------
// 1. view_all_shapes()
// ---------------------------------------------------------------------------

#[test]
fn view_all_shapes() {
    let cases = vec![
        case("once", "1\n12\n"),
        case("twice", "1\n1\n12\n"),
        case("then_eof", "1\n"),
    ];
    check_all("view_shapes", cases);
}

// ---------------------------------------------------------------------------
// 2. create_new_scene(): fgets(name, MAX_SCENE_NAME=64, stdin)
// ---------------------------------------------------------------------------

#[test]
fn create_scene() {
    let cases = vec![
        case("simple", "2\nMy Scene\n6\n12\n"),
        case("empty_name", "2\n\n6\n5\n0\n12\n"),
        // fgets returns NULL at the name prompt -> silent return.
        case("eof_at_name_prompt", "2"),
        case("eof_at_name_prompt_with_newline", "2\n"),
        case("name_62_chars", format!("2\n{}\n6\n12\n", rep("A", 62))),
        // 63 chars fill the buffer, so the newline is left for the next fgets.
        case("name_63_chars", format!("2\n{}\n6\n12\n", rep("A", 63))),
        // Longer names are truncated and the rest becomes the next menu line.
        case("name_70_chars", format!("2\n{}\n6\n12\n", rep("A", 70))),
        case(
            "name_70_digits_tail",
            format!("2\n{}{}\n6\n12\n", rep("N", 60), "12345678"),
        ),
        case("name_with_cr", "2\nA\r\n6\n5\n0\n12\n"),
        case("name_with_tab", "2\n\tA B\t\n6\n12\n"),
        // strcspn() stops at the NUL, so only "A" becomes the name.
        case("nul_in_name", cat(&[b"2\nA\x00B\n", b"6\n12\n"])),
        case("non_utf8_name", cat(&[b"2\n\xff\xfe raw\n", b"6\n5\n0\n12\n"])),
        // MAX_SCENES == 10.
        case(
            "eleven_scenes",
            format!(
                "{}{LIST}{EXIT}",
                (0..11).map(|i| create(&format!("S{i}"))).collect::<String>()
            ),
        ),
        case(
            "duplicate_names",
            join(&[create("X"), create("X"), LIST.into(), cmp_scenes(0, 1), EXIT.into()]),
        ),
    ];
    check_all("create_scene", cases);
}

// ---------------------------------------------------------------------------
// 3. add_shape_to_scene(): scanf("%d") twice, each followed by a getchar drain
// ---------------------------------------------------------------------------

#[test]
fn add_shape() {
    let cases = vec![
        case("no_scenes", "3\n12\n"),
        case("invalid_scene_input", "2\nA\n3\nx\n12\n"),
        case("invalid_scene_input_sign_only", "2\nA\n3\n+x\n12\n"),
        case("scene_index_negative", "2\nA\n3\n-1\n12\n"),
        case("scene_index_too_large", "2\nA\n3\n1\n12\n"),
        case("scene_index_int_min", "2\nA\n3\n-2147483648\n12\n"),
        case("invalid_shape_input", "2\nA\n3\n0\nq\n12\n"),
        case("shape_type_equals_count", "2\nA\n3\n0\n10\n12\n"),
        case("shape_type_negative", "2\nA\n3\n0\n-5\n12\n"),
        case("shape_type_huge", "2\nA\n3\n0\n99999999999999999999\n12\n"),
        case(
            "first_shape",
            join(&[create("A"), add(0, 0), view(0), EXIT.into()]),
        ),
        case(
            "last_shape",
            join(&[create("A"), add(0, 9), view(0), EXIT.into()]),
        ),
        case(
            "all_shape_types",
            format!(
                "{}{}{}{EXIT}",
                create("A"),
                (0..10).map(|i| add(0, i)).collect::<String>(),
                view(0)
            ),
        ),
        case(
            "same_shape_twice",
            join(&[create("A"), add(0, 4), add(0, 4), view(0), EXIT.into()]),
        ),
        // scanf skips arbitrary whitespace, including newlines.
        case("blank_lines_between_numbers", "2\nA\n3\n\n\n0\n\n0\n5\n0\n12\n"),
        // The getchar drain throws away the rest of the line.
        case("two_numbers_on_one_line", "2\nA\n3\n0 1\n12\n"),
        case("digits_then_text", "2\nA\n3\n0abc\n0\n5\n0\n12\n"),
        // MAX_SHAPES_IN_SCENE == 50: the 51st add fails on stderr and stdout.
        case(
            "fifty_one_shapes",
            format!(
                "{}{}{LIST}{EXIT}",
                create("A"),
                rep(&add(0, 0), 51)
            ),
        ),
        case(
            "fifty_shapes_exactly",
            format!("{}{}{LIST}{EXIT}", create("A"), rep(&add(0, 7), 50)),
        ),
        case(
            "add_to_second_scene",
            join(&[
                create("A"),
                create("B"),
                add(1, 7),
                view(1),
                view(0),
                EXIT.into(),
            ]),
        ),
    ];
    check_all("add_shape", cases);
}

// ---------------------------------------------------------------------------
// 4. remove_shape_from_scene()
// ---------------------------------------------------------------------------

#[test]
fn remove_shape() {
    let cases = vec![
        case("no_scenes", "4\n12\n"),
        case("invalid_scene_input", "2\nA\n4\nx\n12\n"),
        case("scene_index_negative", "2\nA\n4\n-3\n12\n"),
        case("scene_index_too_large", "2\nA\n4\n9\n12\n"),
        // An empty scene is listed first, then rejected.
        case("empty_scene", "2\nA\n4\n0\n12\n"),
        case(
            "invalid_shape_input",
            format!("{}{}4\n0\nz\n{EXIT}", create("A"), add(0, 0)),
        ),
        case(
            "remove_only_shape",
            join(&[create("A"), add(0, 0), remove(0, 1), view(0), EXIT.into()]),
        ),
        // index 0 becomes -1 in scene_remove_shape -> error.
        case(
            "remove_index_zero",
            join(&[create("A"), add(0, 0), remove(0, 0), EXIT.into()]),
        ),
        case(
            "remove_index_past_end",
            join(&[create("A"), add(0, 0), remove(0, 2), EXIT.into()]),
        ),
        case(
            "remove_index_negative",
            join(&[create("A"), add(0, 0), remove(0, -7), EXIT.into()]),
        ),
        // shape_idx - 1 overflows to INT_MAX.
        case(
            "remove_index_int_min",
            join(&[create("A"), add(0, 0), remove(0, -2147483648), EXIT.into()]),
        ),
        case(
            "remove_middle_then_view",
            join(&[
                create("A"),
                add(0, 0),
                add(0, 1),
                add(0, 2),
                remove(0, 2),
                view(0),
                EXIT.into(),
            ]),
        ),
        case(
            "remove_first_then_view",
            join(&[
                create("A"),
                add(0, 0),
                add(0, 1),
                remove(0, 1),
                view(0),
                EXIT.into(),
            ]),
        ),
        case(
            "remove_last_then_view",
            join(&[
                create("A"),
                add(0, 0),
                add(0, 1),
                remove(0, 2),
                view(0),
                EXIT.into(),
            ]),
        ),
        case(
            "remove_until_empty",
            join(&[
                create("A"),
                add(0, 0),
                add(0, 1),
                remove(0, 1),
                remove(0, 1),
                remove(0, 1),
                EXIT.into(),
            ]),
        ),
        case(
            "remove_from_fifty",
            format!(
                "{}{}{}{LIST}{EXIT}",
                create("A"),
                rep(&add(0, 3), 50),
                remove(0, 50)
            ),
        ),
    ];
    check_all("remove_shape", cases);
}

// ---------------------------------------------------------------------------
// 5. view_scene() / 6. list_all_scenes()
// ---------------------------------------------------------------------------

#[test]
fn view_and_list_scenes() {
    let cases = vec![
        case("view_no_scenes", "5\n12\n"),
        case("view_invalid_input", "2\nA\n5\nx\n12\n"),
        case("view_bad_index", "2\nA\n5\n4\n12\n"),
        case("view_negative_index", "2\nA\n5\n-1\n12\n"),
        case("view_empty_scene", "2\nA\n5\n0\n12\n"),
        case(
            "view_scene_with_shapes",
            join(&[create("A"), add(0, 0), add(0, 1), view(0), EXIT.into()]),
        ),
        case(
            "view_twice",
            join(&[create("A"), add(0, 7), view(0), view(0), EXIT.into()]),
        ),
        case("list_none", "6\n12\n"),
        case(
            "list_three",
            join(&[create("A"), create("B"), create("C"), LIST.into(), EXIT.into()]),
        ),
        case(
            "list_after_delete",
            join(&[
                create("A"),
                create("B"),
                create("C"),
                del(1),
                LIST.into(),
                EXIT.into(),
            ]),
        ),
        case(
            "list_counts",
            join(&[create("A"), add(0, 0), create("B"), LIST.into(), EXIT.into()]),
        ),
    ];
    check_all("view_list", cases);
}

// ---------------------------------------------------------------------------
// 7. save_scene_to_file() / scene_save()
// ---------------------------------------------------------------------------

#[test]
fn save_scene() {
    let cases = vec![
        case("no_scenes", "7\n12\n"),
        case("invalid_input", "2\nA\n7\nx\n12\n"),
        case("bad_index", "2\nA\n7\n5\n12\n"),
        case("negative_index", "2\nA\n7\n-1\n12\n"),
        // fgets at the filename prompt hits end of file -> silent return.
        case("eof_at_filename", "2\nA\n7\n0\n"),
        // fopen("", "w") fails.
        case("empty_filename", "2\nA\n7\n0\n\n12\n"),
        case(
            "save_empty_scene",
            join(&[create("Empty"), save(0, "empty.txt"), EXIT.into()]),
        ),
        case(
            "save_with_shapes",
            join(&[
                create("With Shapes"),
                add(0, 0),
                add(0, 1),
                add(0, 9),
                save(0, "out.txt"),
                EXIT.into(),
            ]),
        ),
        case(
            "save_fifty_shapes",
            format!(
                "{}{}{}{EXIT}",
                create("Fifty"),
                rep(&add(0, 2), 50),
                save(0, "fifty.txt")
            ),
        ),
        case(
            "overwrite_existing",
            join(&[create("A"), add(0, 2), save(0, "f.txt"), EXIT.into()]),
        )
        .file("f.txt", "much longer previous content\n"),
        case("filename_is_directory", "2\nA\n7\n0\n.\n12\n"),
        case("nonexistent_directory", "2\nA\n7\n0\nsub/f.txt\n12\n"),
        case("readonly_target", "2\nA\n7\n0\nro.txt\n12\n").file_mode("ro.txt", "keep\n", 0o444),
        case(
            "filename_255_chars",
            format!("2\nA\n7\n0\n{}\n6\n12\n", rep("f", 255)),
        ),
        // Only the first 255 bytes are read; the tail becomes the next menu line.
        case(
            "filename_longer_than_buffer",
            format!("2\nA\n7\n0\n{}\n6\n12\n", rep("g", 300)),
        ),
        case(
            "non_utf8_filename",
            cat(&[b"2\nA\n3\n0\n3\n7\n0\n\xff\xfe.txt\n", b"12\n"]),
        ),
        case(
            "name_with_spaces",
            join(&[create("A"), save(0, "with space.txt"), EXIT.into()]),
        ),
        case(
            "save_scene_with_long_name",
            join(&[create(&rep("L", 63)), save(0, "long.txt"), EXIT.into()]),
        ),
        case(
            "save_then_reload",
            join(&[
                create("A"),
                add(0, 5),
                save(0, "rt.txt"),
                load("rt.txt"),
                LIST.into(),
                view(1),
                cmp_scenes(0, 1),
                EXIT.into(),
            ]),
        ),
    ];
    check_all("save_scene", cases);
}

// ---------------------------------------------------------------------------
// 8. load_scene_from_file() / scene_load()
// ---------------------------------------------------------------------------

#[test]
fn load_scene() {
    let fifty: Vec<i32> = (0..50).map(|i| i % 10).collect();
    let fifty_five: Vec<i32> = (0..55).map(|i| i % 10).collect();
    let cases = vec![
        case("missing_file", "8\nnope.txt\n12\n"),
        case("empty_filename", "8\n\n12\n"),
        case("eof_at_filename", "8"),
        case("eof_at_filename_with_newline", "8\n"),
        case("directory", "8\n.\n12\n"),
        // fgets on an empty file returns NULL: no message at all.
        case("empty_file", "8\nf.txt\n6\n12\n").file("f.txt", ""),
        case("name_only", "8\nf.txt\n6\n12\n").file("f.txt", "Only A Name\n"),
        case("name_without_newline", "8\nf.txt\n6\n12\n").file("f.txt", "NoNewline"),
        case("count_not_a_number", "8\nf.txt\n6\n12\n").file("f.txt", "S\nabc\n"),
        case("count_zero", "8\nf.txt\n6\n5\n0\n12\n").file("f.txt", "S\n0\n"),
        case("count_negative", "8\nf.txt\n6\n5\n0\n12\n").file("f.txt", "S\n-3\n"),
        case("count_more_than_entries", "8\nf.txt\n6\n12\n").file("f.txt", "S\n4\n1\n2\n"),
        case("count_junk_suffix", "8\nf.txt\n6\n12\n").file("f.txt", "S\n3junk\n"),
        case("count_with_plus", "8\nf.txt\n6\n5\n0\n12\n").file("f.txt", "S\n+2\n1\n2\n"),
        case("count_with_spaces", "8\nf.txt\n6\n5\n0\n12\n").file("f.txt", "S\n  2  \n 1 \n 2 \n"),
        // 99999999999 truncates to 1215752191, so the load runs out of numbers.
        case("count_overflows_int", "8\nf.txt\n6\n12\n").file("f.txt", "S\n99999999999\n1\n"),
        // fscanf("%d") truncates to int: 4294967298 -> 2, so this file loads.
        case("count_truncates_to_two", "8\nf.txt\n6\n5\n0\n12\n")
            .file("f.txt", "S\n4294967298\n1\n2\n"),
        // 4294967295 -> -1: the loop body never runs, the scene loads empty.
        case("count_truncates_to_minus_one", "8\nf.txt\n6\n5\n0\n12\n")
            .file("f.txt", "S\n4294967295\n1\n2\n"),
        // 4294967296 -> 0 == SHAPE_TREE, so a Tree really is added.
        case("shape_type_truncates_to_zero", "8\nf.txt\n6\n5\n0\n12\n")
            .file("f.txt", "S\n1\n4294967296\n"),
        case("shape_type_out_of_range", "8\nf.txt\n6\n5\n0\n12\n").file("f.txt", "S\n2\n1\n77\n"),
        case("shape_type_negative", "8\nf.txt\n6\n5\n0\n12\n").file("f.txt", "S\n2\n-4\n3\n"),
        case("types_on_one_line", "8\nf.txt\n6\n5\n0\n12\n").file("f.txt", "S\n3\n2 1 3\n"),
        case("crlf_file", "8\nf.txt\n6\n5\n0\n12\n").file("f.txt", "S\r\n2\r\n1\r\n2\r\n"),
        case("trailing_whitespace", "8\nf.txt\n6\n5\n0\n12\n").file("f.txt", "S\n1\n1\n   \n\n"),
        case("extra_trailing_junk", "8\nf.txt\n6\n5\n0\n12\n").file("f.txt", "S\n1\n1\nextra\n"),
        // fgets stops after 63 bytes; the rest of the name line is parsed as the
        // shape count.
        case("name_longer_than_63", "8\nf.txt\n6\n12\n")
            .file("f.txt", format!("{}\n1\n3\n", rep("N", 70))),
        case("name_63_then_digits", "8\nf.txt\n6\n5\n0\n12\n")
            .file("f.txt", format!("{}7\n2\n0\n1\n", rep("N", 63))),
        case("name_exactly_63", "8\nf.txt\n6\n5\n0\n12\n")
            .file("f.txt", format!("{}\n1\n2\n", rep("N", 63))),
        case("nul_in_name", "8\nf.txt\n6\n12\n").file("f.txt", cat(&[b"A\x00B\n1\n0\n"])),
        case("non_utf8_name", "8\nf.txt\n6\n5\n0\n12\n").file("f.txt", cat(&[b"\xff\xfe\n1\n0\n"])),
        case("all_ten_types", "8\nf.txt\n6\n5\n0\n12\n")
            .file("f.txt", scene_file("Every Shape", &(0..10).collect::<Vec<i32>>())),
        case("exactly_fifty", "8\nf.txt\n6\n12\n").file("f.txt", scene_file("Fifty", &fifty)),
        // Past MAX_SHAPES_IN_SCENE scene_add_shape complains on stderr.
        case("fifty_five_shapes", "8\nf.txt\n6\n12\n")
            .file("f.txt", scene_file("TooMany", &fifty_five)),
        case("unreadable_file", "8\nf.txt\n6\n12\n").file_mode("f.txt", "S\n1\n0\n", 0o000),
        case("eleven_loads", format!("{}{LIST}{EXIT}", rep(&load("f.txt"), 11)))
            .file("f.txt", scene_file("Loaded", &[1, 2])),
        case(
            "load_then_add_and_remove",
            join(&[
                load("f.txt"),
                add(0, 4),
                remove(0, 1),
                view(0),
                EXIT.into(),
            ]),
        )
        .file("f.txt", scene_file("Mixed", &[8, 9])),
    ];
    check_all("load_scene", cases);
}

// ---------------------------------------------------------------------------
// 9. compare_shapes()
// ---------------------------------------------------------------------------

#[test]
fn compare_shapes() {
    let cases = vec![
        case("invalid_first_input", "9\nx\n12\n"),
        case("invalid_second_input", "9\n0\ny\n12\n"),
        case("first_type_out_of_range", "9\n10\n0\n12\n"),
        case("second_type_out_of_range", "9\n0\n10\n12\n"),
        case("first_type_negative", "9\n-1\n0\n12\n"),
        case("second_type_negative", "9\n0\n-1\n12\n"),
        case("both_out_of_range", "9\n99\n99\n12\n"),
        case("same_type", "9\n0\n0\n12\n"),
        case("same_type_last", "9\n9\n9\n12\n"),
        case("different_types", "9\n0\n1\n12\n"),
        case("different_types_reversed", "9\n9\n3\n12\n"),
        case("int_min_types", "9\n-2147483648\n0\n12\n"),
        case("overflowing_types", "9\n99999999999999999999\n0\n12\n"),
        case("numbers_on_one_line", "9\n0 5\n1\n12\n"),
        case("twice_in_a_row", "9\n2\n2\n9\n2\n3\n12\n"),
        // Every pair of distinct shapes must report distinct addresses.
        case(
            "all_pairs",
            format!(
                "{}{EXIT}",
                (0..10)
                    .flat_map(|a| (0..10).map(move |b| cmp_shapes(a, b)))
                    .collect::<String>()
            ),
        ),
    ];
    check_all("compare_shapes", cases);
}

// ---------------------------------------------------------------------------
// 10. compare_scenes()
// ---------------------------------------------------------------------------

#[test]
fn compare_scenes() {
    let fifty: Vec<i32> = (0..50).map(|i| i % 10).collect();
    let fifty_rev: Vec<i32> = (0..50).map(|i| 9 - i % 10).collect();
    let fifty_off: Vec<i32> = (0..50).map(|i| i % 9).collect();
    let cases = vec![
        case("no_scenes", "10\n12\n"),
        case("one_scene", "2\nA\n10\n12\n"),
        case("invalid_first_input", "2\nA\n2\nB\n10\nx\n12\n"),
        case("invalid_second_input", "2\nA\n2\nB\n10\n0\ny\n12\n"),
        case("first_index_bad", "2\nA\n2\nB\n10\n5\n0\n12\n"),
        case("second_index_bad", "2\nA\n2\nB\n10\n0\n5\n12\n"),
        case("first_index_negative", "2\nA\n2\nB\n10\n-1\n0\n12\n"),
        case(
            "both_empty",
            join(&[create("A"), create("B"), cmp_scenes(0, 1), EXIT.into()]),
        ),
        case(
            "same_index",
            join(&[create("A"), add(0, 3), create("B"), cmp_scenes(0, 0), EXIT.into()]),
        ),
        case(
            "identical_contents",
            join(&[
                create("A"),
                add(0, 0),
                create("B"),
                add(1, 0),
                cmp_scenes(0, 1),
                EXIT.into(),
            ]),
        ),
        case(
            "permuted_contents",
            join(&[
                create("A"),
                add(0, 0),
                add(0, 1),
                create("B"),
                add(1, 1),
                add(1, 0),
                cmp_scenes(0, 1),
                EXIT.into(),
            ]),
        ),
        case(
            "different_shape_counts",
            join(&[create("A"), add(0, 0), create("B"), cmp_scenes(0, 1), EXIT.into()]),
        ),
        case(
            "same_count_different_shapes",
            join(&[
                create("A"),
                add(0, 0),
                create("B"),
                add(1, 1),
                cmp_scenes(0, 1),
                EXIT.into(),
            ]),
        ),
        // The matched[] bookkeeping means duplicates must line up one for one.
        case(
            "duplicates_matter",
            join(&[
                create("A"),
                add(0, 0),
                add(0, 0),
                create("B"),
                add(1, 0),
                add(1, 1),
                cmp_scenes(0, 1),
                EXIT.into(),
            ]),
        ),
        case(
            "fifty_shapes_permuted",
            join(&[load("f.txt"), load("g.txt"), cmp_scenes(0, 1), EXIT.into()]),
        )
        .file("f.txt", scene_file("F", &fifty))
        .file("g.txt", scene_file("G", &fifty_rev)),
        case(
            "fifty_shapes_different",
            join(&[load("f.txt"), load("g.txt"), cmp_scenes(0, 1), EXIT.into()]),
        )
        .file("f.txt", scene_file("F", &fifty))
        .file("g.txt", scene_file("G", &fifty_off)),
        case(
            "after_removal_becomes_equal",
            join(&[
                load("f.txt"),
                load("g.txt"),
                cmp_scenes(0, 1),
                remove(0, 1),
                cmp_scenes(0, 1),
                EXIT.into(),
            ]),
        )
        .file("f.txt", scene_file("F", &[1, 2, 3]))
        .file("g.txt", scene_file("G", &[2, 3])),
    ];
    check_all("compare_scenes", cases);
}

// ---------------------------------------------------------------------------
// 11. delete_scene()
// ---------------------------------------------------------------------------

#[test]
fn delete_scene() {
    let cases = vec![
        case("no_scenes", "11\n12\n"),
        case("invalid_input", "2\nA\n11\nx\n12\n"),
        case("bad_index", "2\nA\n11\n3\n12\n"),
        case("negative_index", "2\nA\n11\n-1\n12\n"),
        case(
            "delete_only",
            join(&[create("A"), del(0), LIST.into(), EXIT.into()]),
        ),
        case(
            "delete_first_of_three",
            join(&[create("A"), create("B"), create("C"), del(0), LIST.into(), EXIT.into()]),
        ),
        case(
            "delete_middle_of_three",
            join(&[create("A"), create("B"), create("C"), del(1), LIST.into(), EXIT.into()]),
        ),
        case(
            "delete_last_of_three",
            join(&[create("A"), create("B"), create("C"), del(2), LIST.into(), EXIT.into()]),
        ),
        case(
            "delete_all",
            join(&[
                create("A"),
                create("B"),
                del(0),
                del(0),
                LIST.into(),
                view(0),
                EXIT.into(),
            ]),
        ),
        case(
            "delete_then_create",
            join(&[create("A"), create("B"), del(0), create("C"), LIST.into(), EXIT.into()]),
        ),
        case(
            "delete_then_compare",
            join(&[
                create("A"),
                add(0, 0),
                create("B"),
                create("C"),
                add(2, 0),
                del(1),
                cmp_scenes(0, 1),
                EXIT.into(),
            ]),
        ),
        case(
            "ten_scenes_delete_one_then_create",
            format!(
                "{}{}{}{LIST}{EXIT}",
                (0..10).map(|i| create(&format!("S{i}"))).collect::<String>(),
                del(4),
                create("New")
            ),
        ),
        case(
            "delete_scene_with_shapes_then_view",
            join(&[
                create("A"),
                add(0, 0),
                create("B"),
                add(1, 1),
                del(0),
                view(0),
                EXIT.into(),
            ]),
        ),
    ];
    check_all("delete_scene", cases);
}

// ---------------------------------------------------------------------------
// The `while (getchar() != '\n');` drains loop forever once stdin is at end of
// file.  Both programs must hang in exactly the same place, and the bytes that
// glibc had already flushed out of its 4096-byte stdout buffer must match.
// (These inputs deliberately print no `%p` address, so the surviving prefix is
// compared byte for byte without any normalisation.)
// ---------------------------------------------------------------------------

#[test]
fn hangs_at_eof_during_getchar_drain() {
    let cases = vec![
        // scanf fails at EOF -> "Invalid input" -> the drain never sees '\n'.
        case("scanf_eof_first_shape", "9\n").hangs(),
        // A non-numeric token with no newline after it.
        case("no_newline_after_junk", "9\nx").hangs(),
        // A successfully converted number with no newline after it.
        case("no_newline_after_number", "2\nA\n3\n0").hangs(),
        case("no_newline_after_scene_index", "2\nA\n5\n0").hangs(),
        case("delete_index_without_newline", "2\nA\n11\n0").hangs(),
        case("save_index_without_newline", "2\nA\n7\n0").hangs(),
        // More than 4096 bytes of output, so part of the buffer has been
        // written out before the hang.
        case("buffered_prefix_survives", format!("{}9\nx", rep("0\n", 30))).hangs(),
        case("large_buffered_prefix", format!("{}9\nx", rep("1\n", 20))).hangs(),
    ];
    check_all("hangs", cases);
}

// ---------------------------------------------------------------------------
// Longer mixed sessions: the interleaving of fgets and scanf reads on one
// shared stdin is where a translation is most likely to drift.
// ---------------------------------------------------------------------------

#[test]
fn mixed_sessions() {
    let cases = vec![
        case(
            "build_save_delete_reload",
            join(&[
                create("Garden"),
                add(0, 0),
                add(0, 5),
                add(0, 9),
                view(0),
                save(0, "garden.scene"),
                del(0),
                LIST.into(),
                load("garden.scene"),
                LIST.into(),
                view(0),
                cmp_scenes(0, 0),
                EXIT.into(),
            ]),
        ),
        case(
            "everything_invalid",
            "3\n4\n5\n7\n10\n11\n2\nS\n3\nx\n4\nx\n5\nx\n7\nx\n10\nx\n11\nx\n6\n12\n",
        ),
        case(
            "two_scenes_full_workflow",
            join(&[
                create("Alpha"),
                create("Beta"),
                add(0, 3),
                add(1, 3),
                add(1, 7),
                cmp_scenes(0, 1),
                remove(1, 2),
                cmp_scenes(0, 1),
                LIST.into(),
                EXIT.into(),
            ]),
        ),
        case(
            "load_into_max_scenes",
            format!(
                "{}{}{LIST}{EXIT}",
                (0..10).map(|i| create(&format!("S{i}"))).collect::<String>(),
                load("f.txt")
            ),
        )
        .file("f.txt", scene_file("FromFile", &[3])),
        case(
            "save_over_loaded_file",
            join(&[
                load("f.txt"),
                add(0, 1),
                save(0, "f.txt"),
                load("f.txt"),
                LIST.into(),
                view(1),
                EXIT.into(),
            ]),
        )
        .file("f.txt", scene_file("Round", &[0, 1])),
        case(
            "stress_all_shapes_two_scenes",
            format!(
                "{}{}{}{}{}{}{EXIT}",
                create("A"),
                (0..10).map(|i| add(0, i)).collect::<String>(),
                create("B"),
                (0..10).map(|i| add(1, 9 - i)).collect::<String>(),
                cmp_scenes(0, 1),
                view(0)
            ),
        ),
    ];
    check_all("mixed", cases);
}
