//! Differential tests: the C executable versus the translated Rust executable.
//!
//! Each `Case` feeds identical stdin to both programs and asserts that stdout,
//! stderr, the exit status, and any files written to the working directory are
//! identical. The Rust program is always executed as a subprocess.
//!
//! Input classes are derived from the branches in `c_src/`: every `if`, every
//! early `return`, every bounds check and every `scanf`/`fgets` failure path.

mod common;

use common::{with_scene_files, Case};

// ===========================================================================
// Phase A -- the two executables exist and can be driven
// ===========================================================================

#[test]
fn both_executables_are_runnable() {
    // Building the C program happens inside the harness; this asserts it
    // produced something and that the simplest possible session agrees.
    assert!(common::c_binary().is_file());
    assert!(!common::rust_binaries().is_empty());
    Case::new("smoke_exit", b"12\n").assert_matches();
}

// ===========================================================================
// main(): menu reading with fgets + sscanf, and dispatch
// ===========================================================================

#[test]
fn menu_empty_and_minimal_input() {
    // `fgets` returns NULL immediately -> break out of the loop, exit 0.
    Case::new("empty_stdin", b"").assert_matches();
    // A bare newline: sscanf finds no digits -> "Invalid input".
    Case::new("only_newline", b"\n").assert_matches();
    Case::new("two_blank_lines_then_exit", b"\n\n12\n").assert_matches();
    // No trailing newline on the final line.
    Case::new("no_final_newline", b"6").assert_matches();
    Case::new("exit_only", b"12\n").assert_matches();
    // Anything after choice 12 is never read.
    Case::new("input_after_exit_is_ignored", b"12\n6\n").assert_matches();
}

#[test]
fn menu_invalid_and_out_of_range_choices() {
    Case::new("menu_non_numeric", b"abc\n12\n").assert_matches();
    Case::new("menu_only_spaces", b"   \n12\n").assert_matches();
    Case::new("menu_choice_zero", b"0\n12\n").assert_matches();
    Case::new("menu_choice_13", b"13\n12\n").assert_matches();
    Case::new("menu_choice_negative", b"-1\n12\n").assert_matches();
    Case::new("menu_choice_large", b"9999\n12\n").assert_matches();
}

#[test]
fn menu_sscanf_lexing_details() {
    // sscanf skips leading white space and stops at the first non-digit.
    Case::new("menu_leading_spaces", b"   6\n12\n").assert_matches();
    Case::new("menu_leading_tab", b"\t6\n12\n").assert_matches();
    Case::new("menu_trailing_junk", b"6abc\n12\n").assert_matches();
    Case::new("menu_explicit_plus", b"+6\n12\n").assert_matches();
    Case::new("menu_leading_zeros", b"0000000006\n12\n").assert_matches();
    // sscanf stops at the NUL that terminates the fgets buffer.
    Case::new("menu_embedded_nul", b"6\x00junk\n12\n").assert_matches();
    Case::new("menu_crlf", b"6\r\n12\r\n").assert_matches();
}

#[test]
fn menu_lines_longer_than_the_fgets_buffer() {
    // fgets(input, 256, stdin) splits an over-long line; the remainder becomes
    // the next menu line.
    let mut long = vec![b'6'];
    long.extend(std::iter::repeat(b' ').take(300));
    long.push(b'\n');
    long.extend_from_slice(b"12\n");
    Case::new("menu_line_over_255", long).assert_matches();

    let mut junk = std::iter::repeat(b'x').take(260).collect::<Vec<u8>>();
    junk.extend_from_slice(b"12\n12\n");
    Case::new("menu_long_junk_line", junk).assert_matches();

    // Exactly 255 payload bytes: fgets fills the buffer without a newline.
    let mut exact = std::iter::repeat(b'7').take(255).collect::<Vec<u8>>();
    exact.extend_from_slice(b"\n12\n");
    Case::new("menu_exactly_255", exact).assert_matches();
}

// ===========================================================================
// Integer parsing: strtol saturation followed by truncation to `int`
// ===========================================================================

