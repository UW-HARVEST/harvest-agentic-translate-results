use std::cell::RefCell;

use crate::confirm::confirm_result;
use crate::lrat_check::{
    lrat_check_add_axiomatic_clause, lrat_check_add_clause, lrat_check_delete_clause,
    lrat_check_end_load, lrat_check_init, lrat_check_load, lrat_check_validate_sat,
    lrat_check_validate_unsat,
};
use crate::siphash::SipHash;
use crate::trusted_utils::{
    trusted_utils_copy_bytes, trusted_utils_equal_signatures, SIG_SIZE_BYTES,
};

const SECRET_KEY: [u8; 16] = [
    86, 93, 1, 209, 112, 176, 13, 40, 168, 223, 25, 22, 134, 58, 21, 211,
];

thread_local! {
    static FORMULA_SIG: RefCell<[u8; SIG_SIZE_BYTES]> = RefCell::new([0u8; SIG_SIZE_BYTES]);
    static VALID: RefCell<bool> = RefCell::new(true);
}

pub fn top_check_init(nb_vars: i32, check_model: bool, lenient: bool) {
    // siphash_init in C is global; here we (re)initialize the LRAT checker.
    let _ = SipHash::siphash_init(&SECRET_KEY);
    lrat_check_init(nb_vars, check_model, lenient);
    VALID.with(|v| *v.borrow_mut() = true);
}

pub fn top_check_commit_formula_sig(f_sig: &[u8]) {
    FORMULA_SIG.with(|fs| {
        let mut s = fs.borrow_mut();
        trusted_utils_copy_bytes(&mut s[..], f_sig, SIG_SIZE_BYTES as u64);
    });
}

pub fn top_check_validate_sat(
    model: &[i32],
    size: u64,
    out_signature_or_null: &mut Option<Vec<u8>>,
) -> bool {
    let v = lrat_check_validate_sat(model, size);
    VALID.with(|val| {
        let mut b = val.borrow_mut();
        *b = *b && v;
    });
    if !VALID.with(|val| *val.borrow()) {
        return false;
    }
    if let Some(out) = out_signature_or_null.as_mut() {
        out.resize(SIG_SIZE_BYTES, 0);
        let f_sig: Vec<u8> = FORMULA_SIG.with(|fs| fs.borrow().to_vec());
        confirm_result(&f_sig, 10, out);
    }
    true
}

pub fn top_check_delete(ids: &[u64], nb_ids: i32) -> bool {
    lrat_check_delete_clause(ids, nb_ids)
}

pub fn top_check_end_load() -> bool {
    let mut sig_from_chk: Option<Vec<u8>> = None;
    let mut current_valid = VALID.with(|v| *v.borrow());
    current_valid = current_valid && lrat_check_end_load(&mut sig_from_chk);
    VALID.with(|v| *v.borrow_mut() = current_valid);
    if !current_valid {
        return false;
    }
    let f_sig: Vec<u8> = FORMULA_SIG.with(|fs| fs.borrow().to_vec());
    let from_chk = sig_from_chk.unwrap_or_else(|| vec![0u8; SIG_SIZE_BYTES]);
    let equal = trusted_utils_equal_signatures(&from_chk, &f_sig);
    VALID.with(|v| *v.borrow_mut() = equal);
    equal
}

pub fn top_check_import(
    id: u64,
    literals: &[i32],
    nb_literals: i32,
    signature_data: &[u8],
) -> bool {
    let mut computed_sig = vec![0u8; SIG_SIZE_BYTES];
    compute_clause_signature(id, literals, nb_literals, &mut computed_sig);
    if !trusted_utils_equal_signatures(signature_data, &computed_sig) {
        VALID.with(|v| *v.borrow_mut() = false);
        return false;
    }
    let res = lrat_check_add_axiomatic_clause(id, literals, nb_literals);
    VALID.with(|v| {
        let mut b = v.borrow_mut();
        *b = *b && res;
    });
    VALID.with(|v| *v.borrow())
}

pub fn top_check_valid() -> bool {
    VALID.with(|v| *v.borrow())
}

pub fn top_check_load(lit: i32) {
    let res = lrat_check_load(lit);
    VALID.with(|v| {
        let mut b = v.borrow_mut();
        *b = *b && res;
    });
}

pub fn compute_clause_signature(id: u64, lits: &[i32], nb_lits: i32, out: &mut [u8]) {
    let mut sh = SipHash::siphash_init(&SECRET_KEY);
    sh.siphash_reset();
    let id_bytes = id.to_ne_bytes();
    sh.siphash_update(&id_bytes, 8);
    // serialize lits as raw bytes
    let mut lit_bytes: Vec<u8> = Vec::with_capacity((nb_lits as usize) * 4);
    for i in 0..(nb_lits as usize) {
        lit_bytes.extend_from_slice(&lits[i].to_ne_bytes());
    }
    sh.siphash_update(&lit_bytes, lit_bytes.len() as u64);
    let f_sig: Vec<u8> = FORMULA_SIG.with(|fs| fs.borrow().to_vec());
    sh.siphash_update(&f_sig, SIG_SIZE_BYTES as u64);
    let hash_out = sh.siphash_digest();
    trusted_utils_copy_bytes(out, &hash_out, SIG_SIZE_BYTES as u64);
}

pub fn top_check_validate_unsat(out_signature_or_null: &mut Option<Vec<u8>>) -> bool {
    let v = lrat_check_validate_unsat();
    VALID.with(|val| {
        let mut b = val.borrow_mut();
        *b = *b && v;
    });
    if !VALID.with(|val| *val.borrow()) {
        return false;
    }
    if let Some(out) = out_signature_or_null.as_mut() {
        out.resize(SIG_SIZE_BYTES, 0);
        let f_sig: Vec<u8> = FORMULA_SIG.with(|fs| fs.borrow().to_vec());
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
    let res = lrat_check_add_clause(id, literals, nb_literals, hints, nb_hints);
    VALID.with(|v| {
        let mut b = v.borrow_mut();
        *b = *b && res;
    });
    if !VALID.with(|v| *v.borrow()) {
        return false;
    }
    if let Some(out) = out_sig_or_null.as_mut() {
        out.resize(SIG_SIZE_BYTES, 0);
        compute_clause_signature(id, literals, nb_literals, out);
    }
    true
}
