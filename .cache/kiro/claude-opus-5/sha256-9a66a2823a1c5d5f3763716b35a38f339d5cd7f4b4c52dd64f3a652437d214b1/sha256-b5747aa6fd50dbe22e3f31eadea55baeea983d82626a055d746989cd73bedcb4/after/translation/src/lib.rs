//! A translation of PCRE2 (8-bit library) from C to Rust.
//!
//! The crate is built as a `cdylib` exporting the same symbols as the C build of
//! the sources in `../c_src`: public API functions are suffixed `_8`
//! (`pcre2_compile_8`, ...) and library-private cross-module functions are named
//! `_pcre2_<name>_8`, matching the `PCRE2_SUFFIX` / `PRIV` macros in the headers.

#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals)]

/* Generated data tables */
pub mod chars;
pub mod chartables;
pub mod opcodes;
pub mod ucd;
pub mod ucp;
pub mod ucptables;

/* Shared definitions */
pub mod compile_internal;
pub mod internal;

/* Translated modules */
pub mod chkdint;
pub mod config;
pub mod context;
pub mod error;
pub mod extuni;
pub mod find_bracket;
pub mod maketables;
pub mod match_data;
pub mod newline;
pub mod ord2utf;
pub mod pattern_info;
pub mod script_run;
pub mod serialize;
pub mod string_utils;
pub mod substring;
pub mod valid_utf;
pub mod auto_possess;
pub mod jit;
pub mod study;
pub mod tables;
pub mod xclass;
pub mod compile_class;
pub mod compile_cgroup;
pub mod dfa_match;
pub mod match_;
pub mod compile;
pub mod compile_branch;
pub mod compile_parse;
pub mod compile_scan;
pub mod compile_tables;
pub mod convert;
pub mod match_next;
pub mod substitute;
