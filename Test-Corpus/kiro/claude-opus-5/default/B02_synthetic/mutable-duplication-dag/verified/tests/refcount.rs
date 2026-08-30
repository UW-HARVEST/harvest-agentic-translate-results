//! Case 6 (shallow copy), case 7 (delete node) and case 8 (exit): reference
//! counting, and the reads the C program performs through the dangling pointers
//! it keeps in `graph->nodes` after `delete_node` frees a node.

mod harness;
use harness::{same, same_freed_memory, same_merged};

#[test]
fn shallow_copy_single_node() {
    same("copy_single", b"1\nA\n6\nA\n4\nA\n8\n");
}

#[test]
fn shallow_copy_eof_and_unknown() {
    same("copy_eof", b"6\n");
    same("copy_unknown", b"6\nQ\n8\n");
    same("copy_unknown_after_add", b"1\nA\n6\nB\n8\n");
}

#[test]
fn shallow_copy_chain() {
    // Every reachable node is counted once.
    same(
        "copy_chain",
        b"1\nA\n1\nB\n1\nC\n2\nA\nB\n1\n2\nB\nC\n1\n6\nA\n3\n8\n",
    );
}

#[test]
fn shallow_copy_diamond() {
    // D is reachable by two routes and must still only be counted once.
    same(
        "copy_diamond",
        b"1\nA\n1\nB\n1\nC\n1\nD\n2\nA\nB\n1\n2\nA\nC\n1\n2\nB\nD\n1\n2\nC\nD\n1\n6\nA\n3\n8\n",
    );
}

#[test]
fn shallow_copy_cycle() {
    same("copy_cycle", b"1\nA\n1\nB\n2\nA\nB\n1\n2\nB\nA\n1\n6\nA\n6\nB\n3\n8\n");
    same("copy_self_loop", b"1\nA\n2\nA\nA\n0\n6\nA\n6\nA\n3\n8\n");
}

#[test]
fn shallow_copy_repeated() {
    same("copy_repeated", b"1\nA\n6\nA\n6\nA\n6\nA\n4\nA\n8\n");
}

#[test]
fn shallow_copy_deep_recursion() {
    // A 100-node chain reached from one end: recursion depth 100.
    let names: Vec<String> = (0..100).map(|i| format!("N{i:03}")).collect();
    let mut input = Vec::new();
    for n in &names {
        input.extend_from_slice(format!("1\n{n}\n").as_bytes());
    }
    for w in names.windows(2) {
        input.extend_from_slice(format!("2\n{}\n{}\n1\n", w[0], w[1]).as_bytes());
    }
    input.extend_from_slice(format!("6\n{}\n4\n{}\n3\n8\n", names[0], names[99]).as_bytes());
    same("copy_deep_recursion", &input);
}

#[test]
fn shallow_copy_only_follows_edges_out() {
    // Nodes that can only reach the start are untouched.
    same(
        "copy_only_downstream",
        b"1\nA\n1\nB\n1\nC\n2\nB\nA\n1\n2\nA\nC\n1\n6\nA\n3\n8\n",
    );
}

#[test]
fn delete_eof_and_unknown() {
    same("delete_eof", b"7\n");
    same("delete_unknown", b"7\nQ\n8\n");
    same("delete_unknown_after_add", b"1\nA\n7\nB\n8\n");
}

#[test]
fn delete_after_copies_does_not_free() {
    // ref_count 3 -> 2 -> 1: the node stays alive and printable.
    same("delete_after_copies", b"1\nA\n6\nA\n6\nA\n7\nA\n3\n7\nA\n3\n4\nA\n8\n");
}

#[test]
fn delete_last_reference_then_exit() {
    // The node is freed and nothing reads it again before the program ends;
    // free_graph decrements the freed node's counter but prints nothing.
    same("delete_then_exit", b"1\nA\n7\nA\n8\n");
    same("delete_then_eof", b"1\nA\n7\nA\n");
}

#[test]
fn delete_one_of_several_then_exit() {
    same("delete_one_of_three", b"1\nA\n1\nB\n1\nC\n7\nB\n8\n");
}

#[test]
fn delete_then_show_graph() {
    // print_graph walks the dangling pointer and prints whatever free() left in
    // the first bytes of the chunk.
    same_freed_memory("delete_then_show_graph", b"1\nA\n7\nA\n3\n8\n");
    same_freed_memory(
        "delete_middle_then_show_graph",
        b"1\nA\n1\nB\n1\nC\n7\nB\n3\n8\n",
    );
}

