# Verification log: mismatches found between the C reference and the Rust translation

The C program in `c_src/` is the ground truth. Both programs were built, run as
subprocesses over the same inputs, and compared on stdout, stderr, exit status
and the files they leave in the working directory. `c_src/` was not modified;
its sources are byte-identical to the original (verified with `cmp` against a
copy taken before any work started).

Build and run commands:

| | command |
|---|---|
| C | `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .` → `c_src/build/driver` |
| Rust | `cd translation && cargo build --release` → `translation/target/release/driver` |
| Tests | `cd translation && cargo test` (141 tests, all enabled) |

---

## Mismatch 1 — the Rust program survived a closed stdout; the C program is killed

**Symptom**

```
$ set -o pipefail
$ ./c_src/build/driver           < many-menu-1-lines | head -c 1 >/dev/null ; echo $?
141          # killed by SIGPIPE (128 + 13)
$ ./translation/target/release/driver < many-menu-1-lines | head -c 1 >/dev/null ; echo $?
0            # kept running to completion
```

**Cause**

The Rust standard library sets `SIGPIPE` to `SIG_IGN` before `main` runs. A
write to a pipe whose reader has gone away therefore returns `EPIPE`, which
`cio::Out::raw_write` discards, and the program runs to a normal exit. The C
program keeps the default disposition, so the same write kills it with signal
13. Exit status and the amount of output flushed both differed.

This is only observable when the consumer of stdout exits early, which is why it
survived every test that captured output in full — it is invisible to a harness
that reads the whole stream.

**Fix** — `translation/src/main.rs`: restore the default disposition as the first
thing `main` does.

```rust
fn restore_default_sigpipe() {
    const SIGPIPE: i32 = 13;
    const SIG_DFL: usize = 0;
    extern "C" { fn signal(signum: i32, handler: usize) -> usize; }
    unsafe { signal(SIGPIPE, SIG_DFL); }
}
```

**Regression test** — `writing_to_a_closed_pipe_kills_both`. Removing the call
makes it fail with `C=(None, Some(13)) Rust=(Some(0), None)`.

---

## Mismatch 2 — `shape_idx - 1` aborted the Rust program for `INT_MIN`

**Symptom**

Menu 4 (remove a shape) with the shape index `-2147483648`:

```
release build : "Error removing shape"   (matches the C program)
debug build   : thread 'main' panicked at 'attempt to subtract with overflow'
```

**Cause**

`main.c` computes `scene_remove_shape(scene, shape_idx - 1)` on a value taken
straight from `scanf("%d")`. For `INT_MIN` that overflows; gcc at `-O0` emits a
plain `subl` and the result wraps to `INT_MAX`, so the index is rejected and the
C program prints `Error removing shape`. The translation wrote `shape_idx - 1`,
which wraps identically in the release profile but panics whenever
`overflow-checks` is on. The behaviour of the translation therefore depended on
which profile it was built with, and only the release profile happened to agree
with the C.

**Fix** — `translation/src/main.rs`: `shape_idx.wrapping_sub(1)`, which states
the wrap explicitly and is profile-independent.

**Regression tests** — `remove_shape_index_int_min` covers the input;
`release_and_test_profile_binaries_agree` runs the same input through both the
release binary and the binary cargo builds for the test profile and requires
identical stdout, stderr and exit status. Reverting to `shape_idx - 1` makes the
latter fail.

---

## Non-mismatch worth recording: `%p` cannot be compared byte for byte

`scene_list_shapes`, `add_shape_to_scene` and `compare_shapes` print heap
addresses with `printf("%p")`. Those are not reproducible **even between two
runs of the C program**, because the heap base is randomised:

```
$ ./c_src/build/driver <<< $'9\n0\n0\n12\n' | grep ptr
Shape 1: Tree (ptr: 0xa8bb2b0)
$ ./c_src/build/driver <<< $'9\n0\n0\n12\n' | grep ptr
Shape 1: Tree (ptr: 0x1d4bb2b0)
```

No implementation can match those bytes. What *is* observable, and what the
program's logic depends on, is the identity relation the addresses encode: which
printed pointers are equal and which differ (`shape_equals` is pointer equality,
and `Comparison of pointers: %d` prints the result). The harness therefore
replaces each distinct `0x<hex>` run with a token numbered by first appearance,
so two outputs compare equal only if they agree byte for byte everywhere else
**and** print the same pattern of equal/distinct pointers in the same positions.
The harness additionally asserts that every printed pointer has glibc's `%p`
shape (`0x` followed by lowercase hex) in both programs.

