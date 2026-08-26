# CONFIGS.md — configuration-surface table (Phase A / Phase B)

## Build-time configuration axes

* `Cargo.toml` has **no `[features]` table** → the feature power set is the
  single empty combination. `cargo check/test --no-default-features` and the
  plain default build are therefore the same configuration.
* `Cargo.toml` *does* declare `[profile.release] panic = "abort"`, so **debug**
  (`overflow-checks` on, unwinding) and **release** (optimised, `panic = abort`)
  are two genuine Rust build configurations of the same code.
* `c_src/CMakeLists.txt` has **no `option()`, no `#ifdef`-driven flags, no
  `target_compile_definitions`** — a single `SHARED` library from the single
  source file `src/lib.c`, so there are no C feature toggles. `CMAKE_BUILD_TYPE`
  is still an axis, because it changes the optimisation level and the C relies on
  wrap-around signed arithmetic that an optimiser is in principle free to
  exploit (`safe_add`, `multiply_with_log`, the `copy_and_sum` accumulator and
  both `complexmode` case-4 arms).

`check_all_features.sh` therefore runs the whole suite for:

| axis | values covered |
|------|----------------|
| Cargo features | `--no-default-features --features ''` (the only combination) and the default build |
| Rust profile | `dev` and `release` (`panic = "abort"`, optimiser on) |
| C `CMAKE_BUILD_TYPE` | unset (`-O0`, the documented build), `Debug`, `Release`, `RelWithDebInfo`, `MinSizeRel` — built outside `c_src/`, which is never modified, and selected with `CDIFF_C_SO=` |

Every row below is exercised in every one of those configurations.

## Runtime configuration axes actually branched on by the C

| axis | values the C distinguishes | where |
|------|---------------------------|-------|
| A. `mode` selector of `complexmode` | `1`, `2`, `3`, `4`, anything else (`switch`) | L115-171 |
| B. permission word / required mask | `(perms & required) == required` — required `0`, exact, subset, disjoint; the macros `READ_PERM 0400`, `WRITE_PERM 0200`, `EXEC_PERM 0100`, the hard-coded `0644` | L28-30, L48, L52, L154 |
| C. `safe_add` permission gate | granted (`perms & 0600 == 0600`) vs denied | L52 |
| D. `copy_and_sum` element count shape | `0` (empty), `1`, `3` (the shape `complexmode` uses), many | L73-84 |
| E. integer value shape | zero, positive, negative, `INT_MIN`, `INT_MAX`, operands whose `+`/`*` wraps | all arithmetic |
| F. `create_result_string` op-string shape | empty, short, exactly filling the 64-byte buffer, longer (truncated), `NULL` (`(null)` via `%s`), bytes ≥ 0x80 | L39-44 |
| G. `compare_operations` string-pair shape | identical, differ at byte 0, differ inside, prefix (both orders), empty vs non-empty, bytes ≥ 0x80 (unsigned compare), long | L90-97 |
| H. `complexmode` case-2 log branch | `log_message` non-empty (normal) vs `NULL`/`""` (see ERRORS.md row 11) | L131-136 |
| I. `complexmode` case-4 exec-bit branch | `check_permissions(0644, 0100)` — always false, so the `v1+v2+v3` arm is taken (the `v1*v2+v3` arm is dead code that must stay dead) | L154-158 |
| J. entry-point level | all 7 exported functions are driven **directly** through the `.so`, not only through the `complexmode` one-shot wrapper | — |
| K. heap contents | two buffers are read after only a partial write — the 64-byte `create_result_string` block past the NUL, and `Result.operation[32]` past the `strcpy` — so "what `malloc` handed back" is an input shape: all-zero (fresh pages) vs a non-zero fill | L39/43, L105/113-152/174 |
| L. allocation request sizes | `malloc(64)`, `malloc(sizeof(Result))` = 40, and `malloc(count * sizeof(int))` with `count` converted to `size_t`; a wrong conversion can request a different size yet produce the same visible outcome | L39, L73, L105 |

Every row is checked with many pseudo-random inputs (fixed seed, SplitMix64) plus
the listed boundary values, comparing the return value **and** the bytes written
to `stdout` (captured by `dup2`-ing fd 1) between the C `.so` and the Rust `.so`.

