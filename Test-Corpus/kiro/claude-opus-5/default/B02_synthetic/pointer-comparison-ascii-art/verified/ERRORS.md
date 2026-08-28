# Differential testing log: `c_src/` (ground truth) vs `translation/`

Both programs are compared by **running them**: `tests/differential.rs` spawns the
C executable and the Rust executable as subprocesses, feeds each identical
stdin, and asserts that **stdout**, **stderr**, the **exit status** and any
**files written to the working directory** are byte-for-byte identical. The Rust
code is never loaded as a library.

- C executable: built with CMake into `translation/target/c_build/driver`
  (out-of-tree, so nothing under `c_src/` is touched).
  Run as: `setarch --addr-no-randomize .../c_build/driver` — see
  [Addresses printed with `%p`](#addresses-printed-with-p).
- Rust executable: `cargo build --release` → `translation/target/release/driver`,
  run as `translation/target/release/driver`.
  The suite compares against every Rust binary it finds (the one cargo built for
  the test run, plus the other profile if it is present), so both the debug and
  release artifacts are covered.

`cargo test` passes; no test is `#[ignore]`d, skipped or disabled.

---

## Mismatches found and fixed

### 1. `scanf`/`sscanf` integer overflow saturated in the wrong direction

**Symptom.** stdin `-9223372036854775808\n12\n` (and any negative literal whose
magnitude exceeds 2^63):

| | stdout at the menu |
| --- | --- |
| C | `Invalid choice` |
| Rust (before) | the full `=== Available Shapes ===` listing |

**Cause.** glibc's `%d` conversion collects the digit run and converts it with
`strtol` semantics: the *magnitude* is accumulated and, on overflow, the result
saturates at `LONG_MAX` or `LONG_MIN`. The saturated `long` is then truncated to
`int` by the assignment.

The translation accumulated a signed `i64` with `saturating_mul`/`saturating_add`
and negated afterwards. For an out-of-range negative input that produces
`-LONG_MAX` (`0x8000000000000001`) instead of `LONG_MIN`
(`0x8000000000000000`) — and those truncate to **different** `int` values:

```
C:    LONG_MIN         = 0x8000000000000000  -> (int) 0   -> "Invalid choice"
Rust: -LONG_MAX        = 0x8000000000000001  -> (int) 1   -> menu option 1
```

**Fix.** `cio::DigitAcc` accumulates the magnitude in a `u64` with
`checked_mul`/`checked_add`, records overflow, and only then picks `LONG_MIN`
or `LONG_MAX` before truncating to `i32`. Both integer readers now use it:
`cio::scan_int_generic` (the `scanf`/`fscanf` paths) and `main::sscanf_int`
(the `sscanf` on the menu line). It also fixes the exact boundary
`-9223372036854775808`, which is a *representable* `long` and must not
saturate at all.

Covered by `integer_overflow_and_truncation`.

### 2. `shape_idx - 1` could panic instead of wrapping

**Symptom.** Not observable in the release binary (release builds wrap), but
stdin `2\na\n3\n0\n0\n4\n0\n-2147483648\n12\n` aborted the **debug** binary,
which the suite also compares.

**Cause.** `remove_shape_from_scene` passes `shape_idx - 1` to
`scene_remove_shape`. In C this is `int` arithmetic that wraps for `INT_MIN`; in
Rust the debug profile traps on overflow.

**Fix.** `shape_idx.wrapping_sub(1)`, matching the C.

Covered by `integer_overflow_and_truncation`
(`scanf_shape_idx_int_min`).

---

## Addresses printed with `%p`

`compare_shapes`, `add_shape_to_scene` and `scene_list_shapes` print the address
of a `malloc`ed `shape_t` singleton. **The C program does not produce a stable
value for these**: with ASLR on, three consecutive runs of the *same* C binary
print three different addresses.

```
$ for i in 1 2 3; do printf '9\n0\n0\n12\n' | ./driver | grep 'Shape 1'; done
Shape 1: Tree (ptr: 0x77fa2b0)
Shape 1: Tree (ptr: 0x1c3062b0)
Shape 1: Tree (ptr: 0x3337f2b0)
```

With heap randomisation disabled the layout is fully deterministic — the ten
`shape_t` allocations (2444 bytes each) land at a fixed base, `0x9a0` apart —
and it is exactly what `shape.rs` reproduces:

```
$ printf '9\n0\n1\n12\n' | setarch --addr-no-randomize ./driver
Shape 1: Tree    (ptr: 0x4092b0)
Shape 2: Tractor (ptr: 0x409c50)
```

So the harness runs the C child under `setarch --addr-no-randomize` and then
compares **raw bytes, pointers included** (`PtrMode::Exact`). It verifies at
startup that this actually pins the heap by running the C program twice and
requiring identical output. Only if `setarch` is unavailable does it fall back
to canonicalising `0x…` hex runs on both sides (`PtrMode::Normalized`).

The base differs between a terminal and a pipe (`0x4086b0` vs `0x4092b0`)
because glibc allocates the stdout buffer, whose size comes from `st_blksize`,
before the singletons. `shape.rs` models both, and both were confirmed against
the C program.

---

## Behaviour deliberately preserved, including the parts that look like bugs

Each of these was verified against the C program rather than reasoned about.

- **`while (getchar() != '\n');` never terminates at EOF.** After a successful
  `scanf`, the C code drains the line with this loop. At end of file `getchar()`
  returns `EOF` forever, which never equals `'\n'`, so the process spins. Input
  `9\n5` (no trailing newline) hangs both programs; both are killed by
  `timeout` with status 124 and both have flushed zero bytes, because stdout is
  block buffered. `cio::In::skip_to_newline` reproduces this on purpose.
  Covered by `getchar_loop_spins_forever_at_eof`.
- **`scanf` reads across newlines, `fgets` does not.** The menu is read with
  `fgets`, so a blank line is "Invalid input"; the sub-prompts use `scanf`, which
  skips any run of white space including newlines. `add_scanf_crosses_newlines`.
- **Order of validation.** `compare_shapes` reads *both* numbers before
  range-checking either, so an invalid first type still prints the second prompt.
  `remove_shape_from_scene` calls `scene_list_shapes` *before* testing whether
  the scene is empty. `cmp_shapes_bad_first`, `remove_from_empty_scene`.
- **1-based prompt over a 0-based array.** "Select shape to remove (1-N)" then
  `scene_remove_shape(scene, shape_idx - 1)`, so entering `0` yields index `-1`
  and "Error removing shape". `remove_index_zero`.
- **Truncation at 63 bytes.** `fgets(name, MAX_SCENE_NAME, stdin)` stores at most
  63 bytes; the untaken remainder of an over-long line is re-read as the *next
  menu command*. `option2_name_length_boundaries`.
- **`strcspn(s, "\n")` stops at a NUL too.** A name or filename containing a NUL
  byte is truncated there, not at the newline. `create_name_embedded_nul`,
  `save_filename_embedded_nul`.
- **A negative shape count in a saved file is "successful".** `scene_load` reads
  the count, runs `for (i = 0; i < shape_count; i++)` zero times, and still
  prints `Scene loaded from '…'`. `load_negative_count`.
- **Out-of-range shape types in a saved file are silently dropped.**
  `shape_get()` returns `NULL` and `scene_load` skips the `scene_add_shape`
  call, so the loaded scene has fewer shapes than the header claims.
  `load_out_of_range_type`.
- **Overflowing a scene reports on both streams.** The 51st shape makes
  `scene_add_shape` write `Error: Scene is full` to **stderr** and the caller
  write `Error adding shape` to **stdout**. Loading a 55-record file emits five
  stderr lines and then succeeds. `option3_scene_full_at_50_shapes`,
  `option8_load_hits_the_scene_shape_cap`.
- **Silent returns.** `fgets` returning `NULL` at the "Enter scene name" /
  "Enter filename" prompts, and every `scene_load` failure after the `fopen`,
  produce **no output at all**. `create_eof_at_name`, `load_empty_file`,
  `load_name_only`.
- **`printf` spacing.** `Contains %d shape(s)\n\n` emits a blank line;
  `Choice: ` and the `Select … (0-%d): ` prompts have no newline, so they only
  reach the terminal when the buffer flushes.
- **Buffering.** stdout is block buffered (4096 bytes, from `st_blksize`) on a
  pipe or file and line buffered on a terminal; stderr is unbuffered. The
  `merged_streams` cases run `prog > file 2>&1` so the interleaving of the two
  streams is compared, not just their contents.

---

## Input classes enumerated from the C source

Derived by walking every `if`, early `return` and bounds check in
`c_src/src/main.c`, `scene.c` and `shape.c`.

| Area | Cases |
| --- | --- |
| `main` menu loop | empty stdin; bare newline; non-numeric; whitespace-only; leading spaces/tabs; `+6`; leading zeros; trailing junk (`6abc`); embedded NUL; CRLF; choice `0`, `13`, `-1`, `9999`; line >255 bytes; line exactly 255 bytes; missing final newline; input after `12` |
| integer parsing | `2^32+6`; `-(2^32-6)`; `LONG_MAX`; `LONG_MAX+1`; `LONG_MIN`; `LONG_MIN-1`; 20+ digit magnitudes; `INT_MIN`/`INT_MAX` shape index; `-`/`+` with no digits |
| 1 view shapes | all ten art blocks, repeated |
| 2 create scene | normal; empty; spaces; 62/63/70/200-byte names; EOF at prompt; NUL; high bytes; `%s%d%%`; CRLF; 11th scene (MAX_SCENES) |
| 3 add shape | no scenes; index `5`/`-1`/non-numeric; type `10`/`-5`/non-numeric; all ten types; `scanf` across newlines; trailing junk; 51st shape |
| 4 remove shape | no scenes; bad/non-numeric scene index; empty scene; first/middle/last; index `0`; index past end; non-numeric; remove until empty |
| 5 view scene | no scenes; empty scene; bad index; non-numeric; populated |
| 6 list scenes | none; three; with shape counts |
| 7 save scene | no scenes; bad/non-numeric index; EOF at filename; empty filename; missing directory; onto a directory; onto `.`; spaces/high bytes/`%`/NUL in filename; >255-byte filename; overwrite; empty scene; empty scene name; all ten types |
| 8 load scene | missing file; empty filename; a directory; EOF at prompt; empty file; name only; name without newline; non-numeric count; count > records; count `2000000000`; count `2^32`; negative count; zero count; type `99`/`-1`/`2^32`; extra whitespace; records on one line; tabs; CRLF; `+` signs; leading zeros; trailing junk; no final newline; empty/over-long/NUL/high-byte names; 55 records into 50 slots; load twice; load at MAX_SCENES |
| 9 compare shapes | same; different; `0`/`0`; bad first; bad second; both bad; boundary `10`; non-numeric first/second; all ten adjacent pairs |
| 10 compare scenes | 0 scenes; 1 scene; bad first/second index; negative; non-numeric; both empty; same index twice; equal; permuted; reversed; count mismatch; same count different shapes; duplicates vs unique (`matched[]` bookkeeping); after a removal; two 50-shape scenes in opposite order |
| 11 delete scene | no scenes; bad/negative/non-numeric index; only scene; last scene; middle (index shifting); delete at MAX_SCENES then create |
| plumbing | stdin as a regular file **and** as a pipe; >9000 bytes of stdin; multi-digit numbers straddling offsets 4090/4093–4097/4100 (the 4096-byte read boundary, so a digit run and its push-back span two refills); `2>&1` merged streams; the EOF spin paths |
