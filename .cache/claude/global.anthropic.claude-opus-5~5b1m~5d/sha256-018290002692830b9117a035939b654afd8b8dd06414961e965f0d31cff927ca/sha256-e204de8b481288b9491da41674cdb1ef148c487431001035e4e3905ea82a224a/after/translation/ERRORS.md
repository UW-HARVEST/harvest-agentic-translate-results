# Differential-testing report: `c_src` vs `translation`

Ground truth is `c_src`. Every finding below was reproduced by running both
executables and comparing stdout, stderr and exit status byte for byte.

## How each program is run

```
# C
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .
./c_src/build/driver

# Rust
cd translation && cargo build --release
./translation/target/release/driver
```

Both build with **zero** errors and zero warnings.

## Shape of the input space

`c_src/src/main.c` is `int main(void)` and never reads `stdin`. There is no
`scanf`, no `fgets`, no `getenv`, no `argv` use anywhere in the three C files.
The program is a fixed self-checking test driver, so its output is a single
constant triple: 1499 bytes of stdout, 72 bytes of stderr, exit code 0.

That means the executable's *input classes* are process-level conditions, not
records: argv, whatever sits unread on stdin, the environment, the working
directory, and the state of the stdout/stderr descriptors. `tests/differential.rs`
covers all of them (24 tests).

The data-dependent branches inside `tree.c` / `hashmap.c` cannot be steered by
any invocation of the shipped binary, because `main()` hard-codes the call
sequence. They are covered instead by `tests/branch_coverage.rs` (5 tests), which
builds a second driver, `tests/probe/probe.c`, linked against the **pristine**
`c_src/src/tree.c` and `c_src/src/hashmap.c`, plus a mirror-image
`tests/probe/probe.rs` compiled against `src/cio.rs`, `src/hashmap.rs`,
`src/tree.rs`. Both probes are run as subprocesses and diffed the same way.
Nothing in `c_src/` was modified or copied over.

---

## Mismatch 1 — `SIGPIPE` was ignored, so the exit status was wrong

**Found by:** `stdout_reader_gone_dies_by_sigpipe`,
`stderr_reader_gone_dies_by_sigpipe`.

**Symptom.** With the read end of the stdout pipe closed before the program
writes (`./driver | true`):

| | stdout | stderr | status |
|---|---|---|---|
| C | empty | empty | **killed by signal 13** (shell reports 141) |
| Rust (before) | empty | empty | **exited 0** |

The same divergence happened when the *stderr* reader went away, where the C
program dies mid-run at the first `fprintf(stderr, ...)` in
`test_tree_duplicate_id`.

**Cause.** The Rust standard library sets `SIGPIPE` to `SIG_IGN` before `main`
runs. A C program keeps the default disposition, so a write to a pipe with no
reader terminates it with signal 13. In Rust the failed write instead returned
`EPIPE`, which `cio` discards, and the program ran on to a normal `exit(0)`.

**Fix.** `cio::restore_default_sigpipe()` calls `signal(SIGPIPE, SIG_DFL)`
(declared `extern "C"`; libc is already linked, so no new dependency) as the
first statement of `main`, before any output.

---

## Mismatch 2 — stdout was buffered without bound, so mid-run flushes were missing

**Found by:** `probes_agree_under_stream_error_conditions_too`, then pinned down
by `probes_agree_on_buffer_boundary_flush_timing`.

**Symptom.** The branch probe writes ~14 KiB to stdout. With the stdout reader
closed:

| | stderr | status |
|---|---|---|
| C | **0 bytes** — died before the first error message | signal 13 |
| Rust (before) | **246 bytes** — ran all the way to the end | signal 13 |

**Cause.** `cio::StdoutBuf` accumulated *everything* and flushed once, at exit.
glibc gives a fully buffered stream a fixed-size buffer and flushes it whenever
it fills, i.e. repeatedly, mid-run. The C program therefore hit its first
`write(2)` — and its fatal `SIGPIPE` — after only ~4 KiB of output, long before
it reached any `fprintf(stderr, ...)`. The Rust program never wrote to stdout
until exit, so it completed the whole run first.

The same root cause also changes `2>&1` interleaving for any run whose stdout
exceeds one buffer: unbuffered stderr lands between stdout blocks, not before
all of them.

**Fix.** `cio` now models glibc:

* buffer size follows `_IO_file_doallocate`: `BUFSIZ` (8192), reduced to the
  descriptor's `st_blksize` when that is smaller — 4096 for a pipe and for a
  regular file here. If `fstat` fails (e.g. fd 1 closed), glibc keeps `BUFSIZ`,
  and so do we.
* flush timing follows `_IO_new_file_xsputn`: the buffer is flushed only when
  incoming bytes do **not** fit in the remaining space, so a write that fills it
  *exactly* does not flush until the next call. This one-`printf` lag is
  observable, and reproducing it is what makes the byte positions line up.