## Rows

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|-------------------------------------------|-----|
| 1  | `check_permissions` | `required == 0` (vacuously satisfied), random `perms` | [x] |
| 2  | `check_permissions` | `required == perms` (exact match), random | [x] |
| 3  | `check_permissions` | `required` is a strict subset of `perms` (superset grant) | [x] |
| 4  | `check_permissions` | `required` partially overlaps `perms` (at least one bit missing) | [x] |
| 5  | `check_permissions` | `required` fully disjoint from `perms` | [x] |
| 6  | `check_permissions` | sign-bit / boundary words: `0`, `-1`, `INT_MIN`, `INT_MAX` cross-product | [x] |
| 7  | `check_permissions` | the library's own macro values cross-product: `{0,0100,0200,0400,0600,0644,0777}` × same | [x] |
| 8  | `safe_add` | perms grant read+write (`0600` bits present, e.g. `0600`, `0644`, `0777`, `-1`), random `a`,`b` | [x] |
| 9  | `safe_add` | perms missing WRITE (`0400`, `0500`) → denial path, random `a`,`b` | [x] |
| 10 | `safe_add` | perms missing READ (`0200`, `0300`) → denial path | [x] |
| 11 | `safe_add` | perms missing both (`0`, `0100`, `0077`) → denial path | [x] |
| 12 | `safe_add` | granted + wrapping sums: `INT_MAX+1`, `INT_MIN-1`, `INT_MAX+INT_MAX`, `INT_MIN+INT_MIN` | [x] |
| 13 | `copy_and_sum` | `count == 1`, random element | [x] |
| 14 | `copy_and_sum` | `count == 3` (the shape used by `complexmode` mode 3), random elements | [x] |
| 15 | `copy_and_sum` | `count == 0` → `malloc(0)`, empty loop | [x] |
| 16 | `copy_and_sum` | `count` many: 2, 7, 64, 255, 256, 1024, 4096, 65536 random elements | [x] |
| 17 | `copy_and_sum` | elements chosen so the running `int` sum wraps (all `INT_MAX`, all `INT_MIN`, mixed extremes) | [x] |
| 18 | `multiply_with_log` | random `a`,`b`; asserts return value **and** the `char*` log string bytes | [x] |
| 19 | `multiply_with_log` | `a == 0` / `b == 0` / both `0` | [x] |
| 20 | `multiply_with_log` | wrapping products: `INT_MAX*INT_MAX`, `INT_MIN*-1`, `65536*65536`, `INT_MIN*INT_MIN` | [x] |
| 21 | `create_result_string` | `op` empty / short ASCII, `val` random (positive and negative) | [x] |
| 22 | `create_result_string` | `op == NULL` → glibc `%s` prints `(null)` | [x] |
| 23 | `create_result_string` | `op` length tuned so output is 62/63/64/65 bytes → exact-fit and truncation boundary of the 64-byte buffer | [x] |
| 24 | `create_result_string` | `val` boundaries `0`, `-1`, `INT_MIN`, `INT_MAX` (widest `%d` expansions, shrinks room for `op`) | [x] |
| 25 | `create_result_string` | `op` containing bytes ≥ 0x80 and embedded digits/`%` characters | [x] |
| 26 | `compare_operations` | identical strings, incl. both empty | [x] |
| 27 | `compare_operations` | differ at byte 0 (both orderings) | [x] |
| 28 | `compare_operations` | differ at an interior byte (both orderings) | [x] |
| 29 | `compare_operations` | one string a strict prefix of the other (both orderings) | [x] |
| 30 | `compare_operations` | bytes ≥ 0x80 vs < 0x80 → unsigned-char comparison sign | [x] |
| 31 | `compare_operations` | random byte strings, random lengths 0..64, over many seeds | [x] |
| 32 | `complexmode` | `mode == 1`, random `value1`,`value2` (`value3` ignored) | [x] |
| 33 | `complexmode` | `mode == 1`, wrapping `value1+value2` boundaries | [x] |
| 34 | `complexmode` | `mode == 2`, random `value1`,`value2` (log-string print path) | [x] |
| 35 | `complexmode` | `mode == 2`, wrapping product boundaries, incl. products whose `%d` text is longest | [x] |
| 36 | `complexmode` | `mode == 3`, random `value1..value3` (array-of-3 copy+sum) | [x] |
| 37 | `complexmode` | `mode == 3`, wrapping sum boundaries | [x] |
| 38 | `complexmode` | `mode == 4`, random `value1..value3` (exec bit absent → `v1+v2+v3` arm) | [x] |
| 39 | `complexmode` | `mode == 4`, wrapping sum boundaries | [x] |
| 40 | `complexmode` | `mode` sweep over the whole switch neighbourhood `-2..=7` incl. the four valid arms and the `default:` arm | [x] |
| 41 | `complexmode` | fully random 4-tuples `(mode, v1, v2, v3)` with `mode` drawn from `1..=4` and from arbitrary `i32` | [x] |
| 42 | all 7 entry points | stdout bytes compared for every row above (`dup2` capture around each call), not just return values | [x] |
| 43 | `create_result_string` → `multiply_with_log` → `complexmode` | composed pipeline: string produced by the low-level entry point is fed to `compare_operations`, and the pointer is freed with the C runtime's `free` — cross-checks that both `.so`s hand back blocks from the same allocator | [x] |
| 44 | all 7 entry points | **non-zero heap**: every fresh `malloc` block pre-filled with each of 0x01/0x2c/0x30/0x7f/0x80/0xab/0xff, plus glibc `MALLOC_PERTURB_`; all 64 bytes of every returned block are hex-dumped so the untouched tail past the NUL is compared, and `Result.operation[32]` is validated through the `Operation performed: %s` line. Also combined with each armed allocation failure. | [x] |
| 45 | `create_result_string`, `multiply_with_log`, `copy_and_sum`, `complexmode` | **exact `malloc` request-size sequence** per call, logged by the interposer: 64 for the log string, 40 for the tracker, and `count * sizeof(int)` for counts `0,1,3,17,64,-1,-2,-3,-17,-1024,-65536,-2^30,INT_MIN,INT_MIN+1` (sign-extended, e.g. `-1` → 18446744073709551612). Catches size-computation divergences that share the same visible outcome. | [x] |

