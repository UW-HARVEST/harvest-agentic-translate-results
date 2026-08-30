# CONFIGS.md — Configuration surface table (valid inputs)

Mechanically derived from the C source. Axes the C code actually branches on:

* **Entry points** (the *full* set of exported symbols — see `SYMBOLS.md`):
  * `printLine(const char *line)` — the lowest-level entry point (`driver.c:30`).
    Not in the public header, but exported with external linkage, so it IS a public
    entry point and is driven **directly**, not only through `driver`.
  * `driver(int data)` — the one-shot wrapper declared in `driver.h`, which composes
    `memset` + `strncpy` + `printLine`.
* **Runtime options / modes / flags**: none. There is no global state, no setters, no
  `#ifdef`, no `switch`. (`grep -cE '#if|switch|static' c_src/src/driver.c` → 0
  configuration branches.)
* **Branch conditions**: exactly two — `line != NULL` (`driver.c:32`) and
  `data < 100` (`driver.c:44`).
* **Input shapes that the code distinguishes**:
  * for `printLine`: NULL vs non-NULL; empty vs 1 byte vs many bytes; where the NUL
    terminator sits; presence of `%` / high-bit / control bytes in the payload (they
    go through `printf`'s `%s`, so they must be passed through verbatim).
  * for `driver`: sign of `data`; `data == 0`; `0 < data < 99`; `data == 99`
    (largest in-branch); `data == 100` (boundary); `data > 100`; `INT_MAX`; `INT_MIN`.
    `data` also controls whether `strncpy` copies a NUL from `source`
    (it never does for `data <= 99`, since `source[0..99)` is all `'A'`).

Cross-product, pruned to the combinations the C treats differently. Every row is
exercised with **many randomized inputs** where the row has a value range
(fixed seed `0x5EED_1234_ABCD_0001`, xorshift64* PRNG, see `tests/common/mod.rs`).

| #   | entry point(s)          | configuration (options set + input shape)                                                                   | [x] |
|-----|-------------------------|--------------------------------------------------------------------------------------------------------------|-----|
| C1  | `printLine`             | non-NULL, empty string `""` (NUL at offset 0)                                                                | [x] |
| C2  | `printLine`             | non-NULL, single byte payload, randomized over every non-zero byte value `1..=255`                            | [x] |
| C3  | `printLine`             | non-NULL, random length `2..=98` of random non-zero bytes, NUL-terminated (randomized, many samples)          | [x] |
| C4  | `printLine`             | non-NULL, exactly 99 bytes of `'A'` + NUL — the shape `driver` produces for `data == 99`                      | [x] |
| C5  | `printLine`             | non-NULL, long payload (256 / 1024 / 4096 bytes) with random bytes, NUL-terminated                            | [x] |
| C6  | `printLine`             | non-NULL, payload containing `printf` format metacharacters (`%s %n %d %%`) — must be passed through as data  | [x] |
| C7  | `printLine`             | non-NULL, payload containing high-bit / non-UTF-8 bytes (`0x80..0xFF`) and control bytes (`\t \r \x01`)        | [x] |
| C8  | `printLine`             | non-NULL, early-NUL payload: bytes after the first NUL must be ignored (buffer has trailing garbage)           | [x] |
| C9  | `printLine`             | called repeatedly in a loop (10 calls) — checks no hidden state / buffering divergence                        | [x] |
| C10 | `driver`                | `data == 0` — in-branch, zero-length `strncpy`                                                                | [x] |
| C11 | `driver`                | `data == 1` — in-branch, minimal non-empty copy                                                               | [x] |
| C12 | `driver`                | `data` randomized in `2..=98` — in-branch, generic copy (many randomized samples, all values also swept exhaustively) | [x] |
| C13 | `driver`                | `data == 98`, `data == 99` — in-branch upper edge; `99` fills `source` exactly with no NUL copied              | [x] |
| C14 | `driver`                | `data == 100` — first out-of-branch value (boundary)                                                          | [x] |
| C15 | `driver`                | `data == 101`, `1000`, randomized `101..=INT_MAX` — out-of-branch, oversized                                  | [x] |
| C16 | `driver`                | `data == INT_MAX` — out-of-branch extreme                                                                     | [x] |
| C17 | `driver`                | exhaustive sweep of the entire in-branch domain `data ∈ 0..=99` (all 100 values)                              | [x] |
| C18 | `driver`                | exhaustive/dense sweep of the out-of-branch domain `data ∈ 100..=400` plus random large values                 | [x] |
| C19 | `driver`                | called repeatedly with alternating in-branch / out-of-branch values (composed pipeline, no state leak)         | [x] |
| C20 | `driver` → `printLine`  | composed pipeline check: output of `driver(n)` for `n ∈ 0..=99` equals output of `printLine` on a buffer of `n` `'A'`s (verifies the internal composition, both libs) | [x] |
| C21 | `driver` (subprocess)   | `data < 0`: randomized negatives, `-1`, `INT_MIN` — UB path, compared by exit signal + stdout in a child process | [x] |
| C22 | both, subprocess        | stdout redirected to a **file** (fully-buffered stream) vs **pipe** — buffering/flush behaviour must match      | [x] |
| C23 | both, subprocess        | stdout switched to **unbuffered** (`setvbuf(stdout, NULL, _IONBF, 0)`) — the mode in which the concrete libc writer chosen by the compiler (`printf` vs the LLVM/GCC `puts` rewrite) becomes observable | [x] |

## Feature combinations

`translation/Cargo.toml` has no `[features]` section, so the complete set of feature
combinations is `{ default }` == `{ --no-default-features }`. Both are run by
`run_all.sh`; every row above is verified under each.

## Test mapping

| rows | test |
|------|------|
| C1–C9   | `tests/phase_b_configs.rs::c1_*` … `c9_*` (`printLine` driven directly) |
| C10–C20 | `tests/phase_b_configs.rs::c10_*` … `c20_*` (`driver`, incl. exhaustive sweeps) |
| C21     | `tests/phase_c_errors.rs::e7_driver_negative_ub_matches` (out of process) |
| C22–C23 | `tests/phase_b_configs.rs::c22_buffering_pipe_vs_file` |

## Harness sensitivity (mutation checks performed)

The differential harness was validated by deliberately mutating the Rust source and
confirming the suite fails, then reverting:

| mutation in `translation/src/lib.rs` | caught by |
|---|---|
| `if data < 100` → `if data <= 100` | `c14_driver_boundary_100`, `c18_driver_dense_out_of_branch`, `c22_buffering_pipe_vs_file` |
| removed the `line != NULL` check in `printLine` | `e1_print_line_null` |
| `if data < 100` → `if data < 100 && data >= 0` ("fixing" the UB) | `e7_driver_negative_ub_matches`, `e8_driver_int_min` |

`translation/src/lib.rs` is byte-identical to its pre-mutation state (all mutations
reverted; final full run is green).

Note: `cargo test` does **not** rebuild the `cdylib` (no test target links it), so the
harness contains a staleness guard (`tests/common/mod.rs::assert_not_stale`) that fails
loudly if `target/<profile>/libdriver.so` is older than any `src/*.rs`. Always run
`./run_all.sh` (or `cargo build` before `cargo test`).
