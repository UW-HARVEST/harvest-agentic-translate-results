//! Test-case registry.
//!
//! The driver and the worker processes both build cases from these
//! deterministic (fixed-seed) generators, keyed by test id — nothing is
//! serialised between them.

#![allow(dead_code)]

use super::deflate::*;
use super::{Case, Expect, Rng};

pub mod phase_b;
pub mod phase_c;

pub fn all_ids() -> Vec<&'static str> {
    let mut v = phase_b::IDS.to_vec();
    v.extend_from_slice(phase_c::IDS);
    v
}

pub fn build(id: &str) -> Vec<Case> {
    if phase_b::IDS.contains(&id) {
        phase_b::build(id)
    } else if phase_c::IDS.contains(&id) {
        phase_c::build(id)
    } else {
        panic!("unknown test id `{id}`");
    }
}

// ---------------------------------------------------------------------------
// shared helpers
// ---------------------------------------------------------------------------

/// Pads the input so that `last_bytes == want_last` for the given alignment.
pub fn pad_to_last_bytes(input: &mut Vec<u8>, align: usize, want_last: usize, filler: u8) {
    let fb = first_bytes_for(align);
    while (input.len() + 4 - fb) % 4 != want_last % 4 {
        input.push(filler);
    }
}

/// Build a case from a finished stream, sizing `out_bytes` exactly.
pub fn case_exact(
    label: impl Into<String>,
    input: Vec<u8>,
    expected: Option<Vec<u8>>,
    out_len: usize,
    align: usize,
) -> Case {
    let mut c = Case::new(label, input, out_len as i32).in_align(align);
    c = match expected {
        Some(e) => c.expect(Expect::Out { ret: 1, out: e }),
        None => c.expect(Expect::Ret { ret: 1, reason: None }),
    };
    c
}

/// One fixed block: `n` random literals then end-of-block.
pub fn random_fixed_stream(rng: &mut Rng, n: usize) -> (Vec<Tok>, Vec<u8>) {
    let mut toks = Vec::with_capacity(n + 1);
    let mut bytes = Vec::with_capacity(n);
    for _ in 0..n {
        let b = rng.byte();
        toks.push(Tok::Lit(b as u16));
        bytes.push(b);
    }
    toks.push(Tok::End);
    (toks, bytes)
}

/// The default tables, as the C ships them.
pub fn tables() -> Tables {
    Tables::default()
}
