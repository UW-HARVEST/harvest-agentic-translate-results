//! Differential tests: the C reference program and the Rust translation are
//! both executed as subprocesses and their stdout, stderr, exit status and
//! side-effect files are compared.
//!
//! Nothing here links the Rust code as a library; the built executable is
//! driven the way a shell drives it.

mod harness;

use harness::{assert_both_hang, assert_same, Prep};

/// Declares tests that need no files in the working directory.
macro_rules! diff_tests {
    ($( $name:ident : $input:expr ; )*) => {
        $(
            #[test]
            fn $name() {
                assert_same(stringify!($name), $input, &[]);
            }
        )*
    };
}

// ---------------------------------------------------------------------------
// Phase A sanity: both executables exist and run
// ---------------------------------------------------------------------------

#[test]
fn both_executables_exist_and_run() {
    let c = harness::c_bin();
    let r = harness::rust_bin();
    assert!(c.is_file(), "C executable missing at {}", c.display());
    assert!(r.is_file(), "Rust executable missing at {}", r.display());
    assert_same("smoke", b"12\n", &[]);
}

/// The C program is compared against `cargo build --release`, so the release
/// binary must behave the same as the one cargo builds for the test run. This
/// guards against a profile-dependent difference (overflow checks, for
/// instance) hiding behind whichever binary the harness happened to pick.
#[test]
fn release_and_test_profile_binaries_agree() {
    let release = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("release")
        .join("driver");
    if !release.is_file() {
        // Nothing to compare against; the harness is already using this binary.
        return;
    }
    let test_bin = std::path::PathBuf::from(env!("CARGO_BIN_EXE_driver"));
    let inputs: &[&[u8]] = &[
        b"1\n9\n0\n1\n12\n",
        b"2\nS\n3\n0\n0\n4\n0\n-2147483648\n5\n0\n12\n",
        b"99999999999999999999\n-99999999999999999999\n12\n",
    ];
    for (i, inp) in inputs.iter().enumerate() {
        let a = harness::run(&release, inp, &[], &format!("rel-{i}"));
        let b = harness::run(&test_bin, inp, &[], &format!("dbg-{i}"));
        assert_eq!(
            harness::canonicalize_pointers(&a.stdout),
            harness::canonicalize_pointers(&b.stdout),
            "release and test-profile binaries disagree on stdout for input {i}"
        );
        assert_eq!(a.stderr, b.stderr, "…and on stderr for input {i}");
        assert_eq!(
            (a.code, a.signal),
            (b.code, b.signal),
            "…and on exit status for input {i}"
        );
    }
}

// ---------------------------------------------------------------------------
// The menu loop: fgets + sscanf("%d") dispatch
// ---------------------------------------------------------------------------

diff_tests! {
    // No input at all: banner, one menu, then fgets returns NULL.
    empty_input: b"";
    // Nothing but a newline still reaches sscanf, which fails.
    only_newline: b"\n";
    exit_command: b"12\n";
    // fgets returns a line with no trailing newline at EOF.
    exit_without_trailing_newline: b"12";
    invalid_choice_text: b"junk\n12\n";
    invalid_choice_spaces: b"   \n12\n";
    invalid_choice_letter_then_digit: b"x1\n12\n";
    // Below and above the switch's range, plus the boundaries.
    choice_zero: b"0\n12\n";
    choice_thirteen: b"13\n12\n";
    choice_negative: b"-5\n12\n";
    choice_int_max: b"2147483647\n12\n";
    choice_int_min: b"-2147483648\n12\n";
    // strtol saturates and the result is truncated into an int.
    choice_overflow_positive: b"99999999999999999999\n12\n";
    choice_overflow_negative: b"-99999999999999999999\n12\n";
    choice_two_pow_32: b"4294967296\n12\n";
    // sscanf skips leading whitespace and accepts a sign; trailing junk is
    // ignored because only one conversion is requested.
    choice_leading_whitespace: b"   6\n\t12\n";
    choice_plus_sign: b"+12\n";
    choice_leading_zeros: b"0012\n";
    choice_trailing_junk: b"12abc\n";
    // A NUL byte terminates the C string sscanf sees.
    choice_nul_byte: b"\x00 12\n12\n";
    // Every menu entry in one session.
    every_menu_entry: b"1\n6\n9\n0\n1\n2\nS\n3\n0\n0\n5\n0\n4\n0\n1\n11\n0\n6\n12\n";
}

