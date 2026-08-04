use crate::confirm::confirm_result;
use crate::lrat_check;
use crate::siphash::SipHash;
use crate::trusted_utils::{trusted_utils_copy_bytes, trusted_utils_equal_signatures, SIG_SIZE_BYTES};
use std::sync::Mutex;

struct TopState {
    formula_signature: [u8; SIG_SIZE_BYTES],
    valid: bool,
    msgstr: String,
    parsed_formula: bool,
    siphash: SipHash,
}

impl TopState {
    fn new() -> Self {
        let key: [u8; 16] = [
            86, 93, 1, 209, 112, 176, 13, 40, 168, 223, 25, 22, 134, 58, 21, 211,
        ];
        TopState {
            formula_signature: [0u8; SIG_SIZE_BYTES],
            valid: true,
            msgstr: String::new(),
            parsed_formula: false,
            siphash: SipHash::siphash_init(&key),
        }
    }
}

fn state() -> &'static Mutex<TopState> {
    use std::sync::OnceLock;
    static S: OnceLock<Mutex<TopState>> = OnceLock::new();
    S.get_or_init(|| Mutex::new(TopState::new()))
}

pub fn top_check_init(nb_vars: i32, check_model: bool, lenient: bool) {
    {
        let mut s = state().lock().unwrap();
        *s = TopState::new();
    }
    lrat_check::lrat_check_init(nb_vars, check_model, lenient);
}

pub fn top_check_commit_formula_sig(f_sig: &[u8]) {
    let mut s = state().lock().unwrap();
    let len = std::cmp::min(f_sig.len(), SIG_SIZE_BYTES);
    for i in 0..len {
        s.formula_signature[i] = f_sig[i];
    }
}

pub fn top_check_validate_sat(
    model: &[i32],
    size: u64,
    out_signature_or_null: &mut Option<Vec<u8>>,
) -> bool {
    let v = lrat_check::lrat_check_validate_sat(model, size);
    {
        let mut s = state().lock().unwrap();
        s.valid = s.valid && v;
        if !s.valid {
            return false;
        }
    }
    if let Some(out) = out_signature_or_null.as_mut() {
        let s = state().lock().unwrap();
        if out.len() < SIG_SIZE_BYTES {
            out.resize(SIG_SIZE_BYTES, 0);
        }
        confirm_result(&s.formula_signature, 10, out);
    }
    true
}

pub fn top_check_delete(ids: &[u64], nb_ids: i32) -> bool {
    lrat_check::lrat_check_delete_clause(ids, nb_ids)
}

pub fn top_check_end_load() -> bool {
    let mut sig_opt: Option<Vec<u8>> = None;
    let r = lrat_check::lrat_check_end_load(&mut sig_opt);
    let mut s = state().lock().unwrap();
    s.valid = s.valid && r;
    if !s.valid {
        return false;
    }
    let sig = match sig_opt {
        Some(v) => v,
        None => return false,
    };
    s.valid = trusted_utils_equal_signatures(&sig, &s.formula_signature);
    if !s.valid {
        s.msgstr = "Formula signature check failed".to_string();
    }
    s.valid
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
        let mut s = state().lock().unwrap();
        s.valid = false;
        s.msgstr = format!("Signature check of clause {} failed", id);
        return false;
    }
    let r = lrat_check::lrat_check_add_axiomatic_clause(id, literals, nb_literals);
    let mut s = state().lock().unwrap();
    s.valid = s.valid && r;
    s.valid
}

pub fn top_check_valid() -> bool {
    state().lock().unwrap().valid
}

pub fn top_check_load(lit: i32) {
    let r = lrat_check::lrat_check_load(lit);
    let mut s = state().lock().unwrap();
    s.valid = s.valid && r;
}

pub fn compute_clause_signature(id: u64, lits: &[i32], nb_lits: i32, out: &mut [u8]) {
    let mut s = state().lock().unwrap();
    s.siphash.siphash_reset();
    let id_bytes = id.to_le_bytes();
    s.siphash.siphash_update(&id_bytes, 8);
    let mut lit_bytes = Vec::with_capacity((nb_lits as usize) * 4);
    for i in 0..(nb_lits as usize) {
        lit_bytes.extend_from_slice(&lits[i].to_le_bytes());
    }
    s.siphash
        .siphash_update(&lit_bytes, lit_bytes.len() as u64);
    let formula_sig = s.formula_signature;
    s.siphash
        .siphash_update(&formula_sig, SIG_SIZE_BYTES as u64);
    let hash_out = s.siphash.siphash_digest();
    trusted_utils_copy_bytes(out, &hash_out, SIG_SIZE_BYTES as u64);
}

pub fn top_check_validate_unsat(out_signature_or_null: &mut Option<Vec<u8>>) -> bool {
    let r = lrat_check::lrat_check_validate_unsat();
    {
        let mut s = state().lock().unwrap();
        s.valid = s.valid && r;
        if !s.valid {
            return false;
        }
    }
    if let Some(out) = out_signature_or_null.as_mut() {
        let s = state().lock().unwrap();
        if out.len() < SIG_SIZE_BYTES {
            out.resize(SIG_SIZE_BYTES, 0);
        }
        confirm_result(&s.formula_signature, 20, out);
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
    let r = lrat_check::lrat_check_add_clause(id, literals, nb_literals, hints, nb_hints);
    {
        let mut s = state().lock().unwrap();
        s.valid = s.valid && r;
        if !s.valid {
            return false;
        }
    }
    if let Some(out) = out_sig_or_null.as_mut() {
        if out.len() < SIG_SIZE_BYTES {
            out.resize(SIG_SIZE_BYTES, 0);
        }
        compute_clause_signature(id, literals, nb_literals, out);
    }
    true
}
