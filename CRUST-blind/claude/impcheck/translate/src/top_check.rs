use std::sync::Mutex;

const SIG_SIZE_BYTES: usize = 16;

const SECRET_KEY: [u8; 16] = [
    86, 93, 1, 209, 112, 176, 13, 40, 168, 223, 25, 22, 134, 58, 21, 211,
];

#[inline]
fn rotl(x: u64, b: u32) -> u64 {
    (x << b) | (x >> (64 - b))
}

fn sipround(v0: &mut u64, v1: &mut u64, v2: &mut u64, v3: &mut u64) {
    *v0 = v0.wrapping_add(*v1);
    *v1 = rotl(*v1, 13);
    *v1 ^= *v0;
    *v0 = rotl(*v0, 32);
    *v2 = v2.wrapping_add(*v3);
    *v3 = rotl(*v3, 16);
    *v3 ^= *v2;
    *v0 = v0.wrapping_add(*v3);
    *v3 = rotl(*v3, 21);
    *v3 ^= *v0;
    *v2 = v2.wrapping_add(*v1);
    *v1 = rotl(*v1, 17);
    *v1 ^= *v2;
    *v2 = rotl(*v2, 32);
}

fn u8to64_le(p: &[u8]) -> u64 {
    let mut buf = [0u8; 8];
    let n = p.len().min(8);
    buf[..n].copy_from_slice(&p[..n]);
    u64::from_le_bytes(buf)
}

fn siphash_full(data: &[u8]) -> [u8; 16] {
    let k0 = u8to64_le(&SECRET_KEY[0..8]);
    let k1 = u8to64_le(&SECRET_KEY[8..16]);
    let mut v0 = 0x736f6d6570736575u64 ^ k0;
    let mut v1 = (0x646f72616e646f6du64 ^ k1) ^ 0xee;
    let mut v2 = 0x6c7967656e657261u64 ^ k0;
    let mut v3 = 0x7465646279746573u64 ^ k1;

    let inlen = data.len() as u64;
    let mut idx = 0usize;
    while idx + 8 <= data.len() {
        let m = u8to64_le(&data[idx..idx + 8]);
        v3 ^= m;
        for _ in 0..2 {
            sipround(&mut v0, &mut v1, &mut v2, &mut v3);
        }
        v0 ^= m;
        idx += 8;
    }
    let left = data.len() - idx;
    let mut b: u64 = inlen << 56;
    for i in 0..left {
        b |= (data[idx + i] as u64) << (8 * i);
    }

    v3 ^= b;
    for _ in 0..2 {
        sipround(&mut v0, &mut v1, &mut v2, &mut v3);
    }
    v0 ^= b;
    v2 ^= 0xee;
    for _ in 0..4 {
        sipround(&mut v0, &mut v1, &mut v2, &mut v3);
    }
    let mut out = [0u8; 16];
    let bb = v0 ^ v1 ^ v2 ^ v3;
    out[0..8].copy_from_slice(&bb.to_le_bytes());
    v1 ^= 0xdd;
    for _ in 0..4 {
        sipround(&mut v0, &mut v1, &mut v2, &mut v3);
    }
    let bb = v0 ^ v1 ^ v2 ^ v3;
    out[8..16].copy_from_slice(&bb.to_le_bytes());
    out
}

struct TopState {
    formula_signature: [u8; SIG_SIZE_BYTES],
    valid: bool,
}

fn state() -> &'static Mutex<TopState> {
    use std::sync::OnceLock;
    static STATE: OnceLock<Mutex<TopState>> = OnceLock::new();
    STATE.get_or_init(|| {
        Mutex::new(TopState {
            formula_signature: [0u8; SIG_SIZE_BYTES],
            valid: true,
        })
    })
}

pub fn top_check_init(nb_vars: i32, check_model: bool, lenient: bool) {
    {
        let mut s = state().lock().unwrap();
        s.formula_signature = [0u8; SIG_SIZE_BYTES];
        s.valid = true;
    }
    crate::lrat_check::lrat_check_init(nb_vars, check_model, lenient);
}

pub fn top_check_commit_formula_sig(f_sig: &[u8]) {
    let mut s = state().lock().unwrap();
    let n = SIG_SIZE_BYTES.min(f_sig.len());
    s.formula_signature[..n].copy_from_slice(&f_sig[..n]);
}

