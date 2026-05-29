use crate::trusted_utils;
use std::collections::HashMap;
use std::sync::Mutex;

#[allow(dead_code)]
struct LratState {
    clause_table: HashMap<u64, Vec<i32>>,
    var_values: Vec<i8>,
    assigned_units: Vec<i32>,
    check_model: bool,
    lenient: bool,
    id_to_add: u64,
    nb_loaded_clauses: u64,
    clause_to_add: Vec<i32>,
    done_loading: bool,
    unsat_proven: bool,
    msg: String,
    // Running siphash state for formula signature
    sip_v0: u64,
    sip_v1: u64,
    sip_v2: u64,
    sip_v3: u64,
    sip_buf: [u8; 8],
    sip_buflen: usize,
    sip_inlen: u64,
}

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

impl LratState {
    fn new() -> Self {
        let k0 = u8to64_le(&SECRET_KEY[0..8]);
        let k1 = u8to64_le(&SECRET_KEY[8..16]);
        let v0 = 0x736f6d6570736575u64 ^ k0;
        let v1 = (0x646f72616e646f6du64 ^ k1) ^ 0xee;
        let v2 = 0x6c7967656e657261u64 ^ k0;
        let v3 = 0x7465646279746573u64 ^ k1;
        LratState {
            clause_table: HashMap::new(),
            var_values: Vec::new(),
            assigned_units: Vec::new(),
            check_model: false,
            lenient: false,
            id_to_add: 1,
            nb_loaded_clauses: 0,
            clause_to_add: Vec::new(),
            done_loading: false,
            unsat_proven: false,
            msg: String::new(),
            sip_v0: v0,
            sip_v1: v1,
            sip_v2: v2,
            sip_v3: v3,
            sip_buf: [0u8; 8],
            sip_buflen: 0,
            sip_inlen: 0,
        }
    }

    fn sip_update(&mut self, data: &[u8]) {
        let n = data.len();
        let mut datapos = 0usize;
        loop {
            while self.sip_buflen < 8 && datapos < n {
                self.sip_buf[self.sip_buflen] = data[datapos];
                self.sip_buflen += 1;
                datapos += 1;
            }
            if self.sip_buflen < 8 {
                break;
            }
            let m = u8to64_le(&self.sip_buf);
            self.sip_v3 ^= m;
            for _ in 0..2 {
                sipround(
                    &mut self.sip_v0,
                    &mut self.sip_v1,
                    &mut self.sip_v2,
                    &mut self.sip_v3,
                );
            }
            self.sip_v0 ^= m;
            self.sip_buflen = 0;
        }
        self.sip_inlen = self.sip_inlen.wrapping_add(n as u64);
    }

    fn sip_pad(&mut self, n: u64) {
        let z = [0u8];
        for _ in 0..n {
            self.sip_update(&z);
        }
    }

    fn sip_digest(&self) -> Vec<u8> {
        let mut v0 = self.sip_v0;
        let mut v1 = self.sip_v1;
        let mut v2 = self.sip_v2;
        let mut v3 = self.sip_v3;
        let inlen = self.sip_inlen;
        let left = self.sip_buflen;
        let mut b: u64 = inlen << 56;
        for i in 0..left {
            b |= (self.sip_buf[i] as u64) << (8 * i);
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
        let mut out = vec![0u8; 16];
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
}

fn state() -> &'static Mutex<LratState> {
    use std::sync::OnceLock;
    static STATE: OnceLock<Mutex<LratState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(LratState::new()))
}

pub fn reset_assignments() {
    let mut s = state().lock().unwrap();
    let units: Vec<i32> = s.assigned_units.clone();
    for v in units {
        let idx = v as usize;
        if idx < s.var_values.len() {
            s.var_values[idx] = 0;
        }
    }
    s.assigned_units.clear();
}

pub fn lrat_check_add_clause(
    id: u64,
    lits: &[i32],
    nb_lits: i32,
    hints: &[u64],
    nb_hints: i32,
) -> bool {
    if !check_clause(id, lits, nb_lits, hints, nb_hints) {
        return false;
    }
    lrat_check_add_axiomatic_clause(id, lits, nb_lits)
}

