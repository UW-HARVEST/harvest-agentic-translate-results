// Note: This file is not included via `lib.rs`. It is provided for
// completeness of the C-to-Rust translation.
#![allow(dead_code)]

use std::cell::RefCell;

use crate::confirm::confirm_result;
use crate::lrat_check;
use crate::secret::SECRET_KEY;
use crate::siphash::SipHash;
use crate::trusted_utils::{trusted_utils_copy_bytes, trusted_utils_equal_signatures, SIG_SIZE_BYTES};

thread_local! {
    static STATE: RefCell<TopCheckState> = RefCell::new(TopCheckState::new());
}

struct TopCheckState {
    parsed_formula: bool,
    formula_signature: Vec<u8>,
    valid: bool,
    sip: SipHash,
    msgstr: String,
}

impl TopCheckState {
    fn new() -> Self {
        Self {
            parsed_formula: false,
            formula_signature: vec![0u8; SIG_SIZE_BYTES],
            valid: true,
            sip: SipHash::siphash_init(&SECRET_KEY),
            msgstr: String::new(),
        }
    }
}

fn with<R>(f: impl FnOnce(&mut TopCheckState) -> R) -> R {
    STATE.with(|s| f(&mut s.borrow_mut()))
}

pub fn top_check_init(nb_vars: i32, check_model: bool, lenient: bool) {
    with(|s| {
        s.sip = SipHash::siphash_init(&SECRET_KEY);
        s.parsed_formula = false;
        s.formula_signature = vec![0u8; SIG_SIZE_BYTES];
        s.valid = true;
        s.msgstr.clear();
    });
    lrat_check::lrat_check_init(nb_vars, check_model, lenient);
}

pub fn top_check_commit_formula_sig(f_sig: &[u8]) {
    with(|s| {
        trusted_utils_copy_bytes(&mut s.formula_signature, f_sig, SIG_SIZE_BYTES as u64);
    });
}

pub fn top_check_load(lit: i32) {
    let ok = lrat_check::lrat_check_load(lit);
    with(|s| s.valid &= ok);
}

pub fn top_check_end_load() -> bool {
    let mut sig_from_chk: Option<Vec<u8>> = None;
    let ok = lrat_check::lrat_check_end_load(&mut sig_from_chk);
    let result = with(|s| {
        s.valid = s.valid && ok;
        if !s.valid {
            return false;
        }
        let sig = sig_from_chk.unwrap_or_default();
        s.valid = trusted_utils_equal_signatures(&sig, &s.formula_signature);
        if !s.valid {
            s.msgstr = "Formula signature check failed".to_string();
        }
        s.valid
    });
    result
}

pub fn compute_clause_signature(id: u64, lits: &[i32], nb_lits: i32, out: &mut [u8]) {
    with(|s| {
        s.sip.siphash_reset();
        let id_bytes = id.to_ne_bytes();
        s.sip.siphash_update(&id_bytes, 8);
        let mut lit_bytes = Vec::with_capacity(nb_lits as usize * 4);
        for i in 0..nb_lits as usize {
            lit_bytes.extend_from_slice(&lits[i].to_ne_bytes());
        }
        s.sip.siphash_update(&lit_bytes, lit_bytes.len() as u64);
        let fs = s.formula_signature.clone();
        s.sip.siphash_update(&fs, SIG_SIZE_BYTES as u64);
        let hash = s.sip.siphash_digest();
        trusted_utils_copy_bytes(out, &hash, SIG_SIZE_BYTES as u64);
    });
}

pub fn top_check_produce(
    id: u64,
    literals: &[i32],
    nb_literals: i32,
    hints: &[u64],
    nb_hints: i32,
    out_sig_or_null: &mut Option<Vec<u8>>,
) -> bool {
    let ok = lrat_check::lrat_check_add_clause(id, literals, nb_literals, hints, nb_hints);
    with(|s| s.valid &= ok);
    if !ok {
        return false;
    }
    if out_sig_or_null.is_some() {
        let mut buf = vec![0u8; SIG_SIZE_BYTES];
        compute_clause_signature(id, literals, nb_literals, &mut buf);
        *out_sig_or_null = Some(buf);
    }
    true
}

pub fn top_check_import(
    id: u64,
    literals: &[i32],
    nb_literals: i32,
    signature_data: &[u8],
) -> bool {
    let mut computed = vec![0u8; SIG_SIZE_BYTES];
    compute_clause_signature(id, literals, nb_literals, &mut computed);
    if !trusted_utils_equal_signatures(signature_data, &computed) {
        with(|s| {
            s.valid = false;
            s.msgstr = format!("Signature check of clause {} failed", id);
        });
        return false;
    }
    let ok = lrat_check::lrat_check_add_axiomatic_clause(id, literals, nb_literals);
    with(|s| {
        s.valid &= ok;
        s.valid
    })
}

pub fn top_check_delete(ids: &[u64], nb_ids: i32) -> bool {
    lrat_check::lrat_check_delete_clause(ids, nb_ids)
}

pub fn top_check_validate_unsat(out_signature_or_null: &mut Option<Vec<u8>>) -> bool {
    let ok = lrat_check::lrat_check_validate_unsat();
    let valid = with(|s| {
        s.valid &= ok;
        s.valid
    });
    if !valid {
        return false;
    }
    if out_signature_or_null.is_some() {
        let formula_signature = with(|s| s.formula_signature.clone());
        let mut buf = vec![0u8; SIG_SIZE_BYTES];
        confirm_result(&formula_signature, 20, &mut buf);
        *out_signature_or_null = Some(buf);
    }
    true
}

pub fn top_check_validate_sat(
    model: &[i32],
    size: u64,
    out_signature_or_null: &mut Option<Vec<u8>>,
) -> bool {
    let ok = lrat_check::lrat_check_validate_sat(model, size);
    let valid = with(|s| {
        s.valid &= ok;
        s.valid
    });
    if !valid {
        return false;
    }
    if out_signature_or_null.is_some() {
        let formula_signature = with(|s| s.formula_signature.clone());
        let mut buf = vec![0u8; SIG_SIZE_BYTES];
        confirm_result(&formula_signature, 10, &mut buf);
        *out_signature_or_null = Some(buf);
    }
    true
}

pub fn top_check_valid() -> bool {
    with(|s| s.valid)
}
