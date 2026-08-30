//! The `malloc` in `find_shortest_path` asks for `count * sizeof(node_t*)`
//! bytes. For a path of 30 or 31 nodes that lands in the same size class as a
//! `node_t`, so if a node has been freed the path array is handed that node's
//! chunk and the stores into the array overwrite the node's fields. The C
//! program then prints the wreckage and walks off the end of the chunk.

mod harness;
use harness::{same, same_freed_memory};

/// A chain of `n` nodes plus one node that gets deleted before the path query,
/// so its chunk is waiting in the recycling bin. `filler` invalid commands move
/// the crash around inside the block-buffered stdout stream.
fn chain_with_freed_chunk(n: usize, filler: usize) -> Vec<u8> {
    let names: Vec<String> = (0..n).map(|i| format!("N{i:02}")).collect();
    let mut input = Vec::new();
    input.extend_from_slice(b"1\nDEL\n");
    for name in &names {
        input.extend_from_slice(format!("1\n{name}\n").as_bytes());
    }
    for w in names.windows(2) {
        input.extend_from_slice(format!("2\n{}\n{}\n1\n", w[0], w[1]).as_bytes());
    }
    input.extend_from_slice(b"7\nDEL\n");
    input.extend_from_slice(format!("5\n{}\n{}\n", names[0], names[n - 1]).as_bytes());
    for _ in 0..filler {
        input.extend_from_slice(b"x\n");
    }
    input.extend_from_slice(b"3\n8\n");
    input
}

#[test]
fn path_array_lands_on_a_freed_node() {
    // 30 and 31 nodes: the path array's chunk is the freed node's chunk, so
    // printing the graph reads node addresses out of the array, then reads past
    // the chunk and dies. Everything still in the stdout buffer is lost, which
    // is why the flushed prefix is free of heap-derived bytes.
    same("path_array_30", &chain_with_freed_chunk(30, 0));
    same("path_array_31", &chain_with_freed_chunk(31, 0));
}

#[test]
fn path_array_crash_at_various_buffer_offsets() {
    // The lost tail is whatever had not reached a full block yet, so the crash
    // has to happen at the same point in the byte stream. Some offsets push
    // heap-derived bytes into a block that does get flushed, hence the
    // ASLR-controlled comparison.
    for filler in [1usize, 3, 5, 7, 11, 13, 17] {
        same_freed_memory(
            &format!("path_array_30_filler_{filler}"),
            &chain_with_freed_chunk(30, filler),
        );
    }
}

#[test]
fn path_array_lands_elsewhere() {
    // 28, 29, 32 and 33 nodes ask for a different size class, so the freed node
    // keeps the bytes free() left in it and the program runs to completion.
    for n in [28usize, 29, 32, 33] {
        same_freed_memory(&format!("path_array_{n}"), &chain_with_freed_chunk(n, 0));
    }
}

#[test]
fn path_array_without_a_freed_chunk() {
    // Same graph, no delete: the array comes from fresh heap and nothing is
    // overwritten.
    for n in [30usize, 31] {
        let names: Vec<String> = (0..n).map(|i| format!("N{i:02}")).collect();
        let mut input = Vec::new();
        for name in &names {
            input.extend_from_slice(format!("1\n{name}\n").as_bytes());
        }
        for w in names.windows(2) {
            input.extend_from_slice(format!("2\n{}\n{}\n1\n", w[0], w[1]).as_bytes());
        }
        input.extend_from_slice(format!("5\n{}\n{}\n3\n8\n", names[0], names[n - 1]).as_bytes());
        same(&format!("path_array_no_delete_{n}"), &input);
    }
}

#[test]
fn repeated_path_queries_reuse_the_same_chunk() {
    // The array is freed after printing, so a second query of the same length
    // gets the same chunk back.
    let n = 30;
    let names: Vec<String> = (0..n).map(|i| format!("N{i:02}")).collect();
    let mut input = Vec::new();
    for name in &names {
        input.extend_from_slice(format!("1\n{name}\n").as_bytes());
    }
    for w in names.windows(2) {
        input.extend_from_slice(format!("2\n{}\n{}\n1\n", w[0], w[1]).as_bytes());
    }
    for _ in 0..3 {
        input.extend_from_slice(format!("5\n{}\n{}\n", names[0], names[n - 1]).as_bytes());
    }
    input.extend_from_slice(b"1\nAfter\n3\n8\n");
    same("repeated_path_queries", &input);
}

#[test]
fn freed_chunk_is_handed_back_to_add_city() {
    // The chunk the path array used is free again afterwards, so the next city
    // is allocated into it.
    let mut input = chain_with_freed_chunk(30, 0);
    // Drop the trailing "3\n8\n" and add a city first: add_node reinitialises
    // the fields, so the graph prints cleanly again apart from the two entries
    // that now point at the same node.
    input.truncate(input.len() - 4);
    input.extend_from_slice(b"1\nReused\n8\n");
    same_freed_memory("freed_chunk_reused_by_add_city", &input);
}
