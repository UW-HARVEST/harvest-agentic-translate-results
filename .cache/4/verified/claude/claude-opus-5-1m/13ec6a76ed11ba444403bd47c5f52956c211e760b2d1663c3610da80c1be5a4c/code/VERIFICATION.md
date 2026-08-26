# VERIFICATION.md — differential verification of the LZ4 C→Rust translation

The C in `c_src/` is the ground truth and was **never modified** (verified: every file
under `c_src/src/` and `c_src/include/` still carries its original checkout timestamp).
Where the C behaves surprisingly, the Rust was changed to match it — never the reverse.

## How the tests work

Every test loads **both** shared libraries with `libloading` and calls their exported
symbols through the dynamic symbol table:

| | path |
|---|---|
| C | `c_src/build/liblz4.so` |
| Rust | `target/release/liblz4.so` |

Rust functions are **never** called directly — no test file imports the crate
(`grep -c 'use lz4::\|extern crate lz4' tests/*.rs` → 0). Symbols are obtained only via
the harness's `both("name") -> (c_fn, rust_fn)` helper (411 call sites) or `lib.sym()` for
the few cases that must drive one library at a time (custom-allocator failure injection,
per-library heap-overrun probing). This exercises the `#[no_mangle]` / `extern "C"` export
wrappers exactly as an external C caller would.

Shared harness: `tests/common/mod.rs` — dual-library loader, a deterministic
xoshiro256\*\* PRNG (fixed seeds, so every run is reproducible), six input-shape
generators (`random`, `constant`, `periodic`, `text`, `mixed`, `selfref`), an
`AlignedBuf` that can produce deliberately **mis**aligned pointers, byte-diff assertions,
and `#[repr(C)]` mirrors of every public struct plus all `LZ4F_ERROR_*` numeric values.

**Harness rule enforced throughout:** the C destination buffer and the Rust destination
buffer are pre-filled with the *same* sentinel byte (`0xAA`) and the **full** buffer is
compared — not just the reported length. Using different fills produces false positives on
untouched bytes (a mistake made and fixed early); using the same fill means a write past
the reported length is still detected.

## Results

```
14 test binaries, 250 tests, 0 failures      (cargo test --release --offline)
29,416 lines of test code
```

| test binary | tests | scope |
|---|---|---|
| `lz4_block.rs` | 10 | lz4.c one-shot compress/decompress, all `tableType`s, `fillOutput`, `extState` |
| `lz4_stream.rs` | 6 | lz4.c streaming, all `LoadMode`s, `saveDict`, prefix64k/forceExtDict, every obsolete wrapper |
| `lz4_errors.rs` | 20 | Phase C for `lz4.c` |
| `lz4hc_diff.rs` | 26 | lz4hc.c: all three strategies (`lz4mid`/hashChain/optimal), streaming, all deprecated entry points |
| `lz4hc_xxhash_errors.rs` | 34 | Phase C for `lz4hc.c` + `xxhash.c` |
| `xxhash_diff.rs` | 27 | XXH32/64 one-shot, streaming chunk patterns, alignment, canonical, copyState |
| `lz4frame_oneshot_diff.rs` | 25 | `LZ4F_compressFrame`, bounds, `headerSize`, CDict creation |
| `lz4frame_stream_diff.rs` | 18 | `compressBegin*`/`compressUpdate`/`uncompressedUpdate`/`flush`/`compressEnd`, cctx reuse |
| `lz4frame_decompress_diff.rs` | 24 | `LZ4F_decompress` state machine (all 15 `dStage`s), `getFrameInfo`, skippable/multi-frame |
| `lz4frame_errors.rs` | 30 | Phase C for `lz4frame.c` |
| `lz4frame_dstcapacity_overrun.rs` | 1 | regression pin for a real C buffer-overrun bug (below) |
| `lz4file_diff.rs` | 9 | `LZ4F_writeOpen/write/writeClose/readOpen/read/readClose` over real `FILE*` |
| `lz4file_errors.rs` | 17 | Phase C for `lz4file.c` |
| `symbol_parity.rs` | 3 | Phase D symbol parity, enforced as a test |

## Completion gate

### `SYMBOLS.md` — 0 missing, 0 extra

```
C    .so exports: 143 symbols
Rust .so exports: 143 symbols
missing from Rust: 0        extra in Rust: 0
undefined non-libc symbols in the Rust .so: 0
```

No module was skipped — all 143 symbols were already implemented and exported, across
`lz4.c` (50), `lz4hc.c` (35), `lz4frame.c` (33), `xxhash.c` (19), `lz4file.c` (6). Parity
is re-checked on every `cargo test` run by `tests/symbol_parity.rs`, which also
dlsym-resolves each name and rejects any unresolved non-libc import. **All 143 symbols are
additionally exercised by at least one differential test** (verified mechanically).

