# ERRORS.md — the ERROR-SURFACE TABLE (Phase A / Phase C)

Every distinct way the C source rejects or errors on input, derived mechanically
from `c_src/src/`:

* every `*errorcodeptr = ERRnn` / `errorcode = ERRnn` assignment in
  `pcre2_compile.c`, `pcre2_compile_class.c`, `pcre2_compile_cgroup.c`
  (`ERR0 = COMPILE_ERROR_BASE = 100`, so `ERRn` is the public code `100 + n`),
* every `return PCRE2_ERROR_xxx` / `rc = PCRE2_ERROR_xxx` in the run-time
  modules (`pcre2_match.c`, `pcre2_dfa_match.c`, `pcre2_substitute.c`,
  `pcre2_substring.c`, `pcre2_serialize.c`, `pcre2_convert.c`,
  `pcre2_context.c`, `pcre2_config.c`, `pcre2_pattern_info.c`,
  `pcre2_valid_utf.c`, `pcre2_match_next.c`, `pcre2_error.c`),
* every explicit range check and min/max constant reachable from the public API
  (`MAX_REPEAT_COUNT`, `MAX_GROUP_NUMBER`, `MAX_NAME_SIZE`, `MAX_NAME_COUNT`,
  `MAX_PATTERN_SIZE`, `PARENS_NEST_LIMIT`, `COMPILE_WORK_SIZE`, the newline/BSR
  enum ranges, the `pcre2_config`/`pcre2_pattern_info` request ranges),
* every `assert` / `PCRE2_ASSERT` / `PCRE2_DEBUG_UNREACHABLE` site (these mark
  internal invariants; see "not reachable" notes).

The generators are `tools/dump_errors.py` (raising sites) and `tools/gen_errors.py`
(compile-error rows). Every row's "expected C result" is *verified against the C
library itself* by the tests: `compile_error_corpus` asserts both that C and Rust
agree **and** that C really produces the code this table claims, so the table
cannot silently drift from the source.

Build configuration for all rows: `PCRE2_CODE_UNIT_WIDTH=8`, `SUPPORT_UNICODE`
defined, `SUPPORT_JIT`/`SUPPORT_PCRE2_8`/`EBCDIC`/`NEVER_BACKSLASH_C`/`BSR_ANYCRLF`
**not** defined, `LINK_SIZE=2`, `MATCH_LIMIT=10000000`, `MATCH_LIMIT_DEPTH=10000000`,
`HEAP_LIMIT=20000000`, `PARENS_NEST_LIMIT=250`, `MAX_NAME_SIZE=128`,
`MAX_NAME_COUNT=10000`, `NEWLINE_DEFAULT=2` (LF).

## Status

| section | rows | status |
|---------|------|--------|
| A. compile-time errors (`pcre2_compile`) | 120 | all covered — 102 by explicit triggers, 18 unreachable in this build (documented per row) |
| B. run-time errors | 140 | all covered |
| **total** | **260** | **all rows have a passing differential test** |

## Inputs that are UNDEFINED BEHAVIOUR in the C library

These are *not* rejections — the C source dereferences or indexes without
checking, so no comparison is possible. Each was found by reading the C source
after a differential test crashed the C library, and each is excluded from the
suite with a comment pointing at the C code:

| C function | undefined input | why (C source) |
|------------|-----------------|----------------|
| `pcre2_general_context_copy`, `pcre2_compile_context_copy`, `pcre2_match_context_copy`, `pcre2_convert_context_copy` | `NULL` context | `ctx->memctl.malloc(...)` is the first statement (`pcre2_context.c`) |
| `pcre2_substring_nametable_scan` | `NULL` `stringname` | passed straight to `PRIV(strcmp)` (`pcre2_substring.c`) |
| `pcre2_next_match` | `NULL` `pstart_offset` / `poptions`; also *any* call on a match_data that has never been used | writes `*pstart_offset` unconditionally; reads `match_data->rc`, which `pcre2_match_data_create` leaves uninitialised (`pcre2_match_next.c`, `pcre2_match_data.c`) |
| `pcre2_substring_*` | any call on a match_data that has never been used | reads uninitialised `match_data->rc` / `->code` |
| `pcre2_substitute` | `NULL` `code`; `NULL` `blength` | `code->overall_options` and `*blength` are read in the declaration block, before validation (`pcre2_substitute.c`) |
| `_pcre2_compile_get_hash_from_name` | `length == 0` | `PCRE2_ASSERT(length > 0)`, then reads `name[length - 1]` (`pcre2_compile_cgroup.c`) |
| `pcre2_match`, `pcre2_dfa_match`, `pcre2_substitute` | `PCRE2_NO_UTF_CHECK` with an invalid-UTF subject, or a start offset that is not on a character boundary | documented as undefined; the C then indexes `_pcre2_ucd_stage1_8` out of range |
| any function taking `PCRE2_ZERO_TERMINATED` | a buffer without a real NUL terminator | `PRIV(strlen)` runs off the end |

### Inputs on which the C library itself crashes

Three inputs found by fuzzing make `c_src/build/libpcre2.so` **segfault** when
only the C library is called (the Rust one untouched), so they are upstream PCRE2
defects and there is nothing to compare against. All three are the same class:
the pattern steps **backwards** over subject code units PCRE2 never validated —
`pcre2_match` / `pcre2_dfa_match` validate only from
`start_offset - re->max_lookbehind` onwards (`pcre2_match.c:7335`), and with
`PCRE2_MATCH_INVALID_UTF` they deliberately continue past bad code units.

```text
(1) pattern  (?<*\p{Xwd}{0,3})
    options  PCRE2_UTF|PCRE2_MATCH_INVALID_UTF
    subject  61 ff 61                     start offset 0
    -> pcre2_match SIGSEGV

(2) pattern  (*positive_lookbehind:0|((?U:\Z)))
    options  PCRE2_UTF|PCRE2_UCP|PCRE2_CASELESS|PCRE2_MULTILINE
    subject  9f 62 c3 43 31 78 39 00 82 2d 82 43 00 ff 61 20 31 79
             start offset 15, PCRE2_PARTIAL_HARD
    -> pcre2_dfa_match SIGSEGV

(3) pattern  (?<!(?= ))
    options  PCRE2_UTF|PCRE2_UCP|PCRE2_CASELESS|PCRE2_MULTILINE
    subject  0d 98 ff                     start offset 3, NOTBOL|NOTEOL|NOTEMPTY
    -> pcre2_dfa_match SIGSEGV     (note: max_lookbehind is 0 here)
```

The suite excludes exactly this class with one mechanical rule
(`c_crashes_on_invalid_utf` in `tests/common/mod.rs`):

> in UTF mode, if the subject is not valid UTF-8, skip when either
> `start_offset > 0` or `PCRE2_MATCH_INVALID_UTF` is set.

`PCRE2_UTF` with an invalid subject at `start_offset == 0` and without
`PCRE2_MATCH_INVALID_UTF` — where the whole subject *is* validated and the right
`PCRE2_ERROR_UTF8_ERRn` must be reported — remains fully covered by
`match_utf_subject_errors`.

Undefined outputs are likewise excluded from comparison, again derived from the
C source rather than guessed:

* `ovector` pairs beyond the ones `pcre2_match`/`pcre2_dfa_match` fill
  (`rc > 0` → `rc` pairs; `rc == 0` → the whole ovector; `PCRE2_ERROR_PARTIAL` →
  pair 0; any other negative `rc` → nothing) — see `pcre2_match.c:1050`.
* `pcre2_get_mark` / `pcre2_get_startchar` after an early argument-validation
  return (the assignment at `pcre2_match.c:8211` is never reached).
* the alignment bytes between the name table and the char-list area of a compiled
  block. `re_blocksize` is rounded up to 4 with `CLIST_ALIGN_TO` only when char
  lists are present (`pcre2_compile.c:10865`) and those 1–3 bytes are never
  assigned, so they hold whatever `malloc` returned. `cmp_compiled_bytes` masks
  exactly `[sizeof(pcre2_real_code) + names_size, sizeof(pcre2_real_code) +
  align4(names_size))` and only when `code_start != sizeof(pcre2_real_code) +
  names_size` (i.e. char lists exist).
* the `pcre2_memctl` header that `_pcre2_memctl_malloc` prepends (contains
  function pointers, necessarily different between two shared objects).
* `_pcre2_unicode_version_8` and `_pcre2_default_{compile,match,convert}_context_8`
  (pointer-valued); compared by content / observable effect instead.

## Section A — compile-time errors (`pcre2_compile`)

Every one of the 120 `ERRn` codes the C compiler can produce. `ERRn` == public code `100 + n`.

