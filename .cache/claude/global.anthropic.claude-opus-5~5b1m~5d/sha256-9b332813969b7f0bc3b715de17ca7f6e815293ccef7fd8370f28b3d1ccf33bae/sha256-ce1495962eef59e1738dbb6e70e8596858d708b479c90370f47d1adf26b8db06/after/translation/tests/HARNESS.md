# Differential test harness contract

Every test file in `translation/tests/` is a standalone integration test that
loads BOTH shared objects via `libloading` and compares their behaviour. No test
ever calls a Rust function directly — everything goes through the `.so` exports.

## Boilerplate

```rust
mod common;
use common::*;
use std::ffi::{c_char, c_int, c_void};

#[test]
fn my_test() {
    let (c, r) = both();   // (&'static Api for C, &'static Api for Rust)
    unsafe {
        // ... call (c.json_foo)(..) and (r.json_foo)(..) and compare
    }
}
```

`both()` returns the two `Api` structs and guarantees `json_object_seed(FIXED_SEED)`
has already been called on BOTH libraries. That matters: jansson seeds its hash
from `/dev/urandom` on first use, and the seed decides object iteration order and
therefore the exact bytes of every object dump. Always use `both()`, never
`capi()`/`rapi()` directly.

## `Api`

`Api` has one field per exported symbol, named exactly like the C symbol, typed
as an `unsafe extern "C" fn` pointer. Call them as `(c.json_loads)(...)`.

Two symbols are DATA, not functions, and have accessor methods instead:
- `c.dtoa_divmax() -> c_int`
- `c.hashtable_seed() -> u32`

## Comparing

Use the `diff_eq!` macro — it prints which configuration diverged:

```rust
diff_eq!(c_value, rust_value, "json_dumps(flags={flags:#x}) on {input:?}");
```

The context string is a `format!` pattern; always include enough to reproduce
the case (flags, the input, the loop index).

## Helpers in `common`

| helper | purpose |
|---|---|
| `cs("text") -> CString` | NUL-terminated C string; panics on interior NUL |
| `cs_bytes(b"a\0b") -> Vec<c_char>` | NUL-terminated buffer that MAY contain interior NULs |
| `cbytes(ptr) -> Option<Vec<u8>>` | read a `char*` as raw bytes; `None` for NULL, so "returned NULL" stays distinct from "returned empty string" |
| `cstr_lossy(ptr) -> String` | for messages only |
| `incref(j)` / `decref(api, j)` | `json_incref`/`json_decref` are `static inline` in jansson.h so they are NOT exported; these reproduce them exactly. `decref` must be given the SAME api the value came from. |
| `typeof_(j) -> c_int` | `json_typeof` |
| `jfree(api, ptr)` | free a `char*` jansson allocated, via that library's allocator |
| `Api::hashtable_seed()`, `Api::dtoa_divmax()` | the two data symbols |

Never free a C-library pointer with the Rust library's allocator or vice versa.

## Types available

`json_t` (`{ type_: c_int, refcount: size_t }`), `json_error_t`, `hashtable_t`,
`strbuffer_t`, `FILE`, `json_int_t` (= i64), `size_t`.

`json_error_t::new()` zeroes it; `json_error_t::poisoned()` fills it with a
sentinel so "the library did not write here" is distinguishable from "it wrote a
NUL". Compare errors with `.raw()` (full byte image — strongest), or
`.snapshot()` / `.code()` / `.text_str()` for readable assertions.

All flag constants are defined: `JSON_COMPACT`, `JSON_ENSURE_ASCII`,
`JSON_SORT_KEYS`, `JSON_PRESERVE_ORDER`, `JSON_ENCODE_ANY`, `JSON_ESCAPE_SLASH`,
`JSON_EMBED`, `JSON_REJECT_DUPLICATES`, `JSON_DISABLE_EOF_CHECK`,
`JSON_DECODE_ANY`, `JSON_DECODE_INT_AS_REAL`, `JSON_ALLOW_NUL`,
`JSON_VALIDATE_ONLY`, `JSON_STRICT`, plus the helpers `json_indent(n)` and
`json_real_precision(n)`, the `JSON_*` type tags, the `JSON_ERROR_*` code
constants, and `JSON_PARSER_MAX_DEPTH`.

## Randomised (property-style) testing

Use the seeded `Rng` (SplitMix64) so every run is reproducible. Give each test
its own fixed seed constant.

