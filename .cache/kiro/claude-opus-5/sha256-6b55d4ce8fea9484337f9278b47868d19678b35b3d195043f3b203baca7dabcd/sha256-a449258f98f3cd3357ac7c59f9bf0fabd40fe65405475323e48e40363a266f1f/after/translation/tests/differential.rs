//! Differential tests over the input classes `c_src` actually branches on.
//!
//! Every test runs both compiled programs as subprocesses and compares stdout,
//! stderr and the exit status.  The inputs are derived from reading
//! `c_src/src/main.c` and `c_src/src/lib.c` branch by branch; the mapping from
//! branch to test is spelled out in the comments.

mod harness;

use harness::{assert_identical, script};

// ---------------------------------------------------------------------------
// main(): the read loop and choice dispatch
// ---------------------------------------------------------------------------

/// `fgets(input, MAX_INPUT, stdin) == NULL` on the very first read.
#[test]
fn empty_input() {
    assert_identical("empty", b"");
}

/// A single newline: `sscanf("%d")` finds no digits -> "Invalid input".
#[test]
fn blank_lines_are_invalid_input() {
    assert_identical("blank", b"\n\n8\n");
}

/// `sscanf(input, "%d", &choice) != 1`.
#[test]
fn non_numeric_choice() {
    assert_identical("abc", b"abc\n8\n");
    assert_identical("bare-sign", b"+\n-\n8\n");
    assert_identical("nul-first", b"\x003\n8\n");
}

/// `%d` skips leading whitespace and stops at the first non-digit, and accepts a
/// sign.
#[test]
fn choice_whitespace_and_signs() {
    assert_identical("padded", b"  3  \n+8\n");
    assert_identical("vt-ff", b"\x0b\x0c 3\n\t8\n");
    assert_identical("trailing-junk", b"8 exit now\n");
}

/// Values that do not fit an `int`: glibc's `%d` saturates the accumulator and
/// the assignment truncates.
#[test]
fn choice_integer_overflow() {
    assert_identical("overflow", b"99999999999999999999\n-99999999999999999999\n8\n");
    assert_identical("int-min", b"-2147483648\n2147483648\n8\n");
}

/// `default:` in the switch.
#[test]
fn invalid_choice_values() {
    assert_identical("99", b"99\n8\n");
    assert_identical("0-and-neg", b"0\n-1\n8\n");
    assert_identical("9", b"9\n8\n");
}

/// `case 8:` returns from `main` after `free_graph`; falling off the end of
/// stdin leaves the loop the other way.
#[test]
fn exit_and_eof_paths() {
    assert_identical("exit", b"8\n");
    assert_identical("eof-after-menu", b"3\n");
    assert_identical("no-trailing-newline", b"1\nA");
}

/// A line longer than `MAX_INPUT - 1`: `fgets` returns the first 255 bytes and
/// the rest of the line is read by the *next* `fgets`, so the tail of an
/// over-long choice line becomes the next answer.
#[test]
fn overlong_lines_split_across_reads() {
    let mut long = String::from("1");
    long.push_str(&"z".repeat(300));
    assert_identical("long-choice", script(&[&long, "8"]).as_slice());

    let name = "Y".repeat(300);
    assert_identical("long-name", script(&["1", &name, "3", "8"]).as_slice());

    // Exactly at and one past the buffer boundary.
    assert_identical(
        "name-254",
        script(&["1", &"Z".repeat(254), "3", "8"]).as_slice(),
    );
    assert_identical(
        "name-255",
        script(&["1", &"Z".repeat(255), "3", "8"]).as_slice(),
    );
}