pub fn lrat_check_add_axiomatic_clause(id: u64, lits: &[i32], nb_lits: i32) -> bool {
    let cls = clause_init(lits, nb_lits);
    let mut s = state().lock().unwrap();
    if s.clause_table.contains_key(&id) {
        if s.lenient {
            let old = s.clause_table.get(&id).cloned();
            if let Some(o) = old {
                if clauses_equivalent(&o, &cls) {
                    return true;
                }
            }
        }
        s.msg = format!("Insertion of clause {} unsuccessful - already present?", id);
        return false;
    }
    s.clause_table.insert(id, cls);
    if nb_lits == 0 {
        s.unsat_proven = true;
    }
    true
}

pub fn check_clause(
    base_id: u64,
    lits: &[i32],
    nb_lits: i32,
    hints: &[u64],
    nb_hints: i32,
) -> bool {
    let n_lits = nb_lits as usize;
    let n_hints = nb_hints as usize;
    {
        let mut s = state().lock().unwrap();
        // Reserve assigned_units capacity
        s.assigned_units.reserve(n_lits + n_hints);
        // Assume the negation of each literal in the new clause
        for i in 0..n_lits {
            let lit = lits[i];
            let var = if lit > 0 { lit } else { -lit };
            let v = var as usize;
            if v < s.var_values.len() {
                s.var_values[v] = if lit > 0 { -1 } else { 1 };
            }
            s.assigned_units.push(var);
        }
    }

    let mut ok = true;
    let mut error_msg: Option<String> = None;

    for i in 0..n_hints {
        let hint_id = hints[i];
        let cls_opt = {
            let s = state().lock().unwrap();
            s.clause_table.get(&hint_id).cloned()
        };
        let cls = match cls_opt {
            Some(c) => c,
            None => {
                error_msg = Some(format!(
                    "Derivation {}: hint {} not found",
                    base_id, hint_id
                ));
                break;
            }
        };

        let mut new_unit: i32 = 0;
        for &lit in cls.iter() {
            if lit == 0 {
                break;
            }
            let var = if lit > 0 { lit } else { -lit };
            let v_assignment = {
                let s = state().lock().unwrap();
                let vu = var as usize;
                if vu < s.var_values.len() {
                    s.var_values[vu]
                } else {
                    0
                }
            };
            if v_assignment == 0 {
                if new_unit != 0 {
                    error_msg = Some(format!(
                        "Derivation {}: multiple literals unassigned",
                        base_id
                    ));
                    ok = false;
                    break;
                }
                new_unit = lit;
                continue;
            }
            let sign = v_assignment > 0;
            if sign == (lit > 0) {
                error_msg = Some(format!(
                    "Derivation {}: dependency {} is satisfied",
                    base_id, hint_id
                ));
                ok = false;
                break;
            }
        }
        if !ok {
            break;
        }
        if new_unit == 0 {
            // Empty clause derived
            if i + 1 < n_hints {
                error_msg = Some(format!(
                    "Derivation {}: empty clause produced at non-final hint {}",
                    base_id, hint_id
                ));
                break;
            }
            reset_assignments();
            return true;
        }
        // Insert new derived unit
        let var = if new_unit > 0 { new_unit } else { -new_unit };
        let mut s = state().lock().unwrap();
        let vu = var as usize;
        if vu < s.var_values.len() {
            s.var_values[vu] = if new_unit > 0 { 1 } else { -1 };
        }
        s.assigned_units.push(var);
    }

    {
        let mut s = state().lock().unwrap();
        let msg_empty = s.msg.is_empty();
        if let Some(m) = error_msg {
            s.msg = m;
        } else if msg_empty {
            s.msg = format!("Derivation {}: no empty clause was produced", base_id);
        }
    }
    reset_assignments();
    false
}

pub fn lrat_check_end_load(out_sig: &mut Option<Vec<u8>>) -> bool {
    let mut s = state().lock().unwrap();
    if !s.clause_to_add.is_empty() {
        s.msg = "literals left in unterminated clause".to_string();
        return false;
    }
    s.sip_pad(2);
    let sig = s.sip_digest();
    *out_sig = Some(sig);
    s.done_loading = true;
    s.nb_loaded_clauses = s.id_to_add - 1;
    true
}

pub fn lrat_check_delete_clause(ids: &[u64], nb_ids: i32) -> bool {
    let n = nb_ids as usize;
    let mut s = state().lock().unwrap();
    for i in 0..n {
        let id = ids[i];
        if !s.clause_table.contains_key(&id) {
            s.msg = format!("Clause deletion: ID {} not found", id);
            return false;
        }
        if s.check_model && id <= s.nb_loaded_clauses {
            continue;
        }
        s.clause_table.remove(&id);
    }
    true
}

