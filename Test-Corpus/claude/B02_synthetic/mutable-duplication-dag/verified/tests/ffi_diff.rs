//! Phase B (CONFIGS.md rows 1-34) and Phase C (ERRORS.md rows 2-35):
//! differential tests that drive `build_c/libdag_c.so` and
//! `target/debug/libdag_rs.so` through `libloading` and compare return values,
//! `node_t`/`graph_t` memory, stdout and stderr byte for byte.

mod common;

use common::*;

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

fn name(i: usize) -> Vec<u8> {
    format!("city{i}").into_bytes()
}

/// `CreateGraph` + `n` nodes called `city0..city{n-1}` + the given edges.
fn build(n: usize, edges: &[(usize, usize, i32)]) -> Vec<Op> {
    let mut ops = vec![Op::CreateGraph];
    for i in 0..n {
        ops.push(Op::AddNode(0, name(i)));
    }
    for &(a, b, d) in edges {
        ops.push(Op::AddEdge(a, b, d));
    }
    ops
}

fn repeat(b: u8, n: usize) -> Vec<u8> {
    vec![b; n]
}

// ---------------------------------------------------------------------------
// CONFIGS.md rows 1-6: create_graph / add_node / free_graph
// ---------------------------------------------------------------------------

/// CONFIGS row 1, ERRORS row 35
fn cfg_create_free_empty() {
    let ops = vec![
        Op::CreateGraph,
        Op::DumpAll,
        Op::PrintGraph(0),
        Op::FreeGraph(0),
        Op::FreeGraphNull,
    ];
    assert_same("cfg_create_free_empty", &ops);
}

/// CONFIGS row 2
fn cfg_single_node() {
    let ops = vec![
        Op::CreateGraph,
        Op::AddNode(0, b"A".to_vec()),
        Op::DumpAll,
        Op::PrintNode(0),
        Op::PrintGraph(0),
        Op::GetNodeByName(0, b"A".to_vec()),
        Op::FreeGraph(0),
    ];
    assert_same("cfg_single_node", &ops);
}

/// CONFIGS row 3: `strncpy(dst, src, MAX_CITY_NAME - 1)` truncation boundary.
fn cfg_add_node_name_lengths() {
    for len in [
        0usize, 1, 2, 3, 31, 61, 62, 63, 64, 65, 66, 100, 200, 255, 256, 300,
    ] {
        let mut long = Vec::new();
        for i in 0..len {
            long.push(b'a' + (i % 26) as u8);
        }
        let ops = vec![
            Op::CreateGraph,
            Op::AddNode(0, long.clone()),
            Op::DumpAll,
            Op::PrintGraph(0),
            // look the node up by its full name and by its truncated name
            Op::GetNodeByName(0, long.clone()),
            Op::GetNodeByName(0, long[..long.len().min(63)].to_vec()),
            Op::GetNodeByName(0, long[..long.len().min(62)].to_vec()),
            Op::FreeGraph(0),
        ];
        assert_same(&format!("cfg_add_node_name_lengths len={len}"), &ops);
    }
}

/// CONFIGS row 4
fn cfg_add_node_random_names() {
    let mut rng = Rng::new(0xA11CE);
    for round in 0..120 {
        let mut ops = vec![Op::CreateGraph];
        let count = 1 + rng.below(8);
        let mut names = Vec::new();
        for _ in 0..count {
            let n = rng.name(70);
            names.push(n.clone());
            ops.push(Op::AddNode(0, n));
        }
        ops.push(Op::DumpAll);
        ops.push(Op::PrintGraph(0));
        for n in &names {
            ops.push(Op::GetNodeByName(0, n.clone()));
        }
        ops.push(Op::GetNodeByName(0, rng.name(70)));
        ops.push(Op::FreeGraph(0));
        assert_same(&format!("cfg_add_node_random_names round={round}"), &ops);
    }
}

/// CONFIGS row 5 / ERRORS row 7
fn cfg_add_node_shared_prefixes() {
    let base = repeat(b'a', 63);
    let mut a = base.clone();
    a.push(b'X');
    let mut b = base.clone();
    b.push(b'Y');
    let ops = vec![
        Op::CreateGraph,
        Op::AddNode(0, a.clone()),
        Op::AddNode(0, b.clone()),
        Op::AddNode(0, base.clone()),
        Op::AddNode(0, repeat(b'a', 62)),
        Op::AddNode(0, repeat(b'a', 64)),
        Op::DumpAll,
        Op::PrintGraph(0),
        Op::GetNodeByName(0, base.clone()),
        Op::GetNodeByName(0, a),
        Op::GetNodeByName(0, b),
        Op::GetNodeByName(0, repeat(b'a', 62)),
        Op::FreeGraph(0),
    ];
    assert_same("cfg_add_node_shared_prefixes", &ops);
}

/// CONFIGS row 6 / ERRORS row 5
fn cfg_add_node_fill_to_max() {
    let mut ops = vec![Op::CreateGraph];
    for i in 0..MAX_NODES {
        ops.push(Op::AddNode(0, name(i)));
    }
    ops.push(Op::DumpAll);
    ops.push(Op::GetNodeByName(0, name(0)));
    ops.push(Op::GetNodeByName(0, name(MAX_NODES - 1)));
    ops.push(Op::PrintGraph(0));
    ops.push(Op::FreeGraph(0));
    assert_same("cfg_add_node_fill_to_max", &ops);
}

// ---------------------------------------------------------------------------
// CONFIGS rows 7-11: add_edge
// ---------------------------------------------------------------------------

/// CONFIGS row 7
fn cfg_add_edge_first() {
    let mut ops = build(2, &[]);
    ops.push(Op::AddEdge(0, 1, 0));
    ops.push(Op::DumpAll);
    ops.push(Op::PrintNode(0));
    ops.push(Op::PrintNode(1));
    ops.push(Op::FreeGraph(0));
    assert_same("cfg_add_edge_first", &ops);
}

