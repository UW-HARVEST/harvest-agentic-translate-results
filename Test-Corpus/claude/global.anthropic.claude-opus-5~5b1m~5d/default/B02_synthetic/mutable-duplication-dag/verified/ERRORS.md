# Differential findings: `c_src` vs. `translation`

Both programs are compared by running them:

```sh
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .   # -> c_src/build/driver
cd translation && cargo build --release                                # -> translation/target/release/driver
cd translation && cargo test                                           # runs both and diffs them
```

`tests/differential.rs` spawns both binaries, writes the same bytes to stdin and
compares **stdout, stderr and the exit status** (exit code *and* terminating
signal) for every input listed below.

To diff a single input by hand:

```sh
printf '1\nA\n7\nA\n3\n8\n' > /tmp/in
diff <(c_src/build/driver < /tmp/in) <(translation/target/release/driver < /tmp/in)
```

On top of the cases below, ~2000 randomly generated sessions were compared the
same way: mixed and delete-heavy ones (city names including the empty one,
over-long ones and ones that look like menu choices; distances including
negative, `INT_MAX` and unparsable ones), plus sessions biased towards distances
that overflow `int`. Every one of them either matched exactly or fell into the
class of section 3, which is detected by re-running the C program and noticing
that it disagrees with itself. Those harnesses are kept as

```sh
cd translation
python3 tools/fuzz.py <seed> <rounds>           # general sessions
python3 tools/fuzz_overflow.py <seed> <rounds>  # overflowing distances
tools/sweep-overflow-shapes.sh                  # the 29 shapes of 1.5
```

They are not part of `cargo test`, because the random ones would report the
inherently irreproducible inputs of section 3 as differences on some runs.

---

## 1. Mismatches that were found and fixed

### 1.1 A freed node was still found by name

*Input:* `1 A / 7 A / 7 A / 8`, `1 A / 7 A / 4 A / 8`,
`1 A / 1 B / 7 B / 2 A B 5 / 8`, `1 A / 1 B / 2 A B 5 / 7 B / 5 A B / 8`,
`1 A / 7 A / 6 A / 8`

* C: `City 'A' not found`
* Rust (before): `Current ref count: 0` / `City: A ...` — the node was still
  found.

*Cause:* `delete_node()` `free()`s the node while `graph->nodes[]` keeps the
pointer, and `get_node_by_name()`/`add_node()` then `strcmp()` freed memory.
glibc's `free()` writes `tcache_entry { next, key }` over the **first 16 bytes**
of the chunk's user data, and those 16 bytes are the start of
`node_t::city_name`, so the name turns into garbage and never compares equal
again. `ref_count` (offset 64) and `edges`/`edge_count` (offset 72 onwards) are
*not* touched, which is why the C program keeps printing `ref_count: 0` and
keeps following the freed node's edges.

*Fix:* `Arena::free_node()` (in `src/dag_lib.rs`) overwrites the first 16 bytes
of `city_name` with a free-list-metadata placeholder (`FREE_LIST_METADATA`) and
leaves everything else alone.

### 1.1b ... but not every freed node loses its name

*Input:* ten cities, `7` on seven of them (which fills the tcache bin), then

* `7 N10 / 4 N10` — N10 is the highest chunk, so `free()` absorbs it into the
  top chunk and rewrites only the chunk *header*: C prints
  `City: N10 (ref_count: 0)`.
* `7 N8 / 7 N9 / 4 N9 / 4 N8` — N9 is merged into the run that starts at N8, so
  `fd`/`bk` land in N8's user data only: C finds N9 (`ref_count: 0`) but not N8.

Rust (before the fix) reported "not found" in both cases, because every freed
chunk was garbled.

*Fix:* `Arena::free_node()` writes the metadata only into the chunk that really
goes on a list — the chunk itself for the tcache, the *lowest* chunk of the
coalesced run for the unsorted bin, nothing at all when the run is absorbed into
the top chunk — and `malloc_node()` garbles the remainder's lowest chunk when it
splits a run. Covered by `some_freed_nodes_keep_their_name`.

### 1.2 A deleted city could not be added again, and reuse of the chunk was not modelled

*Input:* `1 A / 7 A / 1 A / 3 / 8`, `1 A / 1 B / 7 A / 7 B / 1 C / 1 D / 3 / 8`

* C: the duplicate check does not fire (see 1.1), the city is added a second
  time, and `malloc()` hands back the **chunk that was just freed** — so
  `graph->nodes[0]` and `graph->nodes[1]` are the same pointer and
  `print_graph` prints the same city twice. With two frees and two adds the
  names come back swapped (`D C C D`), because the tcache is LIFO.