/// EOF at each of the nine `fgets` calls that live inside a `case`.  The `break`
/// there leaves the switch, not the loop, so the menu is printed once more
/// before the outer `fgets` finally returns NULL.
#[test]
fn eof_inside_every_case() {
    for (name, input) in [
        ("choice-no-newline", &b"1"[..]),
        ("case1-city", &b"1\n"[..]),
        ("case2-from", &b"2\n"[..]),
        ("case2-to", &b"2\nA\n"[..]),
        ("case2-distance", &b"1\nA\n1\nB\n2\nA\nB\n"[..]),
        ("case4-city", &b"4\n"[..]),
        ("case5-start", &b"5\n"[..]),
        ("case5-end", &b"5\nA\n"[..]),
        ("case6-city", &b"6\n"[..]),
        ("case7-city", &b"7\n"[..]),
    ] {
        assert_identical(name, input);
    }
}

// ---------------------------------------------------------------------------
// case 1 / add_node
// ---------------------------------------------------------------------------

#[test]
fn add_city_basic() {
    assert_identical("one", b"1\nBoston\n3\n8\n");
    assert_identical("several", b"1\nA\n1\nB\n1\nC\n3\n8\n");
}

/// `input[strcspn(input, "\n")] = 0` on a bare newline yields the empty string,
/// which `add_node` accepts and `get_node_by_name` can find.
#[test]
fn add_city_empty_name() {
    assert_identical("empty-name", b"1\n\n3\n4\n\n8\n");
}

/// `strcmp(...) == 0` in `add_node` -> "Error: Node '%s' already exists" on
/// stderr while stdout says "Failed to add city".
#[test]
fn add_city_duplicate() {
    assert_identical("dup", b"1\nA\n1\nA\n8\n");
    assert_identical("dup-empty", b"1\n\n1\n\n8\n");
}

/// `strncpy(node->city_name, city_name, MAX_CITY_NAME - 1)` truncates to 63
/// bytes, so two names sharing a 63 byte prefix collide.
#[test]
fn add_city_name_truncation() {
    let a = "X".repeat(100);
    assert_identical("trunc-100", script(&["1", &a, "4", &"X".repeat(63), "3", "8"]).as_slice());
    assert_identical(
        "trunc-63-64",
        script(&["1", &"A".repeat(63), "1", &"A".repeat(64), "3", "8"]).as_slice(),
    );
}

/// `graph->node_count >= MAX_NODES` -> "Error: Graph is full (max 100 nodes)".
#[test]
fn add_city_graph_full() {
    let mut lines: Vec<String> = Vec::new();
    for i in 0..100 {
        lines.push("1".into());
        lines.push(format!("C{i}"));
    }
    lines.extend(["1".into(), "OVER".into(), "3".into(), "8".into()]);
    let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
    assert_identical("graph-full", script(&refs).as_slice());
}

// ---------------------------------------------------------------------------
// case 2 / add_edge
// ---------------------------------------------------------------------------

#[test]
fn add_route_basic() {
    assert_identical("route", b"1\nA\n1\nB\n2\nA\nB\n100\n3\n8\n");
    assert_identical("route-zero", b"1\nA\n1\nB\n2\nA\nB\n0\n4\nA\n8\n");
    assert_identical("self-edge", b"1\nA\n2\nA\nA\n5\n4\nA\n8\n");
}

/// The "from" check runs before the "to" check, and both run *after* the
/// distance has been parsed.
#[test]
fn add_route_missing_cities() {
    assert_identical("no-cities", b"2\nA\nB\n50\n8\n");
    assert_identical("no-to", b"1\nA\n2\nA\nB\n50\n8\n");
    assert_identical("no-from", b"1\nB\n2\nA\nB\n50\n8\n");
}

/// `sscanf(input, "%d", &distance) != 1` -> "Invalid distance", and the city
/// lookups never happen.
#[test]
fn add_route_invalid_distance() {
    assert_identical("bad-distance", b"1\nA\n1\nB\n2\nA\nB\nxyz\n8\n");
    assert_identical("bad-distance-no-cities", b"2\nQ\nR\nxyz\n8\n");
}

/// `distance < 0` is rejected, but only after the max-edge check.
#[test]
fn add_route_negative_distance() {
    assert_identical("negative", b"1\nA\n1\nB\n2\nA\nB\n-5\n8\n");
}