/// CONFIGS row 8 / ERRORS rows 12, 13
fn cfg_add_edge_fill_to_max() {
    let mut ops = build(12, &[]);
    for i in 1..=MAX_EDGES {
        ops.push(Op::AddEdge(0, i, (i * 3) as i32));
    }
    ops.push(Op::DumpAll);
    ops.push(Op::PrintNode(0));
    ops.push(Op::FreeGraph(0));
    assert_same("cfg_add_edge_fill_to_max", &ops);
}

/// CONFIGS row 9
fn cfg_add_edge_distance_values() {
    for d in [0i32, 1, 2, 7, 1000, 65535, i32::MAX / 2, i32::MAX - 1, i32::MAX] {
        let mut ops = build(3, &[]);
        ops.push(Op::AddEdge(0, 1, d));
        ops.push(Op::AddEdge(1, 2, d));
        ops.push(Op::DumpAll);
        ops.push(Op::PrintGraph(0));
        ops.push(Op::FindShortestPath(0, 1));
        ops.push(Op::FreeGraph(0));
        assert_same(&format!("cfg_add_edge_distance_values d={d}"), &ops);
    }
}

/// CONFIGS row 10 / ERRORS row 16
fn cfg_add_edge_self() {
    let mut ops = build(2, &[]);
    ops.push(Op::AddEdge(0, 0, 5));
    ops.push(Op::AddEdge(0, 0, 7));
    ops.push(Op::AddEdge(0, 1, 1));
    ops.push(Op::DumpAll);
    ops.push(Op::PrintNode(0));
    ops.push(Op::FindShortestPath(0, 0));
    ops.push(Op::FindShortestPath(0, 1));
    ops.push(Op::FreeGraph(0));
    assert_same("cfg_add_edge_self", &ops);
}

/// CONFIGS row 11
fn cfg_add_edge_both_directions() {
    let mut ops = build(2, &[]);
    ops.push(Op::AddEdge(0, 1, 4));
    ops.push(Op::AddEdge(1, 0, 9));
    ops.push(Op::DumpAll);
    ops.push(Op::PrintGraph(0));
    ops.push(Op::FindShortestPath(0, 1));
    ops.push(Op::FindShortestPath(1, 0));
    ops.push(Op::FreeGraph(0));
    assert_same("cfg_add_edge_both_directions", &ops);
}

// ---------------------------------------------------------------------------
// CONFIGS rows 12-13: get_node_by_name
// ---------------------------------------------------------------------------

/// CONFIGS row 12 / ERRORS row 32
fn cfg_get_node_positions() {
    let mut ops = vec![Op::CreateGraph, Op::GetNodeByName(0, name(0))];
    for i in 0..7 {
        ops.push(Op::AddNode(0, name(i)));
    }
    for i in 0..7 {
        ops.push(Op::GetNodeByName(0, name(i)));
    }
    ops.push(Op::GetNodeByName(0, name(7)));
    ops.push(Op::GetNodeByName(0, Vec::new()));
    ops.push(Op::GetNodeByName(0, b"CITY0".to_vec()));
    ops.push(Op::GetNodeByName(0, b"city".to_vec()));
    ops.push(Op::GetNodeByName(0, b"city00".to_vec()));
    ops.push(Op::FreeGraph(0));
    assert_same("cfg_get_node_positions", &ops);
}

/// CONFIGS row 13
fn cfg_get_node_truncated_lookup() {
    let mut long = repeat(b'q', 63);
    long.extend_from_slice(b"tail");
    let ops = vec![
        Op::CreateGraph,
        Op::AddNode(0, long.clone()),
        Op::GetNodeByName(0, long.clone()),
        Op::GetNodeByName(0, repeat(b'q', 63)),
        Op::GetNodeByName(0, repeat(b'q', 64)),
        Op::DumpAll,
        Op::FreeGraph(0),
    ];
    assert_same("cfg_get_node_truncated_lookup", &ops);
}

// ---------------------------------------------------------------------------
// CONFIGS rows 14-17: print_node / print_graph
// ---------------------------------------------------------------------------

/// CONFIGS row 14 / ERRORS rows 33, 34
fn cfg_print_node_no_edges() {
    let ops = vec![
        Op::CreateGraph,
        Op::AddNode(0, Vec::new()),
        Op::PrintNode(0),
        Op::PrintNodeNull,
        Op::PrintGraph(0),
        Op::PrintGraphNull,
        Op::FreeGraph(0),
    ];
    assert_same("cfg_print_node_no_edges", &ops);
}

/// CONFIGS row 15
fn cfg_print_node_edges() {
    let mut rng = Rng::new(0xBEEF);
    for round in 0..40 {
        let count = 1 + rng.below(11);
        let mut ops = vec![Op::CreateGraph];
        for i in 0..=count {
            let mut n = rng.name(70);
            n.extend_from_slice(format!("#{i}").as_bytes());
            ops.push(Op::AddNode(0, n));
        }
        for i in 1..=count {
            let d = match rng.below(4) {
                0 => 0,
                1 => i32::MAX,
                2 => rng.range_i32(0, 1_000_000),
                _ => rng.range_i32(0, 5),
            };
            ops.push(Op::AddEdge(0, i, d));
        }
        ops.push(Op::PrintNode(0));
        ops.push(Op::PrintGraph(0));
        ops.push(Op::DumpAll);
        ops.push(Op::FreeGraph(0));
        assert_same(&format!("cfg_print_node_edges round={round}"), &ops);
    }
}

/// CONFIGS row 16
fn cfg_print_node_ref_counts() {
    let mut ops = build(3, &[(0, 1, 1), (1, 2, 2)]);
    ops.push(Op::PrintNode(0));
    ops.push(Op::ShallowCopy(0));
    ops.push(Op::PrintGraph(0));
    ops.push(Op::ShallowCopy(0));
    ops.push(Op::ShallowCopy(1));
    ops.push(Op::PrintGraph(0));
    ops.push(Op::DumpAll);
    ops.push(Op::FreeGraph(0));
    assert_same("cfg_print_node_ref_counts", &ops);
}