#[test]
fn choice_line_longer_than_input_buffer() {
    // fgets() reads at most 255 bytes, so the tail of an over-long line is
    // re-read as the next menu choice.
    let mut v = b"1".to_vec();
    v.extend(std::iter::repeat(b'0').take(300));
    v.extend_from_slice(b"\n12\n");
    assert_same("choice_line_longer_than_input_buffer", &v, &[]);
}

#[test]
fn choice_line_exactly_at_buffer_boundary() {
    for pad in [252usize, 253, 254, 255] {
        let mut v = b"12".to_vec();
        v.extend(std::iter::repeat(b' ').take(pad));
        v.extend_from_slice(b"\n12\n");
        assert_same(&format!("boundary_{pad}"), &v, &[]);
    }
}

// ---------------------------------------------------------------------------
// Menu 1 / 9: shapes
// ---------------------------------------------------------------------------

diff_tests! {
    view_all_shapes: b"1\n12\n";
    view_all_shapes_repeated: b"1\n1\n1\n12\n";
    compare_shapes_equal: b"9\n3\n3\n12\n";
    compare_shapes_not_equal: b"9\n0\n1\n12\n";
    compare_shapes_first_bad_input: b"9\nzz\n12\n";
    compare_shapes_second_bad_input: b"9\n0\nzz\n12\n";
    compare_shapes_first_negative: b"9\n-1\n0\n12\n";
    compare_shapes_second_negative: b"9\n0\n-1\n12\n";
    compare_shapes_first_too_large: b"9\n10\n0\n12\n";
    compare_shapes_second_too_large: b"9\n0\n10\n12\n";
    compare_shapes_boundary_nine: b"9\n9\n9\n12\n";
    compare_shapes_overflow: b"9\n99999999999999999999\n0\n12\n";
}

#[test]
fn compare_shapes_all_pairs() {
    // Exercises every branch of shape_type_name() and both outcomes of
    // shape_equals() for the whole enum.
    let mut input = Vec::new();
    for a in 0..10 {
        for b in 0..10 {
            input.extend_from_slice(format!("9\n{a}\n{b}\n").as_bytes());
        }
    }
    input.extend_from_slice(b"12\n");
    assert_same("compare_shapes_all_pairs", &input, &[]);
}

// ---------------------------------------------------------------------------
// Menu 2 / 6 / 11: creating, listing and deleting scenes
// ---------------------------------------------------------------------------

diff_tests! {
    create_scene_then_list: b"2\nMyScene\n6\n12\n";
    create_scene_empty_name: b"2\n\n6\n12\n";
    create_scene_name_with_spaces: b"2\n  a b c  \n6\n12\n";
    create_scene_name_with_nul: b"2\nAB\x00CD\n6\n12\n";
    create_scene_non_utf8_name: b"2\n\xff\xfe\x80name\n6\n5\n0\n12\n";
    // fgets() returns NULL: the function returns and the loop then ends.
    create_scene_eof_before_name: b"2\n";
    create_scene_eof_mid_name: b"2\nabc";
    list_scenes_none: b"6\n12\n";
    list_scenes_several: b"2\nA\n2\nB\n2\nC\n6\n12\n";
    delete_scene_none: b"11\n12\n";
    delete_scene_bad_input: b"11\nzz\n12\n";
    delete_scene_index_too_large: b"2\nA\n11\n5\n12\n";
    delete_scene_index_negative: b"2\nA\n11\n-1\n12\n";
    delete_scene_shifts_remainder: b"2\nA\n2\nB\n2\nC\n11\n1\n6\n11\n0\n6\n11\n0\n6\n12\n";
}

#[test]
fn create_scene_name_at_and_over_the_63_byte_limit() {
    // fgets(name, MAX_SCENE_NAME, stdin) keeps 63 bytes; strncpy then copies 63.
    // Anything past that stays in stdin and becomes the next menu choice.
    for len in [62usize, 63, 64, 70, 200] {
        let mut v = b"2\n".to_vec();
        v.extend(std::iter::repeat(b'N').take(len));
        v.extend_from_slice(b"\n6\n12\n");
        assert_same(&format!("name_len_{len}"), &v, &[]);
    }
}

