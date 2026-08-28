use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_TEMP_DIR: AtomicUsize = AtomicUsize::new(0);

struct TempDir(PathBuf);

impl TempDir {
    fn new(label: &str, implementation: &str) -> Self {
        let serial = NEXT_TEMP_DIR.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "driver-differential-{}-{label}-{implementation}-{serial}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("create isolated test directory");
        Self(path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn c_binary() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../c_src/build/driver")
        .canonicalize()
        .expect("build the C driver in c_src/build before running cargo test")
}

fn rust_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

fn run(
    binary: &Path,
    label: &str,
    implementation: &str,
    input: &[u8],
    fixtures: &[(&str, Vec<u8>)],
) -> Output {
    let directory = TempDir::new(label, implementation);
    for (name, contents) in fixtures {
        fs::write(directory.0.join(name), contents).expect("write test fixture");
    }

    let mut child = Command::new(binary)
        .current_dir(&directory.0)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start driver");
    child
        .stdin
        .take()
        .expect("open driver stdin")
        .write_all(input)
        .expect("write driver input");
    child.wait_with_output().expect("wait for driver")
}

fn is_hex(byte: u8) -> bool {
    byte.is_ascii_hexdigit()
}

// Process addresses cannot have equal bytes across independent ASLR-enabled
// processes. Preserve their first-seen identity so singleton reuse is checked.
fn canonicalize_pointers(bytes: &[u8]) -> Vec<u8> {
    let mut identities: HashMap<Vec<u8>, usize> = HashMap::new();
    let mut result = Vec::with_capacity(bytes.len());
    let mut index = 0;

    while index < bytes.len() {
        if bytes.get(index..index + 2) == Some(b"0x")
            && bytes.get(index + 2).is_some_and(|byte| is_hex(*byte))
        {
            let mut end = index + 2;
            while bytes.get(end).is_some_and(|byte| is_hex(*byte)) {
                end += 1;
            }
            let pointer = bytes[index..end].to_vec();
            let next_identity = identities.len();
            let identity = *identities.entry(pointer).or_insert(next_identity);
            result.extend_from_slice(format!("0x<PTR{identity}>").as_bytes());
            index = end;
        } else {
            result.push(bytes[index]);
            index += 1;
        }
    }

    result
}

fn assert_case(label: &str, input: &[u8], fixtures: &[(&str, Vec<u8>)]) {
    let c = run(&c_binary(), label, "c", input, fixtures);
    let rust = run(&rust_binary(), label, "rust", input, fixtures);

    assert_eq!(
        c.status.code(),
        rust.status.code(),
        "{label}: exit status differs"
    );
    assert_eq!(
        c.stderr,
        rust.stderr,
        "{label}: stderr differs\nC: {:?}\nRust: {:?}",
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&rust.stderr)
    );

    let c_stdout = canonicalize_pointers(&c.stdout);
    let rust_stdout = canonicalize_pointers(&rust.stdout);
    assert_eq!(
        c_stdout,
        rust_stdout,
        "{label}: stdout differs\nC: {:?}\nRust: {:?}",
        String::from_utf8_lossy(&c_stdout),
        String::from_utf8_lossy(&rust_stdout)
    );
}

fn repeated_create(count: usize) -> Vec<u8> {
    let mut input = Vec::new();
    for index in 0..count {
        input.extend_from_slice(format!("2\nScene {index}\n").as_bytes());
    }
    input
}

fn repeated_add(count: usize, scene: usize, shape: usize) -> Vec<u8> {
    let mut input = Vec::new();
    for _ in 0..count {
        input.extend_from_slice(format!("3\n{scene}\n{shape}\n").as_bytes());
    }
    input
}

#[test]
fn startup_menu_and_top_level_choices() {
    let cases: &[(&str, &[u8])] = &[
        ("empty_input", b""),
        ("normal_exit", b"12\n"),
        ("all_shapes", b"1\n12\n"),
        ("empty_scene_list", b"6\n12\n"),
        ("invalid_text_and_choices", b"bad\n0\n13\n12\n"),
        ("main_fgets_embedded_nul", b"12\0ignored\n"),
        ("choice_i32_overflow", b"2147483648\n12\n"),
        (
            "choice_far_beyond_i64",
            b"999999999999999999999999999999999999999999\n12\n",
        ),
    ];
    for (label, input) in cases {
        assert_case(label, input, &[]);
    }

    let mut full_line = vec![b'9'; 255];
    full_line.extend_from_slice(b"\n12\n");
    assert_case("main_fgets_full_buffer", &full_line, &[]);
}

#[test]
fn scene_creation_empty_single_and_maximum() {
    let cases: &[(&str, &[u8])] = &[
        ("create_name_eof", b"2\n"),
        ("create_single", b"2\nAlpha\n6\n12\n"),
        ("create_embedded_nul", b"2\nA\0B\n6\n12\n"),
        ("create_crlf_name", b"2\nA\r\n6\n12\n"),
    ];
    for (label, input) in cases {
        assert_case(label, input, &[]);
    }

    let mut maximum = repeated_create(11);
    maximum.extend_from_slice(b"12\n");
    assert_case("maximum_ten_scenes", &maximum, &[]);

    let mut name_at_limit = b"2\n".to_vec();
    name_at_limit.extend(std::iter::repeat_n(b'a', 62));
    name_at_limit.extend_from_slice(b"\n6\n12\n");
    assert_case("scene_name_62_bytes_plus_newline", &name_at_limit, &[]);

    let mut truncated_name = b"2\n".to_vec();
    truncated_name.extend(std::iter::repeat_n(b'b', 63));
    truncated_name.extend_from_slice(b"\n12\n");
    assert_case("scene_name_63_bytes_leaves_newline", &truncated_name, &[]);
}

#[test]
fn adding_shapes_covers_validation_singleton_and_capacity() {
    let cases: &[(&str, &[u8])] = &[
        ("add_without_scene", b"3\n12\n"),
        ("add_invalid_scene_scan", b"2\nA\n3\nbad\n12\n"),
        ("add_negative_scene", b"2\nA\n3\n-1\n12\n"),
        ("add_high_scene", b"2\nA\n3\n1\n12\n"),
        ("add_invalid_shape_scan", b"2\nA\n3\n0\nbad\n12\n"),
        ("add_negative_shape", b"2\nA\n3\n0\n-1\n12\n"),
        ("add_high_shape", b"2\nA\n3\n0\n10\n12\n"),
        ("add_single_max_type", b"2\nA\n3\n0\n9\n5\n0\n12\n"),
        ("scanf_skips_blank_lines", b"2\nA\n3\n\n0\n\n9\n12\n"),
    ];
    for (label, input) in cases {
        assert_case(label, input, &[]);
    }

    let mut maximum = b"2\nA\n".to_vec();
    maximum.extend_from_slice(&repeated_add(51, 0, 0));
    maximum.extend_from_slice(b"12\n");
    assert_case("maximum_fifty_shapes_and_full_error", &maximum, &[]);
}

#[test]
fn removing_and_viewing_shapes_covers_all_indices() {
    let cases: &[(&str, &[u8])] = &[
        ("remove_without_scene", b"4\n12\n"),
        ("remove_invalid_scene_scan", b"2\nA\n4\nbad\n12\n"),
        ("remove_invalid_scene_index", b"2\nA\n4\n1\n12\n"),
        ("remove_empty_scene", b"2\nA\n4\n0\n12\n"),
        (
            "remove_invalid_shape_scan",
            b"2\nA\n3\n0\n0\n4\n0\nbad\n12\n",
        ),
        ("remove_shape_zero", b"2\nA\n3\n0\n0\n4\n0\n0\n12\n"),
        (
            "remove_shape_int_min_underflow",
            b"2\nA\n3\n0\n0\n4\n0\n-2147483648\n12\n",
        ),
        ("remove_shape_too_high", b"2\nA\n3\n0\n0\n4\n0\n2\n12\n"),
        ("remove_single_shape", b"2\nA\n3\n0\n0\n4\n0\n1\n12\n"),
        ("view_without_scene", b"5\n12\n"),
        ("view_invalid_scan", b"2\nA\n5\nbad\n12\n"),
        ("view_negative_index", b"2\nA\n5\n-1\n12\n"),
        ("view_high_index", b"2\nA\n5\n1\n12\n"),
        ("view_empty_scene", b"2\nA\n5\n0\n12\n"),
    ];
    for (label, input) in cases {
        assert_case(label, input, &[]);
    }
}

#[test]
fn save_paths_cover_guards_eof_success_and_open_error() {
    let cases: &[(&str, &[u8])] = &[
        ("save_without_scene", b"7\n12\n"),
        ("save_invalid_scan", b"2\nA\n7\nbad\n12\n"),
        ("save_negative_index", b"2\nA\n7\n-1\n12\n"),
        ("save_high_index", b"2\nA\n7\n1\n12\n"),
        ("save_filename_eof", b"2\nA\n7\n0\n"),
        ("save_success", b"2\nA\n3\n0\n9\n7\n0\nsaved.scene\n12\n"),
        (
            "save_embedded_nul_filename",
            b"2\nA\n7\n0\nsaved\0ignored\n12\n",
        ),
        ("save_open_error", b"2\nA\n7\n0\n/\n12\n"),
    ];
    for (label, input) in cases {
        assert_case(label, input, &[]);
    }

    let mut maximum_filename = b"2\nA\n7\n0\n".to_vec();
    maximum_filename.extend(std::iter::repeat_n(b'f', 255));
    maximum_filename.extend_from_slice(b"\n12\n");
    assert_case("filename_fills_fgets_buffer", &maximum_filename, &[]);
}

#[test]
fn load_paths_cover_every_parse_and_capacity_branch() {
    let full_fifty = b"Full\n50\n"
        .iter()
        .copied()
        .chain((0..50).flat_map(|_| b"0\n".iter().copied()))
        .collect::<Vec<_>>();
    let overfull = b"Overfull\n51\n"
        .iter()
        .copied()
        .chain((0..51).flat_map(|_| b"0\n".iter().copied()))
        .collect::<Vec<_>>();
    let fixtures = vec![
        ("empty", Vec::new()),
        ("no_count", b"Name\n".to_vec()),
        ("bad_count", b"Name\nbad\n".to_vec()),
        ("overflow_count", b"Overflow\n2147483648\n".to_vec()),
        ("negative", b"Negative\n-2\n".to_vec()),
        ("missing_type", b"Missing\n2\n0\n".to_vec()),
        ("invalid_types", b"Invalid\n3\n-1\n10\n9\n".to_vec()),
        ("valid", b"Valid\n2\n0\n9\n".to_vec()),
        ("full_fifty", full_fifty),
        ("overfull", overfull),
    ];
    let cases: &[(&str, &[u8])] = &[
        ("load_filename_eof", b"8\n"),
        ("load_missing_file", b"8\nmissing\n12\n"),
        ("load_empty_file", b"8\nempty\n12\n"),
        ("load_missing_count", b"8\nno_count\n12\n"),
        ("load_invalid_count", b"8\nbad_count\n12\n"),
        ("load_overflowed_count", b"8\noverflow_count\n6\n12\n"),
        ("load_negative_count", b"8\nnegative\n6\n12\n"),
        ("load_missing_shape_type", b"8\nmissing_type\n12\n"),
        (
            "load_ignores_invalid_types",
            b"8\ninvalid_types\n5\n0\n12\n",
        ),
        ("load_single_valid_scene", b"8\nvalid\n5\n0\n12\n"),
        (
            "load_embedded_nul_filename",
            b"8\nvalid\0ignored\n5\n0\n12\n",
        ),
        ("load_maximum_fifty_shapes", b"8\nfull_fifty\n5\n0\n12\n"),
        ("load_reports_shapes_over_capacity", b"8\noverfull\n12\n"),
    ];
    for (label, input) in cases {
        assert_case(label, input, &fixtures);
    }

    let mut max_scenes = repeated_create(10);
    max_scenes.extend_from_slice(b"8\n12\n");
    assert_case("load_rejected_at_ten_scenes", &max_scenes, &fixtures);
}

#[test]
fn shape_and_scene_comparisons_cover_equal_and_unequal_paths() {
    let cases: &[(&str, &[u8])] = &[
        ("compare_shape_first_scan_error", b"9\nbad\n12\n"),
        ("compare_shape_second_scan_error", b"9\n0\nbad\n12\n"),
        ("compare_shape_negative_first", b"9\n-1\n0\n12\n"),
        ("compare_shape_high_second", b"9\n0\n10\n12\n"),
        ("compare_same_shape", b"9\n2\n2\n12\n"),
        ("compare_different_shapes", b"9\n2\n3\n12\n"),
        ("compare_scenes_too_few", b"10\n12\n"),
        (
            "compare_scene_first_scan_error",
            b"2\nA\n2\nB\n10\nbad\n12\n",
        ),
        (
            "compare_scene_second_scan_error",
            b"2\nA\n2\nB\n10\n0\nbad\n12\n",
        ),
        ("compare_scene_negative", b"2\nA\n2\nB\n10\n-1\n0\n12\n"),
        ("compare_scene_high", b"2\nA\n2\nB\n10\n0\n2\n12\n"),
        ("compare_empty_scenes_equal", b"2\nA\n2\nB\n10\n0\n1\n12\n"),
        (
            "compare_different_lengths",
            b"2\nA\n2\nB\n3\n0\n0\n10\n0\n1\n12\n",
        ),
        (
            "compare_different_contents",
            b"2\nA\n2\nB\n3\n0\n0\n3\n1\n1\n10\n0\n1\n12\n",
        ),
        (
            "compare_reordered_multiset_equal",
            b"2\nA\n2\nB\n3\n0\n0\n3\n0\n1\n3\n1\n1\n3\n1\n0\n10\n0\n1\n12\n",
        ),
    ];
    for (label, input) in cases {
        assert_case(label, input, &[]);
    }
}

#[test]
fn deleting_scenes_covers_empty_validation_and_shift() {
    let cases: &[(&str, &[u8])] = &[
        ("delete_without_scene", b"11\n12\n"),
        ("delete_invalid_scan", b"2\nA\n11\nbad\n12\n"),
        ("delete_negative_index", b"2\nA\n11\n-1\n12\n"),
        ("delete_high_index", b"2\nA\n11\n1\n12\n"),
        ("delete_only_scene", b"2\nA\n11\n0\n6\n12\n"),
        ("delete_shifts_scenes", b"2\nA\n2\nB\n11\n0\n6\n12\n"),
    ];
    for (label, input) in cases {
        assert_case(label, input, &[]);
    }
}
