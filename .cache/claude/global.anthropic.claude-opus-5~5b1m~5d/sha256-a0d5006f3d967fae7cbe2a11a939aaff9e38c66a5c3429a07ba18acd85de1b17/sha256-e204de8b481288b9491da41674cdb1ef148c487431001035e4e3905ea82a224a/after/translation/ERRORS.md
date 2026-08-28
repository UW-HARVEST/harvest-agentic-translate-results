# Differential verification of the C → Rust translation

Ground truth: `../c_src` (never modified).
Rust under test: `src/` built with `cargo build --release`.

```
# C program
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .   # -> c_src/build/driver
# Rust program
cd translation && cargo build --release                                 # -> translation/target/release/driver
# Differential suite (builds the C program automatically if it is missing)
cd translation && cargo test
```

`tests/differential.rs` drives **both executables as subprocesses** (never as a
library) over 202 inputs in 13 groups and compares, for each one:

* stdout, byte for byte
* stderr, byte for byte
* the exit status (including "still running", see *Hangs* below)
* every file left behind in the working directory

Result: **no behavioural mismatch was found.** The translation already matched
the C program on every input tried, so no fix to `src/` was required. The rest
of this document records what was checked, the two places where a byte-exact
comparison is *impossible in principle* and how the tests deal with them, and
the C quirks that the translation reproduces on purpose (each of which was
verified by deliberately breaking the Rust code and confirming that the suite
fails — see *Mutation testing*).

---

## 1. Unavoidable divergence: `%p` addresses

`main.c` and `scene.c` print raw `malloc` addresses:

```c
printf("Shape '%s' added to scene (reusing singleton at %p)\n", shape->name, (void*)shape);
printf("  %d. %s (ptr: %p)\n", i + 1, scene->shapes[i]->name, (void*)scene->shapes[i]);
printf("\nShape 1: %s (ptr: %p)\n", s1->name, (void*)s1);
```

Observed for the same input:

| | address printed |
|---|---|
| C    | `0xbdf32b0` (brk heap of the C binary) |
| Rust | `0x5592703ab050` (heap of the PIE Rust binary) |

No independent process can reproduce another process's heap addresses — they
differ between the two programs, and between two runs of the *same* program
(ASLR). So this is not a translation defect and there is nothing to fix in
`src/`.

What the output really encodes is *object identity*: which printed addresses are
equal to which others (`shape_equals` is a pointer comparison, and the C code
prints `Comparison of pointers: %d` right next to the addresses). The harness
therefore maps addresses to `<ptrN>` tokens **numbered by order of first
appearance** (`harness::normalize_ptrs`), which preserves that relation exactly:
if C prints two different addresses where Rust prints one (or vice versa) the
test still fails. Verified by mutation M5 below.

### 1a. Knock-on effect on the truncated output of a killed process

The C program's stdout is fully buffered (4096 bytes, see §2). When the process
is killed mid-run, only the flushed prefix survives. Because C's addresses are
7–9 characters and Rust's are 14, the two programs put a different *number of
logical characters* into each 4096-byte block, so the surviving prefixes stop at
different places in the output.

This was the only diff the random fuzzer ever reported (`tools/fuzz.py`): both
processes were killed after hanging, both had flushed exactly 4096 bytes, and
the tails differed only because the pointer strings have different widths.

Handling: the hang cases in `hangs_at_eof_during_getchar_drain` are chosen so
that **no `%p` address is printed**, which makes the surviving prefix comparable
byte for byte with no normalisation at all (`Case::hangs()` sets
`expect_hang`, and `harness::check` then skips normalisation).

## 2. Environment assumption: the 4096-byte stdout buffer

`src/cio.rs` reimplements glibc's fully-buffered stdout with a hard-coded
4096-byte block, which is what glibc picks (`st_blksize`) for a pipe or a
regular file on this machine. This only becomes observable when the process is
killed before its buffer is flushed. It is verified by
`hangs::buffered_prefix_survives`: with ~12 KiB of output produced before the
hang, both programs had written exactly 12288 bytes to fd 1 and those bytes are
identical. On a filesystem whose `st_blksize` is not 4096 the C program would
flush on a different boundary and that one test would need the constant in
`COut::new()` adjusted to match.

---

## 3. C behaviours the translation deliberately reproduces

Everything here looks like a bug and is *kept*, because the C is the spec.