/// CONFIGS row 17
fn cfg_print_graph_sizes() {
    for n in [0usize, 1, 2, 3, 17] {
        let mut ops = build(n, &[]);
        // a few edges spread over the nodes
        for i in 0..n {
            if i + 1 < n {
                ops.push(Op::AddEdge(i, i + 1, (i * 10) as i32));
            }
            if i % 3 == 0 && n > 2 {
                ops.push(Op::AddEdge(i, (i + 2) % n, 7));
            }
        }
        ops.push(Op::PrintGraph(0));
        ops.push(Op::DumpAll);
        ops.push(Op::FreeGraph(0));
        assert_same(&format!("cfg_print_graph_sizes n={n}"), &ops);
    }
}

// ---------------------------------------------------------------------------
// CONFIGS rows 18-20: shallow_copy
// ---------------------------------------------------------------------------

/// CONFIGS row 18
fn cfg_shallow_copy_single() {
    let ops = vec![
        Op::CreateGraph,
        Op::AddNode(0, b"solo".to_vec()),
        Op::ShallowCopy(0),
        Op::DumpAll,
        Op::PrintNode(0),
        Op::FreeGraph(0),
    ];
    assert_same("cfg_shallow_copy_single", &ops);
}

/// CONFIGS row 19
fn cfg_shallow_copy_topologies() {
    let shapes: Vec<(&str, usize, Vec<(usize, usize, i32)>)> = vec![
        ("chain", 4, vec![(0, 1, 1), (1, 2, 1), (2, 3, 1)]),
        (
            "diamond",
            4,
            vec![(0, 1, 1), (0, 2, 1), (1, 3, 1), (2, 3, 1)],
        ),
        ("cycle", 3, vec![(0, 1, 1), (1, 2, 1), (2, 0, 1)]),
        ("selfloop", 2, vec![(0, 0, 1), (0, 1, 1)]),
        ("disconnected", 4, vec![(0, 1, 1), (2, 3, 1)]),
        (
            "star",
            5,
            vec![(0, 1, 1), (0, 2, 1), (0, 3, 1), (0, 4, 1)],
        ),
        (
            "back_edge",
            3,
            vec![(0, 1, 1), (1, 2, 1), (2, 1, 1), (1, 0, 1)],
        ),
    ];
    for (label, n, edges) in shapes {
        for start in 0..n {
            let mut ops = build(n, &edges);
            ops.push(Op::ShallowCopy(start));
            ops.push(Op::DumpAll);
            ops.push(Op::PrintGraph(0));
            ops.push(Op::FreeGraph(0));
            assert_same(
                &format!("cfg_shallow_copy_topologies {label} start={start}"),
                &ops,
            );
        }
    }
}

/// CONFIGS row 20
fn cfg_shallow_copy_repeated() {
    let mut ops = build(5, &[(0, 1, 1), (1, 2, 1), (2, 3, 1), (3, 1, 1)]);
    for _ in 0..3 {
        ops.push(Op::ShallowCopy(0));
    }
    ops.push(Op::DumpAll);
    ops.push(Op::ShallowCopy(2));
    ops.push(Op::DumpAll);
    ops.push(Op::ShallowCopy(4));
    ops.push(Op::DumpAll);
    ops.push(Op::PrintGraph(0));
    ops.push(Op::FreeGraph(0));
    assert_same("cfg_shallow_copy_repeated", &ops);
}

// ---------------------------------------------------------------------------
// CONFIGS rows 21-30: find_shortest_path
// ---------------------------------------------------------------------------

/// CONFIGS row 21
fn cfg_fsp_start_is_end() {
    let mut ops = build(3, &[(0, 1, 3), (1, 2, 4)]);
    ops.push(Op::FindShortestPath(0, 0));
    ops.push(Op::FindShortestPath(1, 1));
    ops.push(Op::FindShortestPath(2, 2));
    ops.push(Op::FreeGraph(0));
    assert_same("cfg_fsp_start_is_end", &ops);
}

/// CONFIGS row 22
fn cfg_fsp_chain() {
    for hops in 1..=8usize {
        let n = hops + 1;
        let edges: Vec<(usize, usize, i32)> =
            (0..hops).map(|i| (i, i + 1, (i as i32 + 1) * 5)).collect();
        let mut ops = build(n, &edges);
        for a in 0..n {
            for b in 0..n {
                ops.push(Op::FindShortestPath(a, b));
            }
        }
        ops.push(Op::FreeGraph(0));
        assert_same(&format!("cfg_fsp_chain hops={hops}"), &ops);
    }
}

/// CONFIGS row 23: two routes of exactly the same cost — the C keeps the first
/// one because it relaxes with a strict `<`.
fn cfg_fsp_equal_cost_tie() {
    // 0 -> 1 -> 3 and 0 -> 2 -> 3, both cost 10
    let mut ops = build(4, &[(0, 1, 5), (0, 2, 5), (1, 3, 5), (2, 3, 5)]);
    ops.push(Op::FindShortestPath(0, 3));
    ops.push(Op::FreeGraph(0));
    assert_same("cfg_fsp_equal_cost_tie a", &ops);

    // same graph, edges declared in the opposite order
    let mut ops = build(4, &[(0, 2, 5), (0, 1, 5), (2, 3, 5), (1, 3, 5)]);
    ops.push(Op::FindShortestPath(0, 3));
    ops.push(Op::FreeGraph(0));
    assert_same("cfg_fsp_equal_cost_tie b", &ops);

    // three-way tie
    let mut ops = build(
        5,
        &[(0, 1, 2), (0, 2, 2), (0, 3, 2), (1, 4, 3), (2, 4, 3), (3, 4, 3)],
    );
    ops.push(Op::FindShortestPath(0, 4));
    ops.push(Op::FreeGraph(0));
    assert_same("cfg_fsp_equal_cost_tie c", &ops);
}