* Rust (before): `Error: Node 'A' already exists` / `Failed to add city`, and
  every `add_node` created a brand new arena slot.

*Cause:* the arena never reused a slot, so a `NodeRef` could not alias the way a
recycled `node_t *` does.

*Fix:* `Arena` now models glibc 2.34's behaviour for the single size class every
`node_t` falls into (`sizeof(node_t)` = 240, so a 256-byte chunk):

* `free()` pushes the chunk on the tcache bin (LIFO) while it holds fewer than
  `TCACHE_COUNT` = 7 chunks, so `malloc()` returns the most recently freed chunk;
* with a full tcache bin the chunk is coalesced with the adjacent free runs (a
  chunk sitting in the tcache still looks allocated and never coalesces) and is
  then either absorbed into the top chunk, if it borders it, or appended to the
  unsorted bin;
* `malloc()` with an empty tcache bin empties the unsorted bin oldest first,
  moving equally sized chunks into the tcache and sorting longer runs into their
  size bin, and returns the last chunk it cached — so those chunks come back
  **newest freed first**;
* a run that is longer than one chunk is taken from its bin (shortest first) and
  split: the caller gets its lowest chunk, the rest goes back to the unsorted
  bin;
* chunks that are still inside the top chunk are carved off again in ascending
  address order.

Every rule was checked against the C program: 9 nodes freed ascending, 9 freed
descending, 16 freed and re-added, interleaved free/alloc, and two isolated
overflow chunks freed in either order — see
`freed_chunks_are_reused_most_recent_first`,
`freed_chunks_beyond_the_tcache_limit` and
`overflowing_the_tcache_hands_chunks_back_newest_first`.

### 1.2b Reuse order beyond the tcache limit

*Input:* twelve cities, `7` on seven of them, then `7 N9 / 7 N11` (or
`7 N11 / 7 N9`), then nine more `1` commands and `3`.

An intermediate version of the fix handed overflow chunks back in ascending
address order, which matches when they get coalesced but not when they are
isolated: C reuses N11 before N9 when N11 was freed *last*, because `malloc`
moves the whole unsorted bin into the tcache oldest first and then pops it.
Fixed by modelling the unsorted-bin walk itself (see above).

### 1.3 A double free must kill the process

*Input:* `1 A / 1 B / 2 A B 5 / 7 B / 6 A / 8`

* C: `stderr` = `free(): double free detected in tcache 2`, killed by SIGABRT
  (status 134), and **stdout is empty** because the buffered output was never
  flushed.
* Rust (before): exited 0 after printing everything.

*Cause:* `7 B` frees B (`ref_count` 1 → 0). `shallow_copy(A)` walks A's edges
and revives the freed B (`ref_count` 0 → 1) — a write to freed memory that
glibc does not notice because `ref_count` is outside the free-list metadata.
`free_graph()` at exit then decrements B to 0 and frees it a second time.

*Fix:* `Arena::free_node()` detects a free of a chunk that is already on a free
list and calls `cio::malloc_printerr()`, which writes glibc's diagnostic to fd 2
and `abort()`s. Because `COut` keeps its own 4096-byte buffer and `abort()`
neither flushes nor runs destructors, the buffered stdout is discarded exactly
as in C.

The message depends on *where* the chunk sits, and all three variants the
program can produce are reproduced (`Arena::report_double_free`):

| situation | stderr |
|---|---|
| chunk in the tcache bin | `free(): double free detected in tcache 2` |
| chunk absorbed into the top chunk | `double free or corruption (out)` |
| chunk on the unsorted bin / a size bin | `double free or corruption (!prev)` |

The last two are reached by filling the tcache with seven frees first (see
`double_free_aborts_the_process`); there the C program has already written one
full 4096-byte block, so stdout is 4096 bytes rather than empty.

### 1.4 An overflowing distance sum can make the `previous` chain cyclic

*Input:* `1 A / 1 B / 1 C / 2 A B 2000000000 / 2 B C 2000000000 /
2 B A 2000000000 / 5 A C / 8`

* C: dies from a signal (SIGSEGV, occasionally SIGBUS) with empty stderr and
  only the already-written 4096-byte stdout blocks.
* Rust (before): looped for ever, growing a `Vec` (killed by `timeout`).

*Cause:* `find_shortest_path()` relaxes *every* neighbour, including nodes that
are already `visited`. With non-negative weights that never lowers a finalised
distance — but `state[i].distance + edge->distance` overflows `int` here and
becomes negative, so an already visited node gets a new `previous`. That makes
the `previous` chain cyclic (`A->B->A`), and the reconstruction loop
`path[count++] = current_node` runs past the end of the
`node_t *path[MAX_NODES]` **stack** array. Here the writes end up destroying the
frame and the process dies.

