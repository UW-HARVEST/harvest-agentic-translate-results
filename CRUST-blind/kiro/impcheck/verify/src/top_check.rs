use crate::lrat_check;
use crate::siphash::SipHash;
use crate::secret::SECRET_KEY;
use crate::trusted_utils::{SIG_SIZE_BYTES, trusted_utils_copy_bytes, trusted_utils_equal_signatures};
use crate::confirm::confirm_result;
use std::cell::RefCell;

struct TopCheckState {
    formula_signature: [u8; SIG_SIZE_BYTES],
    valid: bool,
    siphash: SipHash,
    msgstr: String,
}

thread_local! {
    static STATE: RefCell<Option<TopCheckState>> = RefCell::new(None);
}

fn with_state<F, R>(f: F) -> R where F: FnOnce(&mut TopCheckState) -> R {
    STATE.with(|s| {
        let mut borrow = s.borrow_mut();
        f(borrow.as_mut().expect("top_check not initialized"))
    })
}

pub fn top_check_init(nb_vars: i32, check_model: bool, lenient: bool) {
    let sip = SipHash::siphash_init(&SECRET_KEY);
    lrat_check::lrat_check_init(nb_vars, check_model, lenient);
    STATE.with(|s| {
        *s.borrow_mut() = Some(TopCheckState {
            formula_signature: [0u8; SIG_SIZE_BYTES],
            valid: true,
            siphash: sip,
            msgstr: String::new(),
        });
    });
}

pub fn top_check_commit_formula_sig(f_sig: &[u8]) {
    with_state(|st| {
        trusted_utils_copy_bytes(&mut st.formula_signature, f_sig, SIG_SIZE_BYTES as u64);
    });
}

pub fn top_check_validate_sat(model: &[i32], size: u64, out_signature_or_null: &mut Option<Vec<u8>>) -> bool {
    with_state(|st| {
        st.valid &= lrat_check::lrat_check_validate_sat(model, size);
        if !st.valid { return false; }
        if out_signature_or_null.is_some() || true {
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
    with_state(|st| {
        let mut sig_from_chk: Option<Vec<u8>> = None;
        st.valid = st.valid && lrat_check::lrat_check_end_load(&mut sig_from_chk);
        if !st.valid { return false; }
        if let Some(sig) = sig_from_chk {
            st.valid = trusted_utils_equal_signatures(&sig, &st.formula_signature);
            if !st.valid {
                st.msgstr = "Formula signature check failed".to_string();
            }
        } else {
            st.valid = false;
        }
        st.valid
    })
}

pub fn top_check_import(id: u64, literals: &[i32], nb_literals: i32, signature_data: &[u8]) -> bool {
    with_state(|st| {
        let mut computed_sig = [0u8; SIG_SIZE_BYTES];
        compute_clause_sig_internal(&st.siphash, id, literals, nb_literals, &st.formula_signature, &mut computed_sig);
        if !trusted_utils_equal_signatures(signature_data, &computed_sig) {
            st.valid = false;
            st.msgstr = format!("Signature check of clause {} failed", id);
            return false;
        }
        let nl = nb_literals as usize;
        st.valid &= lrat_check::lrat_check_add_axiomatic_clause(id, &literals[..nl], nb_literals);
        st.valid
    })
}

pub fn top_check_valid() -> bool {
    with_state(|st| st.valid)
}

pub fn top_check_load(lit: i32) {
    with_state(|st| {
        st.valid &= lrat_check::lrat_check_load(lit);
    });
}

fn compute_clause_sig_internal(sip_template: &SipHash, id: u64, lits: &[i32], nb_lits: i32, formula_sig: &[u8], out: &mut [u8]) {
    let mut sip = SipHash::siphash_init(&SECRET_KEY);
    sip.siphash_reset();
    sip.siphash_update(&id.to_ne_bytes(), std::mem::size_of::<u64>() as u64);
    let nl = nb_lits as usize;
    let lit_bytes: Vec<u8> = lits[..nl].iter().flat_map(|x| x.to_ne_bytes()).collect();
    sip.siphash_update(&lit_bytes, lit_bytes.len() as u64);
    sip.siphash_update(&formula_sig[..SIG_SIZE_BYTES], SIG_SIZE_BYTES as u64);
    let hash_out = sip.siphash_digest();
    trusted_utils_copy_bytes(out, &hash_out, SIG_SIZE_BYTES as u64);
}

pub fn compute_clause_signature(id: u64, lits: &[i32], nb_lits: i32, out: &mut [u8]) {
    with_state(|st| {
        compute_clause_sig_internal(&st.siphash, id, lits, nb_lits, &st.formula_signature.clone(), out);
    });
}

pub fn top_check_validate_unsat(out_signature_or_null: &mut Option<Vec<u8>>) -> bool {
    with_state(|st| {
        st.valid &= lrat_check::lrat_check_validate_unsat();
        if !st.valid { return false; }
        let mut out = vec![0u8; SIG_SIZE_BYTES];
        confirm_result(&st.formula_signature, 20, &mut out);
        *out_signature_or_null = Some(out);
        true
    })
}

pub fn top_check_produce(id: u64, literals: &[i32], nb_literals: i32, hints: &[u64], nb_hints: i32, out_sig_or_null: &mut Option<Vec<u8>>) -> bool {
    with_state(|st| {
        let nl = nb_literals as usize;
        let nh = nb_hints as usize;
        st.valid &= lrat_check::lrat_check_add_clause(id, &literals[..nl], nb_literals, &hints[..nh], nb_hints);
        if !st.valid { return false; }
        if out_sig_or_null.is_some() || true {
            let mut sig = vec![0u8; SIG_SIZE_BYTES];
            compute_clause_sig_internal(&st.siphash, id, literals, nb_literals, &st.formula_signature.clone(), &mut sig);
            *out_sig_or_null = Some(sig);
        }
        true
    })
}