### Phase B — `CONFIGS.md`: 240 / 240 rows `[x]`

240 configuration rows derived from the axes the C actually branches on, covering every
`blockSizeID`, both block modes, both checksum flags, both frame types, `contentSize` and
`dictID` present/absent, all four `tableType`s, every `dict_directive` and
`limitedOutput_directive`, all three LZ4HC strategies, `autoFlush`/`favorDecSpeed`/
`stableSrc`/`stableDst`/`skipChecksums`, and the full public entry-point list including
the lowest-level ones. Each row is driven with many seeded randomized inputs across
lengths and data shapes, not a single hand-picked value.

### Phase C — `ERRORS.md`: 250 / 250 rows accounted for

250 rejection branches, one row per distinct branch. 198 have a differential test that
constructs that exact condition and asserts C and Rust return the **same** error code or
sentinel (the exact `LZ4F_getErrorCode()` number, exact `0` / `-1` / `NULL` /
`XXH_ERROR`, or the exact clamped value for silent-clamp rows). The other 52 are not
observable in this build and each has a recorded, verified reason:

* compile-time `#error` / static asserts (both libraries built — that is the proof)
* code not compiled in (`LZ4_HEAPMODE=0` / `LZ4F_HEAPMODE=0` in `c_src/CMakeLists.txt`
  removes the heap-allocation-failure branches)
* `malloc` failures with no allocator hook in the API
* branches the C's own earlier guard makes unreachable
* 32-bit-only branches, unreachable on x86-64
* `assert`-only guards that are compiled out, so the trigger runs into undefined
  behaviour that would fault both libraries identically

"Hard to test" was not accepted as a reason. Where an allocator hook *does* exist
(`LZ4F_CustomMem` via the `_advanced` constructors), the allocation-failure rows **are**
tested with counting `extern "C"` callbacks that fail the Nth allocation; two of them are
even forced through the *default* allocator by re-executing the test binary as a child
with `RLIMIT_AS` capped.

A subtlety that materially changed which rows are testable, derived mechanically with
`nm -u` per object file: **assert liveness differs per translation unit.** `lz4.c`,
`lz4hc.c` and `lz4frame.c` define their own `#define assert(condition) ((void)0)` because
`LZ4_DEBUG` is undefined, so their asserts are compiled out; `lz4file.c` and `xxhash.c`
include `<assert.h>` unconditionally, so theirs are live. (`-DNDEBUG` is absent but is not
what decides it.) This is documented in `ERRORS.md`.

### Phase D — feature combinations

`Cargo.toml` declares **no** `[features]` section, so there is exactly one valid
combination: the empty set. `./check_all_features.sh` derives the power set from
`Cargo.toml` mechanically (it does not hard-code the list, so a future feature is picked
up automatically) and runs `cargo check --all-targets` / the full `cargo test` suite for
each. Both pass. `c_src/CMakeLists.txt` likewise has a single fixed configuration
(`XXH_NAMESPACE=LZ4_`, `LZ4_HEAPMODE=0`, `LZ4F_HEAPMODE=0`) with no `option()`s, so there
is no C-side axis to mirror, and `grep -rn 'cfg(feature' src/` returns nothing.

Verified reproducible from a clean slate: `rm -rf c_src/build target`, rebuild both
libraries, re-run — 250 tests pass and symbol parity is still 143/143.

## Divergences found and fixed in the Rust

Both had the same root cause, and both are exactly the class of bug that happy-path
testing misses: **out-of-range enum values crossing the FFI boundary.**

The C enums `LZ4F_blockSizeID_t`, `LZ4F_blockMode_t`, `LZ4F_contentChecksum_t`,
`LZ4F_blockChecksum_t` and `LZ4F_frameType_t` (lz4frame.h:123-161) have only non-negative
enumerators, so GCC's compatible type for them is **`unsigned int`**. Confirmed
empirically with a probe compiled against the real header:

```
sizeof(LZ4F_blockSizeID_t)=4  signed?=0
((LZ4F_blockSizeID_t)-2 > LZ4F_max64KB) == 1        /* unsigned comparison */
(size_t)BFSize * (LZ4F_contentChecksum_t)-1 == 17179869180   /* zero-extended */
```

The Rust modelled those fields as `c_int` and therefore compared and extended them as
*signed*. Invisible for the in-range values 0/1/4..7 — observable for any other value a
caller can legally pass across the ABI.

