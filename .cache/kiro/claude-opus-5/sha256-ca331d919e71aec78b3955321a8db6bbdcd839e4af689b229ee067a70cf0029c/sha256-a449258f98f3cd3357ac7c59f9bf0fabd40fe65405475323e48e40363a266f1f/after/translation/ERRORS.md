# Differential verification: `c_src` (ground truth) vs `translation`

Every finding below was reached by building both programs and diffing what they
actually produce — stdout, stderr and the full wait status — never by loading the
Rust code as a library.

## How the two programs are built and run

```
# C (ground truth)
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .
./driver                       # -> c_src/build/driver

# Rust
cd translation && cargo build --release
./target/release/driver
```

The C build type is deliberately left unset. See "Build-configuration hazard"
below: it is not a cosmetic choice.

## What "input" means here

`c_src/src/main.c` declares `int main(void)` and never reads `stdin`, `argv`, the
environment, the clock or an RNG. Grepping the C source for
`scanf|fgets|getchar|read(|argv|argc|getenv|stdin|time(|rand|clock|srand` returns
nothing but three unrelated `"grandchild1"`/`"grandchild2"` string literals and
one `#include <stdint.h>`.

So this executable is a fixed test driver. All of its data-dependent branch
classes — empty tree, single node, sibling children, deep chain, `MAX_CHILDREN`
saturation, duplicate id, hashmap collision plus resize, leaf removal, interior
subtree removal, root removal, depth/height/descendant queries, path finding —
are compiled in and exercised on *every* run. There is no stdin-driven input
space to enumerate.

The axes that genuinely vary between invocations are process-level:

| Axis | Cases covered in `tests/differential.rs` |
| --- | --- |
| stdin contents | empty, closed fd, single line, single item with no trailing newline, blank lines, whitespace only, embedded NULs, all 256 byte values, negative and oversized numbers, a 100 kB line, a ~1.1 MB payload, and stdin as a pipe rather than a file |
| argv | none, one empty string, `-h`, `--help`, `0`, three args, `--` plus negatives, non-ASCII args, 500 args, a 60 kB arg |
| environment | inherited, completely empty, `LC_ALL=C`, `POSIX`, `C.UTF-8`, `en_US.UTF-8`, an ISO-8859-1 `LANG`, `TERM=dumb`, colour variables |
| working directory | `/`, `/tmp`, the crate dir, `c_src` |
| stdout/stderr wiring | separate pipes, separate regular files, both merged into one file, both merged into one pipe, `/dev/null`, `/dev/full`, append-mode file with pre-existing content, closed stdout, closed stderr, both closed, pseudo-terminal |
| pipe readers | full reader (`cat`), partial readers (`head -c 1`, `head -c 100`, `head -n 1`), `wc`, and a reader that is already gone |
| repetition | 5 sequential runs, 8 concurrent runs |
| build profile | Rust `test` profile and Rust `--release` profile, both against the same C binary |

Branch coverage of the C source was measured, not guessed. A `--coverage` build
in `/tmp` (the `c_src` tree was not touched) gives:

```
main.c      lines 100.00% of 247   branches executed 100.00% of 258
tree.c      lines  82.74% of 168   branches executed 100.00% of 100
hashmap.c   lines  74.36% of 117   branches executed  92.86% of 56
```

Every unexecuted line is listed in "Paths unreachable from this executable"
below, together with why it cannot be reached and whether the Rust translation of
it is faithful anyway.

---

## Mismatches found

### 1. `SIGPIPE` disposition — exit status differed (FIXED)

**Symptom.** With stdout connected to a pipe whose reader is gone, the two
programs disagreed on exit status while producing identical (empty) output:

```
$ c_src/build/driver   2>/dev/null | true ; echo ${PIPESTATUS[0]}
141
$ translation/target/release/driver 2>/dev/null | true ; echo ${PIPESTATUS[0]}
0
```

**Cause.** The Rust standard library installs `SIG_IGN` for `SIGPIPE` before
`main` runs. A C program inherits the default disposition, so glibc's exit-time
`fwrite` of the buffered stdout is interrupted by `SIGPIPE` and the process dies
from the signal (bash reports `128 + 13 = 141`). With `SIG_IGN` in force the
Rust write merely failed with `EPIPE`, the return value was discarded exactly as
C discards `printf`'s, and `main` went on to `exit(0)`.

Note that stdout alone was not the whole story: because stderr is unbuffered, the
same divergence appeared *mid-run* when stderr was the reader-less pipe, at the
first `fprintf(stderr, ...)` inside `tree_add_node`.

**Fix.** `translation/src/main.rs` now restores the default disposition as the
first thing `main` does:

```rust
fn reset_sigpipe() {
    const SIGPIPE: i32 = 13;
    const SIG_DFL: usize = 0;
    extern "C" { fn signal(signum: i32, handler: usize) -> usize; }
    unsafe { signal(SIGPIPE, SIG_DFL); }
}
```

