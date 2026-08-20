# CONFIGS.md — configuration / valid-input surface (Phase A, verified in Phase B)

## Axes the C code actually branches on

Derived from `c_src/include/driver.h` (public API), `c_src/src/driver.c`
(every `if`/`for`), and `c_src/CMakeLists.txt`:

* **Runtime options / modes / flags: none.** The library has no init function, no
  global state, no setters, no environment lookups, no `#ifdef`s (only the
  header guard) and no CMake options.  Every call is self-contained.
* **Compile-time configuration: one.** No `[features]` in `Cargo.toml`, no
  `#[cfg(feature)]` in `src/`, no `add_definitions`/`option()` in CMake ⇒ the
  only feature combination is the empty one (`--no-default-features` ==
  default == `--all-features`).  Both cargo profiles (`dev`, and `release` with
  `panic = "abort"`) are still exercised.
* **Entry points (all 5 exported symbols, low-level ones included):**
  `printLine` (lowest level), `printIntLine` (lowest level), `bad`, `good`
  (wraps the two `static` helpers `goodG2B`/`goodB2G`), `driver` (top-level
  one-shot wrapper).  Tests drive the low-level printers directly, then `bad`
  and `good`, then the composed `driver` pipeline.
* **Input shapes the code distinguishes:**
  * `printLine`: NULL vs non-NULL (`c:31`); byte content of the string
    (length 0 / 1 / many, ASCII / high bytes / `%`-specifiers / embedded `\n`,
    length across libc's stdout buffer size ⇒ 0, 1, 4095, 4096, 8191, 8192,
    65536 bytes); interior pointer; embedded NUL (truncation).
  * `printIntLine`: any `int`; the `%d` formatting shapes are 0, positive,
    negative (sign), `INT_MIN` (no positive counterpart), `INT_MAX`, and digit
    widths 1…10.
  * `bad`: `data < 0` (error branch) vs `data >= 0` (store + print), and within
    the accepted set the in-bounds indices `0…9` (each puts the `1` in a
    different printed position) vs the out-of-bounds `data >= 10` (UB store).
  * `good`: `goodG2B` is argument-independent (constant index 7); `goodB2G`
    splits `data` into accepted `0…9` and rejected `<0` / `>=10`.
  * `driver`: cross product of `good`'s three classes × `bad`'s three classes,
    plus the fixed ordering of the seven-part transcript.
  * **Sequencing / buffering shape:** several calls in one capture (ordering of
    `printf` output between the two printers), stdout block-buffered (redirected
    to a file, the default for the test capture) and unbuffered (`_IONBF`, used
    by the forked captures).

## Configuration table

Every row is run against **both** `.so`s through `dlopen`/`dlsym` and compared
byte-for-byte; rows marked *randomised* use ≥100 pseudo-random inputs from a
fixed seed (`splitmix64`, seeds noted in the test).

