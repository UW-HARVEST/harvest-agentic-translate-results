//! Case 1 (add city), case 3 (show all cities) and case 4 (show city details):
//! `add_node`, `get_node_by_name`, `print_node` and `print_graph`.

mod harness;
use harness::{same, same_merged};

#[test]
fn add_one_city() {
    same("add_one_city", b"1\nBoston\n3\n4\nBoston\n8\n");
}

#[test]
fn add_city_eof_after_prompt() {
    // fgets returns NULL inside the case: the switch breaks, the menu is
    // printed once more, and only then does the loop end.
    same("add_city_eof", b"1\n");
    same("add_city_eof_no_newline", b"1");
}

#[test]
fn duplicate_city() {
    // "Error: Node 'A' already exists" on stderr, "Failed to add city" on stdout.
    same("duplicate_city", b"1\nA\n1\nA\n3\n8\n");
}

#[test]
fn empty_city_name() {
    // strcspn puts the NUL at index 0, so the name is the empty string.
    same("empty_city_name", b"1\n\n3\n4\n\n8\n");
    same("empty_name_duplicate", b"1\n\n1\n\n8\n");
}

#[test]
fn whitespace_city_names() {
    same("spaces_name", b"1\n   \n3\n4\n   \n8\n");
    same("tab_name", b"1\nA\tB\n4\nA\tB\n8\n");
    same("cr_in_name", b"1\nA\r\n4\nA\r\n4\nA\n8\n");
}

#[test]
fn city_name_looks_like_a_command() {
    same("name_is_8", b"1\n8\n4\n8\n8\n");
    same("name_is_digits", b"1\n42\n4\n42\n8\n");
}

#[test]
fn city_name_length_boundaries() {
    // strncpy copies at most MAX_CITY_NAME - 1 = 63 bytes and byte 63 is
    // cleared, so 64 bytes and up are truncated.
    for len in [1usize, 62, 63, 64, 65, 70, 128] {
        let name = vec![b'X'; len];
        let mut input = Vec::new();
        input.extend_from_slice(b"1\n");
        input.extend_from_slice(&name);
        input.extend_from_slice(b"\n3\n4\n");
        input.extend_from_slice(&name);
        input.extend_from_slice(b"\n8\n");
        same(&format!("name_len_{len}"), &input);
    }
}

#[test]
fn truncated_names_collide() {
    // Two different long names share the first 63 bytes: the duplicate check
    // compares the full name against the stored, truncated one, so both are
    // added and both print the same truncated name.
    let mut input = Vec::new();
    input.extend_from_slice(b"1\n");
    input.extend_from_slice(&[b'Y'; 63]);
    input.extend_from_slice(b"AAA\n1\n");
    input.extend_from_slice(&[b'Y'; 63]);
    input.extend_from_slice(b"BBB\n3\n4\n");
    input.extend_from_slice(&[b'Y'; 63]);
    input.extend_from_slice(b"\n8\n");
    same("truncated_names_collide", &input);
}

#[test]
fn city_name_longer_than_the_input_buffer() {
    // 255 bytes fit; anything longer is split across fgets calls and the tail
    // is interpreted as the next command.
    for len in [254usize, 255, 256, 257, 300, 600] {
        let mut input = Vec::new();
        input.extend_from_slice(b"1\n");
        input.extend(std::iter::repeat(b'Z').take(len));
        input.extend_from_slice(b"\n3\n8\n");
        same(&format!("name_input_len_{len}"), &input);
    }
}

#[test]
fn city_name_with_embedded_nul() {
    // strcspn stops at the NUL, so only the bytes before it become the name.
    same("nul_in_name", b"1\nA\x00B\n3\n4\nA\n4\nA\x00B\n8\n");
    same("nul_only_name", b"1\n\x00\n3\n8\n");
}

#[test]
fn utf8_city_name() {
    same("utf8_name", "1\nZürich\n3\n4\nZürich\n8\n".as_bytes());
}

#[test]
fn show_empty_graph() {
    same("show_empty_graph", b"3\n8\n");
}

#[test]
fn show_graph_with_edges() {
    same(
        "show_graph_with_edges",
        b"1\nA\n1\nB\n1\nC\n2\nA\nB\n5\n2\nB\nC\n7\n3\n8\n",
    );
}

#[test]
fn city_details_not_found() {
    same("details_not_found", b"4\nQ\n8\n");
    same("details_not_found_after_add", b"1\nA\n4\nB\n8\n");
}

#[test]
fn city_details_eof() {
    same("details_eof", b"4\n");
}

#[test]
fn graph_is_full() {
    // MAX_NODES = 100; the 101st add fails on stderr and stdout.
    let mut input = Vec::new();
    for i in 0..101 {
        input.extend_from_slice(format!("1\nC{i}\n").as_bytes());
    }
    input.extend_from_slice(b"3\n8\n");
    same("graph_is_full", &input);
}

#[test]
fn graph_is_full_then_more_adds() {
    let mut input = Vec::new();
    for i in 0..100 {
        input.extend_from_slice(format!("1\nC{i}\n").as_bytes());
    }
    // Several further adds, including a duplicate: the full check comes first.
    input.extend_from_slice(b"1\nExtra\n1\nC0\n1\n\n4\nC99\n8\n");
    same("graph_full_then_more", &input);
}

#[test]
fn merged_streams_duplicate_city() {
    same_merged("merged_duplicate_city", b"1\nA\n1\nA\n3\n8\n");
}

#[test]
fn merged_streams_full_graph() {
    let mut input = Vec::new();
    for i in 0..101 {
        input.extend_from_slice(format!("1\nC{i}\n").as_bytes());
    }
    input.extend_from_slice(b"3\n8\n");
    same_merged("merged_full_graph", &input);
}
