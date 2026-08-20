# Verification report — C ⇄ Rust differential testing of `liblong.so`

Everything below was produced by loading **both** shared objects with
`libloading` (`dlopen`/`dlsym`) and calling only their exported symbols, so the
`#[unsafe(no_mangle)] extern "C"` wrappers are part of what is under test. Rust
functions are never called directly.

```
c_src/build/liblong.so      # cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
target/debug/liblong.so     # cargo build --no-default-features
target/release/liblong.so   # cargo build --no-default-features --release
```

## Phase A — artifacts

| file | content |
|------|---------|
| `SYMBOLS.md` | every `nm -D` symbol of the C `.so` mapped to the Rust `.so` (3 of 3 present; nothing missing, nothing extra, no stubs) |
| `ERRORS.md` | error-surface table, mechanically derived: the C has **no** error returns/asserts/range checks at all, so the 14 rows are the degenerate/boundary/hostile inputs this ABI can actually receive |
| `CONFIGS.md` | configuration-surface table: 20 rows over the axes the C branches on (entry point × element-value shape × written-subset × pass count × seed × observation channel × build profile × feature combo) |

## Phase A — feature combinations

`Cargo.toml` has **no `[features]` table**, so exactly one combination exists:

```
$ awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{print $1}' Cargo.toml
(empty)
$ cargo check --offline --no-default-features --all-targets   # OK
$ cargo check --offline --all-features        --all-targets   # OK (identical)
```

Because there is no feature axis, the *build-configuration* axis that does exist
— the dev vs. release profile (`opt-level=3`, `panic="abort"`,
`codegen-units=1`) — is treated as the equivalent axis and **every phase is run
against both profiles**.

## Phase B — valid-path differential results

| test binary | profile | result |
|-------------|---------|--------|
| `tests/phase_b_configs.rs` (18 tests, rows C1–C19) | debug | **18 passed** (177 s) |
| `tests/phase_b_configs.rs` (18 tests, rows C1–C19) | release | **18 passed** (99 s) |

Each row compares the **whole 1 MiB `array` object byte-for-byte** (and the XOR
reduction channel) after driving both libraries identically; random rows use a
fixed-seed SplitMix64 (64 + 32 + 16 + 8 … randomized trials, i.e. > 10^7
distinct element values per row).

## Phase C — error/boundary differential results

| test binary | profile | result |
|-------------|---------|--------|
| `tests/phase_c_errors.rs` (11 tests, rows E1–E14) | debug | **11 passed** (37 s) |
| `tests/phase_c_errors.rs` (11 tests, rows E1–E14) | release | **11 passed** (20 s) |

## Phase D — symbol parity

`tests/symbols.rs` (5 tests) passes for both profiles:

```
array                        B  0x100000   both, 32-byte aligned in both
long_exec                    T             both
perform_expensive_operations T             both
symbols missing from Rust .so: none
symbols exported by Rust .so beyond the C surface: none
ldd -r: no "undefined symbol" for either library
Rust .so imports libc srand / rand / printf, exactly like the C
```

## End-to-end `long_exec` (row C15 / E2–E4) — real 2000-iteration runs

`long_exec` performs 2000 × 262144 × 100 ≈ 5.2·10^10 arithmetic steps, i.e.
~700 s per library per seed, so these runs are `#[ignore]`d and were executed
individually (`tests/phase_e2e.rs`, release Rust `.so` vs C `.so`).
Recorded channels: the captured `printf("%d\n", …)` stdout bytes, the XOR of the
final array, the first and last four elements, and (seed 7) a full 1 MiB dump.

| seed | C stdout | Rust stdout | verdict |
|------|----------|-------------|---------|
| 42 | `430392287\n` | `430392287\n` | identical (also first4/last4/XOR) |
| 0 | `42032659\n` | `42032659\n` | identical |
| 4294967295 (`UINT_MAX`) | `494145113\n` | `494145113\n` | identical |
| 7 | `72337063\n` | `72337063\n` | identical, **including the full 1 MiB final `array` byte-for-byte** (`target/e2e_{c,rust}_seed7.bin`) |

## Fix applied to the Rust translation

* `array` was only 4-byte aligned, while gcc gives the 1 MiB C `.bss` array a
  32-byte alignment that a consumer of the exported data symbol can observe.
  Fixed by declaring the storage as
  `#[repr(C, align(32))] pub struct Array(pub [c_int; 256 * 1024])`.
  Asserted by `tests/symbols.rs::array_object_alignment_matches`.

No behavioural divergence was found in the arithmetic core: the Rust
`wrapping_mul`/`wrapping_add`/`wrapping_sub`/`wrapping_shl` + `>>` + `/` + `%`
sequence reproduces the C (including its signed-overflow, arithmetic-shift and
truncating-division behaviour) bit-for-bit. As an extra control, the C source
compiled at `-O0` and at `-O2` was compared against the Rust `.so` on the same
inputs — all three agree, so the result does not depend on how the UB in
`x*3+7` / `x-(x<<1)` is compiled.

## Independent cross-check of the arithmetic core

An independent model of `perform_expensive_operations`' inner loop (100 ×
`x*3+7`, `x^(x>>3)`, `x-(x<<1)`, `x/2 + x%7` with explicit two's-complement
wrapping, arithmetic shift and truncating division/modulo) was written in Python
and agrees with **both** libraries on every probe value:

```
f^100(0)           = -626538949      f^100(INT_MIN)     = -756415197
f^100(1)           = -1057168239     f^100(INT_MIN+1)   = -627633746
f^100(-1)          = -626500583      f^100(INT_MAX)     = -627633746
f^100(7)           = -822186310      f^100(INT_MAX-1)   = -988934373
f^100(-7)          = -626277382      f^100(2^30)        = -1043281421
f^100(14)          = -804838802      f^100(-2^30)       = -951240585
f^100(-14)         = -816976602
```

## Completion gate

- [x] `SYMBOLS.md`: `nm -D` shows 0 missing symbols and 0 undefined non-libc
      symbols in the Rust `.so` (both profiles).
- [x] Phase B: every one of the 20 `CONFIGS.md` rows passes across randomized
      inputs (18 test functions + the 4 end-to-end seeds), both profiles.
- [x] Phase C: every one of the 14 `ERRORS.md` rows has a passing error-path
      differential test (11 test functions), both profiles.
- [x] The single valid feature combination (`--no-default-features`, there is no
      `[features]` table) and both build profiles pass all of the above.

## Reproducing

```
scripts/run_all.sh        # symbol parity + Phase B + Phase C, both profiles
scripts/run_all.sh e2e    # …plus the multi-minute end-to-end rows
```