| # | C behaviour | Where | Test |
|---|---|---|---|
| 1 | `while (getchar() != '\n');` **never terminates at EOF** — `getchar()` keeps returning `EOF`, so the program spins forever and its buffered stdout is never flushed. | `main.c` after every `scanf` | `hangs_at_eof_during_getchar_drain` (7 cases; asserts *both* programs are still running when the timeout expires) |
| 2 | `scanf("%d")` skips arbitrary whitespace **including newlines**, so a number may be read from a later line, while `fgets` never reads past its newline. The two are mixed on one shared `stdin`. | `main.c` | `add_shape::blank_lines_between_numbers`, `mixed_sessions::*` |
| 3 | `%d` accumulates at `long` width, saturates at `LONG_MAX`/`LONG_MIN`, then **truncates to `int`**: `99999999999999999999` → `-1`, `4294967298` → `2`, `4294967295` → `-1`. | `main.c`, `scene.c` | `menu::overflow_to_minus_one`, `menu::truncates_to_two`, `add_shape::shape_type_huge`, `compare_shapes::overflowing_types`, `load_scene::count_truncates_to_*`, `shape_type_truncates_to_zero` |
| 4 | `shape_idx - 1` overflows for `INT_MIN` (→ `INT_MAX`), and index `0` becomes `-1` → "Error removing shape". | `main.c:172` | `remove_shape::remove_index_int_min`, `remove_index_zero` |
| 5 | The menu reads with `fgets(input, 256, stdin)`: a longer line is **split**, and its tail is parsed as the *next* menu choice. | `main.c:398` | `menu::line_longer_than_buffer`, `save_scene::filename_longer_than_buffer` |
| 6 | Scene names are read with `fgets(name, 64, stdin)`: 63 characters fill the buffer, so the newline stays in the stream and becomes the next (empty, "Invalid input") menu line. | `main.c:74` | `create_scene::name_63_chars`, `name_70_chars` |
| 7 | `name[strcspn(name, "\n")] = 0` stops at the **first NUL**, so a name typed as `A\0B` becomes `A` and `B` is silently dropped. | `main.c:77` | `create_scene::nul_in_name`, `load_scene::nul_in_name` |
| 8 | `scene_load` reads the name with `fgets(name, 64, file)`; a longer first line leaves its tail in the stream, where `fscanf("%d")` then tries to read it as the shape count (and fails → silent `NULL`, no message). | `scene.c:170` | `load_scene::name_longer_than_63`, `name_63_then_digits` |
| 9 | A failed `scene_load` prints **nothing at all** (not even an error) unless `fopen` itself failed. Empty file, bad count, missing types → silent. | `scene.c` | `load_scene::empty_file`, `count_not_a_number`, `count_more_than_entries`, `directory` |
| 10 | `fopen(dir, "r")` **succeeds** on Linux and the first `fgets` then fails, so loading a directory is silently treated as an empty file — while loading a *missing* file prints to stderr. | `scene.c:163` | `load_scene::directory` vs `missing_file` (mutation M20) |
| 11 | A negative shape count in a file is accepted: the `for` loop simply does not run and an **empty scene loads successfully**. | `scene.c:191` | `load_scene::count_negative`, `count_truncates_to_minus_one` |
| 12 | `fscanf(file, "%d\n", ...)`: the `\n` is a whitespace directive that eats *any* run of whitespace, so all shape types may sit on one line, and CRLF files load fine (the name keeps its `\r`). | `scene.c:185,193` | `load_scene::types_on_one_line`, `crlf_file`, `count_with_spaces` |
| 13 | Out-of-range shape types in a file are **skipped silently** (`shape_get` returns NULL, `scene_add_shape` is not even called) and the load still succeeds. | `scene.c:199` | `load_scene::shape_type_out_of_range`, `shape_type_negative` |
| 14 | Loading more than `MAX_SHAPES_IN_SCENE` (50) shapes prints "Error: Scene is full" to **stderr** once per extra shape, and the load still reports success. | `scene.c:63` | `load_scene::fifty_five_shapes` |
| 15 | Interactively adding a 51st shape prints "Error: Scene is full" on stderr *and* "Error adding shape" on stdout. | `main.c:132` | `add_shape::fifty_one_shapes` |
| 16 | `scene_equals` is a multiset match over **pointer identity** with a `matched[]` array, so duplicates must pair up one for one; two empty scenes are EQUAL. | `scene.c:101` | `compare_scenes::duplicates_matter`, `both_empty`, `fifty_shapes_permuted` |
| 17 | `fopen("", "w")` / `fopen("", "r")` fail, so an empty filename produces `Error: Could not open file '' for writing`. | `scene.c:136,163` | `save_scene::empty_filename`, `load_scene::empty_filename` |
| 18 | `scene_save` ignores `fclose`'s result, and `save_scene_to_file` ignores `scene_save`'s, so a failed save prints only the stderr line. | `main.c:241` | `save_scene::readonly_target`, `filename_is_directory`, `nonexistent_directory` |
| 19 | `fgets` returning NULL at the "Enter scene name"/"Enter filename" prompt makes the handler return **silently**. | `main.c:74,236,252` | `create_scene::eof_at_name_prompt`, `save_scene::eof_at_filename`, `load_scene::eof_at_filename` |
| 20 | The shape art is printed with `%s` per row, so the hard-coded trailing spaces and the inconsistent row widths (e.g. `Heart` rows of 10/11/9 characters) are printed verbatim. | `shape.c` | `view_all_shapes::once` (mutation M14) |
| 21 | `sscanf(input, "%d", &choice)` stops at a NUL and ignores trailing text: `12abc` exits, `0x3` is choice 0, a leading NUL is "Invalid input". | `main.c:402` | `menu::digits_then_text`, `hex_looking`, `nul_before_digits` |
| 22 | The exit status is always 0 — via `return 0` on choice 12, or via the `break` on EOF followed by the same cleanup. | `main.c` | every case (status compared always) |

