# Differential findings: `c_src` vs `translation`

Both programs are compared by running them, not by linking them: the tests in
`tests/` spawn `c_src/build/driver` and the Cargo-built `driver`, feed both the
same bytes on stdin, and compare stdout, stderr and the exit status.

```
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .   # -> c_src/build/driver
cd translation && cargo build --release                                 # -> target/release/driver
cd translation && cargo test                                            # runs the comparison
```

Note on the C build: `c_src/CMakeLists.txt` sets no build type, so the C program
must be built with **no optimisation flags**. This matters for observable
behaviour, not just speed — see finding 7.

Everything below was found by running the two programs against each other; each
entry says what diverged, what caused it, and what changed in the Rust code.

---

## 1. Reference counting hit zero and the node was freed, but the graph kept the pointer

`delete_node` calls `free(node)` when the count reaches zero and never clears
`graph->nodes[i]`, so every later operation reads memory the allocator owns. The
program's visible behaviour after a deletion is therefore decided by glibc's
malloc, not by the program's own logic. The whole of `src/heap.rs` exists to
reproduce that. The rest of this file is the list of ways the first attempt got
it wrong.

Reproducer for the general shape: `1 A / 7 A / 4 A` — the city can no longer be
found, because the allocator overwrote the start of its name.

## 2. Chunk reuse order was a plain LIFO stack

**Symptom.** After deleting 8 or more cities and adding fresh ones, the new
cities appeared under the wrong graph slots.

```
1 C0 .. 1 C18   (19 cities)
7 C11 7 C10 7 C0 7 C8 7 C7 7 C12 7 C3 7 C18 7 C6 7 C15 7 C9 7 C1 7 C14 7 C4 7 C5
1 N0 .. 1 N14
3
```

C hands the chunks back in the order `3 12 7 8 0 10 11 1 9 14 15 4 5 6 18`; a
LIFO stack predicts `3 12 7 8 0 10 11 1 4 5 6 9 14 15 18`.

**Cause.** Only the first 7 frees go on the per-thread cache. The rest go to the
unsorted bin, coalesce with their free neighbours, and are then re-split by
`_int_malloc` in a specific order.

**Fix.** `src/heap.rs` now models the parts of `_int_malloc`/`_int_free` that a
program allocating a single size class can observe: the tcache (LIFO, 7 entries),
the unsorted bin (freed at the head, scanned from the tail), the small bins
(FIFO) and large bins (descending size, same-size chunks inserted in second
position), backward and forward consolidation, absorption into the top chunk,
`av->last_remainder`, and the "stash spares in the tcache" paths.

## 3. Freed chunks were always treated as having lost their name

**Symptom.** C printed the real city names for several deleted cities:

```
City: O48 (ref_count: 0)
```

while the translation printed an empty name.

**Cause.** The allocator only writes its list pointers over the *start of a free
block*. A chunk that is consolidated into a neighbour, or absorbed into the top
chunk, keeps its bytes untouched — including the name, which then still answers
to `strcmp`.

Reproducer: 8 cities, delete 7 (filling the tcache), then delete the 8th. The
8th borders the top chunk, is merged into it, and `4 C7` still finds it.

**Fix.** Clobbering is applied per write site (`Arena::clobber`) instead of on
every `free`.

## 4. Freeing an already free chunk did not abort

**Symptom.** C printed a heap diagnostic on stderr and died of `SIGABRT`
(status 134 through a shell); the translation exited 0.

```
1 A / 1 B / 2 A B 7 / 7 B / 6 A / 8
      -> free(): double free detected in tcache 2
```

A stale edge lets `shallow_copy` raise the freed chunk's reference count back to
one, and `free_graph` then frees it a second time.

**Cause.** Not modelled at all.

**Fix.** `Arena::free` checks where the chunk currently sits and calls
`cio::malloc_printerr`, which writes the diagnostic to the unbuffered stderr and
`std::process::abort`s. Which diagnostic depends on the chunk's state:

| chunk state when freed again | glibc message |
| --- | --- |
| on the tcache | `free(): double free detected in tcache 2` |
| in an unsorted/small/large bin | `double free or corruption (!prev)` |
| *is* the top chunk | `double free or corruption (top)` |
| in the interior of the merged top chunk | `corrupted size vs. prev_size while consolidating` |

## 5. stdout was flushed even when the program aborted

**Symptom.** After the abort in finding 4, C's stdout was *empty* (or truncated
to a whole number of 4096 byte blocks) while the translation printed everything.

**Cause.** stdout is block buffered when it is not a terminal, and `abort()` does
not flush stdio. Whatever had not yet filled a 4096 byte block is lost.

**Fix.** `cio::Out` already buffered 4096 bytes at a time; `malloc_printerr` uses
`std::process::abort`, which runs no destructors, so the buffer is dropped rather
than flushed. `tests/differential.rs::abort_truncates_stdout_at_a_block_boundary`
pins this down: it asserts the C program's stdout is non-empty and a multiple of
4096 before comparing.

## 6. `shallow_copy` could revive a chunk sitting in a bin

Same shape as finding 4, but with more than 7 deletions pending so the chunk has
fallen out of the tcache. Covered by
`tests/differential.rs::double_free_or_corruption_prev`.

## 7. Not a translation bug: the C build type changes the C program's behaviour

Building `c_src` with `-DCMAKE_BUILD_TYPE=Release` makes it abort on inputs where
the default (unoptimised) build exits 0, for example `1 A / 7 A / 8`. At `-O3`,
GCC drops the `node->ref_count--` store in `delete_node` because `free(node)` ends
the object's lifetime, so the freed chunk keeps `ref_count == 1`; `free_graph`
then decrements it to 0 and frees it again. At `-O0` the store happens, the count
goes to `-1`, and nothing is freed twice.

