# Differential verification: mismatches found and their causes

The C program in `c_src/` is the ground truth. The Rust program in `translation/`
must produce byte-identical stdout, byte-identical stderr and the same exit
status. Everything below was found by building both and running them as
subprocesses (`translation/tests/differential.rs`).

## How the two programs are driven

`driver` takes no arguments and reads no input; it is a fixed self-checking test
program. So the enumerated input classes are not stdin payloads but the branches
inside `tree.c` / `hashmap.c`, plus the ways a shell can wire up the process
(separate streams, merged streams, a pipe, a dead pipe, a closed descriptor, a
full device, a different locale).

| Program | C | Rust |
|---|---|---|
| `driver` (graded) | `c_src/src/main.c` + library | `translation/src/main.rs` |
| `probe` (coverage) | `translation/tests/cprobe/probe.c` + library | `translation/src/bin/probe.rs` |

Run commands:

```
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .   # -> c_src/build/driver
cd translation && cargo build --release                                # -> translation/target/release/driver
cd translation && cargo test                                           # differential suite
```

`probe` exists because `main.c` reaches only a small part of the library: two of
the five reachable `fprintf(stderr, ...)` messages, and never `(empty tree)`,
`hashmap_clear`, `data == NULL`, `strncpy` truncation, tombstone reuse or the
`tree_find_path` length clamps. It is a second driver over the *same*
`c_src/src/*.c` sources, compiled outside `c_src`, selected by `argv[1]`, and
compared the same way. Nothing links the Rust code as a library.

---

## Mismatch 1 — exit status on a dead stdout pipe (translation defect, FIXED)

**Symptom.** With a pipe whose reader is gone, the two programs disagreed on how
they terminate:

```
$ ./c_src/build/driver | true            ; echo ${PIPESTATUS[0]}   ->  141
$ ./translation/target/release/driver | true ; echo ${PIPESTATUS[0]} ->  0
```

Directly, with the read end closed before the write: C returned `-13` (killed by
signal 13), Rust returned `0`.