---

## Behaviour deliberately preserved rather than "fixed"

These all looked like defects and were kept because the C does them:

- **`while (getchar() != '\n');` never terminates at end of file.** `getchar()`
  keeps returning `EOF`, which never equals `'\n'`, so any numeric prompt reached
  exactly at EOF spins forever. `cio::In::eat_until_newline` spins the same way.
  Covered by `eof_inside_the_scanf_drain_loop_spins_in_both`, which asserts that
  *both* programs are still running when the time limit expires and that neither
  has flushed anything.
- **`scanf` crosses newlines, `fgets` does not.** Numeric prompts skip blank
  lines and consume a value from any later line; scene names and filenames stop
  at the first newline and are truncated at 63 and 255 bytes respectively, with
  the remainder left in stdin to be re-read as the next menu choice.
- **`scanf`/`sscanf` `%d` saturates then truncates.** glibc converts via the
  `strtol` family (saturating at `LONG_MAX`/`LONG_MIN`) and stores the result
  into an `int`. `99999999999999999999` becomes `-1`, `-99999999999999999999`
  becomes `0`, `4294967296` becomes `0`. `cio::finish_int` reproduces this.
- **A sign with no digits still consumes the sign.** `-x` at a numeric prompt is
  a matching failure, but `-` has already been read, so the drain loop only has
  `x\n` left.
- **`fopen` on a directory succeeds.** `scene_load("somedir")` opens it, the
  first `fgets` fails, and the function returns `NULL` **without** printing
  anything — no "could not open" message. `scene_load` mirrors this by treating a
  read error as the failing first `fgets`.
- **Over-capacity loads still succeed.** A saved file claiming more than
  `MAX_SHAPES_IN_SCENE` shapes writes `Error: Scene is full` to stderr once per
  extra shape and then reports the scene as loaded.
- **Out-of-range shape types in a file are silently skipped**, not rejected, so
  the loaded scene can have fewer shapes than the file's count.
- **`strcspn(buf, "\n")` stops at a NUL**, so a NUL byte in a scene name or
  filename truncates it there.

---

## Input classes covered

Enumerated by reading `main.c`, `scene.c` and `shape.c` and confirmed with gcov
on an instrumented copy of the C sources (built in `/tmp`, leaving `c_src/`
untouched): **96.75 % line coverage, 554 lines**.

Menu dispatch: empty input, bare newline, non-numeric, leading whitespace, sign,
leading zeros, trailing junk, embedded NUL, `0`, `13`, negative, `INT_MAX`,
`INT_MIN`, values that overflow `long`, `2^32`, lines at and over the 255-byte
`fgets` limit, EOF with no trailing newline.

Per menu entry: the no-scene / fewer-than-two-scenes guards, `scanf` matching
failure at every numeric prompt, every index and shape-type range check at both
ends, the `MAX_SCENES` (10) limit, the `MAX_SHAPES_IN_SCENE` (50) limit, the
empty-scene guard on removal, `shape_idx - 1` at `INT_MIN` and `INT_MAX`, scene
deletion and the index shift that follows, `scene_equals` for equal counts,
unequal counts, reordered shapes and differing multiplicities, and every one of
the 100 shape pairs for `shape_equals` / `shape_type_name`.

File I/O: save with no scenes, a bad index, EOF before the filename, an empty
filename, a missing directory, `.`, `..`, a directory, a read-only file,
truncation of a longer existing file, a subdirectory, a non-UTF-8 filename, a
filename containing NUL, an empty scene and a full 50-shape scene. Load with a
missing file, a directory, an empty file, a name-only file, an unparsable count,
a count larger than the number of entries, a negative / zero / `+`-signed /
padded / saturating / `INT_MAX` count, a float count, CR-LF, all entries on one
line, tabs, trailing garbage, an empty name line, a NUL in the name, no newline
anywhere, names at 62/63/64/100 bytes, a 63-byte name whose 64th byte is a
digit, out-of-range and overflowing shape types, more than 50 entries, a full
scene table, and a save-then-load round trip.

The remaining uncovered C lines are unreachable from stdin: `malloc` failure
handling (`main.c:84`, `scene.c:34`, `scene.c:180-181`, `shape.c:178-179`) and
NULL-argument guards in functions `main` only ever calls with validated
arguments (`scene.c:41,59,87-88,103,133,160,212-213`, `shape.c:212-213,238`).

Beyond the test suite, ~2000 randomly generated sessions and 600 random byte
streams were compared the same way with no further differences.
