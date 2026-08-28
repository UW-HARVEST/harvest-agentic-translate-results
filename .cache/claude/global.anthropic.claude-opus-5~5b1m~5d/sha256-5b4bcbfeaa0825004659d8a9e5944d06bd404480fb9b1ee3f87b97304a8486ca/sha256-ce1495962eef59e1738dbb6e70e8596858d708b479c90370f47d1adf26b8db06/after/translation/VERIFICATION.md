# VERIFICATION.md — how to reproduce, and what was found

## Reproduce

```sh
cd translation
./run_tests.sh          # rebuild BOTH .so's, run the whole differential suite
./stress.sh 45          # 45 independent reproducible random corpora
./features.sh           # every feature combination declared in Cargo.toml
```

`run_tests.sh` rebuilds the C `.so` with exactly the prescribed invocation
(`cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .`) and the
Rust `.so` with `cargo build --release`. Plain `cargo test` is **not** enough:
it builds test harnesses but does not re-emit a `cdylib`, so the Rust `.so`
under test would silently be stale. `tests/common/mod.rs` now refuses to run
against a `.so` older than its source and says so.

Both libraries are loaded with `libloading` and driven **only** through their
exported symbols (`match`, `spectral_contrast`); no Rust function is ever called
directly, so the `#[unsafe(no_mangle)] extern "C"` wrappers are on the critical
path exactly as they are for an external consumer.

Everything is compared on **raw IEEE-754 bits** — of the return value *and* of
every element of every buffer after the call, because both entry points mutate
memory in place.

## Result

| phase | artifact | status |
|-------|----------|--------|
| A | `SYMBOLS.md` | `nm -D` diff is **empty**; 0 missing / 0 undefined non-libc symbols |
| B | `CONFIGS.md` | 50 rows, **all** passing (`tests/configs.rs`, 48 `#[test]`s) |
| C | `ERRORS.md` | 34 rows, **all** passing (`tests/errors.rs`, 27 `#[test]`s + 12 out-of-process cases) |
| D | symbol parity + feature combos | `tests/symbols.rs`; *default* and `--no-default-features` both pass |

Additional robustness runs, all green:

* 45 independent random corpora (`DIFF_SEED=0..44`).
* The **debug-profile** Rust `.so`
  (`RUST_SO=target/debug/libunderhanded_c_nuke_lib.so cargo test --release`),
  which rules out opt-level-dependent NaN handling on the Rust side.
* Exhaustive cross-products of IEEE-754 class representatives:
  25⁴ = 390 625 combinations for `spectral_contrast` at `length = 2`,
  16⁴ = 65 536 for `match` at `bins = 2`, plus `length/bins` ∈ {1, 3} and the
  partial-overlap variant.

## Mutation check — the tests are not vacuous

The same suite was pointed at a `-O2` build of the **same C sources** in the
Rust slot:

```sh
mkdir -p target/scratch && gcc -O2 -fPIC -shared -I../c_src/include -I../c_src/src \
    -o target/scratch/libo2.so ../c_src/src/*.c -lm
RUST_SO=$PWD/target/scratch/libo2.so cargo test --release
```

13 of the 48 `configs` rows **fail**, which proves the rows genuinely
discriminate at the bit level. All 13 are NaN-payload rows; the `match`-only
rows do not fail, confirming that `match`'s `int` result is payload-insensitive.

## What was actually wrong, and what was changed

1. **The crate did not compile at all.** `[lib] name = "underhanded-c-nuke_lib"`
   — Cargo rejects hyphens in library target names, so `cargo check` failed to
   even parse the manifest. Renamed to `underhanded_c_nuke_lib`. (This was the
   only `cargo check` error.) `libloading = "0.8"` added to `[dev-dependencies]`.

2. **NaN-payload operand order was tuned for the wrong build.** The translation
   modelled *optimized* GCC's choice of SSE destination operand. The build the
   task prescribes sets no `CMAKE_BUILD_TYPE`, so GCC compiles at `-O0`, and
   `-O0` picks the **other** operand in three places. Verified from the
   disassembly of the built `.so` and fixed:

   | function | C at `-O0` | was | now |
   |----------|-----------|-----|-----|
   | `total` (`sum += v[i]`) | `movsd v[i]→xmm0; movsd sum→xmm1; addsd %xmm1,%xmm0` ⇒ dst is `v[i]` | `addsd(sum, v[i])` | `addsd(v[i], sum)` |
   | `smoothen` (`sum += v[i+j]`) | same shape ⇒ dst is `v[i+j]` | `addsd(sum, v[i+j])` | `addsd(v[i+j], sum)` |
   | `dot_product` (`sum += a[i]*b[i]`) | `mulss %xmm1,%xmm0` with `xmm0 = b[i]` ⇒ dst is `b[i]`; then `addsd %xmm1,%xmm0` with `xmm0 = product` ⇒ dst is the product | `addsd(sum, mulss(a,b))` | `addsd(mulss(b,a), sum)` |

   Concrete divergence this fixed (`spectral_contrast`, `length = 1`,
   `a[0] = 0x7FC00001`, `b[0] = 0x7FC00002`): C returns
   `0x7FF8000040000000`, the old Rust returned `0x7FF8000020000000`.

3. **`differentiate` could overflow and read out of bounds for `length <= 0`.**
   `length - 1` with `length == INT_MIN` wrapped to `INT_MAX` in a release
   build, so the loop ran and faulted (and a debug build would have panicked on
   the overflow). Now the function returns early for `length <= 0` — a range in
   which the C is undefined anyway (see ERRORS.md E8/E9), and identical to the C
   for every `length >= 1`.

4. **`smoothen`'s `i + j` could overflow** for `length > INT_MAX - 16`. Spelled
   `wrapping_add` so both implementations wrap the same way instead of the Rust
   debug build panicking.

## Two facts about the C that the translation deliberately preserves

1. **`float_t` means different things in the two translation units.** `match.c`
   includes `match.h` (`typedef double float_t`), while `spectral_contrast.c`
   includes only `<math.h>`, whose `float_t` is `float` on x86-64. So `match`
   hands `spectral_contrast` a `double` array that it indexes as a `float`
   array, reading `bins` f32 slots out of the `2*bins` the buffer occupies.
   Confirmed from the disassembly (`lea (,%rax,4)` / `movss` / `mulss` in
   `dot_product` versus `lea (,%rax,8)` / `movsd` in `total`). Reproduced
   verbatim; not "fixed".

2. **`match` cannot survive `bins <= 0`.** For `bins == 0` the zero-length VLAs
   sit exactly at `match`'s stack pointer, so `differentiate`'s unguarded
   `v[length-1] = 0` overwrites `preprocess`'s saved return address and the
   function returns to `0x0`; for `bins < 0` `preprocess`'s
   `memcpy(v, source, length * sizeof(*v))` gets a wrapped-around `size_t`.
   Both SIGSEGV at `-O0` **and** `-O2`. No in-process differential row uses
   `bins <= 0`; those rows are exercised out-of-process and the outcome recorded.

## Known, unavoidable limitation

The NaN *payload* returned by `spectral_contrast` is not determined by the C
source — it depends on which SSE operand the compiler picks as the destination,
and `-O0` and `-O2` of these very sources disagree. No single translation can
match both. This one matches the `.so` produced by the prescribed cmake command
(`-O0`). Everything the C language actually specifies is bit-exact under either
optimization level, and `match`'s `int` return value is unaffected either way,
because any NaN reaching `match` makes both of its ordered comparisons false
regardless of payload.
