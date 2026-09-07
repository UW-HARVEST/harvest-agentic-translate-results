# CONFIGS.md — Configuration-surface table

## Axes actually present in the C source

Mechanically derived from `c_src/include/driver.h` + `c_src/src/driver.c`:

- **Public entry points:** exactly one — `void driver(int x)`. There is no
  higher-level convenience wrapper and no lower-level helper; `driver` *is* the
  lowest-level entry point. No `static` helpers exist either.
- **Runtime options / modes / flags:** none. No context struct, no init/destroy,
  no setters, no globals, no environment reads, no `#ifdef` in either file
  (`grep -c '#if' c_src/src/driver.c` → 1, the header's own include guard only).
- **Hidden state:** none. `y` is a function-local `register int`; the function is
  pure apart from writing to `stdout` via `printf("%d\n", …)`.
- **Input shapes:** the single scalar `int x`. The code has **zero branches**, so
  the only shapes the *behaviour* can distinguish are arithmetic ones:
  1. sign of `x` (negative / zero / positive)
  2. magnitude class of `x`: `2*x` in range vs. `2*x` overflowing `int`
  3. magnitude class of `2*x+300`: in range vs. overflowing `int`
  4. sign / digit-count of the *printed* result, i.e. whether `printf("%d")`
     emits a leading `-` and how many digits (formatting shape)
  5. `2*x+300 == 0` (the root, `x == -150`) — the only value printing `0`
- **Output shape:** bytes written to `fd 1` by libc `printf`. Compared
  byte-for-byte (including the trailing newline and any `-` sign) by capturing
  `fd 1` around each call.
- **Feature combinations:** `translation/Cargo.toml` declares **no `[features]`
  section**, so the only build configuration is the default one (verified by
  `cargo metadata`; the sweep is automated in `verify_all.sh`). Both `--no-default-features`
  and the default build are exercised.

## Configuration rows (cross-product, pruned to what the C distinguishes)

Every row is driven through the `.so` exports of *both* libraries with many
randomized inputs (seeded splitmix64 PRNG, fixed seed) plus the row's boundary values, and
the captured stdout bytes are compared.

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|-------------------------------------------|-----|
| 1  | `driver` | `x == 0` (zero / degenerate input; result `300`, 3 digits, no sign) | [x] |
| 2  | `driver` | `x` small positive, `1..=1000`; no overflow; positive result | [x] |
| 3  | `driver` | `x` small negative, `-1000..=-1`; no overflow; result crosses zero → mixed sign/formatting | [x] |
| 4  | `driver` | `x == -150` exactly: `2*x+300 == 0`; result prints as `0` | [x] |
| 5  | `driver` | `x` in `-149..=-1` → small positive result (1–3 digits, no sign) | [x] |
| 6  | `driver` | `x` in `-1000..=-151` → negative result (leading `-`) | [x] |
| 7  | `driver` | `x` mid-range positive (`2*x` and `2*x+300` both in range): random in `1001..=1_073_741_672` | [x] |
| 8  | `driver` | `x` mid-range negative (no overflow): random in `-1_073_741_673..=-1001` | [x] |
| 9  | `driver` | `x == 1_073_741_672` / `1_073_741_673`: last inputs with `2*x+300 <= INT_MAX`. Note `2*x+300` is always even, so the max attainable result is `INT_MAX-1 = 2147483646` | [x] |
| 10 | `driver` | `x == 1_073_741_674` … `1_073_741_823` (`= INT_MAX/2`): `2*x` in range but `2*x+300` overflows positive → wraps negative | [x] |
| 11 | `driver` | `x == 1_073_741_824` … `INT_MAX`: `2*x` itself overflows positive; random draws plus both endpoints | [x] |
| 12 | `driver` | `x == -1_073_741_824` (`= INT_MIN/2`) … `-1_073_741_674`: `2*x` at/inside the negative limit | [x] |
| 13 | `driver` | `x == -1_073_741_825` … `INT_MIN`: `2*x` overflows negative → wraps positive; random draws plus both endpoints | [x] |
| 14 | `driver` | full-width random `int` over the entire `i32` range (uniform bit patterns, 2000 draws) — hits all of the above classes without bias | [x] |
| 15 | `driver` | 1-digit / 2-digit / … / 10-digit result widths, and both signs, swept explicitly (`printf("%d")` formatting shape) | [x] |
| 16 | `driver` | repeated invocation: the same value called many times in a row, and an interleaved C/Rust call sequence (no hidden state, no init order dependency) | [x] |
| 17 | `driver` | many calls without an intervening `fflush`, so several results accumulate in the libc `stdout` buffer before comparison (buffering / output-accumulation shape) | [x] |
| 18 | `driver` | default build (`cargo test`) — the only feature configuration; also run with `--no-default-features` and `--release` | [x] |

## Additional rows added while verifying

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|--------------------------------------------|-----|
| 19 | `driver` | ground-truth pinning: the nine boundary values of `ERRORS.md` compared against the **exact expected byte strings** (not just C == Rust), so a coordinated regression in both cannot hide (`errors_expected_exact_bytes`) | [x] |
| 20 | `driver` | C `.so` compiled at `-O0` (the `CMakeLists.txt` default) **and** at `-O2 -DNDEBUG` — confirms the two's-complement wrap on signed-overflow inputs is what the C actually does at both optimisation levels, so the Rust `wrapping_*` translation matches either build (`verify_all.sh`) | [x] |
| 21 | `driver` | Rust cdylib built in `debug` **and** `release` (`release` also has `panic = "abort"`) — both loaded via `libloading` (`verify_all.sh`) | [x] |
| 22 | `driver` | **exhaustive**: every one of the 2^32 possible `int` arguments, sharded across parallel workers (`exhaustive.sh`) | [x] |

## Notes

- There is **no binary/driver executable target** in either project
  (`c_src/CMakeLists.txt` builds only `add_library(driver SHARED …)`, and
  `translation/Cargo.toml` has only a `[lib]` with `crate-type = ["cdylib"]`),
  so the "compare the two binaries' stdout" step does not apply. The equivalent
  comparison — the bytes each `.so` writes to fd 1 — is what every row above
  asserts.
