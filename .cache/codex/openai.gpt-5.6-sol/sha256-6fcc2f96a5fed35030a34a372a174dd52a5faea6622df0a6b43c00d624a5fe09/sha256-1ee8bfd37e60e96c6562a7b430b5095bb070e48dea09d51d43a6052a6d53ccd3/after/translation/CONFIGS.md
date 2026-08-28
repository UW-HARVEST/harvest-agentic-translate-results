# Configuration Surface

The sole public entry point is the low-level parser `ima_parse`; there are no
convenience wrappers, runtime options, compile-time features, or public enum
arguments. Rows below enumerate the finite branch-distinct valid paths through
the chunk loop. Values in every row include randomized header flags, ignored
description/packet/data fields, sample-rate bit patterns, frame counts, channel
counts, payload bytes, and applicable chunk sizes.

| # | entry point(s) | configuration (options set + input shape) | status |
|---|----------------|-------------------------------------------|--------|
| 1 | `ima_parse` | Minimal `desc -> pakt -> data`; positive data size; zero, one, and many complete 34-byte IMA blocks | [x] |
| 2 | `ima_parse` | Required metadata reversed: `pakt -> desc -> data` | [x] |
| 3 | `ima_parse` | Unknown chunk before both metadata chunks: `unknown -> desc -> pakt -> data`; zero and positive unknown payload sizes | [x] |
| 4 | `ima_parse` | Unknown chunk between metadata chunks: `desc -> unknown -> pakt -> data`; zero and positive unknown payload sizes | [x] |
| 5 | `ima_parse` | Unknown chunk after metadata: `desc -> pakt -> unknown -> data`; zero and positive unknown payload sizes | [x] |
| 6 | `ima_parse` | Duplicate descriptions: `desc -> desc -> pakt -> data`; the last description supplies outputs | [x] |
| 7 | `ima_parse` | Duplicate packet tables: `pakt -> pakt -> desc -> data`; the last packet table supplies outputs | [x] |
| 8 | `ima_parse` | A positive-size unknown chunk jumps forward over randomized opaque bytes | [x] |
| 9 | `ima_parse` | A negative-size unknown chunk jumps backward to an earlier packet-table chunk, then reaches data | [x] |
| 10 | `ima_parse` | Data chunk size is zero; parser still returns its blocks pointer and zero size | [x] |
| 11 | `ima_parse` | Data chunk size is negative (`-1`, `i64::MIN`, and randomized negative values), converted to `ima_u64_t` in output | [x] |
| 12 | `ima_parse` | Data chunk size is positive through `i64::MAX`, including values larger than the backing input because data size is not traversed | [x] |