#[test]
fn create_scene_name_truncation_leaves_digits_in_stdin() {
    // The 64th byte onwards is re-read by the menu's fgets/sscanf.
    let mut v = b"2\n".to_vec();
    v.extend(std::iter::repeat(b'N').take(63));
    v.extend_from_slice(b"6\n12\n");
    assert_same("name_truncation_leaves_digits", &v, &[]);
}

#[test]
fn create_scene_up_to_and_past_the_maximum() {
    // MAX_SCENES is 10; the eleventh attempt reports the limit.
    let mut v = Vec::new();
    for i in 0..13 {
        v.extend_from_slice(format!("2\nS{i}\n").as_bytes());
    }
    v.extend_from_slice(b"6\n12\n");
    assert_same("create_scene_max", &v, &[]);
}

// ---------------------------------------------------------------------------
// Menu 3: adding shapes
// ---------------------------------------------------------------------------

diff_tests! {
    add_shape_no_scenes: b"3\n12\n";
    add_shape_ok: b"2\nS\n3\n0\n0\n5\n0\n12\n";
    add_shape_scene_bad_input: b"2\nS\n3\nzz\n12\n";
    add_shape_scene_index_too_large: b"2\nS\n3\n5\n12\n";
    add_shape_scene_index_negative: b"2\nS\n3\n-1\n12\n";
    add_shape_type_bad_input: b"2\nS\n3\n0\nzz\n12\n";
    add_shape_type_too_large: b"2\nS\n3\n0\n10\n12\n";
    add_shape_type_negative: b"2\nS\n3\n0\n-1\n12\n";
    add_shape_type_overflow: b"2\nS\n3\n0\n99999999999999999999\n12\n";
    add_shape_boundary_type_nine: b"2\nS\n3\n0\n9\n5\n0\n12\n";
    add_every_shape_type: b"2\nS\n3\n0\n0\n3\n0\n1\n3\n0\n2\n3\n0\n3\n3\n0\n4\n3\n0\n5\n3\n0\n6\n3\n0\n7\n3\n0\n8\n3\n0\n9\n5\n0\n12\n";
}

#[test]
fn add_shape_until_the_scene_is_full() {
    // The 51st shape trips MAX_SHAPES_IN_SCENE: scene_add_shape writes
    // "Error: Scene is full" to *stderr* and main prints "Error adding shape".
    let mut v = b"2\nFull\n".to_vec();
    for _ in 0..52 {
        v.extend_from_slice(b"3\n0\n0\n");
    }
    v.extend_from_slice(b"6\n12\n");
    assert_same("add_shape_until_full", &v, &[]);
}

// ---------------------------------------------------------------------------
// Menu 4: removing shapes
// ---------------------------------------------------------------------------

diff_tests! {
    remove_shape_no_scenes: b"4\n12\n";
    remove_shape_scene_bad_input: b"4\nzz\n12\n";
    remove_shape_scene_index_too_large: b"2\nA\n4\n7\n12\n";
    remove_shape_scene_index_negative: b"2\nA\n4\n-3\n12\n";
    remove_shape_from_empty_scene: b"2\nA\n4\n0\n12\n";
    remove_shape_bad_input: b"2\nA\n3\n0\n0\n4\n0\nzz\n12\n";
    remove_shape_ok: b"2\nA\n3\n0\n0\n3\n0\n1\n4\n0\n1\n5\n0\n12\n";
    remove_shape_last: b"2\nA\n3\n0\n0\n3\n0\n1\n4\n0\n2\n5\n0\n12\n";
    remove_shape_index_zero: b"2\nA\n3\n0\n0\n4\n0\n0\n12\n";
    remove_shape_index_past_end: b"2\nA\n3\n0\n0\n4\n0\n99\n12\n";
    // shape_idx - 1 wraps for INT_MIN, exactly as the C does.
    remove_shape_index_int_min: b"2\nA\n3\n0\n0\n4\n0\n-2147483648\n5\n0\n12\n";
    remove_shape_index_int_max: b"2\nA\n3\n0\n0\n4\n0\n2147483647\n12\n";
    remove_shape_all_then_empty: b"2\nA\n3\n0\n0\n4\n0\n1\n4\n0\n12\n";
}

