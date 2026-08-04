// Direct port of c_src/src/trusted/top_check.c

use crate::confirm::confirm_result;
use crate::lrat_check::{
    lrat_check_add_axiomatic_clause, lrat_check_add_clause, lrat_check_delete_clause,
    lrat_check_end_load, lrat_check_init, lrat_check_load, lrat_check_validate_sat,
    lrat_check_validate_unsat, STATE,
};
use crate::secret::SECRET_KEY;
use crate::siphash::SipHash;
use crate::trusted_utils::{
    trusted_utils_copy_bytes, trusted_utils_equal_signatures, SIG_SIZE_BYTES,
};

use std::cell::RefCell;

pub struct TopCheckState {
    pub parsed_formula: bool,
    pub formula_signature: [u8; SIG_SIZE_BYTES],
    pub valid: bool,
    pub siphash: SipHash,
}

impl TopCheckState {
    fn new() -> Self {
        TopCheckState {
            parsed_formula: false,
            formula_signature: [0u8; SIG_SIZE_BYTES],
            valid: true,
            siphash: SipHash::siphash_init(&SECRET_KEY),
        }
    }
}

thread_local! {
    pub static TC: RefCell<TopCheckState> = RefCell::new(TopCheckState::new());
}

pub fn compute_clause_signature(id: u64, lits: &[i32], nb_lits: i32, out: &mut [u8]) {
    TC.with(|tc| {
        let mut st = tc.borrow_mut();
        st.siphash.siphash_reset();
        let id_bytes = id.to_ne_bytes();
        st.siphash.siphash_update(&id_bytes, 8);
        let mut lit_bytes: Vec<u8> = Vec::with_capacity(nb_lits as usize * 4);
        for i in 0..(nb_lits as usize) {
            lit_bytes.extend_from_slice(&lits[i].to_ne_bytes());
        }
        st.siphash
            .siphash_update(&lit_bytes, (nb_lits as u64) * 4);
        let f_sig = st.formula_signature;
        st.siphash.siphash_update(&f_sig, SIG_SIZE_BYTES as u64);
        let hash_out = st.siphash.siphash_digest();
        trusted_utils_copy_bytes(out, &hash_out, SIG_SIZE_BYTES as u64);
    });
}

pub fn top_check_init(nb_vars: i32, check_model: bool, lenient: bool) {
    TC.with(|tc| {
        let mut st = tc.borrow_mut();
        *st = TopCheckState::new();
        st.siphash = SipHash::siphash_init(&SECRET_KEY);
    });
    // initialize the lrat_check module's siphash too
    lrat_check_init(nb_vars, check_model, lenient);
    STATE.with(|s| {
        let mut st = s.borrow_mut();
        st.siphash = Some(SipHash::siphash_init(&SECRET_KEY));
    });
}

pub fn top_check_commit_formula_sig(f_sig: &[u8]) {
    TC.with(|tc| {
        let mut st = tc.borrow_mut();
        for i in 0..SIG_SIZE_BYTES {
            st.formula_signature[i] = f_sig[i];
        }
    });
}

pub fn top_check_load(lit: i32) {
    let ok = lrat_check_load(lit);
    TC.with(|tc| {
        let mut st = tc.borrow_mut();
        st.valid &= ok;
    });
}

pub fn top_check_end_load() -> bool {
    let mut sig_from_chk: Option<Vec<u8>> = None;
    let valid_pre = TC.with(|tc| tc.borrow().valid);
    let ok = if valid_pre {
        lrat_check_end_load(&mut sig_from_chk)
    } else {
        false
    };
    TC.with(|tc| {
        let mut st = tc.borrow_mut();
        st.valid = st.valid && ok;
    });
    let valid_now = TC.with(|tc| tc.borrow().valid);
    if !valid_now {
        return false;
    }
    let sig = sig_from_chk.unwrap_or_default();
    let f_sig = TC.with(|tc| tc.borrow().formula_signature);
    let eq = trusted_utils_equal_signatures(&sig, &f_sig);
    TC.with(|tc| {
        let mut st = tc.borrow_mut();
        st.valid = eq;
    });
    if !eq {
        STATE.with(|s| {
            let mut st = s.borrow_mut();
            st.msgstr = "Formula signature check failed".to_string();
        });
    }
    eq
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
    TC.with(|tc| {
        let mut st = tc.borrow_mut();
        st.valid &= res;
    });
    let valid = TC.with(|tc| tc.borrow().valid);
    if !valid {
        return false;
    }
    if out_sig_or_null.is_some() {
        let mut sig_buf = vec![0u8; SIG_SIZE_BYTES];
        compute_clause_signature(id, literals, nb_literals, &mut sig_buf);
        *out_sig_or_null = Some(sig_buf);
    }
    true
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
        TC.with(|tc| {
            let mut st = tc.borrow_mut();
            st.valid = false;
        });
        STATE.with(|s| {
            let mut st = s.borrow_mut();
            st.msgstr = format!("Signature check of clause {} failed", id);
        });
        return false;
    }
    let ok = lrat_check_add_axiomatic_clause(id, literals, nb_literals);
    TC.with(|tc| {
        let mut st = tc.borrow_mut();
        st.valid &= ok;
    });
    TC.with(|tc| tc.borrow().valid)
}

pub fn top_check_delete(ids: &[u64], nb_ids: i32) -> bool {
    lrat_check_delete_clause(ids, nb_ids)
}

pub fn top_check_validate_unsat(out_signature_or_null: &mut Option<Vec<u8>>) -> bool {
    let res = lrat_check_validate_unsat();
    TC.with(|tc| {
        let mut st = tc.borrow_mut();
        st.valid &= res;
    });
    let valid = TC.with(|tc| tc.borrow().valid);
    if !valid {
        return false;
    }
    if out_signature_or_null.is_some() {
        let f_sig = TC.with(|tc| tc.borrow().formula_signature);
        let mut out = vec![0u8; SIG_SIZE_BYTES];
        confirm_result(&f_sig, 20, &mut out);
        *out_signature_or_null = Some(out);
    }
    true
}

pub fn top_check_validate_sat(
    model: &[i32],
    size: u64,
    out_signature_or_null: &mut Option<Vec<u8>>,
) -> bool {
    let res = lrat_check_validate_sat(model, size);
    TC.with(|tc| {
        let mut st = tc.borrow_mut();
        st.valid &= res;
    });
    let valid = TC.with(|tc| tc.borrow().valid);
    if !valid {
        return false;
    }
    if out_signature_or_null.is_some() {
        let f_sig = TC.with(|tc| tc.borrow().formula_signature);
        let mut out = vec![0u8; SIG_SIZE_BYTES];
        confirm_result(&f_sig, 10, &mut out);
        *out_signature_or_null = Some(out);
    }
    true
}

pub fn top_check_valid() -> bool {
    TC.with(|tc| tc.borrow().valid)
}
