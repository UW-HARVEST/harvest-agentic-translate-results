# Verification report — C `lib.c` vs Rust `libconfusion_lib.so`

The C implementation is the ground truth. Every test loads **both** shared
objects with `libloading`/`dlopen` and calls them only through their exported
`extern "C"` symbols, so the Rust `#[unsafe(no_mangle)]` wrappers are part of
what is verified. Nothing is called directly against the Rust crate.

## How to reproduce

```bash
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .

cd translation
./run-diff-tests.sh              # all 6 configurations, Phases A–D
./check-suite-detects-bugs.sh    # mutation check: proves the suite is not vacuous
cargo test --offline -- --ignored   # optional 20 000-case soak
```

`cargo test` alone does **not** build a `cdylib`, so each configuration runs
`cargo build` first; the harness refuses to fall back to the other profile's
object rather than silently test the wrong artifact.

## What is compared

For every scenario, on both sides:

| observable | how |
|---|---|
| return values | every call's result is recorded |
| `ProcessState` contents | the 32-bit bit-field word (incl. the otherwise invisible `status` and `reserved` fields), the union word, `capacity`, and the buffer's bytes, read out of the malloc'd struct at the gcc-verified offsets 0/4/8/16 |
| **stdout** | fd 1 is redirected into a file around each call and compared byte for byte — this is most of the library's observable behaviour (`printf`/`puts` of `%d`, `%u`, `%f`, and the `STRINGIFY`-generated `DEBUG_VAR`/`LOG_OPERATION` text) |
| allocator behaviour | `malloc`/`free` counts and total bytes, via an `LD_PRELOAD` shim — rules out leaks, double frees, differently-sized allocations |
| exported symbols | `nm -D` diff, at test time |

The heap *address* of `buffer` is deliberately not compared, and the buffer's
*bytes* are not compared when `capacity == 0`, because `snprintf(buf, 0, …)`
writes nothing and the C leaves them indeterminate.

## Results

| phase | artifact | tests | result |
|---|---|---|---|
| A | `SYMBOLS.md` | `phase_d_symbols.rs` (3) | 6/6 symbols exported by both; symbol diff **empty**; 0 non-libc undefined symbols |
| B | `CONFIGS.md` (34 rows) | `phase_b_valid.rs` (34 + 1 soak) | 34/34 rows pass across randomized inputs |
| C | `ERRORS.md` (26 rows) | `phase_c_errors.rs` (31) | 26/26 rows pass, plus 4 generic-boundary tests and the allocator-parity test |
| D | feature/profile matrix | all of the above | 6/6 configurations pass |

Plus `harness_selfcheck.rs` (6 tests) which asserts the harness itself is sound:
two distinct objects are loaded, the profile-matching `.so` is used, captured
stdout is non-empty and contains the exact text the C prints, captures do not
leak into one another, the assumed struct layout matches what the C library
actually writes, and the differential driver *does* fail when the two sides
disagree.

Totals: **74 tests** per configuration, 6 configurations, plus a 20 000-case
soak run on both the dev and release objects.

## Mutation check

`check-suite-detects-bugs.sh` builds ten deliberately-wrong copies of
`src/lib.rs` and runs the suite against each. All ten are caught:

| injected bug | caught by |
|---|---|
| `(int)float` saturates instead of `cvttss2si`'s `INT_MIN` | 14 tests |
| `status` bit-field at offset 12 instead of 11 | 20 tests |
| `counter` 4 bits wide instead of 5 | 3 tests |
| `create_state` sets `mode = 2` instead of 3 | 29 tests |
| one `printf` format string altered | 9 tests |
| `& 0x7F` instead of `& 0xFF` in `confuse_types(2)` | 15 tests |
| `bytes[0]+bytes[1]` summed as *unsigned* chars | 15 tests |
| off-by-one in the `process_buffer` `memchr` loop | 14 tests |
| `capacity` zero-extended instead of sign-extended into `size_t` | 3 tests |
| `switch` fall-through returns 1 instead of 0 | 12 tests |

## Behavioural details the translation has to get right

Confirmed against the gcc-generated code (`objdump -d`) and a struct-layout
probe, and locked in by tests:

* **Bit-field layout.** `PackedFlags` is one 32-bit unit, allocated LSB-first:
  `flag1`@0, `flag2`@1, `flag3`@2, `counter`@3..7, `mode`@8..10,
  `status`@11..15, `reserved`@16..31. `create_state` assigns every bit, so the
  word is deterministically `0x00007B05` even though it read-modify-writes
  uninitialised `malloc` storage.
* **`ProcessState`** is 24 bytes, align 8, offsets 0/4/8/16.
* **`char` is signed** on x86-64 Linux: `confuse_types(3)` sums two `i8`s and
  `printf("%d", …)` sign-extends them; `process_buffer`'s `target` spans
  `-128..127` and `memchr` compares `(unsigned char)c`.
* **`capacity` is sign-extended** into `size_t` (`cdqe` / `movslq`), so a
  negative capacity becomes an enormous `malloc`/`snprintf` size, which fails.
* **`(int)(float_val * 100)`** is `mulss` then `cvttss2si`: `f32` arithmetic,
  and the "integer indefinite" result `INT_MIN` for NaN, ±Inf and anything
  outside `int32`. Rust's `as` would saturate, so the translation implements the
  hardware rule explicitly. The `f32` grid makes the bound exact: the largest
  `f32` below `2^31` is `2147483520`, and `-2^31` is itself representable.
* **`param >> 3` is an arithmetic shift**, so negative `param` still yields
  `mode = (param >> 3) & 7`.
* **`%` truncates toward zero**, so `param3 % 10` and `param4 % 4` can be
  negative — and a negative `operation` hits the `switch`'s (absent) default,
  returning 0 and printing nothing.
* **All I/O and allocation go through the real glibc** `printf`/`snprintf`/
  `malloc`/`free`/`strlen`/`memchr`, which is what makes `%f`/`%u`/`%d`
  formatting and the allocator ABI byte-identical. (gcc lowers
  `printf("literal\n")` to `puts`; the output bytes are the same.)

## Notes / limits

* Two rows are documented as not reachable through the public ABI and are
  handled honestly rather than hand-waved:
  * **E1/E3/E23** (allocation failure) are driven *for real* with an
    `LD_PRELOAD` shim that fails exactly one `malloc` of a chosen size inside a
    child process — see `tests/fixtures/oom_{preload,driver}.c`.
  * **E26** (signed overflow of `result` inside `confusion`) is proved
    unreachable by a bound argument (see `ERRORS.md`); the test drives both
    reachable extremes instead, and the Rust uses `wrapping_*` to match gcc.
* `capacity == 0` leaves the buffer's bytes indeterminate in the C, so those
  bytes are excluded from the comparison (everything else about that call is
  compared).
* The tests must run sequentially: capturing fd 1 is process-wide. This is
  enforced by `.cargo/config.toml` (`RUST_TEST_THREADS = 1`) *and* asserted by
  the harness, so a parallel run fails loudly instead of comparing corrupted
  captures.