#[test]
fn integer_overflow_and_truncation() {
    // Plain truncation of an in-range long to int.
    Case::new("trunc_2pow32_plus_6", b"4294967302\n12\n").assert_matches();
    Case::new("trunc_negative_to_6", b"-4294967290\n12\n").assert_matches();

    // strtol saturates, then the assignment truncates:
    //   LONG_MAX -> (int)-1 ; LONG_MIN -> (int)0
    Case::new("overflow_positive", b"99999999999999999999\n12\n").assert_matches();
    Case::new("overflow_negative", b"-99999999999999999999\n12\n").assert_matches();
    Case::new("overflow_long_max", b"9223372036854775807\n12\n").assert_matches();
    Case::new("overflow_long_max_plus1", b"9223372036854775808\n12\n").assert_matches();
    // LONG_MIN exactly -- this is where `-LONG_MAX` and `LONG_MIN` diverge.
    Case::new("overflow_long_min", b"-9223372036854775808\n12\n").assert_matches();
    Case::new("overflow_long_min_minus1", b"-9223372036854775809\n12\n").assert_matches();

    // Same arithmetic, but reached through scanf() rather than sscanf().
    Case::new("scanf_scene_idx_long_min", b"2\na\n3\n-9223372036854775808\n0\n5\n0\n12\n")
        .assert_matches();
    Case::new("scanf_scene_idx_overflow", b"2\na\n3\n-9223372036854775809\n12\n").assert_matches();
    Case::new("scanf_scene_idx_pos_overflow", b"2\na\n3\n9999999999999999999999\n12\n")
        .assert_matches();
    Case::new("scanf_shape_type_2pow32", b"2\na\n3\n0\n4294967296\n12\n").assert_matches();
    // INT_MIN, then `shape_idx - 1` wraps around in C.
    Case::new("scanf_shape_idx_int_min", b"2\na\n3\n0\n0\n4\n0\n-2147483648\n12\n")
        .assert_matches();
    Case::new("scanf_shape_idx_int_max", b"2\na\n3\n0\n0\n4\n0\n2147483647\n12\n")
        .assert_matches();
}

#[test]
fn scanf_sign_only_and_pushback() {
    // "-" with no digits is a matching failure; glibc pushes the offending
    // character back, which the following `while (getchar() != '\n')` eats.
    Case::new("scanf_minus_only", b"2\na\n3\n-\n12\n").assert_matches();
    Case::new("scanf_plus_only", b"2\na\n3\n+\n12\n").assert_matches();
    Case::new("scanf_signed_values", b"2\na\n3\n+0\n+7\n5\n0\n12\n").assert_matches();
}

// ===========================================================================
// Option 1 -- view all shapes (all ten art blocks, verbatim)
// ===========================================================================

#[test]
fn option1_view_all_shapes() {
    Case::new("view_all_shapes", b"1\n12\n").assert_matches();
    Case::new("view_all_shapes_twice", b"1\n1\n12\n").assert_matches();
}

// ===========================================================================
// Option 2 -- create scene
// ===========================================================================

#[test]
fn option2_create_scene() {
    Case::new("create_basic", b"2\nMyScene\n6\n12\n").assert_matches();
    Case::new("create_empty_name", b"2\n\n6\n5\n0\n12\n").assert_matches();
    Case::new("create_name_with_spaces", b"2\n   spaced   name   \n6\n12\n").assert_matches();
    // fgets returns NULL at the name prompt -> silent return.
    Case::new("create_eof_at_name", b"2\n").assert_matches();
    // strcspn stops at the NUL, so the name is truncated there.
    Case::new("create_name_embedded_nul", b"2\nab\x00cd\n6\n12\n").assert_matches();
    Case::new("create_name_high_bytes", b"2\n\xff\xfe\n6\n5\n0\n12\n").assert_matches();
    // A name made of printf directives: the C code passes it as %s, not as a
    // format string.
    Case::new("create_name_percent", b"2\n%s%d%%\n6\n5\n0\n12\n").assert_matches();
    Case::new("create_name_crlf", b"2\nabc\r\n6\n12\n").assert_matches();
}

