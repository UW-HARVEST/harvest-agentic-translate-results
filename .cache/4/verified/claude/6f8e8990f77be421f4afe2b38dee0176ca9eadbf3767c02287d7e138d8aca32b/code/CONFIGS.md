# CONFIGS.md — configuration-surface table (Phase A / gate for Phase B)

Derived mechanically from the C source and the build files, the same way
`ERRORS.md` is derived.

## Axis enumeration (what the C actually branches on)

### 1. Build-time configuration

`c_src/CMakeLists.txt`: no `option()`, no `add_definitions`, no
`target_compile_definitions`, no `#cmakedefine`, no generator expressions — a
single unconditional `add_library(hello SHARED src/hello.c)`.

`c_src` preprocessor conditionals: only the `HELLO_H_` include guard. **No
build-time variants.**

`Cargo.toml`: **no `[features]` section at all**, so the feature powerset is a
single element:

| # | feature combination | cargo invocation |
|---|---------------------|------------------|
| F1 | *(none — the only one)* | `cargo check/test --no-default-features` (identical to plain `cargo check/test`; there is no `default` feature and no optional dependency) |

### 2. Runtime options / modes the public API can set

`c_src/include/hello.h` exposes exactly one declaration, `int helloworld();`.
There is no init function, no context/handle struct, no setter, no global,
no flag, and no parameter. **The library itself has zero runtime options.**

The only mutable state the C code touches is the one the C standard library owns
and that `printf` observes: the process-wide `stdout` `FILE*`. That is therefore
the real configuration axis, and the code paths it selects inside libc (full /
line / no buffering, buffer size, destination kind) are what decide the byte
stream and its ordering. Those axes are enumerated below.

### 3. Input shapes the code special-cases

`helloworld` has no input. The "shapes" a caller can still vary across the FFI
boundary are:

* **call arity/ABI shape** — the declaration is unprototyped (`int helloworld();`),
  so 0..N arguments of any type are legal at the call site and ignored by the
  callee (integer args in `%rdi,%rsi,%rdx,%rcx,%r8,%r9`, floats in `%xmm*`, `%al`
  set as for varargs);
* **invocation count** — 1, 2, many (`0` calls is the trivially-empty case and is
  asserted too: no output before the first call, i.e. no constructor writes);
* **return-value width** as seen by the caller (`int` vs `long`);
* **`stdout` destination kind** — regular file / pipe / `/dev/null` / append-mode fd;
* **`stdout` buffering mode and buffer size** — `_IOFBF`, `_IOLBF`, `_IONBF`,
  and pathological buffer sizes (1, 2, 13, 4096);
* **interleaving with other writers** on the same `FILE*` (the caller's own
  `fputs`/`printf`), and interleaving of the C and Rust libraries with each other;
* **loader configuration** — which library is `dlopen`ed first (interposition),
  `RTLD_LOCAL` vs `RTLD_GLOBAL`, `RTLD_NOW` vs `RTLD_LAZY`, repeated
  `dlopen`/`dlclose` cycles;
* **thread configuration** — single-threaded vs many threads calling at once.

## Configuration-surface table

Cross-product of the axes above, pruned to combinations that the code (C library
+ the libc paths it drives) actually distinguishes. Every row is exercised
against **both** `.so`s through `dlsym` and compared byte-for-byte, with many
randomized inputs per row driven by a fixed-seed SplitMix64 PRNG
(`seed = 0x5EED_1234_9ABC_DEF0`).

