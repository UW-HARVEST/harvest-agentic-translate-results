use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn c_binary() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build/driver")
}

fn rust_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

fn run(binary: &Path, input: &[u8]) -> Output {
    assert!(
        binary.is_file(),
        "missing executable {}; complete Phase A first",
        binary.display()
    );

    let mut child = Command::new(binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|error| panic!("failed to run {}: {error}", binary.display()));

    child
        .stdin
        .take()
        .expect("child stdin was not piped")
        .write_all(input)
        .expect("failed to write test input");
    child.wait_with_output().expect("failed to collect output")
}

fn compare_case(name: &str, input: &[u8]) {
    let c = run(&c_binary(), input);
    let rust = run(&rust_binary(), input);

    assert_eq!(rust.status, c.status, "{name}: exit status differs");
    assert_eq!(rust.stdout, c.stdout, "{name}: stdout differs");
    assert_eq!(rust.stderr, c.stderr, "{name}: stderr differs");
}

fn add_city(input: &mut Vec<u8>, city: &str) {
    input.extend_from_slice(b"1\n");
    input.extend_from_slice(city.as_bytes());
    input.push(b'\n');
}

fn add_route(input: &mut Vec<u8>, from: &str, to: &str, distance: &str) {
    input.extend_from_slice(b"2\n");
    input.extend_from_slice(from.as_bytes());
    input.push(b'\n');
    input.extend_from_slice(to.as_bytes());
    input.push(b'\n');
    input.extend_from_slice(distance.as_bytes());
    input.push(b'\n');
}

#[test]
fn startup_menu_parsing_and_exit_paths_match() {
    let cases: &[(&str, &[u8])] = &[
        ("empty stdin", b""),
        ("ordinary exit", b"8\n"),
        ("invalid input", b"not-a-number\n8\n"),
        ("numeric prefix accepted", b"8 trailing text\n"),
        ("invalid choices", b"0\n9\n-1\n8\n"),
        ("positive sign and whitespace", b" \t+8 ignored\n"),
        ("i32 positive overflow", b"2147483648\n8\n"),
        ("long overflow", b"9223372036854775808\n8\n"),
        ("negative long overflow", b"-9223372036854775809\n8\n"),
        ("sign without digits", b"+\n8\n"),
        ("embedded nul in choice", b"8\0not parsed\n"),
    ];

    for (name, input) in cases {
        compare_case(name, input);
    }
}

#[test]
fn eof_at_each_secondary_prompt_matches() {
    let cases: &[(&str, &[u8])] = &[
        ("add city prompt", b"1\n"),
        ("route from prompt", b"2\n"),
        ("route to prompt", b"2\nA\n"),
        ("route distance prompt", b"2\nA\nB\n"),
        ("details prompt", b"4\n"),
        ("path start prompt", b"5\n"),
        ("path end prompt", b"5\nA\n"),
        ("copy prompt", b"6\n"),
        ("delete prompt", b"7\n"),
    ];

    for (name, input) in cases {
        compare_case(name, input);
    }
}

#[test]
fn city_inputs_and_views_match() {
    compare_case("empty graph", b"3\n8\n");
    compare_case(
        "empty and single city",
        b"1\n\n3\n4\n\n1\nAlpha\n3\n4\nAlpha\n8\n",
    );
    compare_case("duplicate city", b"1\nAlpha\n1\nAlpha\n8\n");
    compare_case("missing city details", b"4\nmissing\n8\n");

    let mut nul_and_non_utf8 = b"1\nA\0ignored\n4\nA\n1\n".to_vec();
    nul_and_non_utf8.extend_from_slice(&[0xff, b'\n', b'4', b'\n', 0xff, b'\n', b'8', b'\n']);
    compare_case("byte-oriented city names", &nul_and_non_utf8);
    compare_case(
        "carriage return remains in city name",
        b"1\nA\r\n4\nA\r\n8\n",
    );

    let mut maximum_line = b"1\n".to_vec();
    maximum_line.extend(std::iter::repeat_n(b'L', 255));
    maximum_line.extend_from_slice(b"\n4\n");
    maximum_line.extend(std::iter::repeat_n(b'L', 63));
    maximum_line.extend_from_slice(b"\n8\n");
    compare_case(
        "255-byte fgets chunk and 63-byte stored name",
        &maximum_line,
    );

    let long_name = "X".repeat(64);
    let mut truncation_collision = Vec::new();
    add_city(&mut truncation_collision, &long_name);
    add_city(&mut truncation_collision, &long_name);
    truncation_collision.extend_from_slice(b"3\n8\n");
    compare_case(
        "duplicate check uses untruncated input before storage",
        &truncation_collision,
    );
}

#[test]
fn maximum_node_capacity_and_validation_order_match() {
    let mut input = Vec::new();
    for index in 0..100 {
        add_city(&mut input, &format!("N{index}"));
    }
    add_city(&mut input, "N100");
    add_city(&mut input, "N0");
    input.extend_from_slice(b"3\n8\n");

    compare_case("100 nodes then full graph errors", &input);
}

