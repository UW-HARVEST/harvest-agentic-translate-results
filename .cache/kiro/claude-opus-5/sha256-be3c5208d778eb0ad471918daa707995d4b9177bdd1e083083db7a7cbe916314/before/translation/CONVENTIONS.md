# C-to-Rust translation conventions for this crate

The task is a faithful, behaviour-preserving translation of PCRE2 (the C sources in
`../c_src`) into this Rust crate. The compiled `cdylib` must export the same linker
symbols as the C shared library and behave identically for the same inputs.

## Build configuration being translated

From `c_src/CMakeLists.txt` and `c_src/src/config.h`:

* `PCRE2_CODE_UNIT_WIDTH == 8` (so `PCRE2_UCHAR = u8`, `IMM2_SIZE = 2`)
* `SUPPORT_UNICODE` defined  (therefore `SUPPORT_WIDE_CHARS` is defined too)
* `HAVE_CONFIG_H` defined
* `SUPPORT_JIT` **not** defined
* `EBCDIC` **not** defined (ASCII / UTF-8 character names apply)
* `PCRE2_DEBUG` **not** defined
* `SUPPORT_VALGRIND` **not** defined
* `LINK_SIZE == 2`, `MATCH_LIMIT == 10000000`, `HEAP_LIMIT == 20000000`,
  `PARENS_NEST_LIMIT == 250`, `MAX_VARLOOKBEHIND == 255`, `NEWLINE_DEFAULT == 2`

Only translate the code that is actually compiled under that configuration. Ignore
`#ifdef EBCDIC`, `#ifdef SUPPORT_JIT`, `#ifdef PCRE2_DEBUG`, and 16/32-bit branches.

## Symbol naming

C uses macros in `pcre2.h` / `pcre2_internal.h` that append the code unit width:

* `pcre2_compile(...)`  -> linker symbol `pcre2_compile_8`
* `PRIV(name)` i.e. `_pcre2_name` -> linker symbol `_pcre2_name_8`

So every function that is `PCRE2_EXP_DEFN` or referenced through `PRIV()` from
another module must be exported as:

```rust
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_compile_8(/* exact C signature */) -> *mut pcre2_real_code { ... }
```

For a `PRIV()` function, export the C ABI symbol **and** provide a normal Rust
function that the rest of the crate calls directly, e.g.

```rust
pub unsafe fn valid_utf(string: PCRE2_SPTR, length: PCRE2_SIZE,
                        erroroffset: *mut PCRE2_SIZE) -> c_int { ... }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_valid_utf_8(string: PCRE2_SPTR, length: PCRE2_SIZE,
                                            erroroffset: *mut PCRE2_SIZE) -> c_int {
    unsafe { valid_utf(string, length, erroroffset) }
}
```

Static functions in C stay private Rust functions (no `no_mangle`).

## Types

Use the aliases from `crate::internal`:

| C                        | Rust                        |
|--------------------------|-----------------------------|
| `PCRE2_SIZE`, `size_t`   | `PCRE2_SIZE` (= `usize`)    |
| `PCRE2_UCHAR`            | `PCRE2_UCHAR` (= `u8`)      |
| `PCRE2_SPTR`             | `PCRE2_SPTR` (= `*const u8`)|
| `int`                    | `c_int`                     |
| `BOOL`                   | `BOOL` (= `c_int`), values `TRUE`/`FALSE` |
| `uint32_t` etc.          | `u32` etc.                  |

All the structs (`pcre2_real_code`, `pcre2_real_match_data`, `heapframe`,
`compile_block`, `match_block`, `dfa_match_block`, `pcre2_memctl`, ...), the option
and error constants, the opcodes, `CHAR_*`/`STR_*`/`STRING_*` names, and the UCD
access helpers already exist. Read these before writing code:

* `src/internal.rs`   - types, constants, structs, `GET`/`PUT`/`GETCHAR*` helpers,
                        UCD access, memory helpers, shared tables
* `src/opcodes.rs`    - `OP_*`, `OP_LENGTHS`, `ESC_*`
* `src/chars.rs`      - `CHAR_*` (u32), `STR_*` and `STRING_*` (`&[u8]`)
* `src/ucp.rs`        - `ucp_*` property values
* `src/ucd.rs`        - UCD tables
* `src/ucptables.rs`  - `UTT`, `UTT_NAMES`, `UTT_SIZE`
* `src/chartables.rs` - `DEFAULT_TABLES`
* `src/context.rs`    - an already-translated module to copy the style from

Add new items to your own module; do not edit `internal.rs`, `opcodes.rs`,
`chars.rs`, `ucp.rs`, `ucd.rs`, `ucptables.rs` or `chartables.rs`. If you genuinely
need something missing from them, define it locally in your module instead.

## Fidelity rules

1. **Do not fix bugs.** If the C has surprising or incorrect behaviour, reproduce it.
2. Preserve the exact order of error checks, validation and side effects.
3. Preserve integer widths and wrapping. C arithmetic on `uint32_t` wraps; use
   `wrapping_add`/`wrapping_sub`/`wrapping_mul` where overflow is possible, and be
   careful with signed/unsigned conversions. The crate is built with
   `overflow-checks = false`, but avoid relying on that.
4. Preserve exact output bytes for anything that writes text (error messages, etc.).
5. Keep the same control flow. `goto` chains can become `loop { ... break }`,
   labelled blocks, or small helper functions -- whatever preserves semantics.
6. Pointer arithmetic on the compiled code blob must stay pointer arithmetic
   (`ptr::add`, `ptr::offset_from`); the byte layout of compiled patterns matters.

## Style

* Keep C function names, converting to snake_case only where the C name is already
  snake_case (it usually is). Keep C comments that explain *why*; drop banner
  comments.
* `#![allow(...)]` at the top of the module as needed (`non_snake_case`,
  `non_upper_case_globals`, `unused_parens`, ...).
* Use safe Rust for local logic where it is a direct match, but do not restructure
  algorithms to please the borrow checker -- raw pointers are expected and fine.
* Do not add tests, benchmarks, or new dependencies. The crate has no dependencies.
* Do not run `cargo`; the top-level agent wires up `lib.rs` and builds.