/// CONFIGS row 24 / ERRORS row 27: `state_count` reaches `MAX_NODES`, so
/// further neighbours cannot be recorded any more. Needs more than `MAX_NODES`
/// reachable nodes, which an external caller can build by linking two graphs.
fn cfg_fsp_state_full() {
    let mut ops = vec![Op::CreateGraph, Op::CreateGraph];
    for i in 0..MAX_NODES {
        ops.push(Op::AddNode(0, name(i)));
    }
    for i in 0..5 {
        ops.push(Op::AddNode(1, name(1000 + i)));
    }
    // chain through the whole first graph, then into the second one
    for i in 0..MAX_NODES - 1 {
        ops.push(Op::AddEdge(i, i + 1, 1));
    }
    ops.push(Op::AddEdge(MAX_NODES - 1, MAX_NODES, 1));
    for i in 0..4 {
        ops.push(Op::AddEdge(MAX_NODES + i, MAX_NODES + i + 1, 1));
    }
    ops.push(Op::FindShortestPath(0, MAX_NODES - 1));
    // the target lives past the state array's capacity
    ops.push(Op::FindShortestPath(0, MAX_NODES));
    ops.push(Op::FindShortestPath(0, MAX_NODES + 4));
    // starting further along the chain leaves room again
    ops.push(Op::FindShortestPath(1, MAX_NODES));
    ops.push(Op::FindShortestPath(50, MAX_NODES + 4));
    ops.push(Op::FreeGraph(0));
    ops.push(Op::FreeGraph(1));
    assert_same("cfg_fsp_state_full", &ops);
}

/// CONFIGS row 25
fn cfg_fsp_cycles() {
    let shapes: Vec<(&str, usize, Vec<(usize, usize, i32)>)> = vec![
        ("triangle", 3, vec![(0, 1, 1), (1, 2, 1), (2, 0, 1)]),
        (
            "selfloops",
            3,
            vec![(0, 0, 0), (0, 1, 2), (1, 1, 3), (1, 2, 4), (2, 2, 5)],
        ),
        (
            "two_cycles",
            6,
            vec![
                (0, 1, 1),
                (1, 2, 1),
                (2, 0, 1),
                (2, 3, 1),
                (3, 4, 1),
                (4, 5, 1),
                (5, 3, 1),
            ],
        ),
        (
            "zero_cycle",
            4,
            vec![(0, 1, 0), (1, 2, 0), (2, 1, 0), (2, 3, 0)],
        ),
    ];
    for (label, n, edges) in shapes {
        let mut ops = build(n, &edges);
        for a in 0..n {
            for b in 0..n {
                ops.push(Op::FindShortestPath(a, b));
            }
        }
        ops.push(Op::FreeGraph(0));
        assert_same(&format!("cfg_fsp_cycles {label}"), &ops);
    }
}

/// CONFIGS row 26: a node is discovered on an expensive route first and later
/// relaxed through a cheaper one.
fn cfg_fsp_relaxation() {
    // 0->3 direct costs 100, 0->1->2->3 costs 6
    let mut ops = build(4, &[(0, 3, 100), (0, 1, 2), (1, 2, 2), (2, 3, 2)]);
    ops.push(Op::FindShortestPath(0, 3));
    ops.push(Op::FreeGraph(0));
    assert_same("cfg_fsp_relaxation a", &ops);

    // several successive improvements
    let mut ops = build(
        6,
        &[
            (0, 5, 50),
            (0, 1, 1),
            (1, 5, 40),
            (1, 2, 1),
            (2, 5, 30),
            (2, 3, 1),
            (3, 5, 20),
            (3, 4, 1),
            (4, 5, 1),
        ],
    );
    ops.push(Op::FindShortestPath(0, 5));
    ops.push(Op::FreeGraph(0));
    assert_same("cfg_fsp_relaxation b", &ops);
}

/// CONFIGS row 27
fn cfg_fsp_zero_weights() {
    let mut ops = build(
        5,
        &[(0, 1, 0), (1, 2, 0), (0, 2, 0), (2, 3, 0), (3, 4, 0), (0, 4, 0)],
    );
    for a in 0..5 {
        for b in 0..5 {
            ops.push(Op::FindShortestPath(a, b));
        }
    }
    ops.push(Op::FreeGraph(0));
    assert_same("cfg_fsp_zero_weights", &ops);
}

/// CONFIGS row 28: randomised directed graphs.
fn cfg_fsp_random_graphs() {
    let mut rng = Rng::new(0xC0FFEE);
    for round in 0..200 {
        let n = 1 + rng.below(40);
        let mut ops = vec![Op::CreateGraph];
        for i in 0..n {
            ops.push(Op::AddNode(0, name(i)));
        }
        for i in 0..n {
            let out = rng.below(MAX_EDGES + 2); // sometimes past the limit
            let mut used = Vec::new();
            for _ in 0..out {
                let t = rng.below(n);
                let d = match rng.below(6) {
                    0 => 0,
                    1 => 1,
                    2 => rng.range_i32(0, 3),
                    3 => rng.range_i32(0, 1000),
                    4 => rng.range_i32(0, 1_000_000),
                    _ => rng.range_i32(0, 50),
                };
                ops.push(Op::AddEdge(i, t, d));
                used.push(t);
            }
        }
        // every start/end pair for small graphs, a sample for larger ones
        if n <= 8 {
            for a in 0..n {
                for b in 0..n {
                    ops.push(Op::FindShortestPath(a, b));
                }
            }
        } else {
            for _ in 0..25 {
                ops.push(Op::FindShortestPath(rng.below(n), rng.below(n)));
            }
        }
        ops.push(Op::PrintGraph(0));
        ops.push(Op::DumpAll);
        ops.push(Op::FreeGraph(0));
        assert_same(&format!("cfg_fsp_random_graphs round={round}"), &ops);
    }
}