#[test]
fn option2_name_length_boundaries() {
    // fgets(name, MAX_SCENE_NAME=64, stdin) stores at most 63 bytes.
    let n62 = "0".repeat(62);
    Case::new("create_name_62", format!("2\n{n62}\n6\n12\n")).assert_matches();
    // 63 bytes exactly: no newline is stored, so the leftover "\n" becomes the
    // next menu line and yields "Invalid input".
    let n63 = "0".repeat(63);
    Case::new("create_name_63", format!("2\n{n63}\n6\n12\n")).assert_matches();
    // Over-long: the tail is re-read as menu input.
    let n70 = "0123456789".repeat(7);
    Case::new("create_name_70", format!("2\n{n70}ABC\n6\n12\n")).assert_matches();
    // strncpy copies 63 bytes then the explicit NUL terminator.
    let n200 = "9".repeat(200);
    Case::new("create_name_200", format!("2\n{n200}\n6\n12\n")).assert_matches();
}

#[test]
fn option2_maximum_scenes() {
    // MAX_SCENES is 10; the eleventh create must be refused.
    let mut s = String::new();
    for c in "abcdefghijk".chars() {
        s.push_str(&format!("2\n{c}\n"));
    }
    s.push_str("6\n12\n");
    Case::new("create_11_scenes", s).assert_matches();
}

// ===========================================================================
// Option 3 -- add shape to scene
// ===========================================================================

#[test]
fn option3_add_shape_error_paths() {
    Case::new("add_no_scenes", b"3\n12\n").assert_matches();
    Case::new("add_scene_idx_too_big", b"2\nS\n3\n5\n12\n").assert_matches();
    Case::new("add_scene_idx_negative", b"2\nS\n3\n-1\n12\n").assert_matches();
    Case::new("add_scene_non_numeric", b"2\nS\n3\nxyz\n12\n").assert_matches();
    Case::new("add_shape_type_10", b"2\nS\n3\n0\n10\n12\n").assert_matches();
    Case::new("add_shape_type_negative", b"2\nS\n3\n0\n-5\n12\n").assert_matches();
    Case::new("add_shape_non_numeric", b"2\nS\n3\n0\nqq\n12\n").assert_matches();
}

#[test]
fn option3_add_shape_success_paths() {
    Case::new("add_one_shape", b"2\nS\n3\n0\n0\n5\n0\n12\n").assert_matches();
    // Every shape type, then render the scene.
    let mut s = String::from("2\nS\n");
    for t in 0..10 {
        s.push_str(&format!("3\n0\n{t}\n"));
    }
    s.push_str("5\n0\n12\n");
    Case::new("add_every_shape_type", s).assert_matches();

    // scanf reads across newlines, unlike fgets.
    Case::new("add_scanf_crosses_newlines", b"2\nS\n3\n\n\n0\n\n0\n5\n0\n12\n").assert_matches();
    // Trailing junk after each number is discarded by the getchar loop.
    Case::new("add_trailing_junk", b"2\nS\n3\n0zz\n0yy\n5\n0\n12\n").assert_matches();
}

#[test]
fn option3_scene_full_at_50_shapes() {
    // MAX_SHAPES_IN_SCENE is 50; the 51st add prints to stderr *and* stdout.
    let mut s = String::from("2\nBig\n");
    for _ in 0..51 {
        s.push_str("3\n0\n0\n");
    }
    s.push_str("6\n12\n");
    Case::new("add_51_shapes", s.clone()).assert_matches();
    // Same session with the two streams merged, which also pins C's buffering.
    Case::new("add_51_shapes_merged", s).merged_streams().assert_matches();
}

// ===========================================================================
// Option 4 -- remove shape from scene
// ===========================================================================

#[test]
fn option4_remove_shape() {
    Case::new("remove_no_scenes", b"4\n12\n").assert_matches();
    Case::new("remove_scene_idx_bad", b"2\nS\n4\n9\n12\n").assert_matches();
    Case::new("remove_scene_non_numeric", b"2\nS\n4\nzz\n12\n").assert_matches();
    // scene_list_shapes runs before the emptiness check.
    Case::new("remove_from_empty_scene", b"2\nS\n4\n0\n12\n").assert_matches();
    Case::new("remove_ok_middle", b"2\nS\n3\n0\n0\n3\n0\n7\n4\n0\n1\n5\n0\n12\n")
        .assert_matches();
    Case::new("remove_ok_last", b"2\nS\n3\n0\n0\n3\n0\n7\n4\n0\n2\n5\n0\n12\n").assert_matches();
    // 1-based prompt, 0-based array: index 0 becomes -1 and fails.
    Case::new("remove_index_zero", b"2\nS\n3\n0\n0\n4\n0\n0\n12\n").assert_matches();
    Case::new("remove_index_too_big", b"2\nS\n3\n0\n0\n4\n0\n9\n12\n").assert_matches();
    Case::new("remove_index_non_numeric", b"2\nS\n3\n0\n0\n4\n0\nzz\n12\n").assert_matches();
    // Remove until empty, then try once more.
    Case::new("remove_until_empty", b"2\nS\n3\n0\n0\n4\n0\n1\n4\n0\n12\n").assert_matches();
}