pub fn clauses_equivalent(left_cls: &[i32], right_cls: &[i32]) -> bool {
    let mut left_size = 0;
    for &left_lit in left_cls.iter() {
        if left_lit == 0 {
            break;
        }
        left_size += 1;
        let mut found = false;
        for &right_lit in right_cls.iter() {
            if right_lit == 0 {
                break;
            }
            if right_lit == left_lit {
                found = true;
                break;
            }
        }
        if !found {
            return false;
        }
    }
    let mut right_size = 0;
    for &right_lit in right_cls.iter() {
        if right_lit == 0 {
            break;
        }
        right_size += 1;
    }
    left_size == right_size
}

pub fn lrat_check_validate_sat(model: &[i32], size: u64) -> bool {
    let mut s = state().lock().unwrap();
    if !s.done_loading {
        s.msg = "SAT validation illegal - loading formula was not concluded".to_string();
        return false;
    }
    if !s.check_model {
        s.msg = "SAT validation illegal - not executed to explicitly support this".to_string();
        return false;
    }
    let mut model_local: Vec<i32> = model.to_vec();
    for id in 1..=s.nb_loaded_clauses {
        let cls = match s.clause_table.get(&id) {
            Some(c) => c.clone(),
            None => {
                s.msg = format!("SAT validation: original ID {} not found", id);
                return false;
            }
        };
        let mut satisfied = false;
        for &lit in cls.iter() {
            if lit == 0 {
                break;
            }
            let var = if lit > 0 { lit } else { -lit };
            let var_idx = (var - 1) as i64;
            if var_idx < 0 || (var_idx as u64) >= size {
                s.msg = format!(
                    "SAT validation: model does not cover variable {}",
                    var
                );
                return false;
            }
            let mut model_lit = model_local[var_idx as usize];
            if model_lit != var && model_lit != -var && model_lit != 0 {
                s.msg = format!(
                    "SAT validation: unexpected literal {} in assignment of variable {}",
                    model_lit, var
                );
                return false;
            }
            if model_lit == 0 {
                model_lit = lit;
                model_local[var_idx as usize] = lit;
            }
            if model_lit == lit {
                satisfied = true;
                break;
            }
        }
        if !satisfied {
            s.msg = format!("SAT validation: original clause {} not satisfied", id);
            return false;
        }
    }
    true
}

pub fn lrat_check_load(lit: i32) -> bool {
    if lit == 0 {
        let (id, lits) = {
            let s = state().lock().unwrap();
            (s.id_to_add, s.clause_to_add.clone())
        };
        if !lrat_check_add_axiomatic_clause(id, &lits, lits.len() as i32) {
            return false;
        }
        let mut s = state().lock().unwrap();
        s.id_to_add += 1;
        s.clause_to_add.push(0);
        // Update siphash with little-endian bytes of clause_to_add ints
        let bytes: Vec<u8> = s
            .clause_to_add
            .iter()
            .flat_map(|x| x.to_le_bytes().to_vec())
            .collect();
        s.sip_update(&bytes);
        s.clause_to_add.clear();
        return true;
    }
    let mut s = state().lock().unwrap();
    s.clause_to_add.push(lit);
    true
}

pub fn lrat_check_init(nb_vars: i32, opt_check_model: bool, opt_lenient: bool) {
    let mut s = state().lock().unwrap();
    *s = LratState::new();
    s.var_values = vec![0i8; (nb_vars + 1) as usize];
    s.check_model = opt_check_model;
    s.lenient = opt_lenient;
}

pub fn clause_init(data: &[i32], nb_lits: i32) -> Vec<i32> {
    let n = nb_lits as usize;
    let mut cls = Vec::with_capacity(n + 1);
    for i in 0..n {
        cls.push(data[i]);
    }
    cls.push(0);
    cls
}

pub fn lrat_check_validate_unsat() -> bool {
    let mut s = state().lock().unwrap();
    if !s.done_loading {
        s.msg = "UNSAT validation illegal - loading formula was not concluded".to_string();
        return false;
    }
    if !s.unsat_proven {
        s.msg =
            "UNSAT validation unsuccessful - did not derive or import empty clause".to_string();
        return false;
    }
    true
}

#[allow(dead_code)]
fn touch_unused_imports() {
    let _ = trusted_utils::SIG_SIZE_BYTES;
}