**Covered by** `sigpipe_disposition_matches` and the `release/sigpipe_*` cases in
`release_profile_matches_across_the_matrix`. Both assert C and Rust agree *and*
that the status is 141, so the test cannot pass by both programs quietly
exiting 0.

### 2. A test formulation that was flaky, not a program mismatch (test-side only)

The first version of the `SIGPIPE` test used bash process substitution,
`"$1" > >(exec true)`. That is a race: the reader's exit competes with the
program's single exit-time flush of 1499 bytes, and the C program wins often
enough to be unusable — 8 consecutive runs of the same command gave
`141 0 0 0 0 0 0 0`. The Rust program, being slightly slower to reach its flush,
lost the race every time and reported 141 every time.

This was a measurement artefact, not a behavioural difference: both programs
behave the same for a given fd state. Replacing the construct with a pipeline
(`| true`), where the read end is reliably closed before either program flushes,
makes the outcome deterministic — verified 20/20 runs at 141 for the C build and
for both Rust profiles. The test now asserts the exact status rather than mere
agreement.

---

## Build-configuration hazard: `assert()` with side effects

`main.c` wraps calls that have side effects inside `assert(...)`, e.g.

```c
assert(tree_add_node(tree, 1, 0, "root") == 0);
```

so `-DNDEBUG` does not merely stop checking, it deletes the work. Confirmed by
building the same sources with `-DCMAKE_BUILD_TYPE=Release`:

```
24,33c24
< [1] root
<   [2] child1
...
---
> (empty tree)
```

The Rust translation uses `assert!`, which is *not* removed by
`--release`, so the release Rust binary matches the asserts-enabled C binary.
The reference build (`cmake ..` with no `CMAKE_BUILD_TYPE`, which is what the
task specifies) leaves `NDEBUG` unset, and
`reference_c_build_has_asserts_enabled` fails loudly if the C binary under
comparison ever turns out to be an `NDEBUG` build.

Aside from `NDEBUG`, the C output is invariant to compiler settings: gcc at
`-O0`, `-O1`, `-O2`, `-O3` and `-Os` all produce byte-identical stdout and
stderr, matching the Rust release binary. (clang was not installed on this
machine, so it was not exercised.)

---

## Behaviours that were checked and already matched

These were all verified by diffing real runs, not by reading code.

- **stdout buffering mode.** glibc chooses line buffering when stdout is a
  terminal and full buffering otherwise; stderr is unbuffered either way. The
  translation reproduces this in `src/cstdio.rs` (`is_terminal()` probe,
  `BUFSIZ = 4096`, flush at end of `main`). It matters: with
  `driver >file 2>&1`, the 1499 bytes of stdout sit in the buffer until `exit`,
  so **both stderr lines appear before the opening banner** even though they are
  printed in the middle of the run. Under a pty the order is the natural
  interleaved one. `merged_into_one_file_matches_including_order` and
  `pseudo_terminal_matches` assert the byte order in both regimes, not just
  equality.
- **`printf` formatting.** `%lu` on `tree_id_t` (`uint64_t`), the two-space
  indent per depth level in `tree_print_helper`, `[%lu] %s\n`, and the trailing
  newlines all match byte for byte.
- **Non-ASCII literals.** The box-drawing banner, `✓` and `═` runs come out as
  identical bytes under every locale tried, including an ISO-8859-1 `LANG` and an
  empty environment — `printf` emits the source bytes without consulting the
  locale, and so does the Rust side.
- **Write errors are ignored.** With stdout on `/dev/full` every write fails with
  `ENOSPC`; C ignores `printf`'s return value and still exits 0. The Rust code
  discards the `io::Result` in `cstdio::write_through`, so it also exits 0 rather
  than panicking. Same for a closed stdout (`>&-`, `EBADF`).
- **stderr content and ordering.** Exactly two error paths are reached, in this
  order:
  ```
  Error: Node with ID 2 already exists
  Error: Parent has maximum children
  ```
  from `tree_add_node`'s duplicate-id check and its `MAX_CHILDREN` check.
- **FNV-1a hash byte order.** The C code casts `&key` to `uint8_t *`, so it
  hashes the host byte order; the Rust code uses `to_le_bytes()`. Identical on
  x86-64. On a big-endian host both the C and the Rust would need revisiting; the
  probe order, and therefore nothing observable in this driver, would change.
- **Determinism.** 5 sequential and 8 concurrent runs of each program are
  byte-identical, so no address-, pid- or timing-dependence leaks into the
  output. The Rust arena-index-for-pointer substitution does not show up.