// ===========================================================================
// Option 5 -- view scene
// ===========================================================================

#[test]
fn option5_view_scene() {
    Case::new("view_no_scenes", b"5\n12\n").assert_matches();
    Case::new("view_empty_scene", b"2\nS\n5\n0\n12\n").assert_matches();
    Case::new("view_scene_idx_bad", b"2\nS\n5\n7\n12\n").assert_matches();
    Case::new("view_scene_non_numeric", b"2\nS\n5\nzz\n12\n").assert_matches();
    Case::new("view_scene_with_shapes", b"2\nS\n3\n0\n2\n3\n0\n8\n5\n0\n12\n").assert_matches();
}

// ===========================================================================
// Option 6 -- list all scenes
// ===========================================================================

#[test]
fn option6_list_scenes() {
    Case::new("list_no_scenes", b"6\n12\n").assert_matches();
    Case::new("list_three_scenes", b"2\na\n2\nb\n2\nc\n6\n12\n").assert_matches();
    Case::new(
        "list_scenes_with_counts",
        b"2\na\n2\nb\n3\n0\n0\n3\n0\n1\n3\n1\n5\n6\n12\n",
    )
    .assert_matches();
}

// ===========================================================================
// Option 7 -- save scene
// ===========================================================================

#[test]
fn option7_save_scene_error_paths() {
    Case::new("save_no_scenes", b"7\n12\n").assert_matches();
    Case::new("save_scene_idx_bad", b"2\na\n7\n9\n12\n").assert_matches();
    Case::new("save_scene_non_numeric", b"2\na\n7\nzz\n12\n").assert_matches();
    // fgets returns NULL at the filename prompt -> silent return.
    Case::new("save_eof_at_filename", b"2\na\n7\n0\n").assert_matches();
    // fopen failures all report on stderr.
    Case::new("save_empty_filename", b"2\na\n7\n0\n\n12\n").assert_matches();
    Case::new("save_missing_directory", b"2\na\n7\n0\nno_such_dir/x.txt\n12\n").assert_matches();
    Case::new("save_onto_directory", b"2\na\n7\n0\nadir\n12\n")
        .seed_dir("adir")
        .assert_matches();
    Case::new("save_onto_dot", b"2\na\n7\n0\n.\n12\n").assert_matches();
}

#[test]
fn option7_save_scene_writes_the_file() {
    Case::new("save_with_shapes", b"2\nMyScene\n3\n0\n0\n3\n0\n7\n7\n0\nout.txt\n12\n")
        .assert_matches();
    Case::new("save_empty_scene", b"2\nEmpty\n7\n0\nempty.txt\n12\n").assert_matches();
    Case::new("save_empty_name", b"2\n\n7\n0\nen.txt\n12\n").assert_matches();
    Case::new("save_high_byte_name", b"2\n\xff\xfe\n7\n0\nhn.txt\n12\n").assert_matches();
    Case::new("save_filename_with_spaces", b"2\na\n3\n0\n0\n7\n0\nwith spaces.txt\n12\n")
        .assert_matches();
    Case::new("save_filename_high_bytes", b"2\na\n7\n0\n\xff\xfe.txt\n12\n").assert_matches();
    Case::new("save_filename_percent", b"2\na\n7\n0\n%s%d.txt\n12\n").assert_matches();
    // The filename is truncated at the NUL, exactly as strcspn/fopen do.
    Case::new("save_filename_embedded_nul", b"2\na\n7\n0\nab\x00cd\n6\n12\n").assert_matches();
    // Overwriting truncates.
    Case::new(
        "save_twice_same_name",
        b"2\na\n3\n0\n0\n7\n0\nx.txt\n3\n0\n1\n7\n0\nx.txt\n12\n",
    )
    .assert_matches();
    // A 300 byte filename fits in the 256 byte buffer only up to 255 bytes.
    let long = "f".repeat(300);
    Case::new("save_filename_over_255", format!("2\na\n7\n0\n{long}\n12\n")).assert_matches();
    // Every shape type present, to pin the `%d` type column in the file.
    let mut s = String::from("2\nAll\n");
    for t in 0..10 {
        s.push_str(&format!("3\n0\n{t}\n"));
    }
    s.push_str("7\n0\nall.txt\n12\n");
    Case::new("save_all_shape_types", s).assert_matches();
}