#[test]
fn delete_then_look_up_by_name() {
    // get_node_by_name compares against the freed chunk's contents.
    same_freed_memory("delete_then_details", b"1\nA\n7\nA\n4\nA\n8\n");
    same_freed_memory("delete_then_delete_again", b"1\nA\n7\nA\n7\nA\n8\n");
    same_freed_memory("delete_then_copy", b"1\nA\n7\nA\n6\nA\n8\n");
    same_freed_memory("delete_then_route", b"1\nA\n1\nB\n7\nA\n2\nA\nB\n1\n8\n");
    same_freed_memory("delete_then_path", b"1\nA\n1\nB\n7\nA\n5\nA\nB\n8\n");
    same_freed_memory("delete_then_readd", b"1\nA\n7\nA\n1\nA\n3\n8\n");
}

#[test]
fn delete_two_nodes() {
    // Two chunks in the same recycling bin; the second free links to the first.
    same_freed_memory("delete_two", b"1\nA\n1\nB\n7\nA\n7\nB\n3\n8\n");
    same_freed_memory(
        "delete_two_then_readd",
        b"1\nA\n1\nB\n7\nA\n7\nB\n1\nC\n1\nD\n3\n8\n",
    );
    same_freed_memory("delete_three", b"1\nA\n1\nB\n1\nC\n7\nA\n7\nB\n7\nC\n3\n8\n");
}

#[test]
fn delete_a_node_that_is_an_edge_destination() {
    // print_node on A follows the edge into the freed chunk.
    same_freed_memory(
        "delete_edge_destination",
        b"1\nA\n1\nB\n2\nA\nB\n7\n7\nB\n4\nA\n3\n8\n",
    );
    same_freed_memory(
        "delete_edge_destination_path",
        b"1\nA\n1\nB\n1\nC\n2\nA\nB\n1\n2\nB\nC\n1\n7\nB\n5\nA\nC\n8\n",
    );
}

#[test]
fn delete_after_copy_then_delete_to_zero() {
    same_freed_memory("copy_then_two_deletes", b"1\nA\n6\nA\n7\nA\n3\n7\nA\n3\n8\n");
}

#[test]
fn a_name_that_matches_the_bytes_free_left_behind() {
    // free() leaves its safe-linked next pointer at the head of the chunk, and
    // that pointer aliases city_name. Typing those bytes as a city name makes
    // get_node_by_name hand back the freed node.
    same_freed_memory("lookup_freed_by_garbage_name", b"1\nA\n7\nA\n4\n\x08\x04\n3\n8\n");
    same_freed_memory(
        "delete_freed_by_garbage_name",
        b"1\nA\n7\nA\n4\n\x08\x04\n7\n\x08\x04\n4\n\x08\x04\n3\n8\n",
    );
    same_freed_memory(
        "route_to_freed_by_garbage_name",
        b"1\nA\n1\nB\n7\nB\n2\nA\n\x08\x04\n5\n3\n8\n",
    );
    same_freed_memory(
        "path_to_freed_by_garbage_name",
        b"1\nA\n1\nB\n2\nA\nB\n1\n7\nB\n5\nA\n\x08\x04\n3\n8\n",
    );
    same_freed_memory(
        "two_freed_chunks_by_garbage_name",
        b"1\nA\n1\nB\n7\nA\n7\nB\n4\n\x08\x04\n7\n\x08\x04\n3\n8\n",
    );
}

#[test]
fn a_shallow_copy_of_a_freed_node_makes_the_exit_double_free() {
    // The copy pushes the freed node's counter back up to 1, so free_graph
    // decrements it to 0 and frees a chunk that is already in the recycling
    // bin: glibc reports a double free and aborts, losing the buffered stdout.
    same_freed_memory("copy_freed_then_double_free", b"1\nA\n7\nA\n6\n\x08\x04\n3\n8\n");
    same_freed_memory("copy_freed_then_double_free_eof", b"1\nA\n7\nA\n6\n\x08\x04\n");
}

#[test]
fn exit_frees_the_graph() {
    same("exit_after_work", b"1\nA\n1\nB\n2\nA\nB\n5\n6\nA\n8\n");
}

#[test]
fn merged_streams_refcounts() {
    same_merged("merged_copy_delete", b"1\nA\n6\nA\n7\nA\n7\nA\n6\nQ\n8\n");
}
