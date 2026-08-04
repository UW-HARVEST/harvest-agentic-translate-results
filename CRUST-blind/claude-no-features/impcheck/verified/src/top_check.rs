use crate::lrat_check;
use crate::secret::SECRET_KEY;
use crate::siphash::SipHash;
use crate::trusted_utils::{trusted_utils_copy_bytes, trusted_utils_equal_signatures, SIG_SIZE_BYTES};
use crate::confirm::confirm_result;

use std::cell::RefCell;

struct TopCheckState {
    formula_signature: [u8; SIG_SIZE_BYTES],
    valid: bool,
    parsed_formula: bool,
    siphash: Option<SipHash>,
}

thread_local! {
    static TOP: RefCell<TopCheckState> = RefCell::new(TopCheckState {
        formula_signature: [0u8; SIG_SIZE_BYTES],
        valid: true,
        parsed_formula: false,
        siphash: None,
    });
}

pub fn top_check_init(nb_vars: i32, check_model: bool, lenient: bool) {
    TOP.with(|t| {
        let mut t = t.borrow_mut();
        t.siphash = Some(SipHash::siphash_init(&SECRET_KEY));
        t.valid = true;
        t.parsed_formula = false;
    });
    lrat_check::lrat_check_init(nb_vars, check_model, lenient);
}

pub fn top_check_commit_formula_sig(f_sig: &[u8]) {
    TOP.with(|t| {
        let mut t = t.borrow_mut();
        let mut sig = [0u8; SIG_SIZE_BYTES];
        let n = SIG_SIZE_BYTES.min(f_sig.len());
        sig[..n].copy_from_slice(&f_sig[..n]);
        t.formula_signature = sig;
    });
}

pub fn top_check_validate_sat(
    model: &[i32],
    size: u64,
    out_signature_or_null: &mut Option<Vec<u8>>,
) -> bool {
    let res = lrat_check::lrat_check_validate_sat(model, size);
    let valid = TOP.with(|t| {
        let mut t = t.borrow_mut();
        t.valid = t.valid && res;
        t.valid
    });
    if !valid {
        return false;
    }
    if let Some(out) = out_signature_or_null.as_mut() {
        let f_sig = TOP.with(|t| t.borrow().formula_signature);
        out.resize(SIG_SIZE_BYTES, 0);
        confirm_result(&f_sig, 10, out);
    }
    true
}

pub fn top_check_delete(ids: &[u64], nb_ids: i32) -> bool {
    lrat_check::lrat_check_delete_clause(ids, nb_ids)
}

pub fn top_check_end_load() -> bool {
    let mut sig_from_chk: Option<Vec<u8>> = None;
    let res = lrat_check::lrat_check_end_load(&mut sig_from_chk);
    let mut valid = TOP.with(|t| {
        let mut t = t.borrow_mut();
        t.valid = t.valid && res;
        t.valid
    });
    if !valid {
        return false;
    }
    if let Some(sig) = sig_from_chk {
        let f_sig = TOP.with(|t| t.borrow().formula_signature);
        valid = trusted_utils_equal_signatures(&sig, &f_sig);
        TOP.with(|t| t.borrow_mut().valid = valid);
    }
    valid
}

pub fn top_check_import(
    id: u64,
    literals: &[i32],
    nb_literals: i32,
    signature_data: &[u8],
) -> bool {
    let mut computed_sig = [0u8; SIG_SIZE_BYTES];
    compute_clause_signature(id, literals, nb_literals, &mut computed_sig);
    if !trusted_utils_equal_signatures(signature_data, &computed_sig) {
        TOP.with(|t| t.borrow_mut().valid = false);
        return false;
    }
    let res = lrat_check::lrat_check_add_axiomatic_clause(id, literals, nb_literals);
    let valid = TOP.with(|t| {
        let mut t = t.borrow_mut();
        t.valid = t.valid && res;
        t.valid
    });
    valid
}

pub fn top_check_valid() -> bool {
    TOP.with(|t| t.borrow().valid)
}

pub fn top_check_load(lit: i32) {
    let res = lrat_check::lrat_check_load(lit);
    TOP.with(|t| {
        let mut t = t.borrow_mut();
        t.valid = t.valid && res;
    });
}

pub fn compute_clause_signature(id: u64, lits: &[i32], nb_lits: i32, out: &mut [u8]) {
    let mut sh = SipHash::siphash_init(&SECRET_KEY);
    sh.siphash_reset();
    let id_bytes = id.to_ne_bytes();
    sh.siphash_update(&id_bytes, 8);
    let lits_bytes: Vec<u8> = lits
        .iter()
        .take(nb_lits as usize)
        .flat_map(|&l| l.to_ne_bytes())
        .collect();
    sh.siphash_update(&lits_bytes, lits_bytes.len() as u64);
    let f_sig = TOP.with(|t| t.borrow().formula_signature);
    sh.siphash_update(&f_sig, SIG_SIZE_BYTES as u64);
    let hash_out = sh.siphash_digest();
    trusted_utils_copy_bytes(out, &hash_out, SIG_SIZE_BYTES as u64);
}

pub fn top_check_validate_unsat(out_signature_or_null: &mut Option<Vec<u8>>) -> bool {
    let res = lrat_check::lrat_check_validate_unsat();
    let valid = TOP.with(|t| {
        let mut t = t.borrow_mut();
        t.valid = t.valid && res;
        t.valid
    });
    if !valid {
        return false;
    }
    if let Some(out) = out_signature_or_null.as_mut() {
        let f_sig = TOP.with(|t| t.borrow().formula_signature);
        out.resize(SIG_SIZE_BYTES, 0);
        confirm_result(&f_sig, 20, out);
    }
    true
}

pub fn top_check_produce(
    id: u64,
    literals: &[i32],
    nb_literals: i32,
    hints: &[u64],
    nb_hints: i32,
    out_sig_or_null: &mut Option<Vec<u8>>,
) -> bool {
    let res = lrat_check::lrat_check_add_clause(id, literals, nb_literals, hints, nb_hints);
    let valid = TOP.with(|t| {
        let mut t = t.borrow_mut();
        t.valid = t.valid && res;
        t.valid
    });
    if !valid {
        return false;
    }
    if let Some(out) = out_sig_or_null.as_mut() {
        out.resize(SIG_SIZE_BYTES, 0);
        compute_clause_signature(id, literals, nb_literals, out);
    }
    true
}
