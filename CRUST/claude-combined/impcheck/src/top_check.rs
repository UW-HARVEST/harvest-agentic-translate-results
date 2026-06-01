use crate::confirm::confirm_result;
use crate::lrat_check;
use crate::secret::SECRET_KEY;
use crate::siphash::SipHash;
use crate::siphash_global;
use crate::trusted_utils::{
    trusted_utils_copy_bytes, trusted_utils_equal_signatures, SIG_SIZE_BYTES,
};

use std::cell::RefCell;

struct TopState {
    formula_signature: Vec<u8>,
    valid: bool,
}

thread_local! {
    static STATE: RefCell<TopState> = RefCell::new(TopState {
        formula_signature: vec![0u8; SIG_SIZE_BYTES],
        valid: true,
    });
}

pub fn top_check_init(nb_vars: i32, check_model: bool, lenient: bool) {
    siphash_global::init(&SECRET_KEY);
    STATE.with(|s| {
        let mut st = s.borrow_mut();
        st.valid = true;
        st.formula_signature = vec![0u8; SIG_SIZE_BYTES];
    });
    lrat_check::lrat_check_init(nb_vars, check_model, lenient);
}

pub fn top_check_commit_formula_sig(f_sig: &[u8]) {
    STATE.with(|s| {
        let mut st = s.borrow_mut();
        let mut buf = vec![0u8; SIG_SIZE_BYTES];
        trusted_utils_copy_bytes(&mut buf, f_sig, SIG_SIZE_BYTES as u64);
        st.formula_signature = buf;
    });
}

pub fn top_check_validate_sat(
    model: &[i32],
    size: u64,
    out_signature_or_null: &mut Option<Vec<u8>>,
) -> bool {
    let valid = lrat_check::lrat_check_validate_sat(model, size);
    STATE.with(|s| {
        let mut st = s.borrow_mut();
        st.valid = st.valid && valid;
        if !st.valid {
            return false;
        }
        if out_signature_or_null.is_some() {
            let mut out = vec![0u8; SIG_SIZE_BYTES];
            confirm_result(&st.formula_signature, 10, &mut out);
            *out_signature_or_null = Some(out);
        }
        true
    })
}

pub fn top_check_delete(ids: &[u64], nb_ids: i32) -> bool {
    lrat_check::lrat_check_delete_clause(ids, nb_ids)
}

pub fn top_check_end_load() -> bool {
    let mut sig_from_chk: Option<Vec<u8>> = None;
    let ok = lrat_check::lrat_check_end_load(&mut sig_from_chk);
    STATE.with(|s| {
        let mut st = s.borrow_mut();
        st.valid = st.valid && ok;
        if !st.valid {
            return false;
        }
        if let Some(sig) = sig_from_chk {
            st.valid = trusted_utils_equal_signatures(&sig, &st.formula_signature);
        }
        st.valid
    })
}

pub fn top_check_import(
    id: u64,
    literals: &[i32],
    nb_literals: i32,
    signature_data: &[u8],
) -> bool {
    let mut computed_sig = [0u8; SIG_SIZE_BYTES];
    compute_clause_signature(id, literals, nb_literals, &mut computed_sig);
    let same = trusted_utils_equal_signatures(signature_data, &computed_sig);
    STATE.with(|s| {
        let mut st = s.borrow_mut();
        if !same {
            st.valid = false;
            return false;
        }
        let added = lrat_check::lrat_check_add_axiomatic_clause(id, literals, nb_literals);
        st.valid = st.valid && added;
        st.valid
    })
}

pub fn top_check_valid() -> bool {
    STATE.with(|s| s.borrow().valid)
}

pub fn top_check_load(lit: i32) {
    let ok = lrat_check::lrat_check_load(lit);
    STATE.with(|s| {
        let mut st = s.borrow_mut();
        st.valid = st.valid && ok;
    });
}

pub fn compute_clause_signature(id: u64, lits: &[i32], nb_lits: i32, out: &mut [u8]) {
    siphash_global::reset();
    siphash_global::with_global_siphash(|sh| {
        sh.siphash_update(&id.to_ne_bytes(), 8);
        let mut bytes = Vec::with_capacity(nb_lits as usize * 4);
        for i in 0..nb_lits as usize {
            bytes.extend_from_slice(&lits[i].to_ne_bytes());
        }
        sh.siphash_update(&bytes, bytes.len() as u64);
    });
    let f_sig = STATE.with(|s| s.borrow().formula_signature.clone());
    siphash_global::with_global_siphash(|sh| {
        sh.siphash_update(&f_sig, SIG_SIZE_BYTES as u64);
        let hash_out = sh.siphash_digest();
        out[..SIG_SIZE_BYTES].copy_from_slice(&hash_out[..SIG_SIZE_BYTES]);
    });
    let _ = SipHash::siphash_init; // suppress unused warn
}

pub fn top_check_validate_unsat(out_signature_or_null: &mut Option<Vec<u8>>) -> bool {
    let valid = lrat_check::lrat_check_validate_unsat();
    STATE.with(|s| {
        let mut st = s.borrow_mut();
        st.valid = st.valid && valid;
        if !st.valid {
            return false;
        }
        if out_signature_or_null.is_some() {
            let mut out = vec![0u8; SIG_SIZE_BYTES];
            confirm_result(&st.formula_signature, 20, &mut out);
            *out_signature_or_null = Some(out);
        }
        true
    })
}

pub fn top_check_produce(
    id: u64,
    literals: &[i32],
    nb_literals: i32,
    hints: &[u64],
    nb_hints: i32,
    out_sig_or_null: &mut Option<Vec<u8>>,
) -> bool {
    let added = lrat_check::lrat_check_add_clause(id, literals, nb_literals, hints, nb_hints);
    STATE.with(|s| {
        let mut st = s.borrow_mut();
        st.valid = st.valid && added;
        if !st.valid {
            return false;
        }
        if out_sig_or_null.is_some() {
            let mut out = vec![0u8; SIG_SIZE_BYTES];
            drop(st);
            compute_clause_signature(id, literals, nb_literals, &mut out);
            *out_sig_or_null = Some(out);
        }
        true
    })
}
