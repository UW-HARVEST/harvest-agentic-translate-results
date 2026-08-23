# Differential-test harness briefing

Read `tests/common/mod.rs` first, then skim `tests/phase_b_api.rs` for style.

## Ground rules

1. **The C is ALWAYS correct.** If C and Rust differ, fix `src/*.rs`, never
   `c_src/`. Never weaken an assertion to make a real divergence pass.
2. Both libraries are loaded via `libloading` from their `.so`. NEVER call the
   Rust crate directly. `common::pair()` returns `&'static Pair { c: Api, r: Api }`.
3. Build/run with `--release`: the harness prefers `target/release/libpcre2.so`,
   and the release profile is the one with `overflow-checks = false`
   (matching C). Rebuild with `cargo build --release` after touching `src/`.
4. The Rust `.so` is built with `panic = "abort"`, so a panic inside it kills
   the whole test process (SIGABRT/SIGSEGV with no message). If that happens,
   narrow down with `-- --exact --test-threads=1 <testname>`.
5. Use `timeout 600 cargo test --release ...`. Keep individual tests fast.

## Harness API (`tests/common/mod.rs`)

- `pair() -> &'static Pair` — `p.c` (C) and `p.r` (Rust), both `Api`.
- `Api` has one field per exported function, already resolved to a raw
  `extern "C"` fn pointer. Public ones are named without the `pcre2_`/`_8`
  affixes (`compile`, `do_match`, `dfa_match`, `substitute`, `pattern_info`,
  `match_data_create`, `config`, `get_error_message`, `serialize_encode`, ...);
  the exported internals are prefixed `p_` (`p_valid_utf`, `p_ord2utf`,
  `p_xclass`, `p_study`, ...). Grep the `def_api! { ... }` block for the exact
  field name and signature.
- `Api::data(&self, sym) -> *const u8` — address of an exported data symbol.
- `Diffs` — failure collector. `d.eq(&tag, c_value, rust_value)` records a
  divergence instead of aborting on the first one; `d.finish("row description")`
  panics with a summary if any were recorded. Prefer this over bare
  `assert_eq!` when looping over many inputs.
- `assert_code_eq(a, b, ctx)` — byte-for-byte comparison of two compiled
  patterns (header fields + name table + bytecode).
  `assert_code_eq_masked(a, b, allow_flags, ctx)` permits specific `flags` bits
  to differ (only `PCRE2_DEREF_TABLES` legitimately does, after
  `pcre2_serialize_decode`).
- `read_match_out(api, md, rc) -> MatchOut` for `pcre2_match_8`, and
  `read_match_out_of(api, md, rc, Engine::Dfa)` for `pcre2_dfa_match_8`.
  These read ONLY the fields the C defines for that `rc` — use them rather than
  reading the ovector yourself.
- `Rng::new(seed)` — deterministic SplitMix64: `below(n)`, `range(lo,hi)`,
  `byte()`, `chance(n)`, `pick(&[..])`, `pick_bytes(&[&[u8]])`.
- `gen_ascii`, `gen_utf8`, `gen_raw` — random subject generators.
- `PATTERNS`, `SUBJECTS` — shared corpora.
- `show(&[u8]) -> String` — escaped rendering for messages.
- All `PCRE2_*` option/info/config/error constants are re-exported.
- `bytecode_ptr(code)`, `code_blocksize(code)`, `RealCodeHead` — for the
  internals that consume compiled bytecode.

## Pitfalls already hit (do not repeat)

- **`PCRE2_ZERO_TERMINATED` needs a real NUL-terminated buffer.** `s.as_bytes()`
  is not NUL-terminated; push a `0` first.
- **Character tables are BORROWED.** `pcre2_compile` stores the pointer
  (`re->tables = tables`), which is why `pcre2_code_copy_with_tables` exists.
  Tables from `pcre2_maketables` must outlive every code compiled against them.
- **`pcre2_match_data_create` does not initialise `mark`, `startchar` or the
  ovector.** Only a documented prefix/subset is written per return code. Never
  compare beyond it (that is what `read_match_out` is for).
- **`pcre2_callout_block`, `pcre2_callout_enumerate_block` and
  `pcre2_substitute_callout_block` are three DIFFERENT layouts.** Copy the exact
  field order from `c_src/include/pcre2.h`; watch the padding after a leading
  `uint32_t version`.
- Some rows describe inputs that are out-of-bounds in the C itself (e.g.
  `_pcre2_compile_get_hash_from_name8` with `length == 0` reads `name[-1]`).
  Those are not comparable observables — skip them with a comment saying why,
  rather than crashing the harness.

## Testing malloc-failure rows

`ERRORS.md` has rows reachable only when an allocation fails. Test them with a
counting allocator installed through a general context: fail the Nth `malloc`
for N = 0, 1, 2, ... and assert C and Rust return the SAME code for each N.
`phase_b_api.rs::custom_allocator_identical` already proves the two libraries
make the identical sequence of allocation requests, so this is well-defined.
Sketch:

```rust
static mut BUDGET: i64 = -1;              // -1 = unlimited
unsafe extern "C" fn fallible_malloc(n: usize, _d: *mut c_void) -> *mut c_void {
    let b = &mut *ptr::addr_of_mut!(BUDGET);
    if *b == 0 { return ptr::null_mut(); }
    if *b > 0 { *b -= 1; }
    /* real allocation */
}
```
Use a SEPARATE counter per library so the two runs cannot interfere, and reset
before each call.

## Required annotation

Every test case must record which `ERRORS.md` row(s) it covers, in a field
literally spelled `rows:` holding a slice of row numbers, e.g.

```rust
struct Case { rows: &'static [u32], pat: &'static str, /* ... */ }
const CASES: &[Case] = &[
    Case { rows: &[32], pat: "a\\", expect: 101 },
    Case { rows: &[33], pat: "a\\c", expect: 102 },
];
```

A coverage script greps `rows: &[...]` out of every `tests/phase_c_*.rs`, so the
spelling matters. If a row is genuinely unreachable in this build, still add an
entry for it with a comment explaining why and an assertion that C and Rust
agree on whatever the nearest reachable input does.