/// `from->edges[i].destination == to` -> "Error: Edge already exists".
#[test]
fn add_route_duplicate() {
    assert_identical("dup-edge", b"1\nA\n1\nB\n2\nA\nB\n10\n2\nA\nB\n20\n8\n");
}

/// `from->edge_count >= MAX_EDGES` -> "Error: Node 'A' has maximum edges".
#[test]
fn add_route_max_edges() {
    let mut lines: Vec<String> = vec!["1".into(), "A".into()];
    for i in 0..12 {
        lines.push("1".into());
        lines.push(format!("D{i}"));
    }
    for i in 0..11 {
        lines.extend(["2".into(), "A".into(), format!("D{i}"), format!("{}", i + 1)]);
    }
    lines.extend(["4".into(), "A".into(), "8".into()]);
    let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
    assert_identical("max-edges", script(&refs).as_slice());
}

/// The max-edge check precedes the negative-distance check, so a full node
/// reports "maximum edges" even for a negative distance.
#[test]
fn add_route_check_order() {
    let mut lines: Vec<String> = vec!["1".into(), "A".into()];
    for i in 0..11 {
        lines.push("1".into());
        lines.push(format!("D{i}"));
    }
    for i in 0..10 {
        lines.extend(["2".into(), "A".into(), format!("D{i}"), "1".into()]);
    }
    lines.extend(["2".into(), "A".into(), "D10".into(), "-1".into()]);
    lines.push("8".into());
    let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
    assert_identical("edges-before-negative", script(&refs).as_slice());
}

/// `%d` stops at the first character it cannot use, so a hex or float looking
/// answer becomes the leading decimal digits.
#[test]
fn choice_partial_conversions() {
    assert_identical("hex-and-octal", b"0x10\n007\n8junk\n 8\n");
    assert_identical("signed-zero-and-exponent", b"-0\n+0\n1e3\n8\n");
    assert_identical("200-digits", script(&[&"1".repeat(200), "8"]).as_slice());
    assert_identical("distance-partial", b"1\nA\n1\nB\n2\nA\nB\n 12abc\n4\nA\n2\nA\nB\n0x5\n8\n");
}

// ---------------------------------------------------------------------------
// case 3 / case 4 -- print_graph and print_node
// ---------------------------------------------------------------------------

#[test]
fn show_cities() {
    assert_identical("empty-graph", b"3\n8\n");
    assert_identical("graph-with-edges", b"1\nA\n1\nB\n2\nA\nB\n5\n3\n8\n");
}

#[test]
fn show_city_details() {
    assert_identical("detail-missing", b"4\nZ\n8\n");
    assert_identical("detail-ok", b"1\nA\n1\nB\n2\nA\nB\n5\n4\nA\n4\nB\n8\n");
}

// ---------------------------------------------------------------------------
// case 5 / find_shortest_path
// ---------------------------------------------------------------------------

#[test]
fn shortest_path_missing_endpoints() {
    assert_identical("no-start", b"5\nA\nB\n8\n");
    assert_identical("no-end", b"1\nA\n5\nA\nB\n8\n");
}

/// `state[end_idx].distance == INT_MAX` -> "No path found" on stderr from the
/// library *and* "No path found" on stdout from `main`.
#[test]
fn shortest_path_none() {
    assert_identical("unreachable", b"1\nA\n1\nB\n5\nA\nB\n8\n");
}

#[test]
fn shortest_path_found() {
    assert_identical("direct", b"1\nA\n1\nB\n2\nA\nB\n100\n5\nA\nB\n8\n");
    assert_identical("start-equals-end", b"1\nA\n5\nA\nA\n8\n");
    assert_identical("self-edge", b"1\nA\n2\nA\nA\n5\n5\nA\nA\n8\n");
    // Two routes, the two-hop one is shorter.
    assert_identical(
        "dijkstra",
        b"1\nA\n1\nB\n1\nC\n1\nD\n2\nA\nB\n1\n2\nB\nD\n10\n2\nA\nC\n2\n2\nC\nD\n3\n5\nA\nD\n8\n",
    );
    // A cycle: the visited flag has to stop the search.
    assert_identical(
        "cycle",
        b"1\nA\n1\nB\n2\nA\nB\n1\n2\nB\nA\n1\n5\nA\nB\n5\nB\nA\n8\n",
    );
}