// ===========================================================================
// Option 8 -- load scene
// ===========================================================================

#[test]
fn option8_load_scene_error_paths() {
    with_scene_files(Case::new("load_missing_file", b"8\nnope.txt\n12\n")).assert_matches();
    with_scene_files(Case::new("load_empty_filename", b"8\n\n12\n")).assert_matches();
    with_scene_files(Case::new("load_directory", b"8\nadir\n6\n12\n")).assert_matches();
    // fgets returns NULL at the filename prompt -> silent return.
    with_scene_files(Case::new("load_eof_at_filename", b"8\n")).assert_matches();
    // fgets on an empty file returns NULL: scene_load bails with no message.
    with_scene_files(Case::new("load_empty_file", b"8\nempty.txt\n6\n12\n")).assert_matches();
    // Name present but no shape count -> fscanf fails, still no message.
    with_scene_files(Case::new("load_name_only", b"8\nnameonly.txt\n6\n12\n")).assert_matches();
    with_scene_files(Case::new("load_name_without_newline", b"8\nname_nonl.txt\n6\n12\n"))
        .assert_matches();
    with_scene_files(Case::new("load_bad_count", b"8\nbadcount.txt\n6\n12\n")).assert_matches();
    // Count larger than the number of records -> fscanf fails mid-loop.
    with_scene_files(Case::new("load_truncated_records", b"8\nshort.txt\n6\n12\n"))
        .assert_matches();
    with_scene_files(Case::new("load_count_huge", b"8\ncount_huge.txt\n6\n12\n")).assert_matches();
    with_scene_files(Case::new("load_count_overflow", b"8\ncount_ov.txt\n6\n12\n"))
        .assert_matches();
}

#[test]
fn option8_load_scene_success_paths() {
    with_scene_files(Case::new("load_good", b"8\ngood.txt\n5\n0\n12\n")).assert_matches();
    // A negative count skips the loop entirely but still reports success.
    with_scene_files(Case::new("load_negative_count", b"8\ncount_neg.txt\n5\n0\n6\n12\n"))
        .assert_matches();
    with_scene_files(Case::new("load_zero_count", b"8\ncount_zero.txt\n5\n0\n12\n"))
        .assert_matches();
    // shape_get returns NULL for out of range types, so they are skipped.
    with_scene_files(Case::new("load_out_of_range_type", b"8\nbadtype.txt\n5\n0\n12\n"))
        .assert_matches();
    with_scene_files(Case::new("load_negative_type", b"8\nnegtype.txt\n5\n0\n12\n"))
        .assert_matches();
    with_scene_files(Case::new("load_type_overflow", b"8\ntype_ov.txt\n5\n0\n12\n"))
        .assert_matches();
    // fscanf("%d\n") skips arbitrary white space between records.
    with_scene_files(Case::new("load_extra_whitespace", b"8\nws.txt\n5\n0\n12\n")).assert_matches();
    with_scene_files(Case::new("load_records_on_one_line", b"8\ninline.txt\n5\n0\n12\n"))
        .assert_matches();
    with_scene_files(Case::new("load_tab_separated", b"8\ntabs.txt\n5\n0\n12\n")).assert_matches();
    with_scene_files(Case::new("load_crlf", b"8\ncrlf.txt\n5\n0\n12\n")).assert_matches();
    with_scene_files(Case::new("load_plus_signs", b"8\nplus.txt\n5\n0\n12\n")).assert_matches();
    with_scene_files(Case::new("load_leading_zeros", b"8\nzeros.txt\n5\n0\n12\n")).assert_matches();
    // Junk after the last record is never read.
    with_scene_files(Case::new("load_trailing_junk", b"8\ntrailing.txt\n5\n0\n12\n"))
        .assert_matches();
    with_scene_files(Case::new("load_no_final_newline", b"8\nnonl_end.txt\n5\n0\n12\n"))
        .assert_matches();
    // Names in the file: empty, over-long, NUL-bearing, high bytes.
    with_scene_files(Case::new("load_empty_name", b"8\nemptyname.txt\n5\n0\n7\n0\nre.txt\n12\n"))
        .assert_matches();
    with_scene_files(Case::new("load_over_long_name", b"8\nlongname.txt\n6\n12\n"))
        .assert_matches();
    with_scene_files(Case::new("load_name_63_plus", b"8\nname63plus.txt\n5\n0\n6\n12\n"))
        .assert_matches();
    with_scene_files(Case::new("load_name_with_nul", b"8\nnul.txt\n5\n0\n12\n")).assert_matches();
    with_scene_files(Case::new("load_name_high_bytes", b"8\nhigh.txt\n5\n0\n12\n"))
        .assert_matches();
    // Loading twice yields two independent scenes.
    with_scene_files(Case::new("load_twice", b"8\ngood.txt\n8\ngood.txt\n10\n0\n1\n12\n"))
        .assert_matches();
}