```rust
let mut rng = Rng::new(0x1234_5678);
rng.next_u64(); rng.next_u32(); rng.below(n); rng.range(lo, hi); rng.bool();
rng.choice(&slice);
rng.json_int();          // biased to 0, ±1, INT32/INT64 bounds
rng.real();              // biased to 0.0, -0.0, subnormals, 1e±308, 17-digit values
rng.ascii_string(max);   // pool includes quotes, backslash, slash, control chars
rng.utf8_string(max);    // spans 1-,2-,3-,4-byte sequences + control chars
```

Prefer MANY randomised inputs per configuration over one hand-picked value, and
compare after EVERY step of a mutation sequence rather than only at the end.

## va_list entry points

`json_vpack_ex`, `json_vunpack_ex`, `json_vsprintf` and `jsonp_error_vset` take a
`va_list`, which stable Rust cannot construct. `vashim()` returns trampolines
(compiled from `tests/vashim.c`) that are variadic on the Rust side and forward a
real `va_list`. Get the target function's address with
`sym_addr("C", b"json_vpack_ex")` / `sym_addr("Rust", b"json_vpack_ex")`:

```rust
let sh = vashim();
let cfn = sym_addr("C", b"json_vpack_ex");
let rfn = sym_addr("Rust", b"json_vpack_ex");
let cj = (sh.vpack_ex)(cfn, &mut cerr, flags, fmt.as_ptr(), /* varargs... */);
let rj = (sh.vpack_ex)(rfn, &mut rerr, flags, fmt.as_ptr(), /* varargs... */);
```

Shim signatures (first arg is always the target function pointer):
- `sh.vpack_ex(fn, *mut json_error_t, size_t flags, *const c_char fmt, ...) -> *mut json_t`
- `sh.vunpack_ex(fn, *mut json_t root, *mut json_error_t, size_t flags, *const c_char fmt, ...) -> c_int`
- `sh.vsprintf(fn, *const c_char fmt, ...) -> *mut json_t`
- `sh.error_vset(fn, *mut json_error_t, c_int line, c_int col, size_t pos, c_int code, *const c_char msg, ...)`

When passing varargs, give ints an explicit type (`7 as c_int`), doubles as
`f64`, and strings as `.as_ptr()` of a `CString` kept alive in a local.

## Running

```
cd translation && cargo test --release --test <name>
```
`--release` is required: the tests dlopen `target/release/libjansson.so`.
Cargo is configured offline (`translation/.cargo/config.toml`); crates.io is
unreachable in this environment.

`../run_tests.sh` rebuilds the C `.so`, the Rust `.so` and the va shim, checks
`nm -D` symbol parity, then runs the whole suite.

## MANDATORY: every test must take `global_state_lock()`

```rust
#[test]
fn my_test() {
    let _g = global_state_lock();     // FIRST statement, always
    let (c, r) = both();
    ...
}
```

Cargo runs a binary's `#[test]` functions on several threads, but two pieces of
the C library's state are process-global and **not** thread-safe:

1. **dtoa's `Balloc`/`Bfree` freelist.** `dtoa.c` is compiled without
   `MULTIPLE_THREADS` defined, so `freelist[]` is a bare global with no locking.
   Any two threads formatting a real concurrently — which includes any
   `json_dumps` of a document containing a real — corrupt it. The symptom is
   nasty: the **C** side starts returning plausible-looking but wrong digits
   (e.g. `904692889061487.1882` dumping as `200000000000000.0`), so it reads as a
   translation bug when it is really a harness race.
2. **The allocator function pointers** in memory.c, which
   `json_set_alloc_funcs*` mutates.

Holding the lock for the whole test body makes each file serialise internally,
so results do not depend on `--test-threads`. Separate test binaries are separate
processes with their own `dlopen`ed copies, so they do not interfere.

## The `.so` staleness guard

`cargo test --test <name>` does **not** rebuild `libjansson.so`: the integration
test `dlopen`s it rather than linking it, so cargo sees no dependency. The
harness therefore refuses to run if any `src/*.rs` is newer than the `.so`.
Always `cargo build --release` first, or just use `../run_tests.sh`.

## Rules

- The C is ALWAYS right. On divergence, fix the Rust in `translation/src/`, never
  the C and never the test's expectation.
- Never modify anything in `c_src/`.
- Compare raw bytes (`cbytes`, `.raw()`), not lossy strings, for anything the C
  produces — jansson can emit non-UTF-8 with the `_nocheck` entry points.
- Free what you allocate, with the matching library's allocator, so a long
  randomised loop does not exhaust memory.