1. **`LZ4F_compressBound_internal`** (lz4frame.c:398-399). With
   `blockChecksumFlag = -1`, C returned `68721956791`, Rust returned `2480055`: C
   zero-extends the enum to `0xFFFFFFFF` and multiplies in 64 bits, Rust sign-extended to
   `-4`. Fixed by casting `as u32 as usize` before the multiply.
2. **`LZ4F_optimalBSID`** (lz4frame.c:364). `while (requestedBSID > proposedBSID)` is an
   unsigned comparison, so `blockSizeID = -2` enters the loop and is normalised to
   `LZ4F_max64KB`; the Rust returned `-2` unchanged, which then landed in the frame
   header's BD byte as `0x60` instead of `0x40`. Fixed by comparing `as u32`.

No other divergence was found in any module. Because "zero divergences" is a suspicious
result, several suites were additionally **mutation-tested** — deliberate bugs were
injected into `src/xxhash.rs`, `src/lz4hc.rs` and `src/lz4file.rs`, confirmed to be
caught, and reverted — and `lz4frame_decompress_diff.rs` / `lz4file_diff.rs` carry
permanent `#[should_panic]` self-check tests that feed the Rust side a bit-flipped frame
to prove the comparison machinery actually fires, plus an assertion that the C and Rust
function pointers are at *distinct* addresses (i.e. two real libraries are loaded).
One mutation initially survived and exposed a genuine gap in the xxhash `copyState` test,
which was then strengthened.

## Real bugs in the upstream C that the Rust faithfully reproduces

These are **not** translation defects. They are recorded because they shaped the tests.

* **`LZ4F_compressUpdateImpl` can write past `dstCapacity`.** It validates `dstCapacity`
  *before* calling `LZ4F_flush()` on a `blockCompressMode` switch, then advances `dstPtr`
  by the flushed count without deducting it from the budget (lz4frame.c:1006-1016).
  Measured: `LZ4F_uncompressedUpdate(srcSize=65536, dstCapacity=65560)` returns **65574**
  — a 14-byte overrun of the caller's buffer. Both libraries do it identically.
  Without a slack region this corrupts the heap and aborts the process, which is why
  `tests/lz4frame_stream_diff.rs` allocates `Sess::SLACK` past `dstCapacity` and
  `tests/lz4frame_dstcapacity_overrun.rs` pins the behaviour explicitly.
* **`LZ4F_writeClose` masks a latched error**, returning `LZ4F_OK_NoError` (0) after a
  failed `LZ4F_write` and leaving a truncated frame on disk.
* **`LZ4F_read`'s `RETURN_ERROR(io_read)` is dead code** (lz4file.c:161-163): `ret` is
  `size_t`, so a failed `fread` is indistinguishable from EOF and takes the `break` — the
  caller sees a silent short read.
* **`LZ4F_readOpen` rejects legal short files**: it unconditionally `fread`s 19 bytes, so
  a valid 11-byte empty frame fails with `io_read`.
* **`LZ4F_uncompressedUpdate` with `LZ4F_blockLinked`** violates a documented contract
  (lz4frame.h:707) and hits `assert(blockCompression == LZ4B_COMPRESSED)`
  (lz4frame.c:1071); with that assert compiled out the C corrupts its own heap. Excluded
  as undefined behaviour.
* **`offset == 0` in a block is not rejected by any decoder** — the output is zero-filled
  and a non-negative length returned. A common false assumption; the Rust must not error.

## Two C behaviours that were mistaken for divergences

Both turned out to be correct C that a test had wrongly predicted; the C-vs-Rust
comparison had already passed in each case.

* Reusing a cctx after a CDict frame does **not** reproduce a pristine frame.
  `cctx->cdict` *is* cleared, but `LZ4F_initStream` only does a *fast* reset
  (`LZ4_resetStream_fast` → `LZ4_prepareTable`), which deliberately re-uses the hash table
  and advances `currentOffset` by 64 KB (lz4.c:903-914), so different matches are found.
* A **raw** dictionary has no effect at all on a `LZ4F_blockIndependent` frame:
  `LZ4F_compressBlock` re-inits the stream per block and, with `cdict == NULL`, compresses
  with the one-shot `LZ4_compress_fast_extState_fastReset` (lz4frame.c:911-921), which
  resets the context and discards the loaded dictionary. Only a real `LZ4F_CDict`
  survives, because it is re-attached on every block.

## Reproducing

```bash
cd translated_rust
cmake -S c_src -B c_src/build -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build c_src/build -j4
cargo build --release --offline
cargo test  --release --offline          # 250 tests
./check_all_features.sh check            # cargo check, every feature combination
./check_all_features.sh test             # full suite, every feature combination
```

Tests are run in `--release` because that is the profile the crate declares for its
`cdylib` (`overflow-checks = false`, matching the C's wrapping arithmetic).
