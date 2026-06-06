use std::cell::RefCell;

use crate::confirm::confirm_result;
use crate::lrat_check::{
    lrat_check_add_axiomatic_clause, lrat_check_add_clause, lrat_check_delete_clause,
    lrat_check_end_load, lrat_check_init, lrat_check_load, lrat_check_validate_sat,
    lrat_check_validate_unsat, with_siphash,
};
use crate::trusted_utils::{
    trusted_utils_copy_bytes, trusted_utils_equal_signatures, SIG_SIZE_BYTES,
};

thread_local! {
    static TOP_STATE: RefCell<TopState> = RefCell::new(TopState::new());
}

struct TopState {
    formula_signature: [u8; SIG_SIZE_BYTES],
    valid: bool,
    msg: String,
}

impl TopState {
    fn new() -> Self {
        Self {
            formula_signature: [0u8; SIG_SIZE_BYTES],
            valid: true,
            msg: String::new(),
        }
    }
}

fn with_top<R>(f: impl FnOnce(&mut TopState) -> R) -> R {
    TOP_STATE.with(|s| f(&mut s.borrow_mut()))
}

pub fn top_check_init(nb_vars: i32, check_model: bool, lenient: bool) {
    // Initialize the underlying siphash by touching it.
    with_siphash(|sh| sh.siphash_reset());
    lrat_check_init(nb_vars, check_model, lenient);
    with_top(|st| {
        st.formula_signature = [0u8; SIG_SIZE_BYTES];
        st.valid = true;
        st.msg.clear();
    });
}

pub fn top_check_commit_formula_sig(f_sig: &[u8]) {
    with_top(|st| {
        trusted_utils_copy_bytes(&mut st.formula_signature, f_sig, SIG_SIZE_BYTES as u64);
    });
}

pub fn top_check_validate_sat(
    model: &[i32],
    size: u64,
    out_signature_or_null: &mut Option<Vec<u8>>,
) -> bool {
    let res = lrat_check_validate_sat(model, size);
    let valid_after = with_top(|st| {
        st.valid &= res;
        st.valid
    });
    if !valid_after {
        return false;
    }
    if let Some(out) = out_signature_or_null {
        if out.len() < SIG_SIZE_BYTES {
            out.resize(SIG_SIZE_BYTES, 0);
        }
        with_top(|st| {
            confirm_result(&st.formula_signature, 10, out);
        });
    }
    true
}

pub fn top_check_delete(ids: &[u64], nb_ids: i32) -> bool {
    lrat_check_delete_clause(ids, nb_ids)
}

pub fn top_check_end_load() -> bool {
    let mut sig: Option<Vec<u8>> = None;
    let r = lrat_check_end_load(&mut sig);
    let mut overall = with_top(|st| {
        st.valid = st.valid && r;
        st.valid
    });
    if !overall {
        return false;
    }
    let computed = sig.unwrap_or_default();
    let formula_sig = with_top(|st| st.formula_signature);
    overall = trusted_utils_equal_signatures(&computed, &formula_sig);
    with_top(|st| {
        st.valid = overall;
        if !overall {
            st.msg = "Formula signature check failed".to_string();
        }
    });
    overall
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
        with_top(|st| {
            st.valid = false;
            st.msg = format!("Signature check of clause {} failed", id);
        });
        return false;
    }
    let r = lrat_check_add_axiomatic_clause(id, literals, nb_literals);
    with_top(|st| {
        st.valid &= r;
        st.valid
    })
}

pub fn top_check_valid() -> bool {
    with_top(|st| st.valid)
}

pub fn top_check_load(lit: i32) {
    let r = lrat_check_load(lit);
    with_top(|st| st.valid &= r);
}

pub fn compute_clause_signature(id: u64, lits: &[i32], nb_lits: i32, out: &mut [u8]) {
    let formula_sig = with_top(|st| st.formula_signature);
    with_siphash(|sh| {
        sh.siphash_reset();
        // id as 8 bytes
        let id_bytes = id.to_ne_bytes();
        sh.siphash_update(&id_bytes, 8);
        // lits as bytes
        let mut lit_bytes = Vec::with_capacity(nb_lits as usize * 4);
        for i in 0..nb_lits as usize {
            lit_bytes.extend_from_slice(&lits[i].to_ne_bytes());
        }
        sh.siphash_update(&lit_bytes, lit_bytes.len() as u64);
        sh.siphash_update(&formula_sig, SIG_SIZE_BYTES as u64);
        let digest = sh.siphash_digest();
        trusted_utils_copy_bytes(out, &digest, SIG_SIZE_BYTES as u64);
    });
}

pub fn top_check_validate_unsat(out_signature_or_null: &mut Option<Vec<u8>>) -> bool {
    let r = lrat_check_validate_unsat();
    let valid_after = with_top(|st| {
        st.valid &= r;
        st.valid
    });
    if !valid_after {
        return false;
    }
    if let Some(out) = out_signature_or_null {
        if out.len() < SIG_SIZE_BYTES {
            out.resize(SIG_SIZE_BYTES, 0);
        }
        with_top(|st| {
            confirm_result(&st.formula_signature, 20, out);
        });
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
    let r = lrat_check_add_clause(id, literals, nb_literals, hints, nb_hints);
    let valid_after = with_top(|st| {
        st.valid &= r;
        st.valid
    });
    if !valid_after {
        return false;
    }
    if let Some(out) = out_sig_or_null {
        if out.len() < SIG_SIZE_BYTES {
            out.resize(SIG_SIZE_BYTES, 0);
        }
        compute_clause_signature(id, literals, nb_literals, out);
    }
    true
}