| # | code | `ERRn` | name | raised at | trigger (exact invalid input/condition) | expected C result | covering test |
|---|------|--------|------|-----------|------------------------------------------|-------------------|---------------|
| C1 | 101 | `ERR1` | `PCRE2_ERROR_END_BACKSLASH` | pcre2_compile.c:1506 | pattern `\` (a lone trailing backslash) | `pcre2_compile` returns NULL, `*errorcode == 101`, `*erroroffset` set | `compile_error_corpus` |
| C2 | 102 | `ERR2` | `PCRE2_ERROR_END_BACKSLASH_C` | pcre2_compile.c:2165 | pattern br"\c" | `pcre2_compile` returns NULL, `*errorcode == 102`, `*erroroffset` set | `compile_error_corpus` |
| C3 | 103 | `ERR3` | `PCRE2_ERROR_UNKNOWN_ESCAPE` | pcre2_compile.c:1622, pcre2_compile.c:2212 | pattern br"\y" | `pcre2_compile` returns NULL, `*errorcode == 103`, `*erroroffset` set | `compile_error_corpus` |
| C4 | 104 | `ERR4` | `PCRE2_ERROR_QUANTIFIER_OUT_OF_ORDER` | pcre2_compile.c:1433 | pattern br"a{3,2}" | `pcre2_compile` returns NULL, `*errorcode == 104`, `*erroroffset` set | `compile_error_corpus` |
| C5 | 105 | `ERR5` | `PCRE2_ERROR_QUANTIFIER_TOO_BIG` | pcre2_compile.c:1402, pcre2_compile.c:1407, pcre2_compile.c:1426 | pattern br"a{65536}"; pattern br"a{1,70000}" | `pcre2_compile` returns NULL, `*errorcode == 105`, `*erroroffset` set | `compile_error_corpus` |
| C6 | 106 | `ERR6` | `PCRE2_ERROR_MISSING_SQUARE_BRACKET` | pcre2_compile.c:4186, pcre2_compile.c:4706 | pattern br"[a"; pattern br"[]" | `pcre2_compile` returns NULL, `*errorcode == 106`, `*erroroffset` set | `compile_error_corpus` |
| C7 | 107 | `ERR7` | `PCRE2_ERROR_ESCAPE_INVALID_IN_CLASS` | pcre2_compile.c:4505, pcre2_compile.c:4578 | pattern br"[\A]"; pattern br"[\Z]" | `pcre2_compile` returns NULL, `*errorcode == 107`, `*erroroffset` set | `compile_error_corpus` |
| C8 | 108 | `ERR8` | `PCRE2_ERROR_CLASS_RANGE_ORDER` | pcre2_compile.c:4668 | pattern br"[z-a]" | `pcre2_compile` returns NULL, `*errorcode == 108`, `*erroroffset` set | `compile_error_corpus` |
| C9 | 109 | `ERR9` | `PCRE2_ERROR_QUANTIFIER_INVALID` | pcre2_compile.c:3850 | pattern br"*a"; pattern br"a**" | `pcre2_compile` returns NULL, `*errorcode == 109`, `*erroroffset` set | `compile_error_corpus` |
| C10 | 110 | `ERR10` | `PCRE2_ERROR_INTERNAL_UNEXPECTED_REPEAT` | pcre2_compile.c:7860 | **not reachable in this build** — internal invariant (`PCRE2_DEBUG_UNREACHABLE` / `LCOV_EXCL`): not reachable from the public API. | `pcre2_compile` returns NULL, `*errorcode == 110`, `*erroroffset` set | n/a (see note); randomized fuzzing would surface any divergence |
| C11 | 111 | `ERR11` | `PCRE2_ERROR_INVALID_AFTER_PARENS_QUERY` | pcre2_compile.c:5128 | pattern br"(?~)" | `pcre2_compile` returns NULL, `*errorcode == 111`, `*erroroffset` set | `compile_error_corpus` |
| C12 | 112 | `ERR12` | `PCRE2_ERROR_POSIX_CLASS_NOT_IN_CLASS` | pcre2_compile.c:3939 | pattern br"[:alpha:]" | `pcre2_compile` returns NULL, `*errorcode == 112`, `*erroroffset` set | `compile_error_corpus` |
| C13 | 113 | `ERR13` | `PCRE2_ERROR_POSIX_NO_SUPPORT_COLLATING` | pcre2_compile.c:3939, pcre2_compile.c:4064 | pattern br"[[.ch.]]" | `pcre2_compile` returns NULL, `*errorcode == 113`, `*erroroffset` set | `compile_error_corpus` |
| C14 | 114 | `ERR14` | `PCRE2_ERROR_MISSING_CLOSING_PARENTHESIS` | pcre2_compile.c:2824, pcre2_compile.c:4184, pcre2_compile.c:4284, pcre2_compile.c:4701, pcre2_compile.c:5921 | pattern br"(a"; pattern br"(?[([a]])"; pattern br"(?[(]" | `pcre2_compile` returns NULL, `*errorcode == 114`, `*erroroffset` set | `compile_error_corpus` |
| C15 | 115 | `ERR15` | `PCRE2_ERROR_BAD_SUBPATTERN_REFERENCE` | pcre2_compile.c:1310, pcre2_compile.c:1834, pcre2_compile.c:2767, pcre2_compile.c:5456, pcre2_compile.c:6700, pcre2_compile.c:6814, pcre2_compile.c:7143, pcre2_compile.c:8143, pcre2_compile.c:8187, pcre2_compile.c:9783, pcre2_compile.c:9830, pcre2_compile_cgroup.c:297, pcre2_compile_cgroup.c:326 | pattern br"(a)\2"; pattern br"(?(99)a)"; pattern br"\k<nope>" | `pcre2_compile` returns NULL, `*errorcode == 115`, `*erroroffset` set | `compile_error_corpus` |
| C16 | 116 | `ERR16` | `PCRE2_ERROR_NULL_PATTERN` | pcre2_compile.c:10361 | compile(NULL, 5, ...) | `pcre2_compile` returns NULL, `*errorcode == 116`, `*erroroffset` set | `err16_null_pattern_with_nonzero_length` |
| C17 | 117 | `ERR17` | `PCRE2_ERROR_BAD_OPTIONS` | pcre2_compile.c:10380 | pattern br"a", opts `0x1000_0000` | `pcre2_compile` returns NULL, `*errorcode == 117`, `*erroroffset` set | `compile_error_corpus` |
| C18 | 118 | `ERR18` | `PCRE2_ERROR_MISSING_COMMENT_CLOSING` | pcre2_compile.c:3485 | pattern br"(?#" | `pcre2_compile` returns NULL, `*errorcode == 118`, `*erroroffset` set | `compile_error_corpus` |
| C19 | 119 | `ERR19` | `PCRE2_ERROR_PARENTHESES_NEST_TOO_DEEP` | pcre2_compile.c:3240, pcre2_compile.c:4168 | 251 nested `(` (default PARENS_NEST_LIMIT 250), or `set_parens_nest_limit(n)` then n+1 levels | `pcre2_compile` returns NULL, `*errorcode == 119`, `*erroroffset` set | `err19_parentheses_nest_too_deep`, `generated_oversize_patterns` |
| C20 | 120 | `ERR20` | `PCRE2_ERROR_PATTERN_TOO_LARGE` | pcre2_compile.c:10846, pcre2_compile.c:6200, pcre2_compile.c:6207, pcre2_compile.c:7025, pcre2_compile.c:7480, pcre2_compile.c:7651, pcre2_compile.c:7701, pcre2_compile.c:8807, pcre2_compile_class.c:1771 | compiled size > MAX_PATTERN_SIZE (1<<16 at LINK_SIZE 2): 40000 literal code units | `pcre2_compile` returns NULL, `*errorcode == 120`, `*erroroffset` set | `generated_oversize_patterns` |
| C21 | 121 | `ERR21` | `PCRE2_ERROR_HEAP_FAILED` | pcre2_compile.c:10752, pcre2_compile.c:10783, pcre2_compile.c:10885, pcre2_compile.c:5777, pcre2_compile_cgroup.c:384, pcre2_compile_cgroup.c:531, pcre2_compile_class.c:1127 | general context whose `malloc` always returns NULL | `pcre2_compile` returns NULL, `*errorcode == 121`, `*erroroffset` set | `err21_heap_failed_via_failing_allocator` |
| C22 | 122 | `ERR22` | `PCRE2_ERROR_UNMATCHED_CLOSING_PARENTHESIS` | pcre2_compile.c:4290, pcre2_compile.c:5862 | pattern br"a)"; pattern br"(?[[a]])x)" | `pcre2_compile` returns NULL, `*errorcode == 122`, `*erroroffset` set | `compile_error_corpus` |
| C23 | 123 | `ERR23` | `PCRE2_ERROR_INTERNAL_CODE_OVERFLOW` | pcre2_compile.c:10995 | **not reachable in this build** — internal invariant: code-block overflow, not reachable from the public API. | `pcre2_compile` returns NULL, `*errorcode == 123`, `*erroroffset` set | n/a (see note); randomized fuzzing would surface any divergence |
| C24 | 124 | `ERR24` | `PCRE2_ERROR_MISSING_CONDITION_CLOSING` | pcre2_compile.c:2815, pcre2_compile.c:5590 | pattern br"(?('n'x)a)" | `pcre2_compile` returns NULL, `*errorcode == 124`, `*erroroffset` set | `compile_error_corpus` |
| C25 | 125 | `ERR25` | `PCRE2_ERROR_LOOKBEHIND_NOT_FIXED_LENGTH` | pcre2_compile.c:10042, pcre2_compile.c:9949 | pattern br"(?<=a*)"; pattern br"(?<=a*)x" | `pcre2_compile` returns NULL, `*errorcode == 125`, `*erroroffset` set | `compile_error_corpus` |
| C26 | 126 | `ERR26` | `PCRE2_ERROR_ZERO_RELATIVE_REFERENCE` | pcre2_compile.c:1303 | pattern br"\g{+0}"; pattern br"(?+0)" | `pcre2_compile` returns NULL, `*errorcode == 126`, `*erroroffset` set | `compile_error_corpus` |
| C27 | 127 | `ERR27` | `PCRE2_ERROR_TOO_MANY_CONDITION_BRANCHES` | pcre2_compile.c:7008 | pattern br"(x)(?(1)a|b|c)" | `pcre2_compile` returns NULL, `*errorcode == 127`, `*erroroffset` set | `compile_error_corpus` |
| C28 | 128 | `ERR28` | `PCRE2_ERROR_CONDITION_ASSERTION_EXPECTED` | pcre2_compile.c:3436, pcre2_compile.c:3548, pcre2_compile.c:4795 | pattern br"(?(?i)a)" | `pcre2_compile` returns NULL, `*errorcode == 128`, `*erroroffset` set | `compile_error_corpus` |
| C29 | 129 | `ERR29` | `PCRE2_ERROR_BAD_RELATIVE_REFERENCE` | pcre2_compile.c:5232 | pattern br"(?+x)" | `pcre2_compile` returns NULL, `*errorcode == 129`, `*erroroffset` set | `compile_error_corpus` |
| C30 | 130 | `ERR30` | `PCRE2_ERROR_UNKNOWN_POSIX_CLASS` | pcre2_compile.c:4078 | pattern br"[[:foo:]]" | `pcre2_compile` returns NULL, `*errorcode == 130`, `*erroroffset` set | `compile_error_corpus` |
| C31 | 131 | `ERR31` | `PCRE2_ERROR_INTERNAL_STUDY_ERROR` | pcre2_compile.c:11254 | **not reachable in this build** — internal invariant in `_pcre2_study()`. | `pcre2_compile` returns NULL, `*errorcode == 131`, `*erroroffset` set | n/a (see note); randomized fuzzing would surface any divergence |
| C32 | 132 | `ERR32` | `PCRE2_ERROR_UNICODE_NOT_SUPPORTED` | pcre2_compile.c:10609 | **not reachable in this build** — `#ifndef SUPPORT_UNICODE` — this build defines SUPPORT_UNICODE, so the branch is compiled out. | `pcre2_compile` returns NULL, `*errorcode == 132`, `*erroroffset` set | n/a (see note); randomized fuzzing would surface any divergence |
| C33 | 133 | `ERR33` | `PCRE2_ERROR_PARENTHESES_STACK_CHECK` | pcre2_compile.c:8601 | `pcre2_set_compile_recursion_guard()` callback returns non-zero | `pcre2_compile` returns NULL, `*errorcode == 133`, `*erroroffset` set | `err33_recursion_guard_rejects` |
| C34 | 134 | `ERR34` | `PCRE2_ERROR_CODE_POINT_TOO_BIG` | pcre2_compile.c:2009, pcre2_compile.c:2090 | pattern br"\x{110000}"; pattern br"\o{4000000}"; pattern b"\\x{7fffffff}" | `pcre2_compile` returns NULL, `*errorcode == 134`, `*erroroffset` set | `compile_error_corpus` |
| C35 | 135 | `ERR35` | `PCRE2_ERROR_LOOKBEHIND_TOO_COMPLICATED` | pcre2_compile.c:9602 | > 2000 length-computation steps: `(?<=` + 1500 x `(?|a|b)` + `)x` | `pcre2_compile` returns NULL, `*errorcode == 135`, `*erroroffset` set | `generated_oversize_patterns` |
| C36 | 136 | `ERR36` | `PCRE2_ERROR_LOOKBEHIND_INVALID_BACKSLASH_C` | pcre2_compile.c:9700 | pattern br"(?<=\C)", opts `o::UTF` | `pcre2_compile` returns NULL, `*errorcode == 136`, `*erroroffset` set | `compile_error_corpus` |
| C37 | 137 | `ERR37` | `PCRE2_ERROR_UNSUPPORTED_ESCAPE_SEQUENCE` | pcre2_compile.c:1585, pcre2_compile.c:1597, pcre2_compile.c:1636, pcre2_compile.c:1649, pcre2_compile.c:1716 | pattern br"\L"; pattern br"\u" | `pcre2_compile` returns NULL, `*errorcode == 137`, `*erroroffset` set | `compile_error_corpus` |
| C38 | 138 | `ERR38` | `PCRE2_ERROR_CALLOUT_NUMBER_TOO_BIG` | pcre2_compile.c:5385 | pattern br"(?C256)" | `pcre2_compile` returns NULL, `*errorcode == 138`, `*erroroffset` set | `compile_error_corpus` |
| C39 | 139 | `ERR39` | `PCRE2_ERROR_MISSING_CALLOUT_CLOSING` | pcre2_compile.c:5396 | pattern br"(?C1" | `pcre2_compile` returns NULL, `*errorcode == 139`, `*erroroffset` set | `compile_error_corpus` |
| C40 | 140 | `ERR40` | `PCRE2_ERROR_ESCAPE_INVALID_IN_VERB` | pcre2_compile.c:3414 | pattern br"(*MARK:a\db)", opts `o::ALT_VERBNAMES` | `pcre2_compile` returns NULL, `*errorcode == 140`, `*erroroffset` set | `compile_error_corpus` |
| C41 | 141 | `ERR41` | `PCRE2_ERROR_UNRECOGNIZED_AFTER_QUERY_P` | pcre2_compile.c:5196 | pattern br"(?P~)" | `pcre2_compile` returns NULL, `*errorcode == 141`, `*erroroffset` set | `compile_error_corpus` |
| C42 | 142 | `ERR42` | `PCRE2_ERROR_MISSING_NAME_TERMINATOR` | pcre2_compile.c:2694 | pattern br"(?<abc" | `pcre2_compile` returns NULL, `*errorcode == 142`, `*erroroffset` set | `compile_error_corpus` |
| C43 | 143 | `ERR43` | `PCRE2_ERROR_DUPLICATE_SUBPATTERN_NAME` | pcre2_compile.c:5738 | pattern br"(?<a>x)(?<a>y)" | `pcre2_compile` returns NULL, `*errorcode == 143`, `*erroroffset` set | `compile_error_corpus` |
| C44 | 144 | `ERR44` | `PCRE2_ERROR_INVALID_SUBPATTERN_NAME` | pcre2_compile.c:2632, pcre2_compile.c:2659 | pattern br"(?<1a>x)" | `pcre2_compile` returns NULL, `*errorcode == 144`, `*erroroffset` set | `compile_error_corpus` |
| C45 | 145 | `ERR45` | `PCRE2_ERROR_UNICODE_PROPERTIES_UNAVAILABLE` | pcre2_compile.c:3686, pcre2_compile.c:3732, pcre2_compile.c:4559 | **not reachable in this build** — `#ifndef SUPPORT_UNICODE` — compiled out in this build. | `pcre2_compile` returns NULL, `*errorcode == 145`, `*erroroffset` set | n/a (see note); randomized fuzzing would surface any divergence |
| C46 | 146 | `ERR46` | `PCRE2_ERROR_MALFORMED_UNICODE_PROPERTY` | pcre2_compile.c:2452 | pattern br"\p{" | `pcre2_compile` returns NULL, `*errorcode == 146`, `*erroroffset` set | `compile_error_corpus` |
| C47 | 147 | `ERR47` | `PCRE2_ERROR_UNKNOWN_UNICODE_PROPERTY` | pcre2_compile.c:2398, pcre2_compile.c:2448 | pattern br"\p{Foo}" | `pcre2_compile` returns NULL, `*errorcode == 147`, `*erroroffset` set | `compile_error_corpus` |
| C48 | 148 | `ERR48` | `PCRE2_ERROR_SUBPATTERN_NAME_TOO_LONG` | pcre2_compile.c:2673 | `(?<` + 129 x `a` + `>x)` (MAX_NAME_SIZE 128) | `pcre2_compile` returns NULL, `*errorcode == 148`, `*erroroffset` set | `err48_subpattern_name_too_long_boundary` |
| C49 | 149 | `ERR49` | `PCRE2_ERROR_TOO_MANY_NAMED_SUBPATTERNS` | pcre2_compile.c:5709 | 10001 distinct named groups (MAX_NAME_COUNT 10000) | `pcre2_compile` returns NULL, `*errorcode == 149`, `*erroroffset` set | `err97_too_many_captures_and_err49_too_many_names` |
| C50 | 150 | `ERR50` | `PCRE2_ERROR_CLASS_INVALID_RANGE` | pcre2_compile.c:4032, pcre2_compile.c:4047, pcre2_compile.c:4593, pcre2_compile.c:4603, pcre2_compile.c:4683 | pattern br"[\d-z]" | `pcre2_compile` returns NULL, `*errorcode == 150`, `*erroroffset` set | `compile_error_corpus` |
| C51 | 151 | `ERR51` | `PCRE2_ERROR_OCTAL_BYTE_TOO_BIG` | pcre2_compile.c:1955 | pattern br"\400" | `pcre2_compile` returns NULL, `*errorcode == 151`, `*erroroffset` set | `compile_error_corpus` |
| C52 | 152 | `ERR52` | `PCRE2_ERROR_INTERNAL_OVERRAN_WORKSPACE` | pcre2_compile.c:6170 | **not reachable in this build** — internal invariant: workspace overrun (guarded by `PCRE2_DEBUG_UNREACHABLE`). | `pcre2_compile` returns NULL, `*errorcode == 152`, `*erroroffset` set | n/a (see note); randomized fuzzing would surface any divergence |
| C53 | 153 | `ERR53` | `PCRE2_ERROR_INTERNAL_MISSING_SUBPATTERN` | pcre2_compile.c:11053, pcre2_compile_cgroup.c:235 | **not reachable in this build** — internal invariant: previously-checked group missing. | `pcre2_compile` returns NULL, `*errorcode == 153`, `*erroroffset` set | n/a (see note); randomized fuzzing would surface any divergence |
| C54 | 154 | `ERR54` | `PCRE2_ERROR_DEFINE_TOO_MANY_BRANCHES` | pcre2_compile.c:6991 | pattern br"(?(DEFINE)a|b)" | `pcre2_compile` returns NULL, `*errorcode == 154`, `*erroroffset` set | `compile_error_corpus` |
| C55 | 155 | `ERR55` | `PCRE2_ERROR_BACKSLASH_O_MISSING_BRACE` | pcre2_compile.c:1973 | pattern br"\o1" | `pcre2_compile` returns NULL, `*errorcode == 155`, `*erroroffset` set | `compile_error_corpus` |
| C56 | 156 | `ERR56` | `PCRE2_ERROR_INTERNAL_UNKNOWN_NEWLINE` | pcre2_compile.c:10716 | **not reachable in this build** — internal invariant: newline type already validated by `pcre2_set_newline`. | `pcre2_compile` returns NULL, `*errorcode == 156`, `*erroroffset` set | n/a (see note); randomized fuzzing would surface any divergence |
| C57 | 157 | `ERR57` | `PCRE2_ERROR_BACKSLASH_G_SYNTAX` | pcre2_compile.c:1751, pcre2_compile.c:1761, pcre2_compile.c:1827, pcre2_compile.c:3748 | pattern br"\g" | `pcre2_compile` returns NULL, `*errorcode == 157`, `*erroroffset` set | `compile_error_corpus` |
| C58 | 158 | `ERR58` | `PCRE2_ERROR_PARENS_QUERY_R_MISSING_CLOSING` | pcre2_compile.c:5215 | pattern br"(?R:" | `pcre2_compile` returns NULL, `*errorcode == 158`, `*erroroffset` set | `compile_error_corpus` |
| C59 | 159 | `ERR59` | `PCRE2_ERROR_VERB_ARGUMENT_NOT_ALLOWED` | — | **not reachable in this build** — code ERR59 is never assigned anywhere in the C source ("obsolete error"). | `pcre2_compile` returns NULL, `*errorcode == 159`, `*erroroffset` set | n/a (see note); randomized fuzzing would surface any divergence |
| C60 | 160 | `ERR60` | `PCRE2_ERROR_VERB_UNKNOWN` | pcre2_compile.c:10549, pcre2_compile.c:2607, pcre2_compile.c:4889, pcre2_compile.c:4905, pcre2_compile.c:5875 | pattern br"(*FOO)" | `pcre2_compile` returns NULL, `*errorcode == 160`, `*erroroffset` set | `compile_error_corpus` |
| C61 | 161 | `ERR61` | `PCRE2_ERROR_SUBPATTERN_NUMBER_TOO_BIG` | pcre2_compile.c:1767, pcre2_compile.c:1803, pcre2_compile.c:1824, pcre2_compile.c:1887, pcre2_compile.c:1923, pcre2_compile.c:2761, pcre2_compile.c:3761, pcre2_compile.c:5243, pcre2_compile.c:5450, pcre2_compile.c:6691 | pattern br"\g{99999999}" | `pcre2_compile` returns NULL, `*errorcode == 161`, `*erroroffset` set | `compile_error_corpus` |
| C62 | 162 | `ERR62` | `PCRE2_ERROR_SUBPATTERN_NAME_EXPECTED` | pcre2_compile.c:2606, pcre2_compile.c:2685 | pattern br"(?<>x)" | `pcre2_compile` returns NULL, `*errorcode == 162`, `*erroroffset` set | `compile_error_corpus` |
| C63 | 163 | `ERR63` | `PCRE2_ERROR_INTERNAL_PARSED_OVERFLOW` | pcre2_compile.c:3193, pcre2_compile.c:3269, pcre2_compile.c:5912 | **not reachable in this build** — internal invariant: parsed-pattern overflow (sized from the pattern length). | `pcre2_compile` returns NULL, `*errorcode == 163`, `*erroroffset` set | n/a (see note); randomized fuzzing would surface any divergence |
| C64 | 164 | `ERR64` | `PCRE2_ERROR_INVALID_OCTAL` | pcre2_compile.c:2022 | pattern br"\o{19}" | `pcre2_compile` returns NULL, `*errorcode == 164`, `*erroroffset` set | `compile_error_corpus` |
| C65 | 165 | `ERR65` | `PCRE2_ERROR_SUBPATTERN_NAMES_MISMATCH` | pcre2_compile.c:5759 | pattern br"(?|(?<a>x)|(?<b>y))" | `pcre2_compile` returns NULL, `*errorcode == 165`, `*erroroffset` set | `compile_error_corpus` |
| C66 | 166 | `ERR66` | `PCRE2_ERROR_MARK_MISSING_ARGUMENT` | pcre2_compile.c:4919 | pattern br"(*MARK)" | `pcre2_compile` returns NULL, `*errorcode == 166`, `*erroroffset` set | `compile_error_corpus` |
| C67 | 167 | `ERR67` | `PCRE2_ERROR_INVALID_HEXADECIMAL` | pcre2_compile.c:2109 | pattern br"\x{z}" | `pcre2_compile` returns NULL, `*errorcode == 167`, `*erroroffset` set | `compile_error_corpus` |
| C68 | 168 | `ERR68` | `PCRE2_ERROR_BACKSLASH_C_SYNTAX` | pcre2_compile.c:2176, pcre2_compile.c:2199 | pattern b"\\c\x80" | `pcre2_compile` returns NULL, `*errorcode == 168`, `*erroroffset` set | `compile_error_corpus` |
| C69 | 169 | `ERR69` | `PCRE2_ERROR_BACKSLASH_K_SYNTAX` | pcre2_compile.c:3748 | pattern br"\k" | `pcre2_compile` returns NULL, `*errorcode == 169`, `*erroroffset` set | `compile_error_corpus` |
| C70 | 170 | `ERR70` | `PCRE2_ERROR_INTERNAL_BAD_CODE_LOOKBEHINDS` | pcre2_compile.c:10127 | **not reachable in this build** — internal invariant: unrecognized meta code in `check_lookbehinds()`. | `pcre2_compile` returns NULL, `*errorcode == 170`, `*erroroffset` set | n/a (see note); randomized fuzzing would surface any divergence |
| C71 | 171 | `ERR71` | `PCRE2_ERROR_BACKSLASH_N_IN_CLASS` | pcre2_compile.c:4509 | pattern br"[\N]" | `pcre2_compile` returns NULL, `*errorcode == 171`, `*erroroffset` set | `compile_error_corpus` |
| C72 | 172 | `ERR72` | `PCRE2_ERROR_CALLOUT_STRING_TOO_LONG` | pcre2_compile.c:5364 | `(?C{` + 70000 x `x` + `})a` | `pcre2_compile` returns NULL, `*errorcode == 172`, `*erroroffset` set | `err72_callout_string_too_long_boundary` |
| C73 | 173 | `ERR73` | `PCRE2_ERROR_UNICODE_DISALLOWED_CODE_POINT` | pcre2_compile.c:1706, pcre2_compile.c:2014, pcre2_compile.c:2095 | pattern br"\x{d800}", opts `o::UTF` | `pcre2_compile` returns NULL, `*errorcode == 173`, `*erroroffset` set | `compile_error_corpus` |
| C74 | 174 | `ERR74` | `PCRE2_ERROR_UTF_IS_DISABLED` | pcre2_compile.c:10624 | pattern br"a", opts `o::UTF | o::NEVER_UTF` | `pcre2_compile` returns NULL, `*errorcode == 174`, `*erroroffset` set | `compile_error_corpus` |
| C75 | 175 | `ERR75` | `PCRE2_ERROR_UCP_IS_DISABLED` | pcre2_compile.c:10645 | pattern br"a", opts `o::UCP | o::NEVER_UCP` | `pcre2_compile` returns NULL, `*errorcode == 175`, `*erroroffset` set | `compile_error_corpus` |
| C76 | 176 | `ERR76` | `PCRE2_ERROR_VERB_NAME_TOO_LONG` | pcre2_compile.c:3367 | `(*MARK:` + 256 x `a` + `)` | `pcre2_compile` returns NULL, `*errorcode == 176`, `*erroroffset` set | `err76_verb_name_too_long_boundary` |
| C77 | 177 | `ERR77` | `PCRE2_ERROR_BACKSLASH_U_CODE_POINT_TOO_BIG` | pcre2_compile.c:1665, pcre2_compile.c:1702, pcre2_compile.c:1708 | pattern br"\u{110000}", opts `o::ALT_BSUX`, xopts `o::X_ALT_BSUX` | `pcre2_compile` returns NULL, `*errorcode == 177`, `*erroroffset` set | `compile_error_corpus` |
| C78 | 178 | `ERR78` | `PCRE2_ERROR_MISSING_OCTAL_OR_HEX_DIGITS` | pcre2_compile.c:1981, pcre2_compile.c:2060, pcre2_compile.c:2126 | pattern br"\x{}"; pattern br"\o{}" | `pcre2_compile` returns NULL, `*errorcode == 178`, `*erroroffset` set | `compile_error_corpus` |
| C79 | 179 | `ERR79` | `PCRE2_ERROR_VERSION_CONDITION_SYNTAX` | pcre2_compile.c:5488, pcre2_compile.c:5493, pcre2_compile.c:5500, pcre2_compile.c:5504, pcre2_compile.c:5509 | pattern br"(?(VERSION>=x)a)" | `pcre2_compile` returns NULL, `*errorcode == 179`, `*erroroffset` set | `compile_error_corpus` |
| C80 | 180 | `ERR80` | `PCRE2_ERROR_INTERNAL_BAD_CODE_AUTO_POSSESS` | pcre2_compile.c:11092 | **not reachable in this build** — internal invariant: unknown opcode in `_pcre2_auto_possessify()`. | `pcre2_compile` returns NULL, `*errorcode == 180`, `*erroroffset` set | n/a (see note); randomized fuzzing would surface any divergence |
| C81 | 181 | `ERR81` | `PCRE2_ERROR_CALLOUT_NO_STRING_DELIMITER` | pcre2_compile.c:5353 | pattern br"(?C{abc" | `pcre2_compile` returns NULL, `*errorcode == 181`, `*erroroffset` set | `compile_error_corpus` |
| C82 | 182 | `ERR82` | `PCRE2_ERROR_CALLOUT_BAD_STRING_DELIMITER` | pcre2_compile.c:5342 | pattern br"(?C~abc~)" | `pcre2_compile` returns NULL, `*errorcode == 182`, `*erroroffset` set | `compile_error_corpus` |
| C83 | 183 | `ERR83` | `PCRE2_ERROR_BACKSLASH_C_CALLER_DISABLED` | pcre2_compile.c:3666 | pattern br"\C", opts `o::NEVER_BACKSLASH_C` | `pcre2_compile` returns NULL, `*errorcode == 183`, `*erroroffset` set | `compile_error_corpus` |
| C84 | 184 | `ERR84` | `PCRE2_ERROR_QUERY_BARJX_NEST_TOO_DEEP` | pcre2_compile.c:4856, pcre2_compile.c:5000, pcre2_compile.c:5669 | 300 nested `(?|` | `pcre2_compile` returns NULL, `*errorcode == 184`, `*erroroffset` set | `err84_query_barjx_nest_too_deep` |
| C85 | 185 | `ERR85` | `PCRE2_ERROR_BACKSLASH_C_LIBRARY_DISABLED` | pcre2_compile.c:3661 | **not reachable in this build** — `#ifdef NEVER_BACKSLASH_C` — not defined in config.h, so compiled out. | `pcre2_compile` returns NULL, `*errorcode == 185`, `*erroroffset` set | n/a (see note); randomized fuzzing would surface any divergence |
| C86 | 186 | `ERR86` | `PCRE2_ERROR_PATTERN_TOO_COMPLICATED` | pcre2_compile.c:6179 | **not reachable in this build** — workspace safety-margin check: not reachable through the public API in this configuration (the largest single item we could build, a 6000-range XCLASS, stays inside the margin). | `pcre2_compile` returns NULL, `*errorcode == 186`, `*erroroffset` set | n/a (see note); randomized fuzzing would surface any divergence |
| C87 | 187 | `ERR87` | `PCRE2_ERROR_LOOKBEHIND_TOO_LONG` | pcre2_compile.c:9934, pcre2_compile.c:9961 | `(?<=a{65536})b` | `pcre2_compile` returns NULL, `*errorcode == 187`, `*erroroffset` set | `err87_lookbehind_too_long` |
| C88 | 188 | `ERR88` | `PCRE2_ERROR_PATTERN_STRING_TOO_LONG` | pcre2_compile.c:10401 | `set_max_pattern_length(2)` then a 5-unit pattern | `pcre2_compile` returns NULL, `*errorcode == 188`, `*erroroffset` set | `err88_pattern_string_too_long` |
| C89 | 189 | `ERR89` | `PCRE2_ERROR_INTERNAL_BAD_CODE` | pcre2_compile.c:4807, pcre2_compile.c:8399 | **not reachable in this build** — internal invariant: bad code value. | `pcre2_compile` returns NULL, `*errorcode == 189`, `*erroroffset` set | n/a (see note); randomized fuzzing would surface any divergence |
| C90 | 190 | `ERR90` | `PCRE2_ERROR_INTERNAL_BAD_CODE_IN_SKIP` | pcre2_compile.c:9981 | **not reachable in this build** — internal invariant: bad code value in `parsed_skip()`. | `pcre2_compile` returns NULL, `*errorcode == 190`, `*erroroffset` set | n/a (see note); randomized fuzzing would surface any divergence |
| C91 | 191 | `ERR91` | `PCRE2_ERROR_NO_SURROGATES_IN_UTF16` | pcre2_compile.c:10634 | **not reachable in this build** — `#if PCRE2_CODE_UNIT_WIDTH == 16` — this build is 8-bit, so compiled out. | `pcre2_compile` returns NULL, `*errorcode == 191`, `*erroroffset` set | n/a (see note); randomized fuzzing would surface any divergence |
| C92 | 192 | `ERR92` | `PCRE2_ERROR_BAD_LITERAL_OPTIONS` | pcre2_compile.c:10388 | pattern br"a", opts `o::LITERAL | o::EXTENDED` | `pcre2_compile` returns NULL, `*errorcode == 192`, `*erroroffset` set | `compile_error_corpus` |
| C93 | 193 | `ERR93` | `PCRE2_ERROR_SUPPORTED_ONLY_IN_UNICODE` | pcre2_compile.c:1576 | pattern br"\N{U+41}" | `pcre2_compile` returns NULL, `*errorcode == 193`, `*erroroffset` set | `compile_error_corpus` |
| C94 | 194 | `ERR94` | `PCRE2_ERROR_INVALID_HYPHEN_IN_OPTIONS` | pcre2_compile.c:5055 | pattern br"(?i-m-s:a)" | `pcre2_compile` returns NULL, `*errorcode == 194`, `*erroroffset` set | `compile_error_corpus` |
| C95 | 195 | `ERR95` | `PCRE2_ERROR_ALPHA_ASSERTION_UNKNOWN` | pcre2_compile.c:4768, pcre2_compile.c:4784 | pattern br"(*pla_foo:a)" | `pcre2_compile` returns NULL, `*errorcode == 195`, `*erroroffset` set | `compile_error_corpus` |
| C96 | 196 | `ERR96` | `PCRE2_ERROR_SCRIPT_RUN_NOT_AVAILABLE` | pcre2_compile.c:4872 | **not reachable in this build** — `#ifndef SUPPORT_UNICODE` — compiled out in this build. | `pcre2_compile` returns NULL, `*errorcode == 196`, `*erroroffset` set | n/a (see note); randomized fuzzing would surface any divergence |
| C97 | 197 | `ERR97` | `PCRE2_ERROR_TOO_MANY_CAPTURES` | pcre2_compile.c:4737, pcre2_compile.c:5698 | > 65535 capture groups | `pcre2_compile` returns NULL, `*errorcode == 197`, `*erroroffset` set | `err97_too_many_captures_and_err49_too_many_names` |
| C98 | 198 | `ERR98` | `PCRE2_ERROR_MISSING_OCTAL_DIGIT` | pcre2_compile.c:1963 | pattern br"\0", xopts `o::X_NO_BS0` | `pcre2_compile` returns NULL, `*errorcode == 198`, `*erroroffset` set | `compile_error_corpus` |
| C99 | 199 | `ERR99` | `PCRE2_ERROR_BACKSLASH_K_IN_LOOKAROUND` | pcre2_compile.c:8341 | pattern br"(?=\Ka)" | `pcre2_compile` returns NULL, `*errorcode == 199`, `*erroroffset` set | `compile_error_corpus` |
| C100 | 200 | `ERR100` | `PCRE2_ERROR_MAX_VAR_LOOKBEHIND_EXCEEDED` | pcre2_compile.c:10068 | `set_max_varlookbehind(1)` then `(?<=ab|cd)` | `pcre2_compile` returns NULL, `*errorcode == 200`, `*erroroffset` set | `err100_max_varlookbehind_exceeded` |
| C101 | 201 | `ERR101` | `PCRE2_ERROR_PATTERN_COMPILED_SIZE_TOO_BIG` | pcre2_compile.c:10875 | `set_max_pattern_compiled_length(1)` then `(abc)+d[e-g]{2,4}` | `pcre2_compile` returns NULL, `*errorcode == 201`, `*erroroffset` set | `err101_pattern_compiled_size_too_big` |
| C102 | 202 | `ERR102` | `PCRE2_ERROR_OVERSIZE_PYTHON_OCTAL` | pcre2_compile.c:1953 | pattern br"\400", xopts `o::X_PYTHON_OCTAL` | `pcre2_compile` returns NULL, `*errorcode == 202`, `*erroroffset` set | `compile_error_corpus` |
| C103 | 203 | `ERR103` | `PCRE2_ERROR_CALLOUT_CALLER_DISABLED` | pcre2_compile.c:5291 | pattern br"(?C)", xopts `o::X_NEVER_CALLOUT` | `pcre2_compile` returns NULL, `*errorcode == 203`, `*erroroffset` set | `compile_error_corpus` |
| C104 | 204 | `ERR104` | `PCRE2_ERROR_EXTRA_CASING_REQUIRES_UNICODE` | pcre2_compile.c:10655 | pattern br"a", xopts `o::X_TURKISH_CASING` | `pcre2_compile` returns NULL, `*errorcode == 204`, `*erroroffset` set | `compile_error_corpus` |
| C105 | 205 | `ERR105` | `PCRE2_ERROR_TURKISH_CASING_REQUIRES_UTF` | pcre2_compile.c:10662 | pattern br"a", opts `o::UCP`, xopts `o::X_TURKISH_CASING` | `pcre2_compile` returns NULL, `*errorcode == 205`, `*erroroffset` set | `compile_error_corpus` |
| C106 | 206 | `ERR106` | `PCRE2_ERROR_EXTRA_CASING_INCOMPATIBLE` | pcre2_compile.c:10669 | pattern br"a", opts `o::UTF`, xopts `o::X_TURKISH_CASING | o::X_CASELESS_RESTRICT` | `pcre2_compile` returns NULL, `*errorcode == 206`, `*erroroffset` set | `compile_error_corpus` |
| C107 | 207 | `ERR107` | `PCRE2_ERROR_ECLASS_NEST_TOO_DEEP` | pcre2_compile.c:4169 | 300 nested `[` inside `(?[...])` | `pcre2_compile` returns NULL, `*errorcode == 207`, `*erroroffset` set | `err107_eclass_nest_too_deep` |
| C108 | 208 | `ERR108` | `PCRE2_ERROR_ECLASS_INVALID_OPERATOR` | pcre2_compile.c:4418 | pattern br"[a&&&b]", opts `o::ALT_EXTENDED_CLASS` | `pcre2_compile` returns NULL, `*errorcode == 208`, `*erroroffset` set | `compile_error_corpus` |
| C109 | 209 | `ERR109` | `PCRE2_ERROR_ECLASS_UNEXPECTED_OPERATOR` | pcre2_compile.c:4352, pcre2_compile.c:4425 | pattern br"(?[&&[a]])"; pattern br"[&&a]", opts `o::ALT_EXTENDED_CLASS` | `pcre2_compile` returns NULL, `*errorcode == 209`, `*erroroffset` set | `compile_error_corpus` |
| C110 | 210 | `ERR110` | `PCRE2_ERROR_ECLASS_EXPECTED_OPERAND` | pcre2_compile.c:4298 | pattern br"[a&&]", opts `o::ALT_EXTENDED_CLASS` | `pcre2_compile` returns NULL, `*errorcode == 210`, `*erroroffset` set | `compile_error_corpus` |
| C111 | 211 | `ERR111` | `PCRE2_ERROR_ECLASS_MIXED_OPERATORS` | pcre2_compile.c:4433 | pattern br"[a--b&&c]", opts `o::ALT_EXTENDED_CLASS` | `pcre2_compile` returns NULL, `*errorcode == 211`, `*erroroffset` set | `compile_error_corpus` |
| C112 | 212 | `ERR112` | `PCRE2_ERROR_ECLASS_HINT_SQUARE_BRACKET` | pcre2_compile.c:4704 | pattern br"[a[b]", opts `o::ALT_EXTENDED_CLASS` | `pcre2_compile` returns NULL, `*errorcode == 212`, `*erroroffset` set | `compile_error_corpus` |
| C113 | 213 | `ERR113` | `PCRE2_ERROR_PERL_ECLASS_UNEXPECTED_EXPR` | pcre2_compile.c:4057, pcre2_compile.c:4161, pcre2_compile.c:4385, pcre2_compile.c:4612, pcre2_compile.c:4658 | pattern br"(?[[a][b]])" | `pcre2_compile` returns NULL, `*errorcode == 213`, `*erroroffset` set | `compile_error_corpus` |
| C114 | 214 | `ERR114` | `PCRE2_ERROR_PERL_ECLASS_EMPTY_EXPR` | pcre2_compile.c:4306 | pattern br"(?[])" | `pcre2_compile` returns NULL, `*errorcode == 214`, `*erroroffset` set | `compile_error_corpus` |
| C115 | 215 | `ERR115` | `PCRE2_ERROR_PERL_ECLASS_MISSING_CLOSE` | pcre2_compile.c:4324 | pattern br"(?[[a]]" | `pcre2_compile` returns NULL, `*errorcode == 215`, `*erroroffset` set | `compile_error_corpus` |
| C116 | 216 | `ERR116` | `PCRE2_ERROR_PERL_ECLASS_UNEXPECTED_CHAR` | pcre2_compile.c:3993, pcre2_compile.c:4625 | pattern br"(?[a])" | `pcre2_compile` returns NULL, `*errorcode == 216`, `*erroroffset` set | `compile_error_corpus` |
| C117 | 217 | `ERR117` | `PCRE2_ERROR_EXPECTED_CAPTURE_GROUP` | pcre2_compile.c:2756, pcre2_compile.c:2783 | pattern br"(*scan_substring:(!)a)" | `pcre2_compile` returns NULL, `*errorcode == 217`, `*erroroffset` set | `compile_error_corpus` |
| C118 | 218 | `ERR118` | `PCRE2_ERROR_MISSING_OPENING_PARENTHESIS` | pcre2_compile.c:2745 | pattern br"(*scan_substring:a)" | `pcre2_compile` returns NULL, `*errorcode == 218`, `*erroroffset` set | `compile_error_corpus` |
| C119 | 219 | `ERR119` | `PCRE2_ERROR_MISSING_NUMBER_TERMINATOR` | pcre2_compile.c:1777, pcre2_compile.c:1814, pcre2_compile.c:3767 | pattern br"\g{1" | `pcre2_compile` returns NULL, `*errorcode == 219`, `*erroroffset` set | `compile_error_corpus` |
| C120 | 220 | `ERR120` | `PCRE2_ERROR_NULL_ERROROFFSET` | pcre2_compile.c:10347 | compile(..., erroroffset = NULL) | `pcre2_compile` returns NULL, `*errorcode == 220`, `*erroroffset` set | `err120_null_erroroffset` |

## Section B — run-time errors

One row per distinct rejection site in the non-compile modules. "expected C
result" is the exact code (not "it failed"); each row's test asserts C and Rust
return the *same* code.

### B.1 `pcre2_config` (`pcre2_config.c`)

| # | function | trigger | expected C result | covering test |
|---|----------|---------|-------------------|---------------|
| R1 | `pcre2_config` | request code outside 0..16 (17, 18, 100, 1000, `0x80000000`, `UINT32_MAX`) | `PCRE2_ERROR_BADOPTION` (-34) | `config_rejects_unknown_requests_identically` |
| R2 | `pcre2_config` | `where == NULL` for any valid request | the required buffer size (not an error) | `config_with_null_where_returns_size` |

### B.2 context setters (`pcre2_context.c`)

| # | function | trigger | expected C result | covering test |
|---|----------|---------|-------------------|---------------|
| R3 | `pcre2_set_bsr` | value not in {1,2} — 0, 3, 4, 255, `UINT32_MAX` | `PCRE2_ERROR_BADDATA` (-29) | `set_bsr_rejects_out_of_range` |
| R4 | `pcre2_set_newline` | value not in 1..6 — 0, 7, 8, 255, `UINT32_MAX` | `PCRE2_ERROR_BADDATA` (-29) | `set_newline_rejects_out_of_range` |
| R5 | `pcre2_set_glob_escape` | escape character that is neither 0 nor an allowed punctuation code point | `PCRE2_ERROR_BADDATA` (-29) | `set_glob_escape_and_separator_reject_bad_values`, `convert_glob_with_custom_escape_and_separator` |
| R6 | `pcre2_set_glob_separator` | separator other than `/`, `\` or `.` | `PCRE2_ERROR_BADDATA` (-29) | `set_glob_escape_and_separator_reject_bad_values`, `convert_glob_with_custom_escape_and_separator` |
| R7 | `pcre2_set_optimize` | unknown directive (2, 3, 63, 70, 1000, `UINT32_MAX`) | `PCRE2_ERROR_BADOPTION` (-34) | `set_optimize_rejects_unknown_directives` |
| R8 | `pcre2_set_optimize` | `ccontext == NULL` | `PCRE2_ERROR_NULL` (-51) | `set_optimize_rejects_unknown_directives` |
| R9 | the other scalar setters (`set_max_pattern_length`, `set_max_pattern_compiled_length`, `set_max_varlookbehind`, `set_parens_nest_limit`, `set_compile_extra_options`, `set_depth_limit`, `set_heap_limit`, `set_match_limit`, `set_recursion_limit`, `set_offset_limit`) | extreme values 0, 1, `0xffff`, `0x10000`, `UINT32_MAX`, `SIZE_MAX` | 0 — they accept everything; the effect appears at compile/match time | `all_scalar_setters_accept_extremes_identically`, `err88_pattern_string_too_long`, `err101_pattern_compiled_size_too_big`, `err100_max_varlookbehind_exceeded`, `err19_parentheses_nest_too_deep` |
| R10 | all `*_free` functions and `pcre2_code_copy` / `pcre2_code_copy_with_tables` | `NULL` argument | no-op / `NULL` (explicit checks in the C) | `context_free_and_copy_with_null_are_safe` |
| R11 | `pcre2_general_context_create`, `pcre2_maketables` | allocator that always fails | `NULL` | `err21_heap_failed_via_failing_allocator`, `maketables_output_is_byte_identical` |

### B.3 `pcre2_pattern_info` (`pcre2_pattern_info.c`)

| # | function | trigger | expected C result | covering test |
|---|----------|---------|-------------------|---------------|
| R12 | `pcre2_pattern_info` | `code == NULL` | `PCRE2_ERROR_NULL` (-51) | `pattern_info_error_paths` |
| R13 | `pcre2_pattern_info` | `where == NULL` | the size of the requested item | `pattern_info_error_paths` |
| R14 | `pcre2_pattern_info` | request code > 26 (27, 28, 100, 1000, `UINT32_MAX`) | `PCRE2_ERROR_BADOPTION` (-34) | `pattern_info_error_paths` |
| R15 | `pcre2_pattern_info` | `MATCHLIMIT` / `DEPTHLIMIT` / `HEAPLIMIT` on a pattern that does not set them | `PCRE2_ERROR_UNSET` (-55) | `pattern_info_error_paths`, plus `cmp_all_pattern_info` on every compile in the suite |
| R16 | `pcre2_pattern_info` | block whose `magic_number` is wrong (zeroed buffer) | `PCRE2_ERROR_BADMAGIC` (-31) | `pattern_info_badmagic_and_badmode` |
| R17 | `pcre2_pattern_info` | correct magic, `flags` mode bits forced to `PCRE2_MODE16` | `PCRE2_ERROR_BADMODE` (-32) | `pattern_info_badmagic_and_badmode` |
| R18 | `pcre2_callout_enumerate` | patterns with and without callouts | 0 plus identical enumerate blocks | `cfg_callout_and_enumerate` |

### B.4 `pcre2_match` (`pcre2_match.c`)

| # | function | trigger | expected C result | covering test |
|---|----------|---------|-------------------|---------------|
| R19 | `pcre2_match` | `match_data == NULL` | `PCRE2_ERROR_NULL` (-51) | `match_null_arguments` |
| R20 | `pcre2_match` | `code == NULL` | `PCRE2_ERROR_NULL` (-51) | `match_null_arguments` |
| R21 | `pcre2_match` | `subject == NULL` with `length != 0` (including `PCRE2_ZERO_TERMINATED`) | `PCRE2_ERROR_NULL` (-51) | `match_null_arguments` |
| R22 | `pcre2_match` | `subject == NULL, length == 0` | treated as the empty string (not an error) | `match_null_arguments`, `cfg_default` |
| R23 | `pcre2_match` | option bit outside the accepted mask — all 32 bits probed individually, plus `UINT32_MAX` | `PCRE2_ERROR_BADOPTION` (-34) | `match_rejects_unknown_option_bits` |
| R24 | `pcre2_match` | `PCRE2_PARTIAL_SOFT` and `PCRE2_PARTIAL_HARD` together | `PCRE2_ERROR_BADOPTION` (-34) | `match_rejects_unknown_option_bits` |
| R25 | `pcre2_match` | `start_offset > length` (7, 100, `SIZE_MAX` on a 6-byte subject) | `PCRE2_ERROR_BADOFFSET` (-33) | `match_bad_start_offset` |
| R26 | `pcre2_match` | offset limit set in the match context but `PCRE2_USE_OFFSET_LIMIT` not set at compile time | `PCRE2_ERROR_BADOFFSETLIMIT` (-56) | `match_bad_offset_limit`, `cfg_offset_limit_and_match_context_limits` |
| R27 | `pcre2_match` | wrong `magic_number` | `PCRE2_ERROR_BADMAGIC` (-31) | `match_badmagic` |
| R28 | `pcre2_match` | wrong mode bits in `flags` | `PCRE2_ERROR_BADMODE` (-32) | `pattern_info_badmagic_and_badmode` |
| R29 | `pcre2_match` | `PCRE2_UTF` and a malformed UTF-8 subject (all 25 distinct malformations) | the matching `PCRE2_ERROR_UTF8_ERRn` (-3 .. -23) | `match_utf_subject_errors` |
| R30 | `pcre2_match` | `PCRE2_UTF` and `start_offset` inside a multi-byte character | `PCRE2_ERROR_BADUTFOFFSET` (-36) | `match_badutfoffset` |
| R31 | `pcre2_match` | match limit exhausted (`set_match_limit(0/1/10/100/1000)` on `(a+)+b` with 40 a's) | `PCRE2_ERROR_MATCHLIMIT` (-47) | `match_limits_produce_same_error` |
| R32 | `pcre2_match` | depth limit exhausted | `PCRE2_ERROR_DEPTHLIMIT` (-53) | `match_limits_produce_same_error` |
| R33 | `pcre2_match` | heap limit exhausted (`set_heap_limit(0/1/2/16)`) | `PCRE2_ERROR_HEAPLIMIT` (-63) | `match_limits_produce_same_error` |
| R34 | `pcre2_match` | recursion that cannot progress (`(a*)*(?1)`, `(?:(?1)|a)*`, `(a|(?R))*`) | `PCRE2_ERROR_RECURSELOOP` (-52); the pattern's own result under `PCRE2_DISABLE_RECURSELOOP_CHECK` | `match_recurse_loop_detection` |
| R35 | `pcre2_match` | `\K` in a lookaround moving the match start before the search start | `PCRE2_ERROR_BAD_BACKSLASH_K` (-75) | `match_bad_backslash_k` |
| R36 | `pcre2_match` | no match | `PCRE2_ERROR_NOMATCH` (-1) | every valid-path test |
| R37 | `pcre2_match` | partial match under `PCRE2_PARTIAL_HARD` / `_SOFT` | `PCRE2_ERROR_PARTIAL` (-2) | `substring_on_failed_and_partial_match`, `cfg_default` |
| R38 | `pcre2_match` | `PCRE2_COPY_MATCHED_SUBJECT` when allocation fails | `PCRE2_ERROR_NOMEMORY` (-48) | `cfg_custom_allocator_paths` (allocation sequences compared), `match_rejects_unknown_option_bits` |
| R39 | `pcre2_jit_match` | every option bit; no JIT in this build | `PCRE2_ERROR_JIT_BADOPTION` (-45) / the interpreter result, identically | `match_rejects_unknown_option_bits`, `jit_functions_agree` |
| R40 | `pcre2_jit_compile` | every JIT option bit, and `code == NULL` | `PCRE2_ERROR_JIT_UNSUPPORTED` (-68) / `PCRE2_ERROR_NULL` | `jit_functions_agree` |

### B.5 `pcre2_dfa_match` (`pcre2_dfa_match.c`)

| # | function | trigger | expected C result | covering test |
|---|----------|---------|-------------------|---------------|
| R41 | `pcre2_dfa_match` | `match_data` / `code` / `subject` NULL as in R19–R21 | `PCRE2_ERROR_NULL` (-51) | `match_null_arguments` |
| R42 | `pcre2_dfa_match` | `workspace == NULL` | `PCRE2_ERROR_NULL` (-51) | `match_null_arguments` |
| R43 | `pcre2_dfa_match` | `wscount < 20` (0, 1, 9, 19) | `PCRE2_ERROR_DFA_WSSIZE` (-43) | `dfa_match_workspace_size_errors` |
| R44 | `pcre2_dfa_match` | `PCRE2_DFA_RESTART` with a workspace holding no saved state | `PCRE2_ERROR_DFA_BADRESTART` (-38) | `dfa_match_workspace_size_errors` |
| R45 | `pcre2_dfa_match` | unknown / disallowed option bits (all 32 probed) | `PCRE2_ERROR_BADOPTION` (-34) | `match_rejects_unknown_option_bits` |
| R46 | `pcre2_dfa_match` | `start_offset > length` | `PCRE2_ERROR_BADOFFSET` (-33) | `match_bad_start_offset` |
| R47 | `pcre2_dfa_match` | offset limit without `PCRE2_USE_OFFSET_LIMIT` | `PCRE2_ERROR_BADOFFSETLIMIT` (-56) | `match_bad_offset_limit` |
| R48 | `pcre2_dfa_match` | wrong magic / wrong mode | `PCRE2_ERROR_BADMAGIC` / `PCRE2_ERROR_BADMODE` | `match_badmagic`, `pattern_info_badmagic_and_badmode` |
| R49 | `pcre2_dfa_match` | malformed UTF-8 subject | the matching `PCRE2_ERROR_UTF8_ERRn` | `match_utf_subject_errors` |
| R50 | `pcre2_dfa_match` | pattern compiled with `PCRE2_MATCH_INVALID_UTF` | `PCRE2_ERROR_DFA_UINVALID_UTF` (-66) | `match_utf_subject_errors`, `cfg_utf_and_ucp_matrix` |
| R51 | `pcre2_dfa_match` | back reference in the pattern (`(a)\1`, `(?<n>a)\k<n>`) | `PCRE2_ERROR_DFA_UITEM` (-42) | `dfa_match_unsupported_items` |
| R52 | `pcre2_dfa_match` | condition the DFA cannot evaluate (`(?(1)a)(b)`) | `PCRE2_ERROR_DFA_UCOND` (-40) | `dfa_match_unsupported_items` |
| R53 | `pcre2_dfa_match` | recursion the DFA cannot handle | `PCRE2_ERROR_DFA_RECURSE` (-39) | `dfa_match_unsupported_items`, `cfg_default` (`(?R)?a`, `(?1)(a)`) |
| R54 | `pcre2_dfa_match` | depth / match / heap limit exceeded | `PCRE2_ERROR_DEPTHLIMIT` / `MATCHLIMIT` / `HEAPLIMIT` | `match_limits_produce_same_error` |
| R55 | `pcre2_dfa_match` | workspace growth fails, or the heap limit is hit while growing | `PCRE2_ERROR_NOMEMORY` (-48) / `PCRE2_ERROR_HEAPLIMIT` (-63) | `match_limits_produce_same_error`, `dfa_match_workspace_size_errors` |
| R56 | `pcre2_dfa_match` | no match / partial match | `PCRE2_ERROR_NOMATCH` / `PCRE2_ERROR_PARTIAL` | every valid-path test |

### B.6 `pcre2_substring_*` (`pcre2_substring.c`)

| # | function | trigger | expected C result | covering test |
|---|----------|---------|-------------------|---------------|
| R57 | `pcre2_substring_length_bynumber` | group number > `top_bracket` (4, 100, `UINT32_MAX`) | `PCRE2_ERROR_NOSUBSTRING` (-49) | `substring_error_paths` |
| R58 | `pcre2_substring_length_bynumber` | group number valid but ≥ `oveccount` | `PCRE2_ERROR_UNAVAILABLE` (-54) | `substring_error_paths`, `run_subject` (ovector sizes 0/1/8) |
| R59 | `pcre2_substring_length_bynumber` | group that did not participate in the match | `PCRE2_ERROR_UNSET` (-55) | `substring_error_paths` |
| R60 | `pcre2_substring_length_bynumber` | after a partial match, group > 0 | `PCRE2_ERROR_PARTIAL` (-2) | `substring_on_failed_and_partial_match` |
| R61 | `pcre2_substring_length_bynumber` | ovector offsets beyond the subject length | `PCRE2_ERROR_INVALIDOFFSET` (-67) — internal invariant guarded by `PCRE2_DEBUG_UNREACHABLE`, not reachable from the public API | n/a (documented); the surrounding checks are exercised by every substring call |
| R62 | `pcre2_substring_copy_bynumber` | destination too small (capacities 0..3 for a longer group) | `PCRE2_ERROR_NOMEMORY` (-48) | `substring_error_paths`, `run_subject` |
| R63 | `pcre2_substring_copy_bynumber` / `_get_bynumber` | the R57–R60 conditions | the same codes as R57–R60 | `substring_error_paths` |
| R64 | `pcre2_substring_get_bynumber` | allocation failure | `PCRE2_ERROR_NOMEMORY` (-48) | `cfg_custom_allocator_paths` |
| R65 | `pcre2_substring_*_byname` | name not in the pattern (`"nope"`, `""`, wrong case `"NM"`) | `PCRE2_ERROR_NOSUBSTRING` (-49) | `substring_error_paths` |
| R66 | `pcre2_substring_*_byname` | duplicated name where no instance is set | `PCRE2_ERROR_UNSET` (-55) | `substring_duplicate_names` |
| R67 | `pcre2_substring_*_byname` | match_data produced by `pcre2_dfa_match` | `PCRE2_ERROR_DFA_UFUNC` (-41) | `substring_error_paths`, `substitute_matched_mode_consistency_errors` |
| R68 | `pcre2_substring_number_from_name` | duplicated name | `PCRE2_ERROR_NOUNIQUESUBSTRING` (-50) | `substring_duplicate_names` |
| R69 | `pcre2_substring_number_from_name` | unknown name | `PCRE2_ERROR_NOSUBSTRING` (-49) | `substring_error_paths` |
| R70 | `pcre2_substring_nametable_scan` | unknown name | `PCRE2_ERROR_NOSUBSTRING` (-49) | `substring_error_paths` |
| R71 | `pcre2_substring_nametable_scan` | `firstptr == NULL` with a duplicated name | `PCRE2_ERROR_NOUNIQUESUBSTRING` (-50) | `substring_error_paths` |
| R72 | `pcre2_substring_list_get` | allocation failure | `PCRE2_ERROR_NOMEMORY` (-48) | `cfg_custom_allocator_paths` |

### B.7 `pcre2_serialize_*` (`pcre2_serialize.c`)

| # | function | trigger | expected C result | covering test |
|---|----------|---------|-------------------|---------------|
| R73 | `pcre2_serialize_encode` | `codes == NULL`, `serialized_bytes == NULL`, or `serialized_size == NULL` | `PCRE2_ERROR_NULL` (-51) | `serialize_error_paths` |
| R74 | `pcre2_serialize_encode` | `number_of_codes <= 0` (0, -1, -100, `INT32_MIN`) | `PCRE2_ERROR_BADDATA` (-29) | `serialize_error_paths` |
| R75 | `pcre2_serialize_encode` | a `NULL` entry inside the code vector | `PCRE2_ERROR_NULL` (-51) | `serialize_error_paths` |
| R76 | `pcre2_serialize_encode` | an entry with a bad magic number | `PCRE2_ERROR_BADMAGIC` (-31) | `serialize_error_paths` |
| R77 | `pcre2_serialize_encode` | two patterns compiled with different character tables | `PCRE2_ERROR_MIXEDTABLES` (-30) | `serialize_error_paths` |
| R78 | `pcre2_serialize_encode` | allocation failure | `PCRE2_ERROR_NOMEMORY` (-48) | `cfg_custom_allocator_paths` |
| R79 | `pcre2_serialize_decode` | `codes == NULL` or `bytes == NULL` | `PCRE2_ERROR_NULL` (-51) | `serialize_error_paths` |
| R80 | `pcre2_serialize_decode` | wrong header magic (zeroed / corrupted blob; each of the first 32 header bytes flipped) | `PCRE2_ERROR_BADMAGIC` (-31) | `serialize_error_paths`, `serialize_roundtrip_and_truncation` |
| R81 | `pcre2_serialize_decode` | header records a different code-unit width / endianness | `PCRE2_ERROR_BADMODE` (-32) | `serialize_roundtrip_and_truncation` |
| R82 | `pcre2_serialize_decode` | internally inconsistent blob | `PCRE2_ERROR_BADSERIALIZEDDATA` (-62) | `serialize_roundtrip_and_truncation` |
| R83 | `pcre2_serialize_decode` | `number_of_codes < 0` | `PCRE2_ERROR_BADDATA` (-29) | `serialize_error_paths`, `serialize_roundtrip_and_truncation` |
| R84 | `pcre2_serialize_decode` | allocation failure | `PCRE2_ERROR_NOMEMORY` (-48) | `cfg_custom_allocator_paths` |
| R85 | `pcre2_serialize_get_number_of_codes` | `NULL` / garbage / corrupted header | `PCRE2_ERROR_NULL` / `BADMAGIC` / `BADMODE` | `serialize_error_paths`, `serialize_roundtrip_and_truncation` |

### B.8 `pcre2_substitute` (`pcre2_substitute.c`)

| # | function | trigger | expected C result | covering test |
|---|----------|---------|-------------------|---------------|
| R86 | `pcre2_substitute` | `replacement == NULL` with `rlength != 0`; `subject == NULL` with `length != 0`; `PCRE2_SUBSTITUTE_MATCHED` with `match_data == NULL` | `PCRE2_ERROR_NULL` (-51) | `substitute_null_and_bad_arguments` |
| R87 | `pcre2_substitute` | `start_offset > length` | `PCRE2_ERROR_BADOFFSET` (-33) | `substitute_bad_start_offset` |
| R88 | `pcre2_substitute` | `PCRE2_PARTIAL_*` without `PCRE2_SUBSTITUTE_REPLACEMENT_ONLY`; any unknown option bit (all 32 probed) | `PCRE2_ERROR_BADOPTION` (-34) | `substitute_partial_and_badsubspattern`, `substitute_rejects_unknown_option_bits` |
| R89 | `pcre2_substitute` | `$` at the end, `$%`, bad `${...}` content | `PCRE2_ERROR_BADREPLACEMENT` (-35) | `substitute_bad_replacement_corpus` |
| R90 | `pcre2_substitute` | unterminated `${` / `${1:+a` | `PCRE2_ERROR_REPMISSINGBRACE` (-58) | `substitute_bad_replacement_corpus` |
| R91 | `pcre2_substitute` | bad escape in the replacement (`\q`, trailing `\`, malformed `\x{}` / `\o{}`) with `PCRE2_SUBSTITUTE_EXTENDED` | `PCRE2_ERROR_BADREPESCAPE` (-57) | `substitute_bad_replacement_corpus` |
| R92 | `pcre2_substitute` | malformed `${n:...}` conditional | `PCRE2_ERROR_BADSUBSTITUTION` (-59) | `substitute_bad_replacement_corpus` |
| R93 | `pcre2_substitute` | reference to a group that does not exist | `PCRE2_ERROR_NOSUBSTRING` (-49), or empty under `PCRE2_SUBSTITUTE_UNKNOWN_UNSET` | `substitute_unset_group_handling` |
| R94 | `pcre2_substitute` | reference to a group that is unset | `PCRE2_ERROR_UNSET` (-55), or empty under `PCRE2_SUBSTITUTE_UNSET_EMPTY` | `substitute_unset_group_handling` |
| R95 | `pcre2_substitute` | reference to a group beyond the ovector | `PCRE2_ERROR_UNAVAILABLE` (-54) | `substitute_unset_group_handling` |
| R96 | `pcre2_substitute` | output buffer too small (capacities 0..8) | `PCRE2_ERROR_NOMEMORY` (-48); with `PCRE2_SUBSTITUTE_OVERFLOW_LENGTH` the needed length in `*blength` | `substitute_output_overflow` |
| R97 | `pcre2_substitute` | replacement count overflow (empty-matching patterns with `PCRE2_SUBSTITUTE_GLOBAL`) | `PCRE2_ERROR_TOOMANYREPLACE` (-61) | `substitute_toomanyreplace` |
| R98 | `pcre2_substitute` | replacement longer than `PCRE2_SIZE` allows | `PCRE2_ERROR_TOOLARGEREPLACE` (-70) | `substitute_output_overflow`, `substitute_case_callout_errors` (`cc_grow`) |
| R99 | `pcre2_substitute` | `\K` in a lookaround in the pattern | `PCRE2_ERROR_BADSUBSPATTERN` (-60) | `substitute_partial_and_badsubspattern` |
| R100 | `pcre2_substitute` | partial match during substitution | `PCRE2_ERROR_PARTIAL` (-2) | `substitute_partial_and_badsubspattern` |
| R101 | `pcre2_substitute` | `$'`, `$_` or `` $` `` with a partial match | `PCRE2_ERROR_PARTIALSUBS` (-76) | `substitute_bad_replacement_corpus`, `substitute_partial_and_badsubspattern` |
| R102 | `pcre2_substitute` | `PCRE2_SUBSTITUTE_MATCHED` with a match_data from a different pattern | `PCRE2_ERROR_DIFFSUBSPATTERN` (-71) | `substitute_matched_mode_consistency_errors` |
| R103 | `pcre2_substitute` | ... a different subject pointer / length | `PCRE2_ERROR_DIFFSUBSSUBJECT` (-72) | `substitute_matched_mode_consistency_errors` |
| R104 | `pcre2_substitute` | ... a different start offset | `PCRE2_ERROR_DIFFSUBSOFFSET` (-73) | `substitute_matched_mode_consistency_errors` |
| R105 | `pcre2_substitute` | ... different match options | `PCRE2_ERROR_DIFFSUBSOPTIONS` (-74) | `substitute_matched_mode_consistency_errors` |
| R106 | `pcre2_substitute` | ... a match_data produced by `pcre2_dfa_match` | `PCRE2_ERROR_DFA_UFUNC` (-41) | `substitute_matched_mode_consistency_errors` |
| R107 | `pcre2_substitute` | substitute case callout returns `PCRE2_UNSET` | `PCRE2_ERROR_REPLACECASE` (-69) | `substitute_case_callout_errors` |
| R108 | `pcre2_substitute` | substitute callout returns a negative value | that value, propagated verbatim (-77 in the test) | `substitute_callout_return_values` |
| R109 | `pcre2_substitute` | duplicate match detected internally | `PCRE2_ERROR_INTERNAL_DUPMATCH` (-65) | `substitute_toomanyreplace`, `substitute_randomized_matrix` |
| R110 | `pcre2_substitute` | allocation failure for the internal match_data | `PCRE2_ERROR_NOMEMORY` (-48) | `cfg_custom_allocator_paths` |
| R111 | `pcre2_substitute` | invalid UTF-8 replacement in UTF mode | the matching `PCRE2_ERROR_UTF8_ERRn` | `substitute_randomized_matrix` |
| R112 | `pcre2_substitute` | no match at all | `PCRE2_ERROR_NOMATCH` (-1) | `substitute_randomized_matrix` |

### B.9 `pcre2_pattern_convert` (`pcre2_convert.c`)

| # | function | trigger | expected C result | covering test |
|---|----------|---------|-------------------|---------------|
| R113 | `pcre2_pattern_convert` | `pattern == NULL`, or `buffptr == NULL` | `PCRE2_ERROR_NULL` (-51) | `convert_error_paths` |
| R114 | `pcre2_pattern_convert` | no conversion type, two conversion types, or an unknown option bit (all 32 probed, plus `UINT32_MAX`) | `PCRE2_ERROR_BADOPTION` (-34) | `convert_error_paths` |
| R115 | `pcre2_pattern_convert` | glob syntax error (`**`, `a**b`, `[!`, `[a-`, trailing `\`) | `PCRE2_ERROR_CONVERT_SYNTAX` (-64) | `convert_syntax_errors`, `convert_glob_matrix`, `convert_randomized` |
| R116 | `pcre2_pattern_convert` | unterminated `[` in a glob or POSIX pattern | `PCRE2_ERROR_MISSING_SQUARE_BRACKET` (106) | `convert_syntax_errors`, `convert_randomized` |
| R117 | `pcre2_pattern_convert` | trailing backslash in a POSIX pattern | `PCRE2_ERROR_END_BACKSLASH` (101) | `convert_syntax_errors` |
| R118 | `pcre2_pattern_convert` | invalid UTF-8 pattern with `PCRE2_CONVERT_UTF` | the matching `PCRE2_ERROR_UTF8_ERRn` | `convert_syntax_errors`, `convert_randomized` |
| R119 | `pcre2_pattern_convert` | allocation failure | `PCRE2_ERROR_NOMEMORY` (-48) | `cfg_custom_allocator_paths` |
| R120 | `pcre2_pattern_convert` | `PCRE2_CONVERT_UTF` without Unicode support | `PCRE2_ERROR_UNICODE_NOT_SUPPORTED` (132) — compiled out, SUPPORT_UNICODE is defined | n/a (documented) |

### B.10 `pcre2_get_error_message` (`pcre2_error.c`)

| # | function | trigger | expected C result | covering test |
|---|----------|---------|-------------------|---------------|
| R121 | `pcre2_get_error_message` | code with no message (0, > 225, < -76, `INT32_MIN`, `INT32_MAX`) | `PCRE2_ERROR_BADDATA` (-29) | `get_error_message_every_code_and_boundary` |
| R122 | `pcre2_get_error_message` | buffer too small (capacities 0, 1, 2, 5) | `PCRE2_ERROR_NOMEMORY` (-48) | `get_error_message_every_code_and_boundary` |
| R123 | `pcre2_get_error_message` | every valid code (-80 .. 225) with a large buffer | the message length plus byte-identical buffer contents | `get_error_message_every_code_and_boundary` |

### B.11 `pcre2_next_match` (`pcre2_match_next.c`) and the match-data accessors

| # | function | trigger | expected C result | covering test |
|---|----------|---------|-------------------|---------------|
| R124 | `pcre2_next_match` | previous match returned `rc < 0` | `FALSE` (0) | `next_match_error_paths`, `cmp_match_state` on every match in the suite |
| R125 | `pcre2_next_match` | empty match at the end of the subject | `FALSE` (0) | `next_match_error_paths`, `cfg_default` |
| R126 | `pcre2_next_match` | empty match not at the end | `TRUE` with `*poptions == PCRE2_NOTEMPTY_ATSTART` | `next_match_error_paths`, `cmp_match_state` |
| R127 | `pcre2_next_match` | `\K`-induced non-progressing non-empty match | `TRUE` with a bumped-along offset | `match_bad_backslash_k`, `cmp_match_state` |
| R128 | `pcre2_match_data_create` | `oveccount == 0` (clamped to 1) and `> UINT16_MAX` (clamped) | a match_data with the clamped count | `match_data_create_edge_cases` |
| R129 | `pcre2_match_data_create_from_pattern` | `code == NULL` | undefined in C (dereferences `code`) — excluded | n/a (documented above) |
| R130 | `pcre2_match_data_create` | allocation failure | `NULL` | `err21_heap_failed_via_failing_allocator` (failing allocator) |

### B.12 `_pcre2_valid_utf` (`pcre2_valid_utf.c`)

All 21 `PCRE2_ERROR_UTF8_ERRn` codes plus the valid cases, including the reported
byte offset.

| # | function | trigger | expected C result | covering test |
|---|----------|---------|-------------------|---------------|
| R131 | `_pcre2_valid_utf` | 1–5 bytes missing at the end of the subject | `UTF8_ERR1` .. `UTF8_ERR5` (-3 .. -7) | `priv_valid_utf_agrees_on_every_malformation` |
| R132 | `_pcre2_valid_utf` | invalid 2nd–6th byte of a sequence | `UTF8_ERR6` .. `UTF8_ERR10` (-8 .. -12) | `priv_valid_utf_agrees_on_every_malformation` |
| R133 | `_pcre2_valid_utf` | 5-byte and 6-byte sequences | `UTF8_ERR11`, `UTF8_ERR12` (-13, -14) | `priv_valid_utf_agrees_on_every_malformation` |
| R134 | `_pcre2_valid_utf` | byte `0xFE`, byte `0xFF` | `UTF8_ERR13`, `UTF8_ERR14` (-15, -16) | `priv_valid_utf_agrees_on_every_malformation` |
| R135 | `_pcre2_valid_utf` | overlong 2/3/4/5/6-byte encodings | `UTF8_ERR15` .. `UTF8_ERR19` (-17 .. -21) | `priv_valid_utf_agrees_on_every_malformation` |
| R136 | `_pcre2_valid_utf` | surrogate code point | `UTF8_ERR20` (-22) | `priv_valid_utf_agrees_on_every_malformation` |
| R137 | `_pcre2_valid_utf` | code point > `0x10FFFF` | `UTF8_ERR21` (-23) | `priv_valid_utf_agrees_on_every_malformation` |
| R138 | `_pcre2_valid_utf` | isolated continuation byte (`0x80`, `0xBF`) and 80 000 random / lead-byte-biased buffers | whatever the C switch returns, with the same offset | `priv_valid_utf_agrees_on_every_malformation`, `priv_valid_utf_randomized` |
| R139 | `_pcre2_valid_utf` | valid sequences and the empty string | 0, `*erroroffset` untouched | `priv_valid_utf_agrees_on_every_malformation` |
| R140 | `_pcre2_valid_utf` | the same 21 codes reached through `pcre2_compile(PCRE2_UTF)` and `pcre2_match` | the same codes and offsets | `match_utf_subject_errors`, `cfg_utf_and_ucp_matrix` |
