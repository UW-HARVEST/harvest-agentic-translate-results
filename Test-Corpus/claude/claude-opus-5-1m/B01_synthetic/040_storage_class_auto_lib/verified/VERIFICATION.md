# Verification report — C → Rust translation of `driver`

The C sources in `c_src/` are the ground truth; `src/lib.rs` must be
byte-identical in observable behaviour.  Everything below is enforced by tests
that load **both** shared objects with `libloading` and call only their exported
symbols — the Rust implementation is never called directly, so the
`#[no_mangle] extern "C"` wrapper is part of what is under test.

## The library under verification

`c_src/` contains exactly one translation unit and one public header:

```c
// c_src/include/driver.h
void driver(int x);

// c_src/src/driver.c
void driver(int x) {
    auto int y = 2*x;
    y += 300;
    printf("%d\n", y);
}
```

Observable behaviour = the bytes written to `stdout`.  `auto` is just the
default storage-class specifier and has no semantic effect.  The compiled C
(`objdump -d c_src/build/libdriver.so`) is straight-line, wrapping 32-bit
arithmetic:

```
add    %eax,%eax          ; 2*x, wraps mod 2^32
addl   $0x12c,-0x4(%rbp)  ; += 300, wraps mod 2^32
call   printf@plt
```

which is exactly what `wrapping_mul` / `wrapping_add` reproduce in the Rust
translation.

## Build configurations (Phase A, step 1)

| axis | source | values |
|---|---|---|
| Rust features | `Cargo.toml` has **no `[features]` section** | 1 combination: the empty one |
| C build options | `CMakeLists.txt` has no `option()` / `target_compile_definitions`; no `#ifdef` in the sources except the header guard | 1 configuration |

So the complete set of valid feature combinations is
`{ <none> }`, driven by `./run_all_feature_combos.sh`, which enumerates
`[features]` mechanically from `Cargo.toml` (2^n subsets) and runs
`cargo check` + `cargo build` + the full differential suite for each, then
repeats the suite against the **default** feature set and against the
**release-profile** Rust `.so`.

## Artifacts

| file | phase | content |
|---|---|---|
| `SYMBOLS.md` | A / D | `nm -D` surface of both `.so`s, symbol-by-symbol parity |
| `ERRORS.md` | A / C | error-surface table, 12 rows, mechanically derived |
| `CONFIGS.md` | A / B | configuration-surface table, 19 rows |
| `tests/common/mod.rs` | — | harness: dlopen of both `.so`s, stdout capture (file / pipe / hashing), PRNG, row runner |
| `tests/phase_b_configs.rs` | B | one sub-case per `CONFIGS.md` row |
| `tests/phase_c_errors.rs` | C | one sub-case per `ERRORS.md` row |
| `tests/phase_d_symbols.rs` | D | `nm -D` parity enforced as a test |
| `tests/exhaustive.rs` | B+ | all 2^32 inputs, sharded (`#[ignore]`d by default) |
| `run_all_feature_combos.sh` | A / D | feature-combination sweep |
| `exhaustive_sweep.sh` | B+ | runs the exhaustive sweep in shards |

## How to run

```bash
# C reference library
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .

# fast suite (Phases B, C, D) for every feature combination
./run_all_feature_combos.sh

# exhaustive proof over the complete 2^32 input domain (~13 min, 8 shards)
./exhaustive_sweep.sh 8
```

## Test-harness notes

* **Aliasing guard.** The CMake object carries `DT_SONAME = libdriver.so` while
  the Rust cdylib carries none, so both are copied to distinct file names before
  `dlopen`, and the harness asserts the two resolved `driver` addresses differ.
  Without that check a `dlopen` de-duplication would silently compare the C
  library against itself.
* **stdout capture.** `driver`'s only output channel is the process-wide libc
  `stdout`, so fd 1 is temporarily redirected into a file or pipe and
  `fflush(NULL)` forces the bytes out.  Restoration is RAII, so a failing
  assertion inside a capture window cannot leave stdout hijacked.  Because fd 1
  is global, each capturing test binary contains exactly **one** `#[test]`
  (rows run as sub-cases via `RowRunner`); otherwise libtest's own progress
  output from parallel threads lands in the capture and corrupts the
  comparison.
* **Not vacuous.** Every comparison also pins the *exact* expected text
  (`format!("{}\n", x.wrapping_mul(2).wrapping_add(300))`), so "both produced
  nothing" cannot pass.  Verified by mutation testing (see below).

## Two harness defects found and fixed (neither was a translation bug)

Both showed up only in the long-running exhaustive sweep, and both would have
been reported as "the Rust translation diverges" by a naive harness:

1. **libtest's slow-test warning polluted the captures.** Every shard failed at
   ≈ 60.3 s with exactly ±68 extra bytes on one side — alternating between C and
   Rust from run to run.  Dumping the captured bytes instead of hashing them
   revealed the injected text:

   ```
   654437878\ntest investigate_hunt has been running for over 60 seconds\n37880\n
   ```

   After 60 s libtest's *main* thread writes
   `test <name> has been running for over 60 seconds\n` (exactly 68 bytes) to
   **stdout**, so it lands inside whichever capture happens to be open.  The
   giveaway that this was never library output: in that chunk every line is
   exactly 10 bytes, and 68 is not a multiple of 10, so no set of `driver` lines
   could produce it.
   *Fix:* every capture is validated against the byte length the C semantics
   require (`printed_len`, computed without formatting); a mismatch means
   "polluted", and the chunk is retried instead of failed.