**Why the happy path never caught this.** The shipped `driver` emits 1499 bytes
of stdout — under the 4096-byte buffer — so it never flushes mid-run and all 24
black-box tests passed both before and after the fix. Only the larger-output
probe exposed it. Verified after the fix: the merged `2>&1` capture of the
14 KiB probe, which crosses the buffer boundary three times, is byte-identical.

---

## Deliberate, verified-equivalent behaviours (not defects)

These looked suspicious while reading the C and were each confirmed to match:

* **`hashmap_contains` treats a NULL value as absent.** `hashmap_contains` is
  `hashmap_get(...) != NULL`, so a key stored with a NULL value counts toward
  `size` yet reports `contains == 0` and `get == NULL`. Reproduced in the probe
  (`put(100, NULL)=0` … `get(100)=(null) contains=0`). `Option<V> == None`
  models the NULL pointer, so the quirk carries over for free.
* **`hashmap_remove` returns NULL for a NULL-valued key it did find**, and still
  increments `deleted_count`. Matches.
* **Validation order in `tree_add_node`.** The duplicate-id check runs *before*
  the parent lookup, so adding a duplicate id with a bogus parent reports
  `already exists`, not `Parent node ... not found`. Probe covers both orders,
  including a duplicate on an already-full parent.
* **The first node becomes the root regardless of `parent_id`**, and its
  `parent_id` is overwritten with 0. Probe adds a root with `parent_id = 12345`.
* **`strncpy` truncation.** `strncpy(node->data, data, 255)` + explicit NUL at
  index 255 truncates a 399-byte string to 255 bytes. Probe checks 0, 255, 256
  and 399-byte inputs; all report `datalen=255` where expected.
* **`tree_find_path` truncation keeps the *end* of the path.** The C builds the
  walk node-to-root in `temp_path`, clamps `length` to `max_length`, then
  reverses only that prefix — so a truncated result is the `max_length` nodes
  *closest to the target*, not the ones closest to the root. Probe checks
  `max_length` = 0, 1, 3, 10, 64.
* **The `temp_path[1000]` loop cap.** With a 1010-deep chain the loop exits on
  `length < 1000` failing rather than on reaching the root, so the returned path
  never contains the root. Probe covers it.
* **`%lu` on `uint64_t`.** LP64, so `%lu` and Rust's `{}` agree, including
  `0` and `18446744073709551615`.
* **Tombstone-aware resizing.** `should_resize` counts `size + deleted_count`,
  and `hashmap_resize` drops tombstones and rebuilds. Because the Rust port
  keeps the identical FNV-1a hash (over native-endian key bytes) and identical
  linear probing, slot assignment after a resize matches too — which the probe's
  `size`/`capacity`/`deleted_count` dumps confirm at every step.
* **Two-space-per-level indentation** in `tree_print`, and `(empty tree)` when
  `has_root == 0`.
* **Unused `TEST_FAIL` macro** in `main.c` is genuinely dead; no FAIL line is
  ever printed by either program.

## Branches that no input can reach

The following C branches are unreachable through the executable's interface and
are documented rather than tested:

* Every `if (!ptr)` / OOM guard: `malloc`/`calloc` failure in `tree_create`,
  `hashmap_create`, `hashmap_resize`, `tree_add_node`; and the NULL-argument
  guards in `hashmap_put/get/remove/size/destroy`, `tree_add_node`,
  `tree_remove_node`, `tree_get_node`, `tree_size`, `tree_print`,
  `tree_get_depth`, `tree_find_path`. `main()` never passes NULL, and the Rust
  translation takes `&self`/`&mut self`, so these states cannot be constructed.
* `hashmap_put`'s final `return -1` ("map is full") — resizing keeps the load
  factor below 1, so the probe loop always terminates early.
* `hashmap_clear` is never called from `main.c`. The probe calls it directly.
* **`assert()` failure output.** `c_assert!` prints a glibc-shaped diagnostic and
  calls `abort()`, but the line number and stringified expression are the Rust
  ones, so the text would not match glibc byte for byte. This is unreachable by
  construction: every assertion holds in both programs, which the suite pins
  down by requiring all 14 `✓ PASS:` lines, zero `✗ FAIL`, the
  `All tests passed successfully!` trailer, and exit code 0.

## Supporting change with no behavioural effect

* `Hashmap::put_value` was made `pub` so the probe can store a NULL value, which
  is what C's `void *value` allows. `put` still wraps in `Some`; no call site in
  `driver` changed.

## Status

```
cargo test            #  0 unit + 5 branch_coverage + 24 differential = 29 passed
cargo test --release  #  29 passed
```

29 tests, 0 failed, 0 ignored, none skipped or disabled. `c_src/` is unmodified
(source mtimes untouched; only the instructed `c_src/build/` output directory was
created).
