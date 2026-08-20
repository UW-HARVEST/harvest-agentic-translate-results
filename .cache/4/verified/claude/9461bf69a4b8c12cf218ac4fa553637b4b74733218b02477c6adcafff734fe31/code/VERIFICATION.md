# Verification report

Differential verification of the Rust translation (`src/`) against the C ground
truth (`c_src/`).

## How to reproduce

```bash
./run_tests.sh        # C build + every feature combo, debug AND release
./mutation_check.sh   # proves the suite is not vacuous
```

Both scripts must be used (or at least `cargo build` before `cargo test`):
**`cargo test` does not rebuild a `crate-type = ["cdylib"]` library**, because no
test target links against it. The harness therefore refuses to run against a
`.so` that is older than `src/*.rs` (see `assert_fresh` in
`tests/common/mod.rs`) — otherwise a change to the translation would be
"verified" against a stale binary.

Tests must run with `--test-threads=1`; the harness enforces this, because
stdout is captured by redirecting file descriptor 1, which is process-wide.

## Method

* Both implementations are loaded as shared objects with `libloading`
  (`dlopen`, `RTLD_LOCAL`) and are only ever invoked through their exported C
  symbols — the Rust `#[no_mangle] extern "C"` wrappers are therefore part of
  what is under test. No Rust function is ever called directly.
* `RTLD_LOCAL` is what makes this sound: both `.so`s export the same six names,
  and with `RTLD_GLOBAL` the first library loaded would interpose on the
  second's internal PLT calls (`confusion` → `create_state`). `tests/smoke.rs`
  guards against this by comparing the dlopen'd C library's output against the
  output of a standalone C program linked directly against `libtranslated_rust.so`,
  and by checking that load order does not matter.
* For every call, three things are compared:
  1. the return value,
  2. the **byte-exact stdout** (captured at the fd level; both libraries print
     through the very same glibc `stdout`),
  3. the resulting `ProcessState` — the raw 32-bit `PackedFlags` storage word,
     the raw 32 bits of the `TypeConfusion` union, `capacity`, and the `buffer`
     bytes up to and including the NUL terminator.
* Deliberately *not* compared: the 4 bytes of tail padding in `ProcessState` and
  the `buffer` bytes past the NUL. Neither is ever written by the C code (they
  keep whatever `malloc` returned), so they carry no defined behaviour.

## Results

| artifact | rows | status |
|----------|------|--------|
| `SYMBOLS.md` | 6 exported symbols | all 6 present in the Rust `.so`; `nm -D` diff empty; 0 unresolvable imports |
| `CONFIGS.md` | 30 configuration rows | all 30 pass (`tests/valid_paths.rs`) |
| `ERRORS.md`  | 20 rejection rows   | all 20 pass (`tests/error_paths.rs`) |

58 tests total (30 valid-path + 21 error-path + 4 smoke + 3 symbol), green under

* the single valid feature combination (`Cargo.toml` has no `[features]` table,
  so `--no-default-features` is the only combination), and
* both the unoptimised (`dev`) and optimised (`release`) cdylib.

`./mutation_check.sh` injects 14 deliberate bugs into the Rust translation and
confirms the suite catches every one (wrong bit-field offsets, wrong bit-field
accessor wiring, saturating instead of `cvttss2si` float→int conversion,
unsigned instead of signed `char` promotion, `%d` instead of `%u`, wrong
`snprintf` size, zero- instead of sign-extended `capacity`, wrong error
sentinel, out-of-range `switch` operation falling through, off-by-one counters
and log values, …).

## Behaviours that required care (all verified to match)

| C construct | why it is subtle | how the Rust matches |
|-------------|------------------|----------------------|
| `malloc(capacity)` with `int capacity < 0` | the `int` is *sign*-extended to `size_t`, so `-1` becomes `SIZE_MAX` and `malloc` fails | `ffi::malloc(capacity as isize as usize)` |
| `snprintf(buf, capacity, …)` with tiny `capacity` | truncates, always NUL-terminates; with `capacity == 0` writes *nothing* | same call, same conversion |
| `(int)(state->data.float_val * 100)` | UB for NaN/Inf/out-of-range; gcc emits `mulss` + `cvttss2si` (verified in `objdump`), which yields `INT_MIN`, whereas Rust's `as` **saturates** | `ffi::cvttss2si()` reproduces the x86-64 instruction exactly |
| `printf("%f", float)` | the `float` is promoted to `double` (`cvtss2sd`), NaN sign/payload preserved | `… as f64`; swept over all 256 exponents × mantissas × signs |
| `state->data.bytes[i]` printed with `%d` | `char` is **signed** on x86-64 Linux, so `0x80` prints as `-128` | `c_char` (= `i8`) `as c_int` |
| `unsigned int` bit-fields | assignment truncates to the field width; `status`/`reserved` must be *preserved* by `update_flags`'s read-modify-write | one `c_uint` storage word + masked accessors, layout asserted at compile time |
| `(param >> 3) & 0x7` | arithmetic shift on a signed `int` | `param >> 3` on `i32` |
| `'0' + (param3 % 10)` stored in a `char` | C's `%` truncates toward zero, so negative `param3` yields a byte *below* `'0'`; the `int` result is truncated to `char` | `(b'0' as c_int).wrapping_add(param3 % 10) as c_char` |
| `result += …` chain in `confusion` | can overflow `int` (UB) when `confuse_types` returns `INT_MIN`; gcc wraps | `wrapping_add` / `wrapping_mul` |
| `memchr(ptr, target, …)` | `char target` is promoted to `int` (sign-extended) and compared as `unsigned char` | `target as c_int` |
| `target == '\0'` | the terminator lies *outside* the `remaining = strlen(...)` window, so it can never match | identical loop bounds |
| allocation failure | prints a specific message and returns `NULL` / `-1` | reproduced; exercised for real by re-executing the test binary under `RLIMIT_AS` with an exhausted heap |

## Known non-behavioural differences (deliberately not "fixed")

Two differences exist below the level of observable behaviour. Both were checked
in the disassembly and neither can change what the library computes or prints:

1. **`printf` → `puts` strength reduction.** gcc rewrites the three
   conversion-free `printf("…\n")` calls into `puts`, which is why the C `.so`
   imports `puts`. The debug Rust build calls `printf`; the release Rust build
   performs the same `puts` rewrite. The stdout bytes are identical either way
   (only the discarded return value differs).
2. **Internal call linkage.** In the C `.so`, `confusion` reaches
   `create_state` / `update_flags` / `process_buffer` / `confuse_types` /
   `destroy_state` through the PLT, so those five calls are interposable; the
   Rust `confusion` calls (and, at `-O`, inlines) them directly. This is only
   observable by `LD_PRELOAD`ing those exact six names — it is not reachable
   through the API. It is also precisely why the test harness `dlopen`s both
   libraries with `RTLD_LOCAL`: with `RTLD_GLOBAL`, whichever library loaded
   first would satisfy the other's PLT relocations and the comparison would be
   meaningless. `tests/smoke.rs` proves the isolation holds.

### An optimiser trap worth recording

In the *test harness*, the heap-exhaustion loop originally read
`while !malloc(sz).is_null() {}`. With optimisations enabled LLVM deletes a
`malloc` whose result is only null-checked and folds the check to "non-null", so
the loop silently became a no-op and the release-mode OOM tests reported
"could not exhaust the heap". `std::hint::black_box` around the call fixes it.
The translated library itself is unaffected (it *uses* the returned pointer), and
the release-mode OOM tests confirm that the optimised Rust `create_state` still
returns `NULL` and prints the same message as C.
