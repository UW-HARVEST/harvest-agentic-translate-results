# Verification of the C -> Rust translation

The Rust `cdylib` is compared against the reference C build of `c_src/`
(built with the unmodified `c_src/CMakeLists.txt`).

## 1. ABI / symbol parity

```
$ ./tools/symdiff.sh <c>/libpcre2.so target/release/libpcre2.so
C symbols:    143
Rust symbols: 143
--- missing from Rust:
--- extra in Rust (informational):
```

`nm -D --defined-only` produces the identical set of 143 exported symbols with
identical symbol *types* (`T`/`R`/`D`), including the macro-generated names such
as `_pcre2_compile_add_name_to_table8` (no underscore before the `8`) and
`_pcre2_compile_class_not_nested_8` (with one).

## 2. Data tables are byte-identical

`tools/dump_tables.c` dlopens a library and dumps every exported data table
(`_pcre2_ucd_records_8`, `_pcre2_ucd_stage1_8`, `_pcre2_ucd_stage2_8`,
`_pcre2_utt_8`, `_pcre2_utt_names_8`, `_pcre2_default_tables_8`,
`_pcre2_OP_lengths_8`, `_pcre2_ucd_*_sets_8`, ...). The dumps of the C and the
Rust library are byte-for-byte equal (28 tables, ~120 KB of data).

## 3. Structure layout parity

`src/internal.rs` ends with `const _: () = assert!(...)` static assertions for
the size of every shared structure and the offset of every field that the code
computes with `offsetof` (verified against the C build with a generated C
program): `pcre2_real_code` = 152, `heapframe` = 1048696,
`offset_of!(heapframe, ovector)` = 120, `match_block` = 272, `compile_block` =
360, etc. A layout regression therefore breaks the build.

## 4. Differential behaviour tests

All harnesses dlopen **both** libraries and compare results call by call.

| harness | what it covers | result |
|---|---|---|
| `tools/diffharness.c` | 135 curated patterns x 32 subjects x 9 match option sets: compile error code+offset, **the compiled byte code compared byte for byte**, all `pcre2_pattern_info()` items, `pcre2_match`, `pcre2_dfa_match`, ovectors, marks, startchar, `pcre2_substitute` (11 replacement/option combinations), substring extraction, substring lists, `pcre2_next_match`, match-data sizes, serialize/deserialize round trip, `pcre2_pattern_convert` (15 cases), `pcre2_config` (all 17 items), `pcre2_get_error_message` (all codes -80..230), `pcre2_maketables` | **116,595 checks, 0 mismatches** |
| `tools/apiharness.c` | `pcre2_callout_enumerate` (full callback log), `substring_*_byname`, `substring_nametable_scan`, `substring_length/copy_bynumber`, serialize with many codes + **cross-library decode**, compile with `pcre2_maketables`-generated tables, `code_copy_with_tables`, `next_match` loops, JIT stubs, general contexts | **1,556 checks, 0 mismatches** |
| `tools/fuzzharness.c`, `tools/fuzzharness2.c` | pseudo-random patterns built from ~160 syntax fragments x random subjects (incl. long subjects, invalid UTF-8) x random compile/extra/match options, newline and BSR conventions, optimization directives, match/depth/heap/offset limits, callouts and substitute callouts (the whole callback sequence is logged and compared), plus compiled-byte-code comparison | ~20 seeds x 20,000 iterations = **~4.6 million checks, 0 mismatches** |

Notes on what is deliberately *not* compared: PCRE2 leaves some memory
uninitialised (the ovector when there is no match, the mark/startchar fields
after a failed match, and the 0-3 alignment padding bytes between the name table
and the character lists inside a compiled pattern). Those regions contain
whatever the allocator returned, so they are excluded; everything else,
including every emitted opcode byte, is compared exactly.

## Reproducing

```sh
mkdir -p /tmp/cref && cd /tmp/cref && cmake <repo>/c_src -DCMAKE_BUILD_TYPE=Release && make
cd <repo> && cargo build --release
gcc -O1 tools/diffharness.c  -o /tmp/diffharness  -ldl
gcc -O1 tools/apiharness.c   -o /tmp/apiharness   -ldl
gcc -O1 tools/fuzzharness2.c -o /tmp/fuzzharness2 -ldl
/tmp/diffharness  /tmp/cref/libpcre2.so target/release/libpcre2.so
/tmp/apiharness   /tmp/cref/libpcre2.so target/release/libpcre2.so
/tmp/fuzzharness2 /tmp/cref/libpcre2.so target/release/libpcre2.so 20000 1
./tools/symdiff.sh /tmp/cref/libpcre2.so target/release/libpcre2.so
```
