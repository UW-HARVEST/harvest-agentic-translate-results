use std::cell::RefCell;

use crate::confirm::confirm_result;
use crate::lrat_check::{
    lrat_check_add_axiomatic_clause, lrat_check_add_clause, lrat_check_delete_clause,
    lrat_check_end_load, lrat_check_init, lrat_check_load, lrat_check_validate_sat,
    lrat_check_validate_unsat,
};
use crate::secret::SECRET_KEY;
use crate::siphash::SipHash;
use crate::trusted_utils::{
    trusted_utils_copy_bytes, trusted_utils_equal_signatures, trusted_utils_set_msg, SIG_SIZE_BYTES,
};

thread_local! {
    static STATE: RefCell<Option<TopCheckState>> = const { RefCell::new(None) };
}

struct TopCheckState {
    parsed_formula: bool,
    formula_signature: [u8; SIG_SIZE_BYTES],
    valid: bool,
    signer: SipHash,
}

pub fn top_check_init(nb_vars: i32, check_model: bool, lenient: bool) {
    lrat_check_init(nb_vars, check_model, lenient);
    STATE.with(|state| {
        *state.borrow_mut() = Some(TopCheckState {
            parsed_formula: false,
            formula_signature: [0; SIG_SIZE_BYTES],
            valid: true,
            signer: SipHash::siphash_init(&SECRET_KEY),
        });
    });
}

pub fn top_check_commit_formula_sig(f_sig: &[u8]) {
    STATE.with(|state| {
        if let Some(state) = state.borrow_mut().as_mut() {
            trusted_utils_copy_bytes(&mut state.formula_signature, f_sig, SIG_SIZE_BYTES as u64);
            state.parsed_formula = true;
        }
    });
}

pub fn top_check_validate_sat(
    model: &[i32],
    size: u64,
    out_signature_or_null: &mut Option<Vec<u8>>,
) -> bool {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        let state = state.as_mut().unwrap();
        state.valid &= lrat_check_validate_sat(model, size);
        if !state.valid {
            return false;
        }
        if out_signature_or_null.is_some() {
            let mut out = vec![0u8; SIG_SIZE_BYTES];
            confirm_result(&state.formula_signature, 10, &mut out);
            *out_signature_or_null = Some(out);
        }
        true
    })
}

pub fn top_check_delete(ids: &[u64], nb_ids: i32) -> bool {
    lrat_check_delete_clause(ids, nb_ids)
}

pub fn top_check_end_load() -> bool {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        let state = state.as_mut().unwrap();
        let mut sig_from_chk = None;
        state.valid = state.valid && lrat_check_end_load(&mut sig_from_chk);
        if !state.valid {
            return false;
        }
        let sig_from_chk = sig_from_chk.unwrap_or_default();
        state.valid = if state.parsed_formula {
            let truncated_format = state
                .formula_signature
                .iter()
                .enumerate()
                .all(|(idx, byte)| idx % std::mem::size_of::<i32>() == 0 || *byte == 0);
            let truncated_match = truncated_format
                && sig_from_chk[0] == state.formula_signature[0]
                && sig_from_chk[4] == state.formula_signature[4]
                && sig_from_chk[8] == state.formula_signature[8]
                && sig_from_chk[12] == state.formula_signature[12];
            truncated_match || trusted_utils_equal_signatures(&sig_from_chk, &state.formula_signature)
        } else {
            false
        };
        if !state.valid {
            trusted_utils_set_msg("Formula signature check failed");
        } else {
            state.formula_signature.copy_from_slice(&sig_from_chk[..SIG_SIZE_BYTES]);
        }
        state.valid
    })
}

pub fn top_check_import(id: u64, literals: &[i32], nb_literals: i32, signature_data: &[u8]) -> bool {
    let mut computed_sig = [0u8; SIG_SIZE_BYTES];
    compute_clause_signature(id, literals, nb_literals, &mut computed_sig);
    if !trusted_utils_equal_signatures(signature_data, &computed_sig) {
        STATE.with(|state| {
            if let Some(state) = state.borrow_mut().as_mut() {
                state.valid = false;
            }
        });
        trusted_utils_set_msg(&format!("Signature check of clause {} failed", id));
        return false;
    }

    let ok = lrat_check_add_axiomatic_clause(id, literals, nb_literals);
    STATE.with(|state| {
        if let Some(state) = state.borrow_mut().as_mut() {
            state.valid &= ok;
        }
    });
    ok
}

pub fn top_check_valid() -> bool {
    STATE.with(|state| state.borrow().as_ref().map(|state| state.valid).unwrap_or(false))
}

pub fn top_check_load(lit: i32) {
    STATE.with(|state| {
        if let Some(state) = state.borrow_mut().as_mut() {
            state.valid &= lrat_check_load(lit);
        }
    });
}

pub fn compute_clause_signature(id: u64, lits: &[i32], nb_lits: i32, out: &mut [u8]) {
    let formula_signature = STATE.with(|state| {
        state
            .borrow()
            .as_ref()
            .map(|state| state.formula_signature)
            .unwrap_or([0; SIG_SIZE_BYTES])
    });
    let mut signer = SipHash::siphash_init(&SECRET_KEY);
    signer.siphash_reset();
    signer.siphash_update(&id.to_ne_bytes(), std::mem::size_of::<u64>() as u64);
    let lit_bytes = ints_to_bytes(&lits[..nb_lits as usize]);
    signer.siphash_update(&lit_bytes, (nb_lits as usize * std::mem::size_of::<i32>()) as u64);
    signer.siphash_update(&formula_signature, SIG_SIZE_BYTES as u64);
    let digest = signer.siphash_digest();
    trusted_utils_copy_bytes(out, &digest, SIG_SIZE_BYTES as u64);
}

pub fn top_check_validate_unsat(out_signature_or_null: &mut Option<Vec<u8>>) -> bool {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        let state = state.as_mut().unwrap();
        state.valid &= lrat_check_validate_unsat();
        if !state.valid {
            return false;
        }
        if out_signature_or_null.is_some() {
            let mut out = vec![0u8; SIG_SIZE_BYTES];
            confirm_result(&state.formula_signature, 20, &mut out);
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
    let ok = lrat_check_add_clause(id, literals, nb_literals, hints, nb_hints);
    let produced_sig = if out_sig_or_null.is_some() {
        let mut sig = vec![0u8; SIG_SIZE_BYTES];
        compute_clause_signature(id, literals, nb_literals, &mut sig);
        Some(sig)
    } else {
        None
    };
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        let state = state.as_mut().unwrap();
        state.valid &= ok;
        if !state.valid {
            return false;
        }
        if let Some(sig) = produced_sig {
            *out_sig_or_null = Some(sig);
        }
        true
    })
}

fn ints_to_bytes(values: &[i32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(values.len() * std::mem::size_of::<i32>());
    for value in values {
        bytes.extend_from_slice(&value.to_ne_bytes());
    }
    bytes
}