/// `state[current_idx].distance + current->edges[i].distance` overflows `int`.
#[test]
fn shortest_path_distance_overflow() {
    assert_identical(
        "int-max-chain",
        b"1\nA\n1\nB\n1\nC\n2\nA\nB\n2147483647\n2\nB\nC\n2147483647\n5\nA\nC\n8\n",
    );
    assert_identical(
        "int-max-alt",
        b"1\nA\n1\nB\n1\nC\n2\nA\nB\n2147483646\n2\nA\nC\n5\n2\nB\nC\n2147483646\n5\nA\nC\n8\n",
    );
    // An unparsable then an over-large distance.
    assert_identical(
        "overflow-distance",
        b"1\nA\n1\nB\n2\nA\nB\n99999999999999999999\n4\nA\n8\n",
    );
}

/// A path long enough to exercise the reconstruct-and-reverse loop.
#[test]
fn shortest_path_long_chain() {
    let mut lines: Vec<String> = Vec::new();
    for i in 0..40 {
        lines.push("1".into());
        lines.push(format!("C{i}"));
    }
    for i in 0..39 {
        lines.extend([
            "2".into(),
            format!("C{i}"),
            format!("C{}", i + 1),
            format!("{}", i + 1),
        ]);
    }
    lines.extend(["5".into(), "C0".into(), "C39".into(), "8".into()]);
    let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
    assert_identical("long-chain", script(&refs).as_slice());
}

/// An edge whose distance is `INT_MAX`: `new_distance` equals `INT_MAX`, which is
/// not `< INT_MAX`, so the destination's tentative distance is never lowered and
/// the search reports no path even though the edge exists.
#[test]
fn shortest_path_int_max_edge_is_never_relaxed() {
    assert_identical("int-max-edge", b"1\nA\n1\nB\n2\nA\nB\n2147483647\n5\nA\nB\n3\n8\n");
    // ... unless a finite detour reaches it.
    assert_identical(
        "int-max-edge-with-detour",
        b"1\nA\n1\nB\n1\nC\n2\nA\nB\n2147483647\n2\nA\nC\n1\n2\nC\nB\n1\n5\nA\nB\n8\n",
    );
}

/// A path over the full graph, and a dense graph where every node has the
/// maximum number of edges.
#[test]
fn shortest_path_at_the_size_limits() {
    let mut lines: Vec<String> = Vec::new();
    for i in 0..100 {
        lines.push("1".into());
        lines.push(format!("C{i}"));
    }
    for i in 0..99 {
        lines.extend([
            "2".into(),
            format!("C{i}"),
            format!("C{}", i + 1),
            "1".into(),
        ]);
    }
    // A shallow copy over all 100 nodes exercises the recursion and the
    // visited list in `increment_refs_recursive`.
    lines.extend([
        "6".into(),
        "C0".into(),
        "5".into(),
        "C0".into(),
        "C99".into(),
        "8".into(),
    ]);
    let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
    assert_identical("hundred-node-chain", script(&refs).as_slice());
}

/// 100 nodes with 10 edges each: `state_count` grows to the whole graph.
#[test]
fn shortest_path_dense_graph() {
    // A fixed pseudo-random but reproducible edge set.
    let mut lines: Vec<String> = Vec::new();
    for i in 0..100 {
        lines.push("1".into());
        lines.push(format!("C{i}"));
    }
    let mut x: u64 = 12345;
    let mut next = || {
        x = x.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (x >> 33) as usize
    };
    for i in 0..100 {
        for _ in 0..10 {
            let to = next() % 100;
            let d = next() % 50 + 1;
            lines.extend(["2".into(), format!("C{i}"), format!("C{to}"), d.to_string()]);
        }
    }
    lines.extend(["5".into(), "C0".into(), "C99".into(), "3".into(), "8".into()]);
    let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
    assert_identical("dense", script(&refs).as_slice());
}