| #  | entry point(s) | configuration (options set + input shape) | test | [x] |
|----|----------------|-------------------------------------------|------|-----|
| 1  | `printLine` | empty string `""` (length 0) | `cfg_01_print_line_empty` | [x] |
| 2  | `printLine` | single byte, all 255 non-NUL byte values | `cfg_02_print_line_single_byte_all_values` | [x] |
| 3  | `printLine` | plain ASCII text, randomised length 1…64 and content | `cfg_03_print_line_random_ascii` *randomised* | [x] |
| 4  | `printLine` | arbitrary non-NUL byte strings incl. high/invalid-UTF-8 bytes, randomised length 1…256 | `cfg_04_print_line_random_bytes` *randomised* | [x] |
| 5  | `printLine` | content containing `%s %d %n %%` etc. (format-specifier data) | `cfg_05_print_line_format_specifiers` | [x] |
| 6  | `printLine` | content containing embedded `\n`, `\r`, `\t` | `cfg_06_print_line_embedded_newlines` | [x] |
| 7  | `printLine` | lengths straddling libc's stdout buffer: 4095, 4096, 4097, 8191, 8192, 8193, 65536 | `cfg_07_print_line_buffer_boundary_lengths` | [x] |
| 8  | `printLine` | interior pointer into a larger buffer + embedded NUL (truncation) | `cfg_08_print_line_interior_pointer` | [x] |
| 9  | `printIntLine` | fixed shapes: 0, 1, -1, 9, -9, 10, -10, 99999, `INT_MAX`, `INT_MIN` | `cfg_09_print_int_line_fixed_shapes` | [x] |
| 10 | `printIntLine` | uniformly random `int` over the whole 32-bit range | `cfg_10_print_int_line_random_full_range` *randomised* | [x] |
| 11 | `printIntLine` | one value per digit width 1…10, both signs | `cfg_11_print_int_line_digit_widths` | [x] |
| 12 | `bad` | accepted in-bounds indices, exhaustively `data = 0…9` | `cfg_12_bad_all_in_bounds` | [x] |
| 13 | `bad` | rejected `data < 0`, randomised over `[INT_MIN, -1]` | `cfg_13_bad_random_negative` *randomised* | [x] |
| 14 | `bad` | out-of-bounds `data`, exhaustively `10…64` (UB store; stdout compared) | `cfg_14_bad_oob_sweep` | [x] |
| 15 | `bad` | out-of-bounds `data`, randomised large positives (UB store; stdout compared) | `cfg_15_bad_oob_random` *randomised* | [x] |
| 16 | `good` | accepted `data = 0…9` exhaustively (`goodG2B` block + `goodB2G` block) | `cfg_16_good_all_in_bounds` | [x] |
| 17 | `good` | rejected `data`: `-1`, `INT_MIN`, `10`, `11`, `INT_MAX`, and randomised out-of-range | `cfg_17_good_out_of_range` *randomised* | [x] |
| 18 | `good` | randomised over the *whole* `int` range (mixes both classes) | `cfg_18_good_random_full_range` *randomised* | [x] |
| 19 | `driver` | cross product `goodData ∈ {-1, INT_MIN, 0, 7, 9, 10, INT_MAX}` × `badData ∈ {-1, INT_MIN, 0, 7, 9}` (all non-UB combinations, stdout + exit status) | `cfg_19_driver_cross_product` | [x] |
| 20 | `driver` | cross product with `badData` out of bounds `{10, 11, 12, 20, 40}` × the same `goodData` set (UB store; stdout compared) | `cfg_20_driver_cross_product_oob_bad` | [x] |
| 21 | `driver` | randomised `(goodData, badData)` in the accepted ranges | `cfg_21_driver_random_valid` *randomised* | [x] |
| 22 | `driver` | randomised `(goodData, badData)` over the whole `int` range, `badData` restricted to the non-crashing domain (`< 10` or the safe UB window) | `cfg_22_driver_random_full_range` *randomised* | [x] |
| 23 | sequencing: `printLine` → `printIntLine` → `bad` → `good` → `printLine` in **one** capture | interleaved output ordering / stdout buffering with fd 1 = file (block buffered) | `cfg_23_interleaved_sequence_block_buffered` | [x] |
| 24 | sequencing: same mixed call sequence in a forked child | stdout **unbuffered** (`_IONBF`) — flush-point independence | `cfg_24_interleaved_sequence_unbuffered` | [x] |
| 25 | repeated invocation | same call issued 50× in a row (no residual state between calls in either library) | `cfg_25_repeated_invocation_no_state` | [x] |
| 26 | `driver` vs hand-composed `printLine`+`good`+`bad` | the composed pipeline must equal the sum of the parts in both libraries | `cfg_26_driver_equals_composition` *randomised* | [x] |
| 27 | all 5 entry points | randomised *program* of 1…12 mixed calls (property-style fuzz of the whole API) | `cfg_27_random_call_program` *randomised* | [x] |
| 28 | all 5 entry points | every row above re-run under the `release` profile (`panic = "abort"`) and under `--no-default-features` (the only feature combination) | `run_all_configs.sh` | [x] |
| 29 | all 5 entry points | **both** `.so`s `dlopen`ed in one process, calls interleaved into one stdout stream (both interleavings) — each call must still produce exactly its stand-alone bytes, i.e. neither library's internal `printLine`/`printIntLine` calls are interposed by the other's identically named exports | `cfg_29_both_libraries_interleaved_in_one_process` | [x] |

## How each row is executed

`tests/common/mod.rs` loads both `.so`s with `libloading`
(`RTLD_NOW | RTLD_LOCAL`, so the two libraries can never satisfy each other's
relocations) and resolves the five exported symbols with `dlsym`.  Every call is
made through those function pointers; no Rust function is ever called directly.

Output is captured at the file-descriptor level in a `fork()`ed child (libtest
writes its own progress to fd 1 from other threads, so a process-global
redirection in the test thread is not safe).  A whole batch of calls runs in one
child, which records the stdout file offset after each call, letting the parent
recover the exact bytes of every individual call from a single `fork()`.

* rows in the well-defined domain → per-call bytes **and** process termination
  must be identical (`assert_same_batch` / `assert_same`);
* rows 14, 15, 20 (the deliberate out-of-bounds store) → `assert_same_stdout_ub`:
  identical streams when both processes survive, otherwise the shorter stream must
  be a prefix of the longer one.  See the closing section of `ERRORS.md` for why
  process termination cannot be required to match once the frame is smashed.