*Fix:* `cio::stack_smash()`, called from the reconstruction loop once the writes
have gone past the whole `state` array (see 1.5). It restores the default
`SIGSEGV` disposition — the Rust runtime installs a handler for stack overflow
reporting that would otherwise swallow the signal — and raises `SIGSEGV` without
flushing stdout.

### 1.5 ... but usually the overrun does *not* crash: it lands in `state`

*Input:* `1 A / 1 B / 1 C / 1 D / 2 A B 1 / 2 B C 1 / 2 C B 2147483647 /
2 C D 1 / 5 A D / 8`

* C: exit **0**, 3400 bytes of stdout, printing a **101 entry** path
  (`1. B`, `2. C`, `3. B`, …, `100. C`, `101. D`).
* Rust (after 1.4, before this fix): SIGSEGV with an empty stdout.

*Cause:* gcc lays the frame of `find_shortest_path` out with `path` directly
below `state`, so `&path[MAX_NODES] == &state[0]` and the overrun writes into the
state array one 8-byte word at a time:

| write | lands in |
|---|---|
| `path[100 + 4*s + 0]` | `state[s].node` |
| `path[100 + 4*s + 1]` | `state[s].distance` (+ padding) |
| `path[100 + 4*s + 2]` | `state[s].previous` |
| `path[100 + 4*s + 3]` | `state[s].visited` (+ padding) |

The very first of those writes puts the current node into `state[0].node`, so the
loop's own lookup finds it at index 0 and takes `state[0].previous` — which is
`NULL`, because `state[0]` is the start node. The loop stops with `count == 101`,
`main` prints 101 entries and the program carries on normally. Only if
`state[0].previous` was itself rewritten does the walk continue into
`state[0].distance`, `state[0].previous`, `state[1]`… — that gives 107, 111 or
110 entries for the chains in `path_reconstruction_overruns_into_the_dijkstra_state`
— and only if it never settles down does it reach past `state[99]` and kill the
process.

*Fix:* `find_shortest_path()` now uses a fixed `[DijkstraNode; MAX_NODES]` plus a
separate `state_count`, exactly like the C (the overrun writes into slots beyond
`state_count` too), and the reconstruction loop performs the aliased write
instead of crashing. Writes to `distance`/`visited` are dropped, because nothing
reads them again; `path` keeps every written value, because that is what the
reversal reads back out of the same words. `stack_smash()` is called only once
the writes are past `state[MAX_NODES - 1]`.

Validated by sweeping "chain of `k` cities + one overflowing back-edge into city
`j`" for `k` in 3..10 and every `j`: 29 shapes, all matching, including the ones
that exit 0 with 101/107/110/111 entries and the four that die.

A legitimate path of exactly `MAX_NODES` nodes must of course still be printed,
which the `100 city chain` case in `hundred_node_graph` pins down.

---

## 2. Behaviour that was already correct and is now covered by tests

These were verified rather than fixed, but they are the parts most likely to
drift, so they are listed for the next reader:

* `fgets(buf, 256, stdin)` stops after 255 bytes, so an over-long line is split
  and its tail is read as the *next* answer; `sscanf("%d")` in contrast skips
  leading whitespace, accepts a sign and ignores trailing junk.
* glibc's `%d` converts with `strtol` (saturating at `LONG_MAX`/`LONG_MIN`) and
  then *truncates* to `int`: `2147483648` becomes `-2147483648` (rejected as a
  negative distance), `4294967304` becomes `8` (a valid menu choice) and
  `99999999999999999999` becomes `-1`.
* `strncpy(name, city, 63)` + `name[63] = 0` truncates a city name to 63 bytes,
  while the duplicate check compares against the *untruncated* input — so two
  names that differ only after byte 63 both get added.
* `input[strcspn(input, "\n")] = 0` also stops at a NUL, so a name containing a
  NUL byte is cut there; names are raw bytes and may be invalid UTF-8.
* An edge of length `INT_MAX` is indistinguishable from "unreachable", because
  `INT_MAX` is the sentinel the algorithm starts from and the relaxation uses a
  strict `<`.
* Ties in "next unvisited node with minimum distance" go to the **first**
  candidate found (`<`, not `<=`).
* `EOF` at any sub-prompt only breaks out of the `switch`; the menu is printed
  once more before the outer loop ends.
* stdout is fully buffered (4096-byte blocks) and stderr is unbuffered, which is
  what makes stdout empty vs. 4096/8192 bytes long in the crash cases above.