// ---------------------------------------------------------------------------
// Menu 5: viewing a scene
// ---------------------------------------------------------------------------

diff_tests! {
    view_scene_no_scenes: b"5\n12\n";
    view_scene_bad_input: b"5\nzz\n12\n";
    view_scene_index_too_large: b"2\nA\n5\n7\n12\n";
    view_scene_index_negative: b"2\nA\n5\n-1\n12\n";
    view_scene_empty: b"2\nA\n5\n0\n12\n";
    view_scene_with_shapes: b"2\nA\n3\n0\n0\n3\n0\n4\n3\n0\n9\n5\n0\n12\n";
}

// ---------------------------------------------------------------------------
// Menu 10: comparing scenes
// ---------------------------------------------------------------------------

diff_tests! {
    compare_scenes_needs_two: b"10\n12\n";
    compare_scenes_needs_two_with_one: b"2\nA\n10\n12\n";
    compare_scenes_first_bad_input: b"2\nA\n2\nB\n10\nzz\n12\n";
    compare_scenes_second_bad_input: b"2\nA\n2\nB\n10\n0\nzz\n12\n";
    compare_scenes_first_index_bad: b"2\nA\n2\nB\n10\n9\n0\n12\n";
    compare_scenes_second_index_bad: b"2\nA\n2\nB\n10\n0\n-1\n12\n";
    compare_scenes_both_empty_equal: b"2\nA\n2\nB\n10\n0\n1\n12\n";
    compare_scenes_same_scene_twice: b"2\nA\n3\n0\n0\n2\nB\n10\n0\n0\n12\n";
    compare_scenes_different_counts: b"2\nA\n2\nB\n3\n0\n0\n10\n0\n1\n12\n";
    compare_scenes_same_count_different_shapes: b"2\nA\n3\n0\n0\n2\nB\n3\n1\n5\n10\n0\n1\n12\n";
    // scene_equals() looks for a 1:1 correspondence, so order does not matter.
    compare_scenes_equal_when_reordered: b"2\nA\n3\n0\n0\n3\n0\n3\n2\nB\n3\n1\n3\n3\n1\n0\n10\n0\n1\n12\n";
    // The `matched[]` bookkeeping means multiplicity matters.
    compare_scenes_equal_with_duplicates: b"2\nA\n3\n0\n2\n3\n0\n2\n2\nB\n3\n1\n2\n3\n1\n2\n10\n0\n1\n12\n";
    compare_scenes_duplicates_versus_distinct: b"2\nA\n3\n0\n2\n3\n0\n2\n2\nB\n3\n1\n2\n3\n1\n3\n10\n0\n1\n12\n";
}

// ---------------------------------------------------------------------------
// Menu 7: saving
// ---------------------------------------------------------------------------

diff_tests! {
    save_no_scenes: b"7\n12\n";
    save_scene_bad_input: b"2\nA\n7\nzz\n12\n";
    save_scene_index_too_large: b"2\nA\n7\n9\n12\n";
    save_scene_index_negative: b"2\nA\n7\n-1\n12\n";
    // fgets() for the filename hits EOF and the function returns.
    save_eof_before_filename: b"2\nA\n7\n0\n";
    // fopen("") fails: the message goes to stderr.
    save_empty_filename: b"2\nA\n7\n0\n\n12\n";
    save_filename_missing_directory: b"2\nA\n7\n0\nno_such_dir/f.scn\n12\n";
    save_filename_is_dot: b"2\nA\n7\n0\n.\n12\n";
    save_filename_is_dotdot: b"2\nA\n7\n0\n..\n12\n";
    save_empty_scene: b"2\nEmpty\n7\n0\nempty.scn\n12\n";
    save_scene_with_shapes: b"2\nA\n3\n0\n0\n3\n0\n4\n3\n0\n9\n7\n0\nout.scn\n12\n";
    save_filename_with_nul: b"2\nA\n7\n0\nab\x00cd.scn\n12\n";
    save_non_utf8_filename: b"2\nA\n7\n0\nf\xff\xfe.scn\n12\n";
    save_twice_same_name: b"2\nA\n3\n0\n0\n7\n0\nx.scn\n3\n0\n1\n7\n0\nx.scn\n12\n";
}