**Cause.** The Rust standard library installs `SIG_IGN` for `SIGPIPE` before
`main` runs. The write to the dead pipe therefore returned `EPIPE` instead of
killing the process; `cstdio::write_through` discards write errors (which is
correct — glibc's `exit` ignores them too), so the program ran to a clean `0`.
The C program keeps the default disposition and is killed by the signal.

Every stdout byte in this program is written by a single flush at exit, so the
visible output was identical; only the exit status differed. A test that checked
stdout alone would have passed.

**Fix.** `translation/src/main.rs` (and `src/bin/probe.rs`) restore the default
disposition as the first statement of `main`:

```rust
fn restore_default_sigpipe() {
    const SIGPIPE: i32 = 13;
    const SIG_DFL: usize = 0;
    extern "C" { fn signal(signum: i32, handler: usize) -> usize; }
    unsafe { signal(SIGPIPE, SIG_DFL); }
}
```

Covered by `sigpipe_exit_status_matches` and
`sigpipe_signal_matches_without_a_shell`.

---

## Mismatch 2 — the C reference built with `NDEBUG` (measurement error, FIXED)

**Symptom.** On the first comparison the C program printed `(empty tree)` where
Rust printed a ten-node tree, and produced *no* stderr where Rust produced two
error lines. Ten stdout lines and both stderr lines "differed".

**Cause.** Not the translation. I had configured the C build with
`cmake .. -DCMAKE_BUILD_TYPE=Release`, which adds `-DNDEBUG`. `assert(...)` then
expands to nothing — and `main.c` performs nearly all of its work *inside*
asserts:

```c
assert(tree_add_node(tree, 1, 0, "root") == 0);
```

With `NDEBUG` those calls are deleted outright. No tree is ever built, so
`tree_print` reports `(empty tree)` and no duplicate-id or max-children error is
ever provoked. The C binary was a hollow shell and the comparison measured
nothing.

**Fix.** Build the C side exactly as the task specifies — `cmake ..` with no
build type, which leaves `C_FLAGS` empty and asserts live. The test suite
compiles the C sources itself with `cc -std=c11` and no `-DNDEBUG`, and
`driver_emits_the_expected_error_lines` pins the trap: it asserts the C stderr is
exactly the two expected lines and that stdout contains `[10] ggc1`. An `NDEBUG`
build fails that test instead of passing vacuously.

---

## Mismatch 3 — unspecified `printf` argument order in the probe (harness defect, FIXED)

**Symptom.** The `deep_recursion` scenario differed on one line:

```
C:    remove(1)=0 size=2499 has_root=1
Rust: remove(1)=0 size=0    has_root=0
```

**Cause.** My probe, not the translation. The C probe had written:

```c
printf("remove(1)=%d size=%zu has_root=%d\n",
       tree_remove_node(tree, 1), tree_size(tree), tree->has_root);
```

C does not specify the evaluation order of function arguments; gcc on x86-64
evaluates right-to-left, so `tree_size()` and `has_root` were read *before* the
removal ran. Rust's argument evaluation is defined left-to-right, so it reported
the state *after* the removal. Both programs were behaving correctly; the test
was asking an unanswerable question.

**Fix.** Sequence the mutating call into a local before formatting, in both
probes. The three `hashmap_remove` calls that shared one `printf` were sequenced
too — their result happened to be order-independent, but the fragility was the
same.

---

## Input classes enumerated and verified identical

Each name is a `probe` scenario; all are compared on stdout, stderr and exit
status, with the streams captured separately, merged into one file, and merged
onto one pipe.

| Scenario | C branch reached |
|---|---|
| `empty_print` | `tree_print` with `has_root == 0` → `(empty tree)`; queries on an empty tree |
| `null_data` | `tree_add_node` with `data == NULL` → `node->data[0] = '\0'`; `printf("[%lu] %s\n", ...)` with an empty string, which leaves a trailing space |
| `parent_missing` | `Error: Parent node %lu not found` (never reached by `main.c`) |
| `remove_missing` | `Error: Node %lu not found` (never reached by `main.c`), on an empty tree and on a populated one |
| `queries_missing` | the `-1` returns of `tree_get_depth`, `tree_get_height`, `tree_count_descendants`, `tree_find_path` |
| `find_path_clamp` | `if (length > max_length) length = max_length;` — the clamp happens *after* the path is built, so `max_length=3` on a 5-deep chain returns `[3,4,5]`, not `[1,2,3]`. Also `max_length` of 0 and −1 (which returns −1 and writes nothing) |
| `find_path_deep` | the `while (length < 1000)` bound: a 1200-deep chain stops before reaching the root |
| `data_trunc` | `strncpy(node->data, data, MAX_DATA_LENGTH - 1)` at 254, 255 and 300 bytes, the empty string, and non-UTF-8 bytes plus `%s`/`%d`/`%%` in the data |
| `hashmap_reuse` | the tombstone-reuse arm of `hashmap_put`, the update-existing arm, and `hashmap_remove` of an absent key |
| `hashmap_null_value` | a stored `NULL` value: the slot is occupied, yet `hashmap_contains` reports 0 and `hashmap_remove` returns `NULL` while still decrementing `size` |
| `hashmap_clear` | `hashmap_clear`, which `main.c` never calls, plus reuse afterwards |
| `hashmap_resize` | repeated `hashmap_resize` across 300 keys with interleaved removals, so `deleted_count` contributes to the load factor and is reset by the rehash. The full slot table is dumped, which pins the FNV-1a hash, the byte order of the key, and the linear-probing order |
| `big_ids` | `%lu` at `UINT64_MAX`, at `2^63`, and at 0 |
| `zero_root` | `root_id == 0`, indistinguishable from "no parent" |
| `remove_root_readd` | `has_root`/`root_id` reset after removing the root; the next add becomes root and has its requested parent silently replaced by 0; previously removed ids re-added over tombstones |
| `child_shift` | the `child_ids` shifting loop in `tree_remove_node`, removing first, middle and last child, then refilling |
| `max_children` | `Error: Parent has maximum children` at the `MAX_CHILDREN` boundary, and that freeing one slot admits exactly one more |
| `subtree_removal` | recursive `tree_remove_subtree` over a wide, deep tree |
| `dup_and_reinsert` | `Error: Node with ID %lu already exists` at the root and deeper |
| `interleaved` | stdout/stderr ordering around an explicit `fflush(stdout)` |
| `deep_recursion` | `tree_get_height`, `tree_count_descendants` and `tree_remove_subtree` recursing 5000 deep |

Process-level classes, all verified identical: no arguments and extra arguments
(both ignored, as `main(void)` takes none); stdin left unread; streams separate,
merged to a file, and merged to a pipe; stdout on a pipe with no reader; stdout
closed (`>&-`, `EBADF`); stdout on `/dev/full` (`ENOSPC`, status stays 0); a
controlling terminal, where glibc switches stdout to line buffering (checked with
`script -qec`, outside `cargo test` because it needs util-linux); `LC_ALL` set to
`C`, `C.UTF-8`, `en_US.UTF-8`, `tr_TR.UTF-8`, `de_DE.UTF-8`; and repeated runs,
which must be bit-identical.

### Buffering, which the merged-stream cases depend on

glibc chooses stdout's mode on first use: line-buffered on a terminal, otherwise
block-buffered, while stderr is unbuffered. With `2>&1` the error lines therefore
appear *ahead* of stdout text that was printed before them. `src/cstdio.rs`
reproduces this with its own 4096-byte buffer, an `is_terminal()` check, and a
flush at exit; `println!` would have been line-buffered always and would have
reordered the streams. This was already correct in the translation — no mismatch
was found here, but it is what the `*_with_merged_streams` tests exist to hold in
place, and `hashmap_resize` (11 KB of stdout) exercises the buffer-full flush
path rather than only the flush at exit.

---

## Branches that cannot be reached from either program

Recorded so the next reader does not look for tests that cannot exist.

- **Allocation-failure paths.** `hashmap_create`, `hashmap_resize`,
  `tree_create` and `tree_add_node` all have `if (!p) return ...` arms for a
  failed `malloc`/`calloc`. The Rust side uses infallible allocation (`Vec`,
  owned values), where the equivalent condition aborts. Unreachable without an
  allocator fault injector; the surrounding success paths are covered.
- **`NULL` receiver guards.** `hashmap_put/get/remove/size/clear`,
  `hashmap_destroy`, `tree_delete`, `tree_add_node`, `tree_remove_node`,
  `tree_get_node` and `tree_find_path` each begin with a `!map` / `!tree` /
  `!path` check. No caller in either program passes `NULL`, and the Rust
  translation takes `&self`/`&mut self`, which cannot be null. Structurally
  absent rather than untested.
- **`hashmap_put` returning −1 for a full table.** Guarded by the resize on
  entry; the comment in `hashmap.c` says as much. Reaching it needs a failed
  resize, i.e. the allocation-failure case above. `Error: Failed to add node to
  hashmap` in `tree_add_node` is downstream of it and equally unreachable.
- **`tree_remove_subtree` returning −1.** Requires a `child_ids` entry naming a
  node that is not in the map. Every id in `child_ids` is inserted by
  `tree_add_node` and removed only by the same recursion, so the list cannot go
  stale while it is walked.
- **`tree_get_depth` returning −1 from inside its loop.** Requires a node whose
  ancestor chain is broken before it reaches `root_id`. Removals always take the
  whole subtree, so no node can outlive its parent.
- **Stack exhaustion.** `tree_get_height`, `tree_count_descendants` and
  `tree_remove_subtree` recurse once per level. Verified equal at 5000 levels.
  Deep enough to overflow, the two would differ in kind — C faults, Rust prints
  a stack-overflow message and aborts — but the graded `driver` never exceeds
  depth 4, and nothing in it can be driven deeper.

## Deliberate representation differences, none observable

- The C code stores `void *` node pointers in the hashmap; Rust stores `usize`
  indices into an arena, with `Option::None` for `NULL`. `hashmap_contains` is
  `get(key) != NULL` in C and `get(key).is_some()` in Rust, so the
  stored-`NULL`-value quirk survives — verified by `hashmap_null_value`.
- `malloc` leaves `tree_node_t` indeterminate; Rust zeroes it. Every field read
  by this program is assigned before use, and `data` is fully defined either way
  because `strncpy` zero-pads to `n` and byte 255 is set explicitly.
- Freed arena slots are never reused, matching the fact that this program never
  hands a freed pointer back out. Freed nodes are unreachable through the map,
  whose entry is marked deleted, so no use-after-free is observable in C either.
- `tree_delete` keeps the C walk order even though nothing is printed.

## Guard rails on the suite itself

A comparison suite that cannot fail proves nothing, so two tests check the
harness rather than the translation:

- `harness_detects_a_planted_difference` builds a copy of `tree.c` with
  `(empty tree)` altered (written to the target directory, never into `c_src`)
  and asserts the comparison notices.
- `probe_scenarios_are_not_vacuous` asserts every scenario emits real output and
  exits 0, and that the error-path scenarios actually write to stderr.

The suite was additionally mutation-tested by breaking the Rust side one change
at a time and confirming a failure each time: removing the `SIGPIPE` fix
(caught by both sigpipe tests), altering the `(empty tree)` literal, "fixing" the
`find_path` truncate-then-reverse quirk, changing the `strncpy` limit by one,
perturbing the FNV-1a multiplier (slot layout only — no user-visible output
changes, caught by the table dump), and dropping the `deleted_count` decrement on
tombstone reuse. All six were detected; the sources were then restored and
verified byte-identical.

`c_src/` is only ever read. The C probe lives in `translation/tests/cprobe/`, and
all C build products go to the Cargo target directory or `c_src/build/` (the
directory the task's own build command creates).