`c_src/CMakeLists.txt` specifies no build type, so **the unoptimised build is the
reference** and that is what the tests build and compare against.

---

## Known limitations

These are inputs where the C program's own behaviour cannot be reproduced. They
are listed with reproducers so the next reader can check them rather than
rediscover them.

### L1. Printing a freed node's name is not reproducible in C

```
1 A / 7 A / 3 / 8
```

`print_graph` follows the dangling pointer and prints whatever sits where the
name was. For a chunk on the tcache that is `tcache_entry::next`, which glibc
stores as `PROTECT_PTR(&e->next, NULL) == chunk_address >> 12`. The heap address
moves with ASLR, so the C program prints different bytes on every run:

```
$ for i in 1 2 3; do ./c_src/build/driver < case | sed -n 38p; done
City: <d7><ac><02> (ref_count: 0)
City: <8d><c7><01> (ref_count: 0)
City: <f6>x<01> (ref_count: 0)
```

No translation can be byte identical to a reference that is not byte identical to
itself. The Rust program prints an empty name there. Everything else about these
inputs *is* reproducible and does match:

* stderr and the exit status;
* the reference count read back out of the freed chunk (`ref_count: 0`);
* the freed node's edges;
* the surrounding output, byte for byte, before and after the name field.

`tests/randomized.rs::freed_name_reads_are_not_reproducible_in_c` asserts exactly
that: 24 runs of the C program disagree with each other, and the Rust output
matches every one of them apart from 1..8 bytes in the name field. The randomized
sweeps detect this class per input by running the C program twice, and for those
inputs still require stderr and the exit status to match exactly.

Measured rate: across ~2400 generated inputs, 711 fell into this class and all
711 agreed on stderr and exit status.

### L2. `find_shortest_path`'s own `malloc` is not modelled

`find_shortest_path` allocates `node_t *[count]` and frees it immediately. That
allocation is served from the same heap as the nodes, so it can

* carve bytes off the top chunk and overwrite a node chunk that was absorbed
  into it, or
* split a binned node-sized block, taking it out of the reuse order and leaving
  a 224 byte remainder that can never hold a node again.

Reproducer for the first:

```
1 C0 .. 1 C7 / 7 C0 .. 7 C7 / 1 SRC / 2 SRC C7 3 / 4 C7 / 5 SRC C7 / 4 C7
```

C reports `City 'C7' not found` on the second lookup (the path array overwrote
the name); the translation still finds it.

Modelling this needs a byte-granular allocator over several size classes rather
than the single-size-class model in `heap.rs`, and the immediate output it
produces is a raw heap pointer, i.e. ASLR dependent (L1) in any case. Reaching it
requires at least 8 deletions, a node absorbed into the top chunk, a successful
path query and then a further operation on that same city.

Measured rate: 0 mismatches in ~2400 inputs from the general sweeps; 4 in 1196
comparable inputs (0.3%) from a sweep written specifically to provoke it.

### L3. Which glibc diagnostic fires can depend on garbage heap metadata

Once a chunk in the interior of the merged top chunk is freed, glibc reads a
header that is whatever the program last wrote there. The table in finding 4
covers the four cases observed, but deeper inputs in that corner produce other
outcomes. Two reproducers, both requiring the same depth of use-after-free as L2
(each token on its own line):

`double free or corruption (out)` where the translation says
`corrupted size vs. prev_size while consolidating` (C status 134, so the abort,
the truncated stdout and the exit status all still agree — only the wording of
the diagnostic differs):

```
1 C0 1 C1 1 C2 1 C3 1 C4 1 C5 1 C6 1 C7 1 C8 1 C9 1 C10 1 C11 1 C12 1 C13 1 C14 1 C15
2 C0 C1 3 2 C1 C2 4 2 C2 C3 5 2 C3 C4 2 2 C4 C5 4 2 C5 C6 5 2 C6 C7 2 2 C7 C8 0
2 C9 C10 3 2 C11 C12 4 2 C12 C13 3 2 C13 C14 5 2 C14 C15 5
7 C4 7 C11 7 C2 7 C0 7 C11 7 C2 7 C0 7 C2 7 C1 7 C10 7 C13 7 C15 7 C2 7 C11 7 C5 7 C13
4 C7 6 C15 5 C5 C12 6 C0 5 C5 C2 8
```

`SIGSEGV` (status 139, empty stderr) from dereferencing a wild edge pointer,
where the translation aborts with a diagnostic instead:

```
1 C0 1 C1 1 C2 1 C3 1 C4 1 C5 1 C6 1 C7 1 C8 1 C9 1 C10 1 C11 1 C12 1 C13 1 C14 1 C15 1 C16 1 C17 1 C18
2 C0 C1 0 2 C1 C2 5 2 C3 C4 4 2 C4 C5 4 2 C6 C7 2 2 C7 C8 5 2 C8 C9 0 2 C9 C10 4
2 C10 C11 4 2 C12 C13 2 2 C13 C14 0 2 C15 C16 0 2 C17 C18 1
7 C5 7 C10 7 C7 7 C1 7 C1 7 C12 7 C1 7 C2 7 C0 7 C16 7 C9 7 C17
4 C16 5 C14 C7 6 C15 7 C0 4 C6 8
```

Predicting either would mean emulating the exact byte contents of corrupted
malloc headers, which the single-size-class model in `heap.rs` deliberately does
not attempt.
