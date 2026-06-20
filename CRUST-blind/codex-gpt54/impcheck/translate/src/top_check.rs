use std::cell::RefCell;

struct TopCheckState {
    parsed_formula: bool,
    formula_signature: [u8; crate::trusted_utils::SIG_SIZE_BYTES],
    valid: bool,
}

thread_local! {
    static TOP_CHECK_STATE: RefCell<TopCheckState> = const { RefCell::new(TopCheckState {
        parsed_formula: false,
        formula_signature: [0; crate::trusted_utils::SIG_SIZE_BYTES],
        valid: true,
    }) };
}

fn secret_key() -> [u8; 16] {
    [86_u8, 93, 1, 209, 112, 176, 13, 40, 168, 223, 25, 22, 134, 58, 21, 211]
}

pub fn top_check_init(nb_vars: i32, check_model: bool, lenient: bool) {
    let _ = secret_key();
    crate::lrat_check::lrat_check_init(nb_vars, check_model, lenient);
    TOP_CHECK_STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.parsed_formula = false;
        state.formula_signature = [0; crate::trusted_utils::SIG_SIZE_BYTES];
        state.valid = true;
    });
}
pub fn top_check_commit_formula_sig(f_sig: &[u8]) {
    TOP_CHECK_STATE.with(|state| {
        let mut state = state.borrow_mut();
        crate::trusted_utils::trusted_utils_copy_bytes(
            &mut state.formula_signature,
            f_sig,
            crate::trusted_utils::SIG_SIZE_BYTES as u64,
        );
        state.parsed_formula = true;
    });
}
pub fn top_check_validate_sat(model: &[i32], size: u64, out_signature_or_null: &mut Option<Vec<u8>>) -> bool {
    let res = crate::lrat_check::lrat_check_validate_sat(model, size);
    TOP_CHECK_STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.valid &= res;
        if res {
            let mut out = vec![0_u8; crate::trusted_utils::SIG_SIZE_BYTES];
            crate::confirm::confirm_result(&state.formula_signature, 10, &mut out);
            *out_signature_or_null = Some(out);
        }
    });
    res
}
pub fn top_check_delete(ids: &[u64], nb_ids: i32) -> bool {
    crate::lrat_check::lrat_check_delete_clause(ids, nb_ids)
}
pub fn top_check_end_load() -> bool {
    let mut sig = None;
    let res = crate::lrat_check::lrat_check_end_load(&mut sig);
    TOP_CHECK_STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.valid &= res;
        if !res {
            return;
        }
        if let Some(sig_from_chk) = sig {
            state.valid = crate::trusted_utils::trusted_utils_equal_signatures(
                &sig_from_chk,
                &state.formula_signature,
            );
        }
    });
    top_check_valid()
}
pub fn top_check_import(id: u64, literals: &[i32], nb_literals: i32, signature_data: &[u8]) -> bool {
    let mut computed = vec![0_u8; crate::trusted_utils::SIG_SIZE_BYTES];
    compute_clause_signature(id, literals, nb_literals, &mut computed);
    let res = crate::trusted_utils::trusted_utils_equal_signatures(signature_data, &computed)
        && crate::lrat_check::lrat_check_add_axiomatic_clause(id, literals, nb_literals);
    TOP_CHECK_STATE.with(|state| {
        state.borrow_mut().valid &= res;
    });
    res
}
pub fn top_check_valid() -> bool {
    TOP_CHECK_STATE.with(|state| state.borrow().valid)
}
pub fn top_check_load(lit: i32) {
    let res = crate::lrat_check::lrat_check_load(lit);
    TOP_CHECK_STATE.with(|state| {
        state.borrow_mut().valid &= res;
    });
}
pub fn compute_clause_signature(id: u64, lits: &[i32], nb_lits: i32, out: &mut [u8]) {
    let mut hasher = crate::siphash::SipHash::siphash_init(&secret_key());
    hasher.siphash_update(&id.to_ne_bytes(), std::mem::size_of::<u64>() as u64);
    let mut lit_bytes = Vec::with_capacity(nb_lits.max(0) as usize * std::mem::size_of::<i32>());
    for lit in lits.iter().take(nb_lits.max(0) as usize) {
        lit_bytes.extend_from_slice(&lit.to_ne_bytes());
    }
    hasher.siphash_update(&lit_bytes, lit_bytes.len() as u64);
    TOP_CHECK_STATE.with(|state| {
        let formula_sig = state.borrow().formula_signature;
        hasher.siphash_update(&formula_sig, crate::trusted_utils::SIG_SIZE_BYTES as u64);
    });
    let sig = hasher.siphash_digest();
    crate::trusted_utils::trusted_utils_copy_bytes(
        out,
        &sig,
        crate::trusted_utils::SIG_SIZE_BYTES as u64,
    );
}
pub fn top_check_validate_unsat(out_signature_or_null: &mut Option<Vec<u8>>) -> bool {
    let res = crate::lrat_check::lrat_check_validate_unsat();
    TOP_CHECK_STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.valid &= res;
        if res {
            let mut out = vec![0_u8; crate::trusted_utils::SIG_SIZE_BYTES];
            crate::confirm::confirm_result(&state.formula_signature, 20, &mut out);
            *out_signature_or_null = Some(out);
        }
    });
    res
}
pub fn top_check_produce(id: u64, literals: &[i32], nb_literals: i32, hints: &[u64], nb_hints: i32, out_sig_or_null: &mut Option<Vec<u8>>) -> bool {
    let res = crate::lrat_check::lrat_check_add_clause(id, literals, nb_literals, hints, nb_hints);
    TOP_CHECK_STATE.with(|state| {
        state.borrow_mut().valid &= res;
    });
    if res && out_sig_or_null.is_some() {
        let mut sig = vec![0_u8; crate::trusted_utils::SIG_SIZE_BYTES];
        compute_clause_signature(id, literals, nb_literals, &mut sig);
        *out_sig_or_null = Some(sig);
    }
    res
}
