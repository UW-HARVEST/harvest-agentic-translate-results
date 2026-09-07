# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

## Build commands

```
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libharvest-work-PjVoOU.so

cd translation && cargo build --release
# -> translation/target/release/libupdate_frame_header_lib.so
```

## `nm -D --defined-only` — C shared object

| symbol | type |
|--------|------|
| `update_frame_header` | `T` (global text) |

Raw output:

```
00000000000010f9 T update_frame_header
```

## `nm -D --defined-only` — Rust shared object

| symbol | type |
|--------|------|
| `update_frame_header` | `T` (global text) |

Raw output:

```
00000000000116b0 T update_frame_header
```

## Diff

```
$ comm -23 <(c symbols) <(rust symbols)
(empty)
```

**0 symbols missing from the Rust `.so`.**

The C library consists of exactly one translation unit (`c_src/src/lib.c`) and
exposes exactly one non-static function. `enum TFLAC_CHANNEL_MODE` is a
file-local enum in `src/lib.c` (not in the public header) and generates no
symbol. No macro-generated symbols exist. No whole module was skipped, so no
additional translation work was required for symbol parity.

## Undefined (imported) symbols

C `.so` undefined non-libc symbols: none.
Rust `.so` undefined non-libc symbols: none (only the usual libc/`ld` startup
and unwinder entries, verified with `nm -D -u`).

## Struct ABI parity

`struct tflac` layout, measured from the real C compiler
(`offsetof`/`sizeof` probe compiled against `c_src/include/lib.h`):

| field | C offset | Rust `#[repr(C)]` offset |
|-------|----------|--------------------------|
| `samplerate`    | 0  | 0  |
| `channels`      | 4  | 4  |
| `bitdepth`      | 8  | 8  |
| `channel_mode`  | 12 | 12 |
| `frame_header`  | 16 | 16 |
| `cur_blocksize` | 20 | 20 |
| **size / align** | 24 / 4 | 24 / 4 |

Layout parity is additionally enforced at runtime by the differential tests,
which hand the *same 24 raw bytes* to both `.so`s and compare all 24 output
bytes; any offset disagreement would show up as a divergence.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section** and no optional
dependencies, so the only build configuration is the default one. There is no
`[[bin]]` target and no `src/bin/`, so there is no driver executable to
stdout-compare.

## Completion gate (Phase D)

- [x] `nm -D` shows **0** missing / undefined non-libc symbols in the Rust `.so`
      (verified for both the `debug` and `release` artifacts by
      `run_all_configs.sh` and by `tests/phase_d_symbols.rs::phase_d_symbol_parity`).
- [x] Phase B: all **28** `CONFIGS.md` rows pass across randomized inputs
      (`tests/phase_b_valid_paths.rs`, 28 tests), plus a 10 000 000-iteration
      full-axis soak (`soak_10m_random`, `--ignored`) with zero divergences.
- [x] Binary/stdout comparison: **N/A** — the project builds no executable
      (`crate-type = ["cdylib"]` only; no `[[bin]]`, no `src/bin/`, and the
      CMake project builds only `add_library(... SHARED ...)`).
- [x] Phase C: all **16** `ERRORS.md` rows have a passing error-path
      differential test (`tests/phase_c_error_paths.rs`, 17 tests including the
      extra out-of-bounds-write guard test).
- [x] Holds under **every** configuration: there is exactly one feature
      combination (no `[features]`), verified under both the `debug` and
      `release` profiles — 47 tests × 2 profiles, all passing.

## Fix applied during verification

`update_frame_header` originally began with `let t = &mut *t;`. Creating a
reference from the raw pointer makes rustc emit a debug-profile null/alignment
UB check, so a `NULL tflac*` aborted (SIGABRT) in the debug build instead of
faulting like the C (SIGSEGV). The body was changed to read and write through
raw-pointer place expressions (`(*t).field`), and `wrapping_sub(1) << 4` was
made `wrapping_sub(1).wrapping_shl(4)` to state the C `unsigned int` shift
semantics explicitly. Release-profile behaviour is unchanged; debug now matches
the C except for the compiler-inserted UB check itself (see `ERRORS.md` row 15).

## Harness sensitivity (anti-vacuity check)

The differential harness was validated by mutation testing: 13 single-token
mutations were injected into `translation/src/lib.rs` and the suite re-run.

| mutation | failing tests |
|----------|---------------|
| block-size `8192 => 0x0D` → `0x0E` | 21 |
| dropped the `samplerate/1000 < 256` guard | 21 |
| `channel_mode % 4` → `% 5` | 29 |
| `wrapping_sub` → `saturating_sub` | 21 |
| `t.frame_header =` → `\|=` | 29 |
| `samplerate < 65536` → `<= 65536` | 2 |
| bit-depth `20 => 5` → `4` | 28 |
| sample-rate `882000` → `88200` | 22 |
| `samplerate/10 < 65536` → `<= 65536` | 3 |
| sync word `0xFFF8` → `0xFFF9` | 28 |
| `SIDE_RIGHT` nibble `0x09` → `0x0A` | 21 |
| bit-depth shift `<< 1` → `<< 2` | 28 |
| channel shift `wrapping_shl(4)` → `wrapping_shl(3)` | 26 |
| block-size `default`: `<= 256` → `< 256` | **0** |

The last one is a genuinely **equivalent mutant**, not a coverage gap: `256` is
itself an explicit `case` in the block-size `switch`, so the `default:` branch is
never reached with `cur_blocksize == 256` and the two predicates are
indistinguishable. Every non-equivalent mutation was caught.

## Reproducing

```
cd translation && ./run_all_configs.sh          # both profiles + all feature combos + nm parity
cd translation && cargo test --release          # 47 differential tests
cd translation && cargo test --release -- --ignored soak   # 10M-iteration soak
```