#[test]
fn save_into_existing_subdirectory() {
    assert_same(
        "save_into_subdir",
        b"2\nA\n3\n0\n2\n7\n0\nsub/x.scn\n12\n",
        &[Prep::Dir("sub")],
    );
}

#[test]
fn save_over_a_directory() {
    assert_same(
        "save_over_dir",
        b"2\nA\n7\n0\nsub\n12\n",
        &[Prep::Dir("sub")],
    );
}

#[test]
fn save_over_a_read_only_file() {
    assert_same(
        "save_over_readonly",
        b"2\nA\n7\n0\nro.scn\n12\n",
        &[Prep::ReadOnlyFile("ro.scn", b"untouched\n")],
    );
}

#[test]
fn save_truncates_a_longer_existing_file() {
    assert_same(
        "save_truncates",
        b"2\nA\n7\n0\nbig.scn\n12\n",
        &[Prep::File("big.scn", b"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA\n")],
    );
}

#[test]
fn save_a_full_scene() {
    let mut v = b"2\nFull\n".to_vec();
    for i in 0..50 {
        v.extend_from_slice(format!("3\n0\n{}\n", i % 10).as_bytes());
    }
    v.extend_from_slice(b"7\n0\nfull.scn\n8\nfull.scn\n10\n0\n1\n12\n");
    assert_same("save_full_scene", &v, &[]);
}

// ---------------------------------------------------------------------------
// Menu 8: loading
// ---------------------------------------------------------------------------

const GOOD: &[u8] = b"Loaded\n3\n0\n5\n9\n";

#[test]
fn load_missing_file() {
    assert_same("load_missing", b"8\nnosuch.scn\n6\n12\n", &[]);
}

#[test]
fn load_empty_filename() {
    assert_same("load_empty_filename", b"8\n\n6\n12\n", &[]);
}

#[test]
fn load_eof_before_filename() {
    assert_same("load_eof_filename", b"8\n", &[]);
}

#[test]
fn load_a_good_file() {
    assert_same(
        "load_good",
        b"8\ngood.scn\n5\n0\n6\n12\n",
        &[Prep::File("good.scn", GOOD)],
    );
}

#[test]
fn load_file_variants() {
    // Each entry is one early-return or skip inside scene_load().
    let cases: &[(&str, &[u8])] = &[
        // fgets() succeeds but the fscanf for the count fails: NULL, silently.
        ("name_only", b"OnlyName\n"),
        // fgets() returns NULL on an empty file: NULL, silently.
        ("empty_file", b""),
        // The count parses but an entry is missing: NULL, silently.
        ("count_exceeds_entries", b"Name\n5\n0\n1\n"),
        ("count_not_a_number", b"Name\nnotanumber\n"),
        // A float stops at the '.', so the count is 2 and the entries follow.
        ("count_is_float", b"Fl\n2.5\n0\n1\n"),
        // shape_get() returns NULL for these, so they are skipped, not added.
        ("types_out_of_range", b"Name\n3\n99\n-5\n0\n"),
        ("type_overflows_int", b"T\n3\n2147483648\n10\n-1\n"),
        ("negative_count", b"Neg\n-3\n0\n"),
        ("zero_count", b"Zero\n0\n"),
        ("plus_count", b"Plus\n+2\n0\n1\n"),
        ("padded_count", b"Sp\n   2   \n0\n1\n"),
        ("crlf", b"CR\r\n2\r\n0\r\n1\r\n"),
        ("all_on_one_line", b"OL\n3\n1 2 3\n"),
        ("tabs", b"Tab\n\t2\n\t0\n\t9\n"),
        ("trailing_garbage", b"Ex\n1\n0\nGARBAGE\n"),
        ("empty_name_line", b"\n4\n0\n1\n2\n3\n"),
        ("nul_in_name", b"AB\x00CD\n2\n0\n1\n"),
        ("no_newline_at_all", b"XXXXXXXXXXXXXXXX"),
        ("count_saturates", b"H\n99999999999999999999\n0\n"),
        ("count_int_max", b"Big\n2147483647\n0\n1\n"),
    ];
    for (name, body) in cases {
        assert_same(
            &format!("load_{name}"),
            b"8\nf.scn\n5\n0\n6\n12\n",
            &[Prep::Owned("f.scn".to_string(), body.to_vec())],
        );
    }
}