---

## 4. Mutation testing (proof the suite is not vacuous)

Each mutation was applied to `src/`, the suite was run, and the source was
restored. "caught" means at least one named case failed.

| | Mutation | Result |
|---|---|---|
| M1 | `"Invalid choice"` → `"Invalid Choice"` | caught — `menu::choice_zero` (stdout) |
| M2 | stderr `"Error: Scene is full"` → lower case | caught — `add_shape::fifty_one_shapes`, `load_scene::fifty_five_shapes` (stderr) |
| M3 | `shape_idx.wrapping_sub(1)` → `shape_idx` | caught — `remove_shape::remove_only_shape` |
| M4 | stdout buffer 4096 → 1 (unbuffered) | caught — `hangs::scanf_eof_first_shape` |
| M5 | every shape prints the *same* address | caught — `compare_shapes::different_types` (address-identity relation) |
| M6 | saved file writes `type + 1` | caught — `save_scene::save_with_shapes` (working-directory file) |
| M7 | exit with status 1 on choice 12 | caught — `menu::non_numeric` (exit status) |
| M8 | `strncpy` limit 63 → 64 in `scene_create` | **not caught — equivalent mutant.** Every caller obtains the name from `fgets(..., 64, ...)`, so it is never longer than 63 bytes and the truncation branch is unreachable. |
| M9 | `fgets` reads past the newline | caught — `create_scene::simple` |
| M10 | `scene_equals` ignores `matched[]` | caught — `compare_scenes::duplicates_matter` |
| M11 | a 51st shape is accepted | caught — `add_shape::fifty_one_shapes` |
| M12 | `%d` overflow yields 0 instead of `LONG_MAX` | caught — `compare_shapes::overflowing_types`, `add_shape::shape_type_huge` |
| M13 | the EOF drain returns instead of spinning | caught — `hangs::*` (exit status: C timed out, Rust exited) |
| M14 | trailing spaces stripped from art rows | caught — `view_all_shapes::once` |
| M15 | `%d` clamps to `INT_MAX` instead of truncating | caught — `menu::truncates_to_two`, `load_scene::count_truncates_to_two` |
| M16 | newline kept in the scene name | caught — `create_scene::simple`, `save_scene::invalid_input` |
| M17 | NUL not treated as end of string | caught — `create_scene::nul_in_name`, `load_scene::nul_in_name` |
| M20 | a directory reported as an unopenable file | caught — `load_scene::directory` (stderr) |
| M21 | empty-filename guard removed from `write_file` | **not caught — equivalent mutant.** Rust's `File::create("")` already fails with `ENOENT`, exactly like `fopen("", "w")`; the guard is redundant. The error path itself *is* covered by `save_scene::empty_filename`. |

## 5. Random differential fuzzing

`tools/cmp.py` (single input), `tools/fuzz.py` (random menu sessions) and
`tools/fuzz_file.py` (random scene-file contents) run both binaries and diff
them the same way the Rust tests do. 800 random menu sessions plus 500 random
scene files produced no mismatch other than the pointer-width truncation
artefact described in §1a.