#[test]
fn option8_load_hits_the_scene_shape_cap() {
    // 55 records into a 50 slot scene: five stderr messages, then success.
    with_scene_files(Case::new("load_overfull", b"8\noverfull.txt\n5\n0\n12\n")).assert_matches();
    with_scene_files(Case::new("load_overfull_merged", b"8\noverfull.txt\n5\n0\n12\n"))
        .merged_streams()
        .assert_matches();
}

#[test]
fn option8_load_at_maximum_scene_count() {
    let mut s = String::new();
    for c in "abcdefghij".chars() {
        s.push_str(&format!("2\n{c}\n"));
    }
    s.push_str("8\ngood.txt\n12\n");
    with_scene_files(Case::new("load_when_scenes_full", s)).assert_matches();
}

#[test]
fn save_then_load_round_trip() {
    Case::new(
        "round_trip",
        b"2\nRound\n3\n0\n4\n3\n0\n9\n7\n0\nrt.txt\n8\nrt.txt\n5\n1\n10\n0\n1\n12\n",
    )
    .assert_matches();
    Case::new(
        "round_trip_empty_name",
        b"2\n\n7\n0\nen.txt\n8\nen.txt\n10\n0\n1\n12\n",
    )
    .assert_matches();
}

// ===========================================================================
// Option 9 -- compare two shapes
// ===========================================================================

#[test]
fn option9_compare_shapes() {
    Case::new("cmp_shapes_same", b"9\n3\n3\n12\n").assert_matches();
    Case::new("cmp_shapes_different", b"9\n0\n9\n12\n").assert_matches();
    Case::new("cmp_shapes_first_and_last", b"9\n0\n0\n12\n").assert_matches();
    // The range check happens only after BOTH numbers are read.
    Case::new("cmp_shapes_bad_first", b"9\n99\n0\n12\n").assert_matches();
    Case::new("cmp_shapes_bad_second", b"9\n0\n99\n12\n").assert_matches();
    Case::new("cmp_shapes_bad_both", b"9\n-3\n-4\n12\n").assert_matches();
    Case::new("cmp_shapes_boundary_10", b"9\n10\n10\n12\n").assert_matches();
    // A matching failure on either read returns early.
    Case::new("cmp_shapes_non_numeric_first", b"9\nzz\n0\n12\n").assert_matches();
    Case::new("cmp_shapes_non_numeric_second", b"9\n0\nzz\n12\n").assert_matches();
    // Every pair of adjacent types, to pin the printed addresses.
    let mut s = String::new();
    for t in 0..10 {
        s.push_str(&format!("9\n{t}\n{}\n", (t + 1) % 10));
    }
    s.push_str("12\n");
    Case::new("cmp_shapes_all_pairs", s).assert_matches();
}

// ===========================================================================
// Option 10 -- compare two scenes
// ===========================================================================