/// CONFIGS row 29: every node saturated at `MAX_EDGES` out-edges.
fn cfg_fsp_dense_max_edges() {
    let mut rng = Rng::new(0xDEAD10CC);
    for round in 0..30 {
        let n = 12 + rng.below(20);
        let mut ops = vec![Op::CreateGraph];
        for i in 0..n {
            ops.push(Op::AddNode(0, name(i)));
        }
        for i in 0..n {
            let mut targets: Vec<usize> = Vec::new();
            while targets.len() < MAX_EDGES.min(n) {
                let t = rng.below(n);
                if !targets.contains(&t) {
                    targets.push(t);
                }
            }
            for t in targets {
                ops.push(Op::AddEdge(i, t, rng.range_i32(0, 10_000)));
            }
        }
        for _ in 0..20 {
            ops.push(Op::FindShortestPath(rng.below(n), rng.below(n)));
        }
        ops.push(Op::DumpAll);
        ops.push(Op::FreeGraph(0));
        assert_same(&format!("cfg_fsp_dense_max_edges round={round}"), &ops);
    }
}

// ---------------------------------------------------------------------------
// CONFIGS rows 31-33: delete_node / free_graph and reference counts
// ---------------------------------------------------------------------------

/// CONFIGS row 31
fn cfg_delete_node_positive() {
    let mut ops = build(3, &[(0, 1, 1), (1, 2, 1)]);
    ops.push(Op::ShallowCopy(0));
    ops.push(Op::ShallowCopy(0));
    ops.push(Op::DumpAll);
    ops.push(Op::DeleteNode(0));
    ops.push(Op::DumpAll);
    ops.push(Op::PrintNode(0));
    ops.push(Op::DeleteNode(0));
    ops.push(Op::DumpAll);
    ops.push(Op::PrintGraph(0));
    ops.push(Op::DeleteNodeNull);
    ops.push(Op::FreeGraph(0));
    assert_same("cfg_delete_node_positive", &ops);
}

/// CONFIGS row 32
fn cfg_free_graph_refcount_one() {
    let mut ops = build(4, &[(0, 1, 1), (1, 2, 2), (2, 3, 3)]);
    ops.push(Op::DumpAll);
    ops.push(Op::FreeGraph(0));
    assert_same("cfg_free_graph_refcount_one", &ops);
}

/// CONFIGS row 33
fn cfg_free_graph_refcount_many() {
    let mut ops = build(4, &[(0, 1, 1), (1, 2, 2), (2, 3, 3)]);
    ops.push(Op::ShallowCopy(0));
    ops.push(Op::DumpAll);
    ops.push(Op::FreeGraph(0));
    // every node still has ref_count 1 afterwards, so nothing was released
    ops.push(Op::DumpNode(0));
    ops.push(Op::DumpNode(1));
    ops.push(Op::DumpNode(2));
    ops.push(Op::DumpNode(3));
    ops.push(Op::PrintNode(0));
    assert_same("cfg_free_graph_refcount_many", &ops);
}

// ---------------------------------------------------------------------------
// CONFIGS row 34: randomised whole-API sequences
// ---------------------------------------------------------------------------

/// A model of the C's own bookkeeping, precise enough to keep the generated
/// scripts inside defined behaviour (`delete_node` is only emitted while the
/// reference count provably stays above zero).
struct Model {
    /// `city_name` as stored by `strncpy` (truncated to `MAX_CITY_NAME - 1`).
    stored: Vec<Vec<u8>>,
    /// The names as passed in (used to pick plausible lookup keys).
    given: Vec<Vec<u8>>,
    succ: Vec<Vec<usize>>,
    ref_count: Vec<i32>,
}

impl Model {
    fn new() -> Model {
        Model {
            stored: Vec::new(),
            given: Vec::new(),
            succ: Vec::new(),
            ref_count: Vec::new(),
        }
    }

    fn len(&self) -> usize {
        self.stored.len()
    }

    /// `add_node`: rejected when the graph is full or when `strcmp` matches an
    /// already stored name (note that the *given* name is compared against the
    /// *stored*, i.e. truncated, one).
    fn add_node(&mut self, name: &[u8]) {
        if self.len() >= MAX_NODES {
            return;
        }
        if self.stored.iter().any(|s| s.as_slice() == name) {
            return;
        }
        self.stored.push(name[..name.len().min(MAX_CITY_NAME - 1)].to_vec());
        self.given.push(name.to_vec());
        self.succ.push(Vec::new());
        self.ref_count.push(1);
    }

    /// `add_edge`: rejected when the source is saturated, the distance is
    /// negative or the destination is already an out-neighbour.
    fn add_edge(&mut self, from: usize, to: usize, distance: i32) {
        if self.succ[from].len() >= MAX_EDGES || distance < 0 || self.succ[from].contains(&to) {
            return;
        }
        self.succ[from].push(to);
    }

    /// `shallow_copy`: `+1` on every node reachable from `start`.
    fn shallow_copy(&mut self, start: usize) {
        let mut seen = vec![false; self.len()];
        let mut stack = vec![start];
        while let Some(n) = stack.pop() {
            if seen[n] {
                continue;
            }
            seen[n] = true;
            self.ref_count[n] += 1;
            for &m in self.succ[n].iter().rev() {
                if !seen[m] {
                    stack.push(m);
                }
            }
        }
    }
}

