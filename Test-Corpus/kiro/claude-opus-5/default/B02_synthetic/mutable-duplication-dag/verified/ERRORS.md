# Differential findings: `c_src` vs `translation`

Both programs are built and run as executables and compared on stdout, stderr and
exit status:

```
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .   # -> c_src/build/driver
cd translation && cargo build --release                                # -> translation/target/release/driver
cd translation && cargo test                                           # the differential suite
```

The suite lives in `translation/tests/` (`menu.rs`, `cities.rs`, `routes.rs`,
`paths.rs`, `refcount.rs`, `heap.rs`, plus the harness in `tests/harness/`). It
spawns both binaries, feeds them the same bytes on stdin, and compares all three
observables. `tests/harness/mod.rs` builds the C program on first use if
`c_src/build/driver` is missing. Two extra scripts at the repository root,
`fuzz_diff.py` and `fuzz_heap.py`, run the same comparison over randomly
generated command streams; ~3200 random cases were run while writing this and all
were byte-identical.

## Mismatches found and fixed

### 1. The path array can be handed a freed node's chunk, and the C program dies

`find_shortest_path` ends with `malloc(sizeof(node_t*) * count)`. For a path of
30 or 31 nodes that request (240 or 248 bytes) falls in the same glibc size class
as `node_t` (chunk size 256). If a node has been deleted to a reference count of
0 beforehand, its chunk is sitting in the tcache and is handed to that `malloc`,
so `result[i] = ...` overwrites the node's fields with node addresses:

* `city_name` becomes the first eight pointers,
* `ref_count` (offset 64) becomes the low half of `path[8]`,
* `edges[0..=10]` become pointers and pointer halves,
* `edge_count` (offset 232) becomes the low half of `path[29]`, i.e. a number in
  the millions.

The next `3` (show all cities) then prints that wreckage and keeps walking
`edges[i]`. At `i == 11` the read lands past the 248 usable bytes of the chunk, on
the next chunk's size field (`0x101`), and dereferencing that address kills the
process: `SIGSEGV`, exit status 139, with everything still in the block-buffered
stdout buffer discarded (16384 of 17554 bytes reach the pipe in the reference
case).

Before the fix the Rust program ran to completion and exited 0. Two things were
missing:

* `Heap` modelled a freed chunk's contents only as far as the eight bytes
  `free()` writes; the stores into the path array were not modelled at all
  (`alloc_scratch` was immediately followed by `free_scratch` and neither touched
  the aliased node).
* `free(path)` happens in `main` *after* the path is printed. The model did it
  inside `find_shortest_path`, so the order of "array contents overwrite the
  node" and "free() writes its next pointer over the first eight bytes" was
  wrong. That ordering is observable: the name printed later is the safe-linked
  pointer, not the first array entry.

Fixed in `src/dag_lib.rs`:

* `Heap::corrupt` keeps a byte image of any chunk that was reused for something
  that is not a `node_t`, and `name`, `ref_count`, `edge_count`, `read_edge` and
  `push_edge` read and write through it at the ABI offsets of `node_t`.
* `Heap::write_scratch` applies the stores the path array makes;
  `Heap::free_scratch` leaves the safe-linked next pointer behind, like
  `Heap::free`.
* `read_edge` returns `EdgeRead::Wild` for an offset past the usable chunk (or an
  address that is not a node), and every place the C code dereferences such a
  pointer calls `segfault()`, which performs the same read of an unmapped
  address. In `print_node` the literal `"    -> "` is appended to the stdout
  buffer first, because that is what glibc's `printf` does before it touches the
  `%s` argument.
* `find_shortest_path` now returns a `PathArray` and `main` calls `free_path`
  where `main.c` calls `free(path)`.

Verified for path lengths 28 to 33 (only 30 and 31 crash) and with the crash
pushed to seven different offsets inside the 4096-byte stdout block, with ASLR
both enabled and disabled: stdout, stderr and exit status are identical
(`tests/heap.rs`).

### 2. Reachable double free

Found while probing the same area, and already handled correctly by the existing
`glibc_abort` emulation, but worth recording because it looks unreachable and is
not:

1. `1 A` then `7 A` frees A's chunk. `city_name` now holds the safe-linked next
   pointer, which for the reference build is the two bytes `08 04`.
2. `6 <0x08><0x04>` — those bytes typed as a city name — makes
   `get_node_by_name` match the freed node, and `shallow_copy` pushes its
   reference count from 0 back to 1.
3. On exit, `free_graph` decrements it to 0 and calls `free()` on a chunk that is
   already in the tcache. glibc prints `free(): double free detected in tcache 2`
   and aborts: exit status 134, buffered stdout lost.

Both programs agree, including the message and the lost stdout
(`tests/refcount.rs::a_shallow_copy_of_a_freed_node_makes_the_exit_double_free`).