// ---------------------------------------------------------------------------
// case 6 / shallow_copy
// ---------------------------------------------------------------------------

#[test]
fn shallow_copy() {
    assert_identical("copy-missing", b"6\nZ\n8\n");
    assert_identical("copy-one", b"1\nA\n6\nA\n8\n");
    assert_identical("copy-chain", b"1\nA\n1\nB\n2\nA\nB\n5\n6\nA\n3\n8\n");
    // A cycle: `increment_refs_recursive` must stop on the visited list.
    assert_identical(
        "copy-cycle",
        b"1\nA\n1\nB\n2\nA\nB\n1\n2\nB\nA\n1\n6\nA\n3\n8\n",
    );
    // Repeated copies keep bumping the reference counts.
    assert_identical("copy-many", b"1\nA\n6\nA\n6\nA\n6\nA\n6\nA\n6\nA\n3\n8\n");
}

// ---------------------------------------------------------------------------
// case 7 / delete_node
// ---------------------------------------------------------------------------

#[test]
fn delete_missing_city() {
    assert_identical("delete-missing", b"7\nZ\n8\n");
}

/// Deleting drops the reference count to zero and frees the node, but leaves the
/// dangling pointer in the graph.  The allocator overwrites the first bytes of
/// the freed chunk, so the city can no longer be looked up by name.
#[test]
fn delete_then_lookup_fails() {
    assert_identical("delete-twice", b"1\nA\n7\nA\n7\nA\n8\n");
    assert_identical("delete-then-detail", b"1\nA\n7\nA\n4\nA\n8\n");
    assert_identical("delete-then-copy", b"1\nA\n7\nA\n6\nA\n8\n");
    assert_identical("delete-then-route", b"1\nA\n1\nB\n7\nA\n2\nA\nB\n1\n8\n");
    assert_identical("delete-then-path", b"1\nA\n1\nB\n7\nA\n5\nA\nB\n8\n");
}

/// A reference count above one only decrements.
#[test]
fn delete_with_extra_references() {
    assert_identical("copy-then-delete", b"1\nA\n6\nA\n7\nA\n4\nA\n8\n");
    assert_identical(
        "copy-then-delete-twice",
        b"1\nA\n1\nB\n2\nA\nB\n5\n6\nA\n7\nB\n4\nB\n8\n",
    );
}

/// The freed chunk goes on the tcache, so re-adding the same city hands the very
/// same chunk back out and both graph slots alias the new node.
#[test]
fn delete_then_readd_aliases_the_chunk() {
    assert_identical("readd", b"1\nA\n7\nA\n1\nA\n3\n8\n");
    assert_identical("readd-other", b"1\nA\n7\nA\n1\nB\n3\n4\nB\n8\n");
    assert_identical("readd-twice", b"1\nA\n7\nA\n1\nB\n7\nB\n1\nC\n3\n8\n");
}

/// The graph's `node_count` never shrinks, so the 100 slot limit counts deleted
/// cities too.
#[test]
fn deleting_does_not_free_a_graph_slot() {
    let mut lines: Vec<String> = Vec::new();
    for i in 0..100 {
        lines.push("1".into());
        lines.push(format!("C{i}"));
    }
    for i in (0..100).step_by(7) {
        lines.push("7".into());
        lines.push(format!("C{i}"));
    }
    for i in 0..5 {
        lines.push("1".into());
        lines.push(format!("N{i}"));
    }
    lines.push("8".into());
    let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
    assert_identical("full-then-delete", script(&refs).as_slice());
}

/// A long session, so stdout crosses many 4096 byte flush boundaries.
#[test]
fn long_session_crosses_flush_boundaries() {
    let mut lines: Vec<String> = Vec::new();
    for i in 0..60 {
        lines.push("1".into());
        lines.push(format!("C{i}"));
        lines.push("3".into());
    }
    lines.push("8".into());
    let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
    let input = script(&refs);
    assert!(harness::run_c(&input).stdout.len() > 20 * 4096);
    assert_identical("long-session", &input);
}