pub fn top_check_validate_sat(
    model: &[i32],
    size: u64,
    out_signature_or_null: &mut Option<Vec<u8>>,
) -> bool {
    let mut valid = state().lock().unwrap().valid;
    valid = valid && crate::lrat_check::lrat_check_validate_sat(model, size);
    state().lock().unwrap().valid = valid;
    if !valid {
        return false;
    }
    if out_signature_or_null.is_some() {
        let f_sig = state().lock().unwrap().formula_signature;
        let mut out = vec![0u8; SIG_SIZE_BYTES];
        crate::confirm::confirm_result(&f_sig, 10, &mut out);
        *out_signature_or_null = Some(out);
    }
    true
}

pub fn top_check_delete(ids: &[u64], nb_ids: i32) -> bool {
    crate::lrat_check::lrat_check_delete_clause(ids, nb_ids)
}

pub fn top_check_end_load() -> bool {
    let mut sig: Option<Vec<u8>> = None;
    let mut valid = state().lock().unwrap().valid;
    valid = valid && crate::lrat_check::lrat_check_end_load(&mut sig);
    if !valid {
        state().lock().unwrap().valid = valid;
        return false;
    }
    let computed = sig.unwrap_or_else(|| vec![0u8; SIG_SIZE_BYTES]);
    let s = state().lock().unwrap();
    let equal = computed.len() >= SIG_SIZE_BYTES
        && (0..SIG_SIZE_BYTES).all(|i| computed[i] == s.formula_signature[i]);
    drop(s);
    state().lock().unwrap().valid = equal;
    equal
}

pub fn top_check_import(
    id: u64,
    literals: &[i32],
    nb_literals: i32,
    signature_data: &[u8],
) -> bool {
    let mut computed = [0u8; SIG_SIZE_BYTES];
    compute_clause_signature(id, literals, nb_literals, &mut computed);
    if signature_data.len() < SIG_SIZE_BYTES
        || (0..SIG_SIZE_BYTES).any(|i| signature_data[i] != computed[i])
    {
        state().lock().unwrap().valid = false;
        return false;
    }
    let mut valid = state().lock().unwrap().valid;
    valid = valid && crate::lrat_check::lrat_check_add_axiomatic_clause(id, literals, nb_literals);
    state().lock().unwrap().valid = valid;
    valid
}

pub fn top_check_valid() -> bool {
    state().lock().unwrap().valid
}

pub fn top_check_load(lit: i32) {
    let ok = crate::lrat_check::lrat_check_load(lit);
    let mut s = state().lock().unwrap();
    s.valid = s.valid && ok;
}

pub fn compute_clause_signature(id: u64, lits: &[i32], nb_lits: i32, out: &mut [u8]) {
    let s = state().lock().unwrap();
    let f_sig = s.formula_signature;
    drop(s);
    let mut data = Vec::new();
    data.extend_from_slice(&id.to_le_bytes());
    let n = nb_lits as usize;
    for i in 0..n {
        data.extend_from_slice(&lits[i].to_le_bytes());
    }
    data.extend_from_slice(&f_sig);
    let sig = siphash_full(&data);
    let n = SIG_SIZE_BYTES.min(out.len());
    out[..n].copy_from_slice(&sig[..n]);
}

pub fn top_check_validate_unsat(out_signature_or_null: &mut Option<Vec<u8>>) -> bool {
    let mut valid = state().lock().unwrap().valid;
    valid = valid && crate::lrat_check::lrat_check_validate_unsat();
    state().lock().unwrap().valid = valid;
    if !valid {
        return false;
    }
    if out_signature_or_null.is_some() {
        let f_sig = state().lock().unwrap().formula_signature;
        let mut out = vec![0u8; SIG_SIZE_BYTES];
        crate::confirm::confirm_result(&f_sig, 20, &mut out);
        *out_signature_or_null = Some(out);
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
    let mut valid = state().lock().unwrap().valid;
    valid = valid
        && crate::lrat_check::lrat_check_add_clause(id, literals, nb_literals, hints, nb_hints);
    state().lock().unwrap().valid = valid;
    if !valid {
        return false;
    }
    if out_sig_or_null.is_some() {
        let mut out = vec![0u8; SIG_SIZE_BYTES];
        compute_clause_signature(id, literals, nb_literals, &mut out);
        *out_sig_or_null = Some(out);
    }
    true
}