/// CONFIGS row 34
fn cfg_random_api_sequences() {
    let mut rng = Rng::new(0x5EED_1234);
    for round in 0..250 {
        let mut ops = vec![Op::CreateGraph];
        let mut m = Model::new();

        for _ in 0..60 {
            match rng.below(11) {
                0 | 1 | 2 => {
                    let n = if rng.bool() && m.len() > 0 {
                        m.given[rng.below(m.len())].clone()
                    } else {
                        rng.name(70)
                    };
                    ops.push(Op::AddNode(0, n.clone()));
                    m.add_node(&n);
                }
                3 | 4 => {
                    if m.len() >= 1 {
                        let a = rng.below(m.len());
                        let b = rng.below(m.len());
                        let d = match rng.below(5) {
                            0 => 0,
                            1 => 1,
                            2 => rng.range_i32(0, 5),
                            3 => rng.range_i32(0, 100_000),
                            _ => rng.range_i32(-5, 100),
                        };
                        ops.push(Op::AddEdge(a, b, d));
                        m.add_edge(a, b, d);
                    }
                }
                5 => {
                    if m.len() >= 1 {
                        let a = rng.below(m.len());
                        ops.push(Op::ShallowCopy(a));
                        m.shallow_copy(a);
                    }
                }
                6 => {
                    if m.len() >= 1 {
                        ops.push(Op::FindShortestPath(rng.below(m.len()), rng.below(m.len())));
                    }
                }
                7 => {
                    let n = if rng.bool() && m.len() > 0 {
                        m.given[rng.below(m.len())].clone()
                    } else {
                        rng.name(70)
                    };
                    ops.push(Op::GetNodeByName(0, n));
                }
                8 => {
                    if m.len() >= 1 {
                        ops.push(Op::PrintNode(rng.below(m.len())));
                    }
                }
                9 => ops.push(Op::PrintGraph(0)),
                _ => {
                    // delete only where the reference count provably stays > 0
                    if m.len() >= 1 {
                        let a = rng.below(m.len());
                        if m.ref_count[a] > 1 {
                            m.ref_count[a] -= 1;
                            ops.push(Op::DeleteNode(a));
                        }
                    }
                }
            }
        }
        ops.push(Op::DumpAll);
        ops.push(Op::PrintGraph(0));
        ops.push(Op::FreeGraph(0));
        assert_same(&format!("cfg_random_api_sequences round={round}"), &ops);
    }
}

// ---------------------------------------------------------------------------
// Phase C — ERRORS.md rows 2-35
// ---------------------------------------------------------------------------

/// ERRORS row 2
fn err_add_node_null_graph() {
    let ops = vec![
        Op::CreateGraph,
        Op::AddNodeNullGraph(b"x".to_vec()),
        Op::AddNodeNullGraph(Vec::new()),
        Op::DumpAll,
        Op::FreeGraph(0),
    ];
    assert_same("err_add_node_null_graph", &ops);
}

/// ERRORS row 3
fn err_add_node_null_name() {
    let ops = vec![
        Op::CreateGraph,
        Op::AddNodeNullName(0),
        Op::AddNode(0, b"a".to_vec()),
        Op::AddNodeNullName(0),
        Op::DumpAll,
        Op::FreeGraph(0),
    ];
    assert_same("err_add_node_null_name", &ops);
}

/// ERRORS row 4
fn err_add_node_null_both() {
    let ops = vec![Op::AddNodeNullBoth, Op::AddNodeNullBoth];
    assert_same("err_add_node_null_both", &ops);
}

/// ERRORS row 5
fn err_add_node_graph_full() {
    let mut ops = vec![Op::CreateGraph];
    for i in 0..MAX_NODES {
        ops.push(Op::AddNode(0, name(i)));
    }
    // three more attempts, one of them a duplicate name: the "full" check comes
    // first, so all three report "Graph is full"
    ops.push(Op::AddNode(0, name(MAX_NODES)));
    ops.push(Op::AddNode(0, name(0)));
    ops.push(Op::AddNode(0, Vec::new()));
    ops.push(Op::DumpAll);
    ops.push(Op::FreeGraph(0));
    assert_same("err_add_node_graph_full", &ops);
}

/// ERRORS row 6
fn err_add_node_duplicate() {
    let ops = vec![
        Op::CreateGraph,
        Op::AddNode(0, b"dup".to_vec()),
        Op::AddNode(0, b"dup".to_vec()),
        Op::AddNode(0, b"other".to_vec()),
        Op::AddNode(0, b"dup".to_vec()),
        Op::AddNode(0, Vec::new()),
        Op::AddNode(0, Vec::new()),
        Op::AddNode(0, vec![0xff, 0xfe]),
        Op::AddNode(0, vec![0xff, 0xfe]),
        Op::DumpAll,
        Op::PrintGraph(0),
        Op::FreeGraph(0),
    ];
    assert_same("err_add_node_duplicate", &ops);
}

/// ERRORS row 7
fn err_add_node_duplicate_truncated() {
    let base = repeat(b'z', 63);
    let mut a = base.clone();
    a.push(b'1');
    let mut b = base.clone();
    b.push(b'2');
    let ops = vec![
        Op::CreateGraph,
        Op::AddNode(0, a.clone()),
        // differs only past the truncation point -> accepted
        Op::AddNode(0, b.clone()),
        // exactly the stored name -> duplicate
        Op::AddNode(0, base.clone()),
        Op::DumpAll,
        Op::FreeGraph(0),
    ];
    assert_same("err_add_node_duplicate_truncated", &ops);
}

/// ERRORS rows 9, 10, 11
fn err_add_edge_nulls() {
    let mut ops = build(2, &[]);
    ops.push(Op::AddEdgeNullFrom(0, 5));
    ops.push(Op::AddEdgeNullTo(0, 5));
    ops.push(Op::AddEdgeNullBoth(5));
    // the NULL check runs before the negative-distance check
    ops.push(Op::AddEdgeNullFrom(0, -1));
    ops.push(Op::AddEdgeNullTo(0, -1));
    ops.push(Op::AddEdgeNullBoth(i32::MIN));
    ops.push(Op::DumpAll);
    ops.push(Op::FreeGraph(0));
    assert_same("err_add_edge_nulls", &ops);
}

