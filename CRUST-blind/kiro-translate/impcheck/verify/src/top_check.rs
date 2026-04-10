use crate::siphash::SipHash;
use crate::trusted_utils::{self, SIG_SIZE_BYTES};
use crate::lrat_check;
use crate::confirm::confirm_result;
use std::cell::RefCell;

const SECRET_KEY: [u8; 16] = [86, 93, 1, 209, 112, 176, 13, 40, 168, 223, 25, 22, 134, 58, 21, 211];

struct TopCheckState {
    siphash: SipHash,
    formula_signature: [u8; SIG_SIZE_BYTES],
    valid: bool,
    msgstr: String,
}

thread_local! {
    static STATE: RefCell<Option<TopCheckState>> = RefCell::new(None);
}

fn with_state<F, R>(f: F) -> R
where
    F: FnOnce(&mut TopCheckState) -> R,
{
    STATE.with(|s| {
        let mut borrow = s.borrow_mut();
        f(borrow.as_mut().expect("top_check not initialized"))
    })
}

pub fn top_check_init(nb_vars: i32, check_model: bool, lenient: bool) {
    let siphash = SipHash::siphash_init(&SECRET_KEY);
    lrat_check::lrat_check_init(nb_vars, check_model, lenient);
    STATE.with(|s| {
        *s.borrow_mut() = Some(TopCheckState {
            siphash,
            formula_signature: [0u8; SIG_SIZE_BYTES],
            valid: true,
            msgstr: String::new(),
        });
    });
}

pub fn top_check_commit_formula_sig(f_sig: &[u8]) {
    with_state(|st| {
        trusted_utils::trusted_utils_copy_bytes(&mut st.formula_signature, f_sig, SIG_SIZE_BYTES as u64);
    });
}

pub fn top_check_validate_sat(model: &[i32], size: u64, out_signature_or_null: &mut Option<Vec<u8>>) -> bool {
    with_state(|st| {
        st.valid &= lrat_check::lrat_check_validate_sat(model, size);
        if !st.valid {
            return false;
        }
        let mut sig = [0u8; SIG_SIZE_BYTES];
        confirm_result(&st.formula_signature, 10, &mut sig);
        *out_signature_or_null = Some(sig.to_vec());
        true
    })
}

pub fn top_check_delete(ids: &[u64], nb_ids: i32) -> bool {
    lrat_check::lrat_check_delete_clause(ids, nb_ids)
}

pub fn top_check_end_load() -> bool {
    with_state(|st| {
        let mut sig_opt: Option<Vec<u8>> = None;
        st.valid = st.valid && lrat_check::lrat_check_end_load(&mut sig_opt);
        if !st.valid {
            return false;
        }
        if let Some(sig) = sig_opt {
            st.valid = trusted_utils::trusted_utils_equal_signatures(&sig, &st.formula_signature);
            if !st.valid {
                st.msgstr = "Formula signature check failed".to_string();
            }
        }
        st.valid
    })
}

pub fn top_check_import(id: u64, literals: &[i32], nb_literals: i32, signature_data: &[u8]) -> bool {
    with_state(|st| {
        let mut computed_sig = [0u8; SIG_SIZE_BYTES];
        compute_clause_signature_internal(&st.siphash, id, literals, nb_literals, &st.formula_signature, &mut computed_sig);
        if !trusted_utils::trusted_utils_equal_signatures(signature_data, &computed_sig) {
            st.valid = false;
            st.msgstr = format!("Signature check of clause {} failed", id);
            return false;
        }
        let ok = lrat_check::lrat_check_add_axiomatic_clause(id, literals, nb_literals);
        st.valid &= ok;
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

fn compute_clause_signature_internal(siphash: &SipHash, id: u64, lits: &[i32], nb_lits: i32, formula_sig: &[u8], out: &mut [u8]) {
    // Clone the siphash and reset it to compute clause signature
    let mut hasher = SipHash::siphash_init(&SECRET_KEY);
    hasher.siphash_reset();
    hasher.siphash_update(&id.to_ne_bytes(), std::mem::size_of::<u64>() as u64);
    let lit_bytes: Vec<u8> = lits[..nb_lits as usize].iter()
        .flat_map(|x| x.to_ne_bytes())
        .collect();
    hasher.siphash_update(&lit_bytes, lit_bytes.len() as u64);
    hasher.siphash_update(&formula_sig[..SIG_SIZE_BYTES], SIG_SIZE_BYTES as u64);
    let hash_out = hasher.siphash_digest();
    trusted_utils::trusted_utils_copy_bytes(out, &hash_out, SIG_SIZE_BYTES as u64);
}

pub fn compute_clause_signature(id: u64, lits: &[i32], nb_lits: i32, out: &mut [u8]) {
    with_state(|st| {
        compute_clause_signature_internal(&st.siphash, id, lits, nb_lits, &st.formula_signature.clone(), out);
    });
}

pub fn top_check_validate_unsat(out_signature_or_null: &mut Option<Vec<u8>>) -> bool {
    with_state(|st| {
        st.valid &= lrat_check::lrat_check_validate_unsat();
        if !st.valid {
            return false;
        }
        let mut sig = [0u8; SIG_SIZE_BYTES];
        confirm_result(&st.formula_signature, 20, &mut sig);
        *out_signature_or_null = Some(sig.to_vec());
        true
    })
}

pub fn top_check_produce(id: u64, literals: &[i32], nb_literals: i32, hints: &[u64], nb_hints: i32, out_sig_or_null: &mut Option<Vec<u8>>) -> bool {
    with_state(|st| {
        let ok = lrat_check::lrat_check_add_clause(id, literals, nb_literals, hints, nb_hints);
        st.valid &= ok;
        if !st.valid {
            return false;
        }
        // compute signature
        let mut sig = [0u8; SIG_SIZE_BYTES];
        compute_clause_signature_internal(&st.siphash, id, literals, nb_literals, &st.formula_signature.clone(), &mut sig);
        *out_sig_or_null = Some(sig.to_vec());
        true
    })
}

// Helper to get msgstr for error reporting
pub fn top_check_get_msgstr() -> String {
    with_state(|st| {
        if st.msgstr.is_empty() {
            lrat_check::lrat_get_msgstr()
        } else {
            st.msgstr.clone()
        }
    })
}