#[test]
fn load_name_longer_than_the_scene_name_buffer() {
    // fgets(name, 64, file) keeps 63 bytes; the rest of the line is then fed to
    // the fscanf that reads the shape count.
    for len in [62usize, 63, 64, 100] {
        let mut body: Vec<u8> = std::iter::repeat(b'N').take(len).collect();
        body.extend_from_slice(b"\n3\n0\n1\n2\n");
        assert_same(
            &format!("load_longname_{len}"),
            b"8\nf.scn\n5\n0\n6\n12\n",
            &[Prep::Owned("f.scn".to_string(), body)],
        );
    }
    // 63 name bytes followed immediately by a digit: the digit becomes the count.
    let mut body: Vec<u8> = std::iter::repeat(b'N').take(63).collect();
    body.extend_from_slice(b"2\n0\n1\n");
    assert_same(
        "load_longname_digit_tail",
        b"8\nf.scn\n5\n0\n6\n12\n",
        &[Prep::Owned("f.scn".to_string(), body)],
    );
}

#[test]
fn load_more_shapes_than_a_scene_holds() {
    // Past 50, scene_add_shape() writes to stderr but the scene still loads.
    let mut body = b"Big\n55\n".to_vec();
    for i in 0..55 {
        body.extend_from_slice(format!("{}\n", i % 10).as_bytes());
    }
    assert_same(
        "load_over_capacity",
        b"8\nf.scn\n6\n12\n",
        &[Prep::Owned("f.scn".to_string(), body)],
    );
}

#[test]
fn load_a_directory() {
    // fopen() on a directory succeeds; the first read then fails.
    assert_same("load_dir", b"8\nsub\n6\n12\n", &[Prep::Dir("sub")]);
}

#[test]
fn load_when_the_scene_table_is_full() {
    let mut v = Vec::new();
    for i in 0..10 {
        v.extend_from_slice(format!("2\nS{i}\n").as_bytes());
    }
    v.extend_from_slice(b"8\ngood.scn\n6\n12\n");
    assert_same("load_max_scenes", &v, &[Prep::File("good.scn", GOOD)]);
}

#[test]
fn save_then_load_round_trip() {
    assert_same(
        "round_trip",
        b"2\nRT\n3\n0\n2\n3\n0\n7\n3\n0\n0\n7\n0\nrt.scn\n8\nrt.scn\n5\n1\n10\n0\n1\n6\n12\n",
        &[],
    );
}

#[test]
fn load_non_utf8_filename() {
    assert_same(
        "load_non_utf8",
        b"2\nA\n3\n0\n0\n7\n0\nf\xff\xfe.scn\n8\nf\xff\xfe.scn\n6\n12\n",
        &[],
    );
}

// ---------------------------------------------------------------------------
// Reading behaviour: scanf crosses newlines, fgets does not
// ---------------------------------------------------------------------------

diff_tests! {
    // scanf("%d") skips any run of whitespace, blank lines included.
    scanf_skips_blank_lines: b"3\n\n\n\n0\n \t 0 \n5\n0\n12\n";
    // Two numbers on one line: the drain-to-newline loop eats the second.
    scanf_two_numbers_one_line: b"2\nS\n3\n0 5\n5\n0\n12\n";
    // A digit run followed by letters: the letters are left for the drain loop.
    scanf_digits_then_letters: b"2\nS\n3\n0abc\n0\n12\n";
    // A sign with no digits is a matching failure; the sign is already consumed.
    scanf_sign_without_digits: b"2\nS\n3\n-x\n12\n";
    scanf_plus_without_digits: b"2\nS\n3\n+\n0\n12\n";
    // Vertical tab and form feed count as whitespace for scanf.
    scanf_vertical_tab_and_form_feed: b"2\nS\n3\n\x0b\x0c 0\n0\n5\n0\n12\n";
    // Carriage returns are not newlines for fgets but are whitespace for scanf.
    carriage_returns_everywhere: b"2\r\nSc\r\n6\r\n12\r\n";
    // fgets keeps reading the same line; scanf's drain loop stops at '\n'.
    interleaved_fgets_and_scanf: b"2\nA\n3\n0\n0\n2\nB\n4\n1\n1\n12\n";
}