/// ERRORS rows 12, 13
fn err_add_edge_max_edges() {
    let mut ops = build(14, &[]);
    for i in 1..=MAX_EDGES {
        ops.push(Op::AddEdge(0, i, i as i32));
    }
    // 11th edge
    ops.push(Op::AddEdge(0, 11, 1));
    // full *and* negative distance: "maximum edges" wins
    ops.push(Op::AddEdge(0, 12, -5));
    // full *and* duplicate: "maximum edges" wins as well
    ops.push(Op::AddEdge(0, 1, 1));
    ops.push(Op::DumpAll);
    ops.push(Op::FreeGraph(0));
    assert_same("err_add_edge_max_edges", &ops);
}

/// ERRORS row 14
fn err_add_edge_negative_distance() {
    let mut ops = build(2, &[]);
    for d in [-1i32, -2, -1000, i32::MIN, i32::MIN + 1] {
        ops.push(Op::AddEdge(0, 1, d));
    }
    ops.push(Op::DumpAll);
    ops.push(Op::FreeGraph(0));
    assert_same("err_add_edge_negative_distance", &ops);
}

/// ERRORS rows 15, 16
fn err_add_edge_duplicate() {
    let mut ops = build(3, &[]);
    ops.push(Op::AddEdge(0, 1, 5));
    ops.push(Op::AddEdge(0, 1, 5));
    ops.push(Op::AddEdge(0, 1, 6));
    ops.push(Op::AddEdge(0, 2, 5));
    ops.push(Op::AddEdge(0, 2, 0));
    // duplicate self edge
    ops.push(Op::AddEdge(1, 1, 1));
    ops.push(Op::AddEdge(1, 1, 2));
    // duplicate *and* negative: the negative check runs first
    ops.push(Op::AddEdge(0, 1, -3));
    ops.push(Op::DumpAll);
    ops.push(Op::FreeGraph(0));
    assert_same("err_add_edge_duplicate", &ops);
}

/// ERRORS row 17
fn err_delete_node_null() {
    let ops = vec![
        Op::DeleteNodeNull,
        Op::DeleteNodeNull,
        Op::CreateGraph,
        Op::DeleteNodeNull,
        Op::FreeGraph(0),
    ];
    assert_same("err_delete_node_null", &ops);
}

/// ERRORS row 20
fn err_shallow_copy_null() {
    let ops = vec![Op::ShallowCopyNull, Op::ShallowCopyNull];
    assert_same("err_shallow_copy_null", &ops);
}

/// ERRORS rows 21, 22, 23
fn err_fsp_nulls() {
    let mut ops = build(2, &[(0, 1, 1)]);
    ops.push(Op::FindShortestPathNullStart(0));
    ops.push(Op::FindShortestPathNullEnd(0));
    ops.push(Op::FindShortestPathNullLen(0, 1));
    ops.push(Op::FindShortestPathNullLen(0, 0));
    ops.push(Op::FreeGraph(0));
    assert_same("err_fsp_nulls", &ops);
}

/// ERRORS row 25
fn err_fsp_unreachable() {
    // two disconnected components
    let mut ops = build(4, &[(0, 1, 1), (2, 3, 1)]);
    ops.push(Op::FindShortestPath(0, 2));
    ops.push(Op::FindShortestPath(0, 3));
    ops.push(Op::FindShortestPath(1, 0));
    ops.push(Op::FindShortestPath(3, 0));
    ops.push(Op::FreeGraph(0));
    assert_same("err_fsp_unreachable", &ops);

    // isolated node
    let mut ops = build(2, &[]);
    ops.push(Op::FindShortestPath(0, 1));
    ops.push(Op::FindShortestPath(1, 0));
    ops.push(Op::FreeGraph(0));
    assert_same("err_fsp_unreachable isolated", &ops);
}

/// ERRORS row 26: `end` is recorded in `state` but never relaxed, so its
/// distance is still `INT_MAX`.
fn err_fsp_end_seen_but_infinite() {
    // 0 -> 1 with distance INT_MAX: the relaxation `new_distance <
    // state[idx].distance` fails because INT_MAX is not < INT_MAX.
    let mut ops = build(2, &[(0, 1, i32::MAX)]);
    ops.push(Op::FindShortestPath(0, 1));
    ops.push(Op::DumpAll);
    ops.push(Op::FreeGraph(0));
    assert_same("err_fsp_end_seen_but_infinite", &ops);

    // longer chain where the last hop is INT_MAX
    let mut ops = build(3, &[(0, 1, 0), (1, 2, i32::MAX)]);
    ops.push(Op::FindShortestPath(0, 2));
    ops.push(Op::FindShortestPath(0, 1));
    ops.push(Op::FreeGraph(0));
    assert_same("err_fsp_end_seen_but_infinite chain", &ops);
}

/// ERRORS rows 30, 31, 32
fn err_get_node_nulls_and_misses() {
    let mut ops = vec![Op::CreateGraph];
    ops.push(Op::GetNodeByNameNullGraph(b"a".to_vec()));
    ops.push(Op::GetNodeByNameNullName(0));
    ops.push(Op::GetNodeByName(0, b"a".to_vec()));
    ops.push(Op::AddNode(0, b"a".to_vec()));
    ops.push(Op::GetNodeByNameNullGraph(b"a".to_vec()));
    ops.push(Op::GetNodeByNameNullName(0));
    ops.push(Op::GetNodeByName(0, b"b".to_vec()));
    ops.push(Op::GetNodeByName(0, Vec::new()));
    ops.push(Op::GetNodeByName(0, repeat(b'a', 200)));
    ops.push(Op::FreeGraph(0));
    assert_same("err_get_node_nulls_and_misses", &ops);
}

/// ERRORS rows 33, 34, 35
fn err_print_and_free_nulls() {
    let ops = vec![
        Op::PrintNodeNull,
        Op::PrintGraphNull,
        Op::FreeGraphNull,
        Op::PrintNodeNull,
        Op::PrintGraphNull,
        Op::FreeGraphNull,
    ];
    assert_same("err_print_and_free_nulls", &ops);
}