* `print_graph()` iterates `graph->nodes[]`, so a node that was deleted (and a
  node that was added twice) is printed once per entry.

---

## 3. Inputs whose C behaviour is not reproducible

### 3.1 Printing a node whose name was overwritten by free list metadata

*Input:* `1 A / 7 A / 3 / 8` (and any other input that prints such a node, e.g.
`print_node` following an edge into one).

This is only about the chunks that really received the metadata (see 1.1b — a
node absorbed into the top chunk or merged into a run keeps its name and is
compared byte for byte like any other). `city_name` then contains glibc's
`tcache_entry::next`, which is `PROTECT_PTR(NULL)`, i.e. `&next >> 12` — a
**heap address**. With ASLR the C program prints different bytes on every run:

```
City: \x0e\xc8\x03 (ref_count: 0)
City: \xaa\x60 (ref_count: 0)
City: C\xce\x02 (ref_count: 0)
```

No translation can match that byte for byte. The Rust program produces garbage
of the same shape (`FREE_LIST_METADATA`, NUL-terminated inside the first 8
bytes) so that everything *around* it — the `ref_count: 0`, the `Edges:` list
and every later lookup — still matches.

`printing_a_freed_node_is_not_reproducible_in_c` asserts this explicitly: it
runs the C program six times and checks that stderr, the exit status, the output
before the name and the whole tail after it are identical to the Rust program's.
It also requires that the six C runs disagree with each other whenever
`/proc/sys/kernel/randomize_va_space` is not `0`, so that the test fails — rather
than silently passing — if this output ever turns out to be reproducible on a
machine with ASLR enabled.

### 3.2 Which signal the smashed stack produces (see 1.4/1.5)

For the inputs where the overrun really does destroy the frame, stdout, stderr
and "died from a signal" are stable, but the C program dies from SIGSEGV in
roughly four runs out of five and from SIGBUS otherwise, depending on what the
runaway writes clobber first. The Rust program always raises SIGSEGV, the common
case; `distance_overflow_makes_the_previous_chain_cyclic` accepts either signal
from the C side and requires identical output.

Note that this is *only* about the inputs of 1.4. The much more common overrun of
1.5 ends with a normal exit 0 and a fully deterministic 101-entry path, and that
is compared byte for byte.

---

## 4. Branches that no input can reach in either program

The C contains 57 distinct `printf`/`fprintf` format strings. Every fixed piece
of 48 of them appears verbatim in the Rust sources; the nine that do not are
exactly the ones listed below, which nothing can reach.

Recorded so the absence of tests is not mistaken for a gap:

* every `if (!ptr)` guard in `lib.c` (`add_node`, `add_edge`, `shallow_copy`,
  `find_shortest_path`, `get_node_by_name`, `delete_node`, `free_graph`) —
  `main()` never passes a null pointer, so the `Error: NULL ... ` messages are
  dead code. The translation encodes non-nullness in its types instead.
* `printf("NULL node\n")` / `printf("NULL graph\n")` in `print_node`/
  `print_graph`, for the same reason.
* every `malloc` failure path (`Failed to create graph`, `Error: Failed to
  allocate node/graph/path`) — not reachable without an allocation failure.
* `shallow_copy()` returning `NULL`, hence `Failed to create shallow copy` in
  case 6.
* `current_idx == -1` in the Dijkstra loop and `current_state_idx == -1` in the
  reconstruction loop: both look the node up in the array it was taken from.
* the `state_count < MAX_NODES` limit on adding a neighbour, and the
  `*visited_count < MAX_NODES` limit in `increment_refs_recursive`: `add_node`
  caps the graph at `MAX_NODES` nodes, so neither array can overflow.

## 5. The one allocation the arena deliberately does not model

`find_shortest_path` allocates `sizeof(node_t *) * count` for its result and
`main` frees it again right after printing. For `count` 30 or 31 that request
lands in the **same size class as `node_t`** (`sizeof(node_t)` is 240, so both
round up to a 256-byte chunk), so such a path can take a chunk off the node free
list. The arena does not model this allocation, because it cannot be observed:

* `malloc` then `free` with no `add_city` in between is LIFO, so a chunk taken
  from the tcache goes straight back on top of it;
* with an empty tcache the chunk comes from the lowest overflow chunk (or from
  the top) and is pushed onto the empty tcache, which is exactly the chunk the
  next `add_city` would have been given anyway;
* the resulting addresses are only observable through pointer *identity*, which
  a `NodeRef` reproduces.

Two sessions that force this — a 30-city path taken with a drained tcache and
nine chunks on the overflow list, and the same with a full tcache — are part of
`freed_chunks_and_the_shortest_path_allocation` and match byte for byte.