## Things that look like mismatches but are properties of the C program

### The C build configuration changes the behaviour of the undefined parts

Built as the task describes (`cmake ..`, i.e. no `CMAKE_BUILD_TYPE`, so no
optimisation) the input `1 / A / 7 / A / 8` runs to completion and exits 0. The
same source configured with `-DCMAKE_BUILD_TYPE=Release` aborts with
`free(): double free detected in tcache 2`, because `delete_node`'s
`node->ref_count--` and the following comparison are reordered around the
`free()` in a way the unoptimised build does not do. The Rust program matches the
unoptimised build, which is the one the quoted build commands produce. Anything
that reads a freed node is undefined behaviour in C and cannot be matched by
*any* implementation across compiler settings.

### Reads of freed memory are only reproducible with ASLR disabled

Where the C program prints bytes out of a freed chunk (`City: <name>` after a
delete, the numbers in the wrecked node above) it is printing pieces of heap
addresses. With ASLR on, the C program produces different output on every run;
with ASLR off it is stable, and `HEAP_NODE_BASE` in `src/dag_lib.rs` is the
address the reference build uses in that configuration. The suite therefore runs
those cases through `setarch -R`. If a machine cannot disable ASLR, the harness
runs the C program twice and requires the Rust program to match the part the C
program itself reproduces, comparing stderr and exit status exactly either way
(`same_freed_memory` in `tests/harness/mod.rs`).

## Known differences that are not fixed

### `SIGPIPE`

If the reader of stdout closes early, the C program is killed by `SIGPIPE`
(status 141 as a shell reports it) at its next write. The Rust program exits 0
instead: the Rust runtime sets `SIGPIPE` to `SIG_IGN` before `main`, and
restoring the default disposition needs either a C dependency (`libc`) or an
architecture-specific `rt_sigaction` syscall written in inline assembly. Neither
seemed worth the build risk, because this cannot be reached the way the programs
are compared: stdout and stderr are captured to completion, so no write ever
fails. Everything up to the point of death is identical; only the manner of death
differs.

### Allocation failure

`create_graph`, `add_node` and `find_shortest_path` each have a branch for
`malloc` returning NULL (`"Failed to create graph"` and exit 1,
`"Error: Failed to allocate node"`, `"Error: Failed to allocate path"`). The
translation does not model an allocator that fails, so those three messages are
unreachable in Rust. Reaching them in C needs an artificial memory limit rather
than an input.

### Unreachable code kept for fidelity

These C branches cannot be reached from any input and are translated but never
exercised: the `!graph` / `!city_name` / `!from` / `!to` / `!node` /
`!path_length` null-parameter guards (main never passes null), `print_node`'s
`"NULL node"` and `print_graph`'s `"NULL graph"`, `find_shortest_path`'s
`current_idx == -1` and `current_state_idx == -1` early exits, the
`state_count < MAX_NODES` and `visited_count < MAX_NODES` limits (the graph holds
at most `MAX_NODES` nodes, so neither counter can be driven past it), and
`shallow_copy`'s `copy == NULL` failure path in `main`.

## Input classes covered

* Menu loop: empty input, EOF at every prompt in every case, a final line with no
  newline, blank and whitespace-only lines, sign-only input, trailing junk after
  the digits, leading zeros, values outside `int` (including the `long`
  saturation glibc's `%d` performs and the truncation to `int`, e.g.
  `4294967297` meaning "add city"), lines longer than the 256-byte buffer being
  split into several commands, embedded NUL bytes, CRLF, non-ASCII bytes, and
  every out-of-range choice.
* Cities: first city, duplicate, empty name, names of 1/62/63/64/65/70/128 bytes
  (`strncpy` truncation at 63), two long names that collide only after
  truncation, names of 254 to 600 input bytes, names that look like commands,
  the 100-node limit and the 101st add, and lookups of names too long to have
  been stored intact.
* Routes: invalid and out-of-range distances, negative and negative-zero
  distances, unknown from/to city (from is reported first), duplicate edge, self
  edge, zero distance, the ten-edge limit, and the order of `add_edge`'s three
  checks (edge count, then sign, then duplicate).
* Shortest path: unknown endpoints, start equal to end, disconnected graph, wrong
  direction, ties, a cheaper longer route, zero-weight edges, an `INT_MAX` edge
  (which is never an improvement, so a directly connected node reports no path),
  a wrapping distance sum, cycles and self loops, a 100-node chain and a graph
  where every node has ten edges.
* Reference counting: repeated shallow copies, diamonds, cycles, a 100-deep
  recursion, deletes that do and do not free, deleting an edge destination, and
  every read that then goes through a dangling pointer.
* Streams: stdout and stderr compared separately through pipes, and also with
  both redirected to one file, which pins down where the block-buffered stdout
  stream is flushed relative to the unbuffered stderr writes.