#[test]
fn route_success_and_every_validation_error_match() {
    compare_case(
        "invalid distance checked first",
        b"2\nmissing\nother\nx\n8\n",
    );
    compare_case("missing from city", b"2\nmissing\nother\n3\n8\n");

    let mut missing_to = Vec::new();
    add_city(&mut missing_to, "A");
    add_route(&mut missing_to, "A", "missing", "3");
    missing_to.extend_from_slice(b"8\n");
    compare_case("missing to city", &missing_to);

    let mut validations = Vec::new();
    add_city(&mut validations, "A");
    add_city(&mut validations, "B");
    add_route(&mut validations, "A", "B", "-1");
    add_route(&mut validations, "A", "B", "7");
    add_route(&mut validations, "A", "B", "9");
    validations.extend_from_slice(b"4\nA\n3\n8\n");
    compare_case("negative successful and duplicate routes", &validations);

    let mut overflow_distance = Vec::new();
    add_city(&mut overflow_distance, "A");
    add_city(&mut overflow_distance, "B");
    add_route(&mut overflow_distance, "A", "B", "9223372036854775808");
    overflow_distance.extend_from_slice(b"8\n");
    compare_case("overflowing distance parse", &overflow_distance);
}

#[test]
fn maximum_edge_capacity_and_check_order_match() {
    let mut input = Vec::new();
    add_city(&mut input, "Hub");
    for index in 0..11 {
        add_city(&mut input, &format!("D{index}"));
    }
    for index in 0..10 {
        add_route(&mut input, "Hub", &format!("D{index}"), &index.to_string());
    }
    add_route(&mut input, "Hub", "D10", "-1");
    add_route(&mut input, "Hub", "D0", "1");
    input.extend_from_slice(b"4\nHub\n8\n");

    compare_case("10 edges then capacity errors precede validation", &input);
}

#[test]
fn shortest_path_branches_match() {
    compare_case("missing path start", b"5\nmissing\nother\n8\n");

    let mut missing_end = Vec::new();
    add_city(&mut missing_end, "A");
    missing_end.extend_from_slice(b"5\nA\nmissing\n8\n");
    compare_case("missing path end", &missing_end);

    let mut same_city = Vec::new();
    add_city(&mut same_city, "A");
    same_city.extend_from_slice(b"5\nA\nA\n8\n");
    compare_case("start equals end", &same_city);

    let mut no_path = Vec::new();
    add_city(&mut no_path, "A");
    add_city(&mut no_path, "B");
    no_path.extend_from_slice(b"5\nA\nB\n8\n");
    compare_case("unreachable destination", &no_path);

    let mut maximum_distance = Vec::new();
    add_city(&mut maximum_distance, "A");
    add_city(&mut maximum_distance, "B");
    add_route(&mut maximum_distance, "A", "B", "2147483647");
    maximum_distance.extend_from_slice(b"5\nA\nB\n8\n");
    compare_case(
        "INT_MAX route remains at unreachable sentinel",
        &maximum_distance,
    );

    let mut competing_paths = Vec::new();
    for city in ["A", "B", "C", "D"] {
        add_city(&mut competing_paths, city);
    }
    add_route(&mut competing_paths, "A", "B", "10");
    add_route(&mut competing_paths, "A", "C", "2");
    add_route(&mut competing_paths, "C", "B", "3");
    add_route(&mut competing_paths, "B", "C", "10");
    add_route(&mut competing_paths, "B", "D", "1");
    add_route(&mut competing_paths, "C", "D", "20");
    competing_paths.extend_from_slice(b"5\nA\nD\n8\n");
    compare_case("distance update and multi-hop path", &competing_paths);

    let mut wrapping_path = Vec::new();
    for city in ["A", "B", "C"] {
        add_city(&mut wrapping_path, city);
    }
    add_route(&mut wrapping_path, "A", "B", "2147483646");
    add_route(&mut wrapping_path, "B", "C", "10");
    add_route(&mut wrapping_path, "A", "C", "5");
    wrapping_path.extend_from_slice(b"5\nA\nC\n8\n");
    compare_case("signed distance addition overflow", &wrapping_path);
}

#[test]
fn shallow_copy_and_delete_paths_match() {
    compare_case("copy missing city", b"6\nmissing\n8\n");
    compare_case("delete missing city", b"7\nmissing\n8\n");
    compare_case("delete sole reference then exit", b"1\nA\n7\nA\n8\n");

    let mut cycle = Vec::new();
    add_city(&mut cycle, "A");
    add_city(&mut cycle, "B");
    add_route(&mut cycle, "A", "B", "1");
    add_route(&mut cycle, "B", "A", "1");
    cycle.extend_from_slice(b"6\nA\n3\n7\nA\n3\n8\n");
    compare_case("copy cycle then delete referenced node", &cycle);
}
