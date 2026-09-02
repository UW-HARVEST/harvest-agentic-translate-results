# CONFIGS.md — Phase A configuration-surface table

Derived mechanically from `c_src/src/lib.c` and `c_src/include/lib.h`.

## Axes the C code actually branches on

There is **no options struct, no init/context object, no `#ifdef`, and no
compile-time feature flag** in this library, and `translation/Cargo.toml`
declares no `[features]`. The configuration surface is therefore made of the
*runtime arguments* that select code paths:

1. **`mode` selector** (`complexmode` arg 1) — `switch` with cases
   `1`, `2`, `3`, `4`, `default`. Five branches.
2. **Permission bitmask** (`safe_add` arg 3, `check_permissions` args) — the
   library's only "flag word". Distinct states: contains `0600`, missing
   `0400`, missing `0200`, missing both, zero, negative / high-bit set,
   `required == 0`, `required` a superset.
3. **Pointer nullity** on every pointer parameter (`op`, `src`, `op1`, `op2`,
   `log_msg`) — see ERRORS.md.
4. **Input shape for `copy_and_sum`**: `count` = 0 / 1 / 2 / 3 / many, plus
   negative and huge (ERRORS.md rows 11–14). The `int[3]` shape is the only one
   `complexmode` mode 3 ever produces.
5. **String shape for `create_result_string` / `compare_operations`**: empty,
   one char, many, high-bit (non-ASCII) bytes, embedded-`%` format-looking
   bytes, at/below/above the 64-byte truncation boundary, equal vs. differing
   vs. prefix-of.
6. **Integer value shape**: 0, ±1, small, `INT_MAX`, `INT_MIN`, values chosen to
   overflow `a+b`, `a*b`, `sum +=`, and `v1*v2+v3`.
7. **Entry-point level**: the six lower-level exports
   (`create_result_string`, `check_permissions`, `safe_add`,
   `multiply_with_log`, `copy_and_sum`, `compare_operations`) are called
   **directly** through the `.so`, not only through the `complexmode`
   convenience wrapper — the wrapper reaches only a fixed subset of their
   inputs (it hard-codes `permissions = 0644`, `op = "multiply"`, `count = 3`).

Every row below is compared **byte-for-byte on the return value, on every
out-parameter / heap buffer the call produces, and on the exact bytes the call
writes to stdout** (fd 1 is redirected to a temp file around each call), for
both the C `.so` and the Rust `.so`. Rows marked *randomized* run many
pseudo-random inputs from a fixed seed (SplitMix64, seed `0x5EED_1234_ABCD_F00D`).

## Rows

