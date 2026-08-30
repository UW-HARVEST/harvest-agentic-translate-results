//! Case 5 (find shortest path): `find_shortest_path`, including the paths where
//! it gives up and the ones where its arithmetic overflows.

mod harness;
use harness::{same, same_merged};

/// `1\n<name>\n` for each name, then an edge for each triple.
fn build(cities: &[&str], edges: &[(&str, &str, i64)]) -> Vec<u8> {
    let mut input = Vec::new();
    for c in cities {
        input.extend_from_slice(format!("1\n{c}\n").as_bytes());
    }
    for (from, to, d) in edges {
        input.extend_from_slice(format!("2\n{from}\n{to}\n{d}\n").as_bytes());
    }
    input
}

#[test]
fn simple_path() {
    let mut input = build(&["A", "B"], &[("A", "B", 5)]);
    input.extend_from_slice(b"5\nA\nB\n8\n");
    same("simple_path", &input);
}

#[test]
fn path_eof_at_each_prompt() {
    same("path_eof_start", b"5\n");
    same("path_eof_end", b"5\nA\n");
    same("path_eof_end_no_newline", b"5\nA");
}

#[test]
fn unknown_endpoints() {
    same("path_start_unknown", b"5\nX\nY\n8\n");
    same("path_end_unknown", b"1\nA\n5\nA\nZ\n8\n");
    same("path_start_unknown_end_known", b"1\nA\n5\nZ\nA\n8\n");
    same("path_empty_names", b"1\n\n5\n\n\n8\n");
}

#[test]
fn start_equals_end() {
    // The loop breaks on the first iteration: a one-node path.
    same("same_node_no_edges", b"1\nA\n5\nA\nA\n8\n");
    let mut input = build(&["A", "B"], &[("A", "B", 3)]);
    input.extend_from_slice(b"5\nA\nA\n5\nB\nB\n8\n");
    same("same_node_with_edges", &input);
}

#[test]
fn no_path() {
    // "No path found" goes to stderr from the library and to stdout from main.
    let mut input = build(&["A", "B"], &[]);
    input.extend_from_slice(b"5\nA\nB\n8\n");
    same("no_path_disconnected", &input);

    let mut input = build(&["A", "B", "C"], &[("A", "B", 1)]);
    input.extend_from_slice(b"5\nA\nC\n5\nB\nA\n5\nC\nB\n8\n");
    same("no_path_wrong_direction", &input);
}

#[test]
fn multi_hop_and_tie_breaking() {
    let mut input = build(
        &["A", "B", "C", "D", "E"],
        &[
            ("A", "B", 1),
            ("A", "C", 1),
            ("B", "D", 1),
            ("C", "D", 1),
            ("D", "E", 1),
        ],
    );
    input.extend_from_slice(b"5\nA\nE\n5\nE\nA\n5\nA\nA\n5\nB\nE\n8\n");
    same("multi_hop", &input);
}

#[test]
fn cheaper_longer_route_wins() {
    let mut input = build(
        &["A", "B", "C", "D"],
        &[("A", "D", 100), ("A", "B", 1), ("B", "C", 1), ("C", "D", 1)],
    );
    input.extend_from_slice(b"5\nA\nD\n8\n");
    same("cheaper_longer_route", &input);
}

#[test]
fn zero_weight_edges() {
    let mut input = build(&["A", "B", "C"], &[("A", "B", 0), ("B", "C", 0)]);
    input.extend_from_slice(b"5\nA\nC\n8\n");
    same("zero_weight_edges", &input);
}

#[test]
fn int_max_edge_is_never_an_improvement() {
    // A neighbour starts at INT_MAX and `new_distance < distance` is false when
    // the edge weight is INT_MAX, so the reachable node reports no path.
    let mut input = build(&["A", "B"], &[("A", "B", 2147483647)]);
    input.extend_from_slice(b"5\nA\nB\n8\n");
    same("int_max_edge", &input);
}

#[test]
fn int_max_edge_with_an_alternative() {
    let mut input = build(
        &["A", "B", "C"],
        &[("A", "B", 2147483647), ("A", "C", 1), ("C", "B", 1)],
    );
    input.extend_from_slice(b"5\nA\nB\n8\n");
    same("int_max_edge_alternative", &input);
}

#[test]
fn distance_sum_overflows() {
    // 2147483647 + 2147483647 wraps to -2, which beats every other candidate.
    let mut input = build(
        &["A", "B", "C"],
        &[("A", "B", 2147483646), ("B", "C", 2147483647), ("A", "C", 10)],
    );
    input.extend_from_slice(b"5\nA\nC\n3\n8\n");
    same("distance_sum_overflows", &input);
}

#[test]
fn cycles() {
    let mut input = build(&["A", "B", "C"], &[("A", "B", 1), ("B", "C", 1), ("C", "A", 1)]);
    input.extend_from_slice(b"5\nA\nC\n5\nC\nB\n5\nA\nA\n8\n");
    same("cycles", &input);

    let mut input = build(&["A", "B"], &[("A", "A", 0), ("A", "B", 1)]);
    input.extend_from_slice(b"5\nA\nB\n8\n");
    same("self_loop_then_path", &input);
}

#[test]
fn long_chain() {
    // A 100-node chain: the longest path this program can hold.
    let names: Vec<String> = (0..100).map(|i| format!("N{i:03}")).collect();
    let refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
    let edges: Vec<(&str, &str, i64)> = refs
        .windows(2)
        .map(|w| (w[0], w[1], 2i64))
        .collect();
    let mut input = build(&refs, &edges);
    input.extend_from_slice(format!("5\n{}\n{}\n", refs[0], refs[99]).as_bytes());
    input.extend_from_slice(format!("5\n{}\n{}\n", refs[99], refs[0]).as_bytes());
    input.extend_from_slice(format!("5\n{}\n{}\n", refs[50], refs[99]).as_bytes());
    input.extend_from_slice(b"8\n");
    same("long_chain", &input);
}

#[test]
fn dense_graph() {
    // Every node with the full complement of ten edges.
    let names: Vec<String> = (0..20).map(|i| format!("D{i}")).collect();
    let mut input = Vec::new();
    for n in &names {
        input.extend_from_slice(format!("1\n{n}\n").as_bytes());
    }
    for (i, from) in names.iter().enumerate() {
        for step in 1..=10 {
            let to = &names[(i + step) % names.len()];
            input.extend_from_slice(format!("2\n{from}\n{to}\n{}\n", step * 3).as_bytes());
        }
    }
    input.extend_from_slice(b"5\nD0\nD19\n5\nD19\nD0\n5\nD5\nD5\n3\n8\n");
    same("dense_graph", &input);
}

#[test]
fn merged_streams_no_path() {
    let mut input = build(&["A", "B"], &[]);
    input.extend_from_slice(b"5\nA\nB\n5\nA\nQ\n8\n");
    same_merged("merged_no_path", &input);
}
