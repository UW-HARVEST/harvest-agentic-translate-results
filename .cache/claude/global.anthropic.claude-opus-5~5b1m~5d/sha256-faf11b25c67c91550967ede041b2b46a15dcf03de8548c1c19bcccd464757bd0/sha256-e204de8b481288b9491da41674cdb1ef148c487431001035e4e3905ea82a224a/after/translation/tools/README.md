# Tooling used to build the differential test suite

* `dump_stack_residue.c` - compiled with plain `gcc dump_stack_residue.c`, this
  prints the raw stack contents around `main()`'s frame pointer.  The bytes that
  land where `c_src`'s `ref_buffer`, `input_buffer` and `main()` locals live
  (`%rbp-0x830`, `%rbp-0x430`, `%rbp-0x30`) are what `../src/residue.rs`
  contains.  Every one of those 2048 bytes was afterwards cross-checked against
  the real C program by probing it with crafted inputs; see `../ERRORS.md`.
* `gen_differential_tests.py` - regenerates `../tests/differential.rs`.  It
  refuses to write the file unless the C program answers reproducibly for every
  enumerated input and the Rust program agrees with it.

Neither file is part of the crate; `cargo build` and `cargo test` ignore them.