// ---------------------------------------------------------------------------
// The allocator behaviour the program's output depends on
// ---------------------------------------------------------------------------

/// Deletions up to the tcache limit come back out last-in-first-out.
#[test]
fn tcache_reuse_is_lifo() {
    for n in 1..=7usize {
        let mut lines: Vec<String> = Vec::new();
        for i in 0..n {
            lines.push("1".into());
            lines.push(format!("C{i}"));
        }
        for i in 0..n {
            lines.push("7".into());
            lines.push(format!("C{i}"));
        }
        for i in 0..n {
            lines.push("1".into());
            lines.push(format!("N{i}"));
        }
        lines.extend(["3".into(), "8".into()]);
        let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
        assert_identical(&format!("tcache-lifo-{n}"), script(&refs).as_slice());
    }
}

/// More deletions than the tcache holds: the surplus goes through the unsorted
/// and small/large bins and comes back in a different order.
#[test]
fn bin_reuse_beyond_the_tcache() {
    for n in [8usize, 9, 12, 20, 31] {
        let mut lines: Vec<String> = Vec::new();
        for i in 0..n {
            lines.push("1".into());
            lines.push(format!("C{i}"));
        }
        for i in 0..n {
            lines.push("7".into());
            lines.push(format!("C{i}"));
        }
        for i in 0..n {
            lines.push("1".into());
            lines.push(format!("N{i}"));
        }
        lines.extend(["3".into(), "8".into()]);
        let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
        assert_identical(&format!("bin-reuse-{n}"), script(&refs).as_slice());
    }
}

/// Deleting in reverse order leaves a single coalesced block, which is then
/// carved up from its low address end.
#[test]
fn coalesced_block_is_split_from_the_bottom() {
    let n = 20usize;
    let mut lines: Vec<String> = Vec::new();
    for i in 0..n {
        lines.push("1".into());
        lines.push(format!("C{i}"));
    }
    for i in (0..n).rev() {
        lines.push("7".into());
        lines.push(format!("C{i}"));
    }
    for i in 0..n {
        lines.push("1".into());
        lines.push(format!("N{i}"));
    }
    lines.extend(["3".into(), "8".into()]);
    let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
    assert_identical("reverse-delete", script(&refs).as_slice());
}

/// Freeing the chunk that borders the top chunk merges it into the top chunk.
/// Only the chunk header is rewritten, so the city name survives and the
/// "deleted" city can still be found by name.
#[test]
fn top_chunk_merge_keeps_the_city_name() {
    let mut lines: Vec<String> = Vec::new();
    for i in 0..8 {
        lines.push("1".into());
        lines.push(format!("C{i}"));
    }
    for i in 0..7 {
        lines.push("7".into());
        lines.push(format!("C{i}"));
    }
    lines.extend(["7".into(), "C7".into()]); // merged into the top chunk
    lines.extend(["4".into(), "C7".into()]); // still findable, name intact
    lines.push("8".into());
    let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
    assert_identical("top-merge-intact", script(&refs).as_slice());
}

// ---------------------------------------------------------------------------
// The heap consistency checks: stderr diagnostic, truncated stdout, SIGABRT
// ---------------------------------------------------------------------------

/// A stale edge lets `shallow_copy` raise a freed chunk's reference count back
/// to one; `free_graph` then frees it a second time.  The chunk is still on the
/// tcache, so glibc reports "double free detected in tcache 2", aborts, and the
/// buffered stdout is never flushed.
#[test]
fn double_free_detected_in_tcache() {
    assert_identical("tcache-double-free", b"1\nA\n1\nB\n2\nA\nB\n7\n7\nB\n6\nA\n8\n");
    assert_identical(
        "tcache-double-free-after-show",
        b"1\nA\n1\nB\n2\nA\nB\n7\n7\nB\n6\nA\n3\n8\n",
    );
}