// ---------------------------------------------------------------------------
// End-of-file inside the scanf idiom
// ---------------------------------------------------------------------------

#[test]
fn eof_inside_the_scanf_drain_loop_spins_in_both() {
    // `while (getchar() != '\n');` never terminates once the stream is at end
    // of file, because getchar() keeps returning EOF. The translation has to
    // reproduce that, not "fix" it.
    assert_both_hang("spin_after_prompt", b"2\nS\n3\n");
    assert_both_hang("spin_after_value", b"2\nS\n3\n0");
}

// ---------------------------------------------------------------------------
// Signal disposition
// ---------------------------------------------------------------------------

/// A C program keeps the default SIGPIPE disposition, so it is killed when the
/// reader of its stdout goes away. The Rust runtime ignores SIGPIPE unless the
/// program restores the default, which would leave the Rust program exiting 0
/// where the C program dies with signal 13.
#[test]
fn writing_to_a_closed_pipe_kills_both() {
    use std::process::{Command, Stdio};

    fn producer_status(exe: &std::path::Path, input: &[u8]) -> (Option<i32>, Option<i32>) {
        let dir = std::env::temp_dir().join(format!(
            "ascii-art-sigpipe-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::create_dir_all(&dir);
        let inp = dir.join("in.txt");
        std::fs::write(&inp, input).expect("write stdin file");

        // The consumer exits after one byte, closing the pipe. The producer
        // writes far more than a pipe buffer, so it is guaranteed to write
        // again afterwards.
        let mut producer = Command::new(exe)
            .current_dir(&dir)
            .stdin(Stdio::from(std::fs::File::open(&inp).unwrap()))
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn producer");
        let pipe = producer.stdout.take().expect("piped stdout");
        let mut consumer = Command::new("head")
            .args(["-c", "1"])
            .stdin(Stdio::from(pipe))
            .stdout(Stdio::null())
            .spawn()
            .expect("`head` is required for the SIGPIPE test");
        let _ = consumer.wait();
        let status = producer.wait().expect("wait producer");
        let _ = std::fs::remove_dir_all(&dir);

        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            (status.code(), status.signal())
        }
        #[cfg(not(unix))]
        {
            (status.code(), None)
        }
    }

    // ~800 KB of output, far past the 64 KB pipe buffer.
    let mut input = Vec::new();
    for _ in 0..400 {
        input.extend_from_slice(b"1\n");
    }
    input.extend_from_slice(b"12\n");

    let c = producer_status(harness::c_bin(), &input);
    let r = producer_status(&harness::rust_bin(), &input);
    assert_eq!(
        c, r,
        "the C program and the translation must agree on (exit code, signal) \
         when stdout is closed early; got C={c:?} Rust={r:?}"
    );
}

// ---------------------------------------------------------------------------
// Longer sessions mixing everything
// ---------------------------------------------------------------------------

#[test]
fn long_mixed_session() {
    let mut v = Vec::new();
    v.extend_from_slice(b"1\n");
    for i in 0..10 {
        v.extend_from_slice(format!("2\nScene{i}\n").as_bytes());
        for s in 0..(i % 4) {
            v.extend_from_slice(format!("3\n{i}\n{s}\n").as_bytes());
        }
    }
    v.extend_from_slice(b"6\n");
    v.extend_from_slice(b"2\nOneTooMany\n");
    v.extend_from_slice(b"7\n3\nsaved.scn\n8\nsaved.scn\n");
    v.extend_from_slice(b"11\n0\n11\n0\n8\nsaved.scn\n6\n");
    v.extend_from_slice(b"10\n0\n1\n5\n0\n4\n0\n1\n5\n0\n");
    v.extend_from_slice(b"9\n8\n8\n9\n8\n2\n");
    v.extend_from_slice(b"junk\n0\n13\n12\n");
    assert_same("long_mixed_session", &v, &[]);
}

#[test]
fn stress_output_beyond_the_stdio_buffer() {
    // Far more than BUFSIZ of output, so any buffering difference would show.
    let mut v = Vec::new();
    for _ in 0..60 {
        v.extend_from_slice(b"1\n");
    }
    v.extend_from_slice(b"12\n");
    assert_same("stress_output", &v, &[]);
}