Entry points: the library has exactly one, `helloworld`, and it *is* the
lowest-level entry point — there is no convenience wrapper above it and no
internal helper below it, so "start at the lowest level" and "cover every public
entry point" coincide here.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| C1 | `helloworld` | baseline: fresh `dlopen`(`RTLD_LAZY\|RTLD_LOCAL`), `stdout` → regular file, default `_IOFBF`, exactly 1 call, return read as `int` | [x] |
| C2 | `helloworld` | 0 calls: `dlopen` only, then flush — asserts neither library writes anything at load time (no constructor/`.init_array` output) | [x] |
| C3 | `helloworld` | N calls in a row, `N` randomized in `1..=64`, `stdout` → regular file, `_IOFBF` default buffer | [x] |
| C4 | `helloworld` | N randomized calls, `stdout` → regular file, `setvbuf(_IOFBF, size)` with `size` randomized over `{1, 2, 3, 13, 14, 64, 4096}` (forces partial/segmented writes) | [x] |
| C5 | `helloworld` | N randomized calls, `stdout` → regular file, `setvbuf(_IOLBF, size)` (line buffered: flush per `\n`) | [x] |
| C6 | `helloworld` | N randomized calls, `stdout` → regular file, `setvbuf(_IONBF, NULL, 0)` (unbuffered: one `write(2)` per call) | [x] |
| C7 | `helloworld` | N randomized calls, `stdout` → **pipe** (read end drained), `_IOFBF` — the block-buffered non-seekable path | [x] |
| C8 | `helloworld` | N randomized calls, `stdout` → **pipe**, `_IONBF` — unbuffered non-seekable path | [x] |
| C9 | `helloworld` | N randomized calls, `stdout` → fd opened `O_APPEND` on a non-empty file (output must land at EOF, nothing overwritten) | [x] |
| C10 | `helloworld` | N randomized calls, `stdout` → `/dev/null` (character device; return value + no-crash only) | [x] |
| C11 | `helloworld` | interleaved with the caller's own `fputs`/`printf` on the same `stdout`: a randomized script of {library call, caller writes a randomized ASCII blob} — checks ordering through the shared `FILE*` buffer | [x] |
| C12 | `helloworld` | randomized interleaving of the **C** and **Rust** libraries into the *same* stream (e.g. C,R,R,C,…): the combined byte stream must equal the all-C stream for the same script | [x] |
| C13 | `helloworld` | ABI shape: called with 0..6 extra `int` arguments (unprototyped declaration), values randomized incl. `0`, `-1`, `INT_MIN`, `INT_MAX` | [x] |
| C14 | `helloworld` | ABI shape: called with randomized `f64` arguments in `xmm0..xmm3` plus integer args and `%al` set (varargs-style call site) | [x] |
| C15 | `helloworld` | ABI shape: called with pointer-shaped arguments (`NULL`, valid `&buf`, `0xDEAD_BEEF`) | [x] |
| C16 | `helloworld` | return value read through a `-> i64` signature to prove the whole `%rax` is `0`, and through `-> i32`; both, many repeats | [x] |
| C17 | `helloworld` | loader config: all four of `{RTLD_NOW, RTLD_LAZY} × {RTLD_LOCAL, RTLD_GLOBAL}` for both libraries (symbol interposition: each handle must still call its own `helloworld`) | [x] |
| C18 | `helloworld` | loader config: C `.so` loaded first vs Rust `.so` loaded first (both orders), symbols resolved once and called alternately | [x] |
| C19 | `helloworld` | loader config: repeated `dlopen`/`dlsym`/call/`dlclose` cycles (randomized 1..=16 cycles) — statelessness across load/unload | [x] |
| C20 | `helloworld` | thread config: `T` threads (randomized `2..=8`) × `K` calls each (randomized `1..=32`), `stdout` → file, `_IOFBF`; compare total byte count, that every line is intact `Hello World!`, and every return value is `0` | [x] |
| C21 | `helloworld` | thread config: same as C20 but `_IONBF` (one `write(2)` per call, maximum interleaving pressure) | [x] |
| C22 | `helloworld` | build config **F1** (the only feature combination) applied to every row above: `cargo test --no-default-features` | [x] |
| C23 | `helloworld` | both `.so`s built in the **dev** profile and the **release** profile (`panic = "abort"` only affects release) export and behave identically | [x] |

## Phase B status — every row passes over randomized inputs

Rows C1–C21 are implemented in `tests/phase_b.rs` (functions `b_c1_…` …
`b_c21_…`, driven by one `#[test]`); C22 is the feature/profile loop in
`run_differential_tests.sh`; C23 is `tests/phase_d.rs::d5_dev_and_release_profiles_agree`.

Every row calls BOTH `.so`s via `dlopen`/`dlsym` and compares the emitted bytes
byte-for-byte plus the full vector of return values, and additionally checks the
C stream against an independently derived expectation (`n × "Hello World!\n"`,
interleaved with the caller's own writes where applicable) so that "both wrote
nothing" can never pass.

```text
=== Phase B — CONFIGS.md ===
  [x] C1  baseline, 1 call, file, default buffering
  [x] C2  0 calls: no output at dlopen/dlclose
  [x] C3  N randomized calls, file, default buffering
  [x] C4  _IOFBF with buffer sizes 1..4096
  [x] C5  _IOLBF with buffer sizes 1..4096
  [x] C6  _IONBF unbuffered
  [x] C7  stdout is a pipe, _IOFBF
  [x] C8  stdout is a pipe, _IONBF
  [x] C9  O_APPEND onto a non-empty file
  [x] C10 stdout is /dev/null
  [x] C11 interleaved with the caller's own writes
  [x] C12 C and Rust interleaved in one stream
  [x] C13 extra integer arguments (arity 0..6)
  [x] C14 float / true-varargs call shapes
  [x] C15 pointer-shaped arguments
  [x] C16 return value read as i32 and i64
  [x] C17 RTLD_NOW/LAZY x LOCAL/GLOBAL
  [x] C18 load order: C first / Rust first
  [x] C19 repeated dlopen/dlclose cycles
  [x] C20 many threads, _IOFBF
  [x] C21 many threads, _IONBF
=== Phase B — CONFIGS.md : 21 rows run, 0 failed ===
```

Randomization: fixed-seed SplitMix64 (`SEED = 0x5EED_1234_9ABC_DEF0`, per-row
salt), covering randomized call counts (1..64), buffer sizes, `O_APPEND`
prefixes, interleaving scripts, argument values (including `INT_MIN`, `INT_MAX`,
`NaN`, `±inf`, `NULL`, `0xDEADBEEF`), thread counts (2..8) and per-thread call
counts (1..32).

### Harness note

Each test binary exposes exactly **one** `#[test]`, because libtest writes its
own progress lines to fd 1 and those bytes otherwise land inside the fd-1
captures these differential tests depend on. (That was observed for real: rows
C7/C8 initially failed with `test b_c6_unbuffered ... ok` spliced into the
captured pipe.) With one test per binary nothing else can write to fd 1 during a
capture window, at any `--test-threads` setting.