- **Debug vs release Rust.** `release_and_test_profile_behave_identically`
  confirms the two profiles agree, so the `test`-profile matrix really does speak
  for the shipped `--release` binary. This is not free: the release profile sets
  `panic = "abort"` and drops `overflow-checks`, and `hashmap.rs` contains
  `deleted_count -= 1`, which would panic in debug and wrap in release if it ever
  underflowed. It cannot: that branch requires an entry with
  `occupied && deleted`, and `resize` only ever installs freshly zeroed entries
  while resetting `deleted_count` to 0.

---

## Paths unreachable from this executable

`main.c` never passes a NULL pointer and never induces an allocation failure, so
the following lines cannot execute in the compiled program. Each was still read
against its Rust counterpart; the verdicts are from that reading, not from
execution, and are labelled as such.

| C location | Why unreachable | Rust counterpart |
| --- | --- | --- |
| `tree.c:34,39-40` `tree_create` malloc/`hashmap_create` failure | allocation never fails here | `Tree::create` is infallible; no `NULL` return to model |
| `tree.c:58` `tree_delete(NULL)` | never called with NULL | `delete(self)` takes ownership; not representable |
| `tree.c:75,159,199` `!tree` guards | never called with NULL | methods take `&self`/`&mut self`; not representable |
| `tree.c:87-88` `"Error: Failed to allocate node"` | malloc never fails | no allocation failure path exists in Rust |
| `tree.c:99` `node->data[0] = '\0'` for `data == NULL` | every call site passes a literal | `None => node.data[0] = 0` — present and faithful |
| `tree.c:111-113` `"Error: Parent node %lu not found"` | no call uses a bad parent id | present, same text and `%lu` formatting |
| `tree.c:127-129` `"Error: Failed to add node to hashmap"` | `hashmap_put` only fails on allocation failure | present; Rust also grows its arena before the `put`, which is unobservable |
| `tree.c:139,252,264,285,316` inner `!node` → `-1` | callers always pass live ids | all five present and return `-1` |
| `tree.c:164-165` `"Error: Node %lu not found"` | `tree_remove_node` is never called with a missing id | present, same text |
| `tree.c:216` `tree_print_helper` `!node` early return | ids are always live during the walk | present |
| `tree.c:234-235` `"(empty tree)"` | `tree_print` is only called on a populated tree | present |
| `tree.c:299` `!path` guard in `tree_find_path` | never called with NULL | a `&mut [TreeId]` cannot be null; not representable |
| `tree.c:323` `length = max_length` truncation | the only call has `max_length = 10` and a depth of 3 | present and faithful |
| `hashmap.c:57-59,79,88-89` calloc/malloc failure | allocation never fails | infallible in Rust |
| `hashmap.c:104,151,177,214-215` `!map` guards | never called with NULL | methods take `&self`/`&mut self`; not representable |
| `hashmap.c:109` `hashmap_resize` failure | resize only fails on allocation failure | `resize` always returns 0; the `&&` short-circuit mirrors C's nested `if` |
| `hashmap.c:131-136` reuse of a tombstoned slot | no `put` follows a `remove` on a colliding key in this driver | present, including leaving `occupied` set and decrementing `deleted_count` |
| `hashmap.c:143,146,199,202,172,188` probe exhaustion / miss returns | the table is never full and lookups hit or find an empty slot | all present |
| `hashmap.c:213-224` `hashmap_clear` | never called | present as `Hashmap::clear`, marked `#[allow(dead_code)]` |

Two other C-isms deserve naming even though they are not defects here:

- `tree_find_path` writes `path[0..length]` after clamping `length` to
  `max_length`. If a caller ever passed a `max_length` larger than the real
  buffer, C would scribble out of bounds while Rust would panic. The only call
  site passes a 10-element array with `max_length = 10`, so the behaviours
  coincide.
- `tree_add_node` compares `parent->child_count >= MAX_CHILDREN` with a signed
  `int`; the Rust casts to `usize` first. A negative `child_count` would diverge,
  but the field is only ever incremented from 0 and decremented while positive.

## Housekeeping

`translation/src/cout.rs` was removed. It was not declared as a module anywhere,
so it was never compiled, and it contained a *second* stdio emulation that —
unlike the live `src/cstdio.rs` — had no line-buffered/terminal path. Leaving two
divergent buffering implementations in the tree, one of them dead, invites the
next reader to consult the wrong one. Removing it changed no behaviour: the
release binary rebuilt clean and all 22 tests still pass.

## Current state

```
$ cd c_src/build && cmake --build .          # no errors
$ cd translation && cargo build --release    # no errors, no warnings
$ cargo test
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

No test is `#[ignore]`d, skipped or conditionally bypassed; the pty case asserts
that `script` is present rather than silently opting out. Nothing in `c_src/` was
modified: every source file there still carries the workspace's original
timestamp (`2026-09-01 16:02:37.798812851`, identical across all six files),
which predates the first command run against the tree. Only the generated
`c_src/build/` directory is new.