#[test]
fn option10_compare_scenes_guards() {
    Case::new("cmp_scenes_none", b"10\n12\n").assert_matches();
    Case::new("cmp_scenes_only_one", b"2\na\n10\n12\n").assert_matches();
    Case::new("cmp_scenes_bad_first", b"2\na\n2\nb\n10\n5\n0\n12\n").assert_matches();
    Case::new("cmp_scenes_bad_second", b"2\na\n2\nb\n10\n0\n5\n12\n").assert_matches();
    Case::new("cmp_scenes_negative", b"2\na\n2\nb\n10\n-1\n0\n12\n").assert_matches();
    Case::new("cmp_scenes_non_numeric_first", b"2\na\n2\nb\n10\nzz\n0\n12\n").assert_matches();
    Case::new("cmp_scenes_non_numeric_second", b"2\na\n2\nb\n10\n0\nzz\n12\n").assert_matches();
}

#[test]
fn option10_compare_scenes_equality_logic() {
    Case::new("cmp_scenes_both_empty", b"2\na\n2\nb\n10\n0\n1\n12\n").assert_matches();
    Case::new("cmp_scenes_same_index", b"2\na\n3\n0\n0\n10\n0\n0\n12\n").assert_matches();
    Case::new("cmp_scenes_equal", b"2\na\n2\nb\n3\n0\n0\n3\n1\n0\n10\n0\n1\n12\n")
        .assert_matches();
    Case::new(
        "cmp_scenes_permuted",
        b"2\na\n2\nb\n3\n0\n0\n3\n0\n1\n3\n1\n1\n3\n1\n0\n10\n0\n1\n12\n",
    )
    .assert_matches();
    Case::new(
        "cmp_scenes_reversed",
        b"2\na\n2\nb\n3\n0\n0\n3\n0\n1\n3\n0\n2\n3\n1\n2\n3\n1\n1\n3\n1\n0\n10\n0\n1\n12\n",
    )
    .assert_matches();
    Case::new("cmp_scenes_count_mismatch", b"2\na\n2\nb\n3\n0\n0\n10\n0\n1\n12\n")
        .assert_matches();
    Case::new(
        "cmp_scenes_same_count_diff_shapes",
        b"2\na\n2\nb\n3\n0\n0\n3\n1\n1\n10\n0\n1\n12\n",
    )
    .assert_matches();
    // Duplicates: the `matched[]` bookkeeping must refuse [Tree,Tree] vs
    // [Tree,House].
    Case::new(
        "cmp_scenes_duplicate_vs_unique",
        b"2\na\n2\nb\n3\n0\n0\n3\n0\n0\n3\n1\n0\n3\n1\n2\n10\n0\n1\n12\n",
    )
    .assert_matches();
    Case::new(
        "cmp_scenes_duplicate_vs_duplicate",
        b"2\na\n2\nb\n3\n0\n0\n3\n0\n0\n3\n1\n0\n3\n1\n0\n10\n0\n1\n12\n",
    )
    .assert_matches();
    Case::new(
        "cmp_scenes_after_remove",
        b"2\na\n2\nb\n3\n0\n0\n3\n0\n7\n3\n1\n7\n4\n0\n1\n10\n0\n1\n12\n",
    )
    .assert_matches();
}

#[test]
fn option10_compare_full_scenes() {
    // Two 50 shape scenes: identical multisets in opposite order.
    with_scene_files(Case::new(
        "cmp_scenes_fifty_permuted",
        b"8\nfifty.txt\n8\nfiftyrev.txt\n10\n0\n1\n12\n",
    ))
    .assert_matches();
    with_scene_files(Case::new(
        "cmp_scenes_fifty_same",
        b"8\nfifty.txt\n8\nfifty.txt\n10\n0\n1\n12\n",
    ))
    .assert_matches();
}

// ===========================================================================
// Option 11 -- delete scene
// ===========================================================================

#[test]
fn option11_delete_scene() {
    Case::new("delete_no_scenes", b"11\n12\n").assert_matches();
    Case::new("delete_bad_index", b"2\na\n11\n4\n12\n").assert_matches();
    Case::new("delete_negative_index", b"2\na\n11\n-1\n12\n").assert_matches();
    Case::new("delete_non_numeric", b"2\na\n11\nzz\n12\n").assert_matches();
    Case::new("delete_only_scene", b"2\na\n11\n0\n6\n5\n0\n12\n").assert_matches();
    Case::new("delete_last_scene", b"2\na\n2\nb\n11\n1\n6\n5\n0\n12\n").assert_matches();
    // Deleting shifts the remaining scenes down.
    Case::new(
        "delete_middle_then_view",
        b"2\na\n2\nb\n2\nc\n3\n2\n5\n11\n0\n5\n1\n6\n12\n",
    )
    .assert_matches();
    // Free a slot at the maximum, then create again.
    let mut s = String::new();
    for c in "abcdefghij".chars() {
        s.push_str(&format!("2\n{c}\n"));
    }
    s.push_str("11\n0\n2\nnew\n6\n12\n");
    Case::new("delete_then_create_at_max", s).assert_matches();
}