| #  | entry point(s) | configuration (options set + input shape) | randomized | [x] |
|----|----------------|-------------------------------------------|-----------|-----|
| 1  | `check_permissions` | `required == 0` (every `perms` accepted) | yes, 512 | [x] |
| 2  | `check_permissions` | `required` an exact subset of `perms` (accept) | yes, 512 | [x] |
| 3  | `check_permissions` | `required` shares only some bits with `perms` (reject) | yes, 512 | [x] |
| 4  | `check_permissions` | fully random `(perms, required)` incl. negatives / `INT_MIN` / `INT_MAX` / sign bit | yes, 4096 | [x] |
| 5  | `check_permissions` | the library's own constants: `perms=0644` × `required ∈ {0400,0200,0100,0600,0644,0777}` | no, exhaustive | [x] |
| 6  | `safe_add` | `perms` ⊇ `0600` (accept path), random `a`, `b` | yes, 4096 | [x] |
| 7  | `safe_add` | `perms` ⊇ `0600`, `a+b` overflows `INT_MAX` / underflows `INT_MIN` | yes, 512 + fixed extremes | [x] |
| 8  | `safe_add` | `perms` missing `0400` / missing `0200` / missing both / `0` / random-without-`0600` (reject path + stdout message) | yes, 2048 | [x] |
| 9  | `create_result_string` | `op` = empty string, `val` random | yes, 256 | [x] |
| 10 | `create_result_string` | `op` short ASCII, `val` random incl. `INT_MIN`/`INT_MAX` | yes, 2048 | [x] |
| 11 | `create_result_string` | `op` length sweep 0..80 — brackets the 64-byte `snprintf` truncation boundary (buffer compared up to and including the NUL; bytes past the NUL are left uninitialized by `snprintf` in both libraries and are therefore not comparable) | no, exhaustive sweep | [x] |
| 12 | `create_result_string` | `op` containing high-bit / non-ASCII / `%d`-looking bytes | yes, 512 | [x] |
| 13 | `multiply_with_log` | random `a`, `b`; compares return value **and** the full heap log string (bytes up to and including the NUL) | yes, 4096 | [x] |
| 14 | `multiply_with_log` | `a*b` overflows (incl. `INT_MIN * -1`, `INT_MAX * 2`) | yes, 512 + fixed extremes | [x] |
| 15 | `multiply_with_log` | `a*b == 0` (return value collides with the failure sentinel `0`) | no, fixed set | [x] |
| 16 | `copy_and_sum` | `count == 0`, non-NULL `src` | no | [x] |
| 17 | `copy_and_sum` | `count == 1` | yes, 512 | [x] |
| 18 | `copy_and_sum` | `count == 2` | yes, 512 | [x] |
| 19 | `copy_and_sum` | `count == 3` (the shape `complexmode` mode 3 uses) | yes, 2048 | [x] |
| 20 | `copy_and_sum` | `count` random in `4..=4096` ("many"), random element values | yes, 256 | [x] |
| 21 | `copy_and_sum` | `count` valid but element values chosen so `sum` overflows repeatedly | yes, 512 | [x] |
| 22 | `copy_and_sum` | `count` smaller than the buffer actually provided (no over-read) and `count` exactly the buffer length | yes, 256 | [x] |
| 23 | `compare_operations` | equal strings (incl. both empty) | yes, 1024 | [x] |
| 24 | `compare_operations` | differing at a random position — exact `strcmp` magnitude compared | yes, 4096 | [x] |
| 25 | `compare_operations` | one string a strict prefix of the other (length-difference path) | yes, 1024 | [x] |
| 26 | `compare_operations` | high-bit bytes — distinguishes signed vs. unsigned char comparison | yes, 2048 | [x] |
| 27 | `complexmode` | `mode == 1` (addition; `permissions=0644` ⇒ accept branch of `safe_add`), random `v1..v3` | yes, 4096 | [x] |
| 28 | `complexmode` | `mode == 1` with `v1+v2` overflowing | yes, 512 | [x] |
| 29 | `complexmode` | `mode == 2` (multiplication + log string; compares the `Mode 2:` stdout line, which embeds the heap string) | yes, 4096 | [x] |
| 30 | `complexmode` | `mode == 2` with `v1*v2` overflowing / zero | yes, 512 + fixed | [x] |
| 31 | `complexmode` | `mode == 3` (`int[3]` array sum via `copy_and_sum`) | yes, 4096 | [x] |
| 32 | `complexmode` | `mode == 3` with the 3-element sum overflowing | yes, 512 | [x] |
| 33 | `complexmode` | `mode == 4` (`check_permissions(0644,0100)` is false ⇒ `v1+v2+v3`, the `else` branch) | yes, 4096 | [x] |
| 34 | `complexmode` | `mode == 4` with `v1+v2+v3` overflowing | yes, 512 | [x] |
| 35 | `complexmode` | `mode ∈ {1,2,3,4}` × `v1,v2,v3 ∈ {0, 1, -1, INT_MAX, INT_MIN}` — full cross-product of extremes | no, exhaustive (4 × 125 = 500) | [x] |
| 36 | `complexmode` | `mode` invalid (`default:` branch) — see ERRORS.md rows 20–23 | yes, 4096 | [x] |
| 37 | `complexmode` | full random cross-product over all four args (`mode` unconstrained, so it mixes valid and invalid modes in one sweep) | yes, 8192 | [x] |
| 38 | composed pipeline | `create_result_string` → `compare_operations` on the two returned heap strings → `copy_and_sum` over the derived values, C-vs-C and Rust-vs-Rust chains compared end to end (catches divergence only visible when outputs feed inputs) | yes, 1024 | [x] |
| 39 | composed pipeline | `multiply_with_log` output string fed to `compare_operations` against `create_result_string("multiply", a*b)` — must be equal (`0`) in both libraries | yes, 1024 | [x] |
| 40 | cross-library | Rust-produced heap string compared against C-produced heap string with the *C* `compare_operations`, and vice-versa (proves the buffers are interchangeable across the ABI) | yes, 1024 | [x] |

## Feature combinations (Phase D)

`Cargo.toml` has no `[features]` table, so the complete set of combinations is:

| combo | command |
|-------|---------|
| default (= empty) | `cargo test --release` |
| `--no-default-features` | `cargo test --release --no-default-features` |
| `--all-features` | `cargo test --release --all-features` |

All three are run by `run_all_features.sh` and are identical by construction.