2. **`SIGBUS` from restaging the shared objects.** The harness copies both
   `.so`s to distinct names before `dlopen`; the staging directory was shared,
   so a second test process overwriting `libdriver_rust.so` while the first was
   executing from that mmap'd file killed it with `SIGBUS`.
   *Fix:* the staging directory is per-process (`difftest-libs-<pid>`).

To keep the exhaustive sweep both robust and precise, a chunk that fails four
attempts is **bisected** down to a single input, and a divergence is reported
only if one specific `x` fails five clean repetitions in a row.  Verified end to
end: with a mutant that is wrong only for `x == 1234567891`, the sweep reports

```
x in [1233125376, 1237319680): 4 attempts failed (content differs: ...); bisecting
REPRODUCIBLE DIVERGENCE at x = 1234567891 (0x499602d3)
```

while transient pollution is logged as retries and the chunk passes.

## C compiled with optimisation (`-O3`)

Signed overflow is UB in C, so the optimiser could in principle treat `2*x` as
non-wrapping.  It does not: at `-O3` gcc emits a 32-bit `lea`,

```
lea 0x12c(%rdi,%rdi,1),%esi     ; y = 2*x + 300, wrapping mod 2^32
```

The differential suite is therefore also run with `DRIVER_C_SO` pointing at a
`-O3` build (`cmake -S c_src -B target/c_o2 -DCMAKE_BUILD_TYPE=Release`), and
the Rust translation matches that build too — so the translation does not depend
on the C reference being built unoptimised.

## Mutation testing (does the suite actually detect divergence?)

| mutant applied to `src/lib.rs` | detected? |
|---|---|
| `+ 300` → `+ 301` | yes — every Phase B row fails |
| `wrapping_mul`/`wrapping_add` → plain `*` / `+` (debug overflow check) | yes — process aborts, test binary fails |
| format `"%d\n"` → `"%d"` | yes — every Phase B row fails |
| i64 intermediate truncated to `c_int` | no — and correctly so: truncation of the low 32 bits *is* wrapping arithmetic, an equivalent mutant |
| wrong result for exactly **one** input (`x == 1234567891`) | not detected by the 4.2 M-sample sweep; **detected by the exhaustive sweep** (shard 50/64 reports the divergence) |

The last row is why the exhaustive sweep exists: for a one-parameter `int` API
the whole domain is only 2^32 values, so sampling can be replaced by proof.

## Completion gate

- [x] **`SYMBOLS.md`** — the C `.so` exports exactly one API symbol, `driver`;
      the Rust `.so` exports it too.  Symbol diff is empty; no C module was
      left untranslated and no stub was introduced.  `nm -D` shows no
      unresolvable non-libc undefined symbol in the Rust `.so`
      (`tests/phase_d_symbols.rs`).
- [x] **Phase B** — all 19 `CONFIGS.md` rows pass, with randomized inputs from
      fixed seeds (≈ 4.2 M inputs in row 18 alone).
- [x] **Phase C** — all 12 `ERRORS.md` rows have a passing differential test,
      including the generic boundaries (null/zero/oversized/one-past-range and
      out-of-range values crossing the FFI boundary).
- [x] **Every feature combination** — the only valid combination (`<none>`),
      plus the default feature set and the release profile, all pass
      (`./run_all_feature_combos.sh` → `ALL FEATURE COMBINATIONS PASSED`).
- [x] **Exhaustive** — all 4 294 967 296 possible `int` arguments produce
      byte-identical stdout in C and Rust (`./exhaustive_sweep.sh` → 8 shards ×
      536 870 912 values, ~100 s each):

      SHARD 0/8 OK: 536870912 values byte-identical in  99s (1 capture retries, 0 polluted chunks)
      SHARD 1/8 OK: 536870912 values byte-identical in 106s (1 capture retries, 0 polluted chunks)
      SHARD 2/8 OK: 536870912 values byte-identical in 106s (1 capture retries, 0 polluted chunks)
      SHARD 3/8 OK: 536870912 values byte-identical in 111s (1 capture retries, 0 polluted chunks)
      SHARD 4/8 OK: 536870912 values byte-identical in 114s (1 capture retries, 0 polluted chunks)
      SHARD 5/8 OK: 536870912 values byte-identical in 101s (1 capture retries, 0 polluted chunks)
      SHARD 6/8 OK: 536870912 values byte-identical in 102s (0 capture retries, 0 polluted chunks)
      SHARD 7/8 OK: 536870912 values byte-identical in 100s (1 capture retries, 0 polluted chunks)
      EXHAUSTIVE SWEEP COMPLETE: all 4294967296 inputs byte-identical

      (The single retry per shard is the 60 s libtest warning described above;
      no chunk ever needed bisection, i.e. no candidate divergence at all.)
- [x] **Optimised C reference** — the whole fast suite also passes with
      `DRIVER_C_SO=target/c_o2/libdriver.so` (gcc `-O3`), so the match does not
      depend on the C side being built unoptimised.

**Result: the Rust translation matches the C implementation for every possible
input.  No divergence was found, and no change to `src/lib.rs` was required.**