// ===========================================================================
// Buffering, stream interleaving and stdin delivery
// ===========================================================================

#[test]
fn stderr_and_stdout_interleaving() {
    // stdout is block buffered on a pipe/file, stderr is unbuffered, so the
    // merged ordering is a property of the buffer size.
    Case::new("merged_save_error", b"2\na\n7\n0\nno_such_dir/x.txt\n12\n")
        .merged_streams()
        .assert_matches();
    Case::new("merged_load_error", b"8\nnope.txt\n12\n")
        .merged_streams()
        .assert_matches();
    Case::new("merged_view_all_shapes", b"1\n1\n1\n12\n")
        .merged_streams()
        .assert_matches();
}

#[test]
fn stdin_delivered_through_a_pipe() {
    // A pipe can hand back short reads where a regular file would not.
    Case::new("pipe_exit", b"12\n").piped_stdin().assert_matches();
    Case::new("pipe_view_shapes", b"1\n12\n").piped_stdin().assert_matches();
    Case::new("pipe_scanf_session", b"2\nS\n3\n0\n0\n5\n0\n9\n1\n2\n12\n")
        .piped_stdin()
        .assert_matches();
    with_scene_files(Case::new("pipe_load", b"8\ngood.txt\n5\n0\n12\n"))
        .piped_stdin()
        .assert_matches();
}

#[test]
fn stdin_larger_than_the_read_buffer() {
    // Well past 4096 bytes of input and far past it in output, mixing fgets and
    // scanf so buffer refills land in the middle of both.
    let mut s = String::from("2\nBigScene\n");
    let mut n = 0;
    while s.len() < 9000 {
        s.push_str("6\n");
        s.push_str(&format!("3\n0\n{}\n", n % 10));
        s.push_str(&format!("9\n{}\n{}\n", n % 10, (n + 3) % 10));
        n += 1;
    }
    s.push_str("5\n0\n12\n");
    Case::new("big_stdin_session", s.clone()).assert_matches();
    Case::new("big_stdin_session_piped", s).piped_stdin().assert_matches();
}

#[test]
fn numbers_straddling_the_stdin_buffer_boundary() {
    // Place a multi-digit scanf number across the 4096 byte read boundary so
    // that the digit run, and the push-back of the terminating character, span
    // two refills.
    for off in [4090u32, 4093, 4094, 4095, 4096, 4097, 4100] {
        let mut head = String::new();
        let target = off as usize;
        while head.len() + 8 < target {
            head.push_str("6\n");
        }
        while head.len() + 8 < target {
            head.push(' ');
        }
        head.push_str("9\n1234567\n0\n12\n");
        Case::new(&format!("straddle_{off}"), head).assert_matches();
    }
}

// ===========================================================================
// The C code's EOF spin: `while (getchar() != '\n');`
// ===========================================================================

#[test]
fn getchar_loop_spins_forever_at_eof() {
    // After a successful scanf the C code drains the rest of the line with
    // `while (getchar() != '\n');`. At end of file getchar() keeps returning
    // EOF, which never equals '\n', so the program never terminates. The
    // translation must hang identically -- both are killed by `timeout` and
    // must have flushed exactly the same bytes (none, because stdout is block
    // buffered).
    Case::new("eof_spin_after_compare_shapes", b"9\n5")
        .timeout(3)
        .assert_matches();
    Case::new("eof_spin_after_view_scene", b"2\na\n5\n0")
        .timeout(3)
        .assert_matches();
    Case::new("eof_spin_after_bad_scanf", b"2\na\n3\nx")
        .timeout(3)
        .assert_matches();
    Case::new("eof_spin_after_delete", b"2\na\n11\n0")
        .timeout(3)
        .assert_matches();

    // For contrast: option 3 with no scenes returns before any scanf, so this
    // input terminates normally.
    Case::new("no_spin_when_option3_returns_early", b"3\n0").assert_matches();
}