/// The same trick on a chunk that has fallen out of the tcache into a bin trips
/// the `!prev_inuse (nextchunk)` test instead.
#[test]
fn double_free_or_corruption_prev() {
    let mut lines: Vec<String> = Vec::new();
    for i in 0..10 {
        lines.push("1".into());
        lines.push(format!("C{i}"));
    }
    lines.extend(["2".into(), "C0".into(), "C2".into(), "5".into()]);
    for i in [9, 8, 7, 6, 5, 4, 3, 2] {
        lines.push("7".into());
        lines.push(format!("C{i}"));
    }
    lines.extend(["6".into(), "C0".into(), "8".into()]);
    let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
    assert_identical("binned-double-free", script(&refs).as_slice());
}

/// And on a chunk that became the top chunk, `p == av->top` fires first.
#[test]
fn double_free_or_corruption_top() {
    let mut lines: Vec<String> = Vec::new();
    for i in 0..8 {
        lines.push("1".into());
        lines.push(format!("C{i}"));
    }
    for i in 0..8 {
        lines.push("7".into());
        lines.push(format!("C{i}"));
    }
    lines.extend(["6".into(), "C7".into(), "7".into(), "C7".into(), "8".into()]);
    let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
    assert_identical("top-double-free", script(&refs).as_slice());
}

/// A chunk in the interior of the merged top chunk has a stale header, and the
/// backward consolidation size check reports the corruption.
#[test]
fn corrupted_size_vs_prev_size() {
    let mut lines: Vec<String> = Vec::new();
    for i in 0..10 {
        lines.push("1".into());
        lines.push(format!("C{i}"));
    }
    lines.extend(["2".into(), "C0".into(), "C9".into(), "5".into()]);
    for i in 1..10 {
        lines.push("7".into());
        lines.push(format!("C{i}"));
    }
    lines.extend(["6".into(), "C0".into(), "8".into()]);
    let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
    assert_identical("top-interior-double-free", script(&refs).as_slice());
}

/// The abort has to happen after enough output to have flushed whole 4096 byte
/// blocks, so that stdout is truncated at a block boundary rather than empty.
#[test]
fn abort_truncates_stdout_at_a_block_boundary() {
    let mut lines: Vec<String> = Vec::new();
    for i in 0..30 {
        lines.push("1".into());
        lines.push(format!("C{i}"));
    }
    lines.extend(["2".into(), "C0".into(), "C29".into(), "5".into()]);
    for i in 1..30 {
        lines.push("7".into());
        lines.push(format!("C{i}"));
    }
    lines.extend(["6".into(), "C0".into(), "8".into()]);
    let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
    let input = script(&refs);

    let c = harness::run_c(&input);
    assert!(
        c.status == Err(6) && c.stdout.len() % 4096 == 0 && !c.stdout.is_empty(),
        "expected an abort with a block-aligned stdout, got {c:?}"
    );
    assert_identical("abort-truncation", &input);
}

// ---------------------------------------------------------------------------
// Byte-level input handling
// ---------------------------------------------------------------------------

/// `fgets` copies NUL bytes into the buffer, but `strcspn`, `strcmp` and `%s`
/// all stop at the first one, so everything after it is invisible.
#[test]
fn embedded_nul_bytes() {
    assert_identical("nul-in-name", b"1\nA\x00B\n3\n4\nA\n1\nA\n3\n8\n");
    assert_identical("nul-only", b"1\n\x00\n3\n4\n\n8\n");
}

/// City names are raw bytes, not text.
#[test]
fn non_utf8_names() {
    assert_identical(
        "high-bytes",
        b"1\n\xff\xfe\x80caf\xc3\xa9\n3\n4\n\xff\xfe\x80caf\xc3\xa9\n8\n",
    );
    assert_identical("tab-and-space", b"1\n\tA \n3\n4\n\tA \n8\n");
}

/// `strcspn(input, "\n")` only strips the newline, so a CR stays part of the
/// city name.
#[test]
fn crlf_line_endings() {
    assert_identical("crlf", b"1\r\nA\r\n3\r\n8\r\n");
    assert_identical("cr-name", b"1\nA\r\n4\nA\n4\nA\r\n8\n");
}