## Why rows 44 and 45 exist (mutation evidence)

Rows 1-43 compare return values and stdout on a *fresh* heap, which leaves two
blind spots.  Both were confirmed by deliberately mutating `src/lib.rs` and
re-running the suite:

| mutant | rows 1-43 | row 44 | row 45 |
|--------|-----------|--------|--------|
| `strcpy_lit` copies `len - 1` bytes (drops the NUL terminator) | **all 44 pass** | FAILS (caught) | – |
| `snprintf` bound 64 → 63 | FAILS (row 23) | FAILS | – |
| `complexmode` case-4 branch inverted | FAILS (row 38/39) | FAILS | – |
| `safe_add` mask `0o600` → decimal `600` | FAILS (rows 8-11) | – | – |
| `count as isize as usize` → `count as u32 as usize` (zero- instead of sign-extension) | **all 44 pass** | pass | FAILS (caught) |

The first mutant is invisible without a non-zero heap because the byte after a
short `strcpy` is already `0` on fresh pages; the last is invisible without the
size log because both request sizes are far too large to satisfy, so both
implementations print `Memory allocation failed` and return `-1`.

## Divergence found and fixed (release configuration)

Running the matrix against the **optimised** Rust build exposed a real
divergence that the debug build could not show:

| | C (`-O0` … `-O3`) | Rust before fix (`--release`) |
|---|---|---|
| `complexmode(3, 6, 7, 8)` while `malloc(12)` fails | `Memory allocation failed` / `Mode 3: Array Sum` / `Result: -1`, returns `-1` | `Mode 3: Array Sum` / `Result: 21`, returns `21` |

Cause: `copy_and_sum` is inlined into `complexmode`'s mode-3 arm, after which its
buffer no longer escapes, so LLVM deleted the `malloc`/`free` pair. With the
allocation gone the `dest == NULL` branch became unreachable and the C's
allocation-failure path simply did not exist in the Rust `.so`. Row 45's
request-size log caught it first (`SIZES cm 3: [40,12]` vs `[40]`), and the
behavioural difference was then confirmed directly.

Fix: `src/lib.rs` routes all three `malloc()` results through
`keep_allocation()` (a `core::hint::black_box` identity function), so every
allocation — and therefore every `NULL` check the C performs — survives at all
optimisation levels. Regression test:
`error_paths.rs::row05c_copy_and_sum_allocation_failure_through_complexmode`.

This is why the suite is run for every configuration rather than just the
default: the code is identical, but the observable behaviour was not.