/// Generic FFI boundary sweep: every entry point with a NULL for each pointer
/// argument, plus out-of-range integers where the C takes an `int`.
fn err_generic_null_and_range_sweep() {
    let mut ops = build(2, &[]);
    ops.push(Op::AddNodeNullGraph(Vec::new()));
    ops.push(Op::AddNodeNullName(0));
    ops.push(Op::AddNodeNullBoth);
    for d in [
        i32::MIN,
        i32::MIN + 1,
        -1,
        0,
        1,
        i32::MAX - 1,
        i32::MAX,
    ] {
        ops.push(Op::AddEdgeNullFrom(0, d));
        ops.push(Op::AddEdgeNullTo(0, d));
        ops.push(Op::AddEdgeNullBoth(d));
    }
    ops.push(Op::DeleteNodeNull);
    ops.push(Op::ShallowCopyNull);
    ops.push(Op::FindShortestPathNullStart(0));
    ops.push(Op::FindShortestPathNullEnd(1));
    ops.push(Op::FindShortestPathNullLen(0, 1));
    ops.push(Op::GetNodeByNameNullGraph(Vec::new()));
    ops.push(Op::GetNodeByNameNullName(0));
    ops.push(Op::PrintNodeNull);
    ops.push(Op::PrintGraphNull);
    ops.push(Op::DumpAll);
    ops.push(Op::FreeGraph(0));
    ops.push(Op::FreeGraphNull);
    assert_same("err_generic_null_and_range_sweep", &ops);
}

/// The two `.so`s must agree on the published data layout, otherwise every
/// memory comparison above would be meaningless.
fn layout_matches_header() {
    assert_eq!(std::mem::size_of::<EdgeT>(), 16);
    assert_eq!(std::mem::size_of::<NodeT>(), 240);
    assert_eq!(std::mem::align_of::<NodeT>(), 8);
    assert_eq!(std::mem::size_of::<GraphT>(), 808);
}

// ---------------------------------------------------------------------------
// Serial harness (see common::run_suite)
// ---------------------------------------------------------------------------

fn main() {
    let cases: &[(&str, fn())] = &[
        ("cfg_create_free_empty", cfg_create_free_empty),
        ("cfg_single_node", cfg_single_node),
        ("cfg_add_node_name_lengths", cfg_add_node_name_lengths),
        ("cfg_add_node_random_names", cfg_add_node_random_names),
        ("cfg_add_node_shared_prefixes", cfg_add_node_shared_prefixes),
        ("cfg_add_node_fill_to_max", cfg_add_node_fill_to_max),
        ("cfg_add_edge_first", cfg_add_edge_first),
        ("cfg_add_edge_fill_to_max", cfg_add_edge_fill_to_max),
        ("cfg_add_edge_distance_values", cfg_add_edge_distance_values),
        ("cfg_add_edge_self", cfg_add_edge_self),
        ("cfg_add_edge_both_directions", cfg_add_edge_both_directions),
        ("cfg_get_node_positions", cfg_get_node_positions),
        ("cfg_get_node_truncated_lookup", cfg_get_node_truncated_lookup),
        ("cfg_print_node_no_edges", cfg_print_node_no_edges),
        ("cfg_print_node_edges", cfg_print_node_edges),
        ("cfg_print_node_ref_counts", cfg_print_node_ref_counts),
        ("cfg_print_graph_sizes", cfg_print_graph_sizes),
        ("cfg_shallow_copy_single", cfg_shallow_copy_single),
        ("cfg_shallow_copy_topologies", cfg_shallow_copy_topologies),
        ("cfg_shallow_copy_repeated", cfg_shallow_copy_repeated),
        ("cfg_fsp_start_is_end", cfg_fsp_start_is_end),
        ("cfg_fsp_chain", cfg_fsp_chain),
        ("cfg_fsp_equal_cost_tie", cfg_fsp_equal_cost_tie),
        ("cfg_fsp_state_full", cfg_fsp_state_full),
        ("cfg_fsp_cycles", cfg_fsp_cycles),
        ("cfg_fsp_relaxation", cfg_fsp_relaxation),
        ("cfg_fsp_zero_weights", cfg_fsp_zero_weights),
        ("cfg_fsp_random_graphs", cfg_fsp_random_graphs),
        ("cfg_fsp_dense_max_edges", cfg_fsp_dense_max_edges),
        ("cfg_delete_node_positive", cfg_delete_node_positive),
        ("cfg_free_graph_refcount_one", cfg_free_graph_refcount_one),
        ("cfg_free_graph_refcount_many", cfg_free_graph_refcount_many),
        ("cfg_random_api_sequences", cfg_random_api_sequences),
        ("err_add_node_null_graph", err_add_node_null_graph),
        ("err_add_node_null_name", err_add_node_null_name),
        ("err_add_node_null_both", err_add_node_null_both),
        ("err_add_node_graph_full", err_add_node_graph_full),
        ("err_add_node_duplicate", err_add_node_duplicate),
        ("err_add_node_duplicate_truncated", err_add_node_duplicate_truncated),
        ("err_add_edge_nulls", err_add_edge_nulls),
        ("err_add_edge_max_edges", err_add_edge_max_edges),
        ("err_add_edge_negative_distance", err_add_edge_negative_distance),
        ("err_add_edge_duplicate", err_add_edge_duplicate),
        ("err_delete_node_null", err_delete_node_null),
        ("err_shallow_copy_null", err_shallow_copy_null),
        ("err_fsp_nulls", err_fsp_nulls),
        ("err_fsp_unreachable", err_fsp_unreachable),
        ("err_fsp_end_seen_but_infinite", err_fsp_end_seen_but_infinite),
        ("err_get_node_nulls_and_misses", err_get_node_nulls_and_misses),
        ("err_print_and_free_nulls", err_print_and_free_nulls),
        ("err_generic_null_and_range_sweep", err_generic_null_and_range_sweep),
        ("layout_matches_header", layout_matches_header),
    ];
    common::run_suite(cases);
}
