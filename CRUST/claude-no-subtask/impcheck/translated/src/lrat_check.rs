use std::cell::RefCell;
use std::collections::HashMap;

use crate::secret::SECRET_KEY;
use crate::siphash::SipHash;

// Module-internal mutable state. The C version uses globals; we keep the
// data behind thread-locals so the Rust API remains free-function based.
thread_local! {
    static STATE: RefCell<LratState> = RefCell::new(LratState::new());
    pub(crate) static SIPHASH: RefCell<Option<SipHash>> = const { RefCell::new(None) };
}

pub(crate) struct LratState {
    // clauses keyed by id; each clause is a list of literals (no terminating 0).
    pub clauses: HashMap<u64, Vec<i32>>,
    pub var_values: Vec<i8>,
    pub assigned_units: Vec<i32>,
    pub clause_to_add: Vec<i32>,
    pub check_model: bool,
    pub lenient: bool,
    pub id_to_add: u64,
    pub nb_loaded_clauses: u64,
    pub done_loading: bool,
    pub unsat_proven: bool,
    pub msg: String,
}

impl LratState {
    fn new() -> Self {
        Self {
            clauses: HashMap::new(),
            var_values: Vec::new(),
            assigned_units: Vec::new(),
            clause_to_add: Vec::new(),
            check_model: false,
            lenient: false,
            id_to_add: 1,
            nb_loaded_clauses: 0,
            done_loading: false,
            unsat_proven: false,
            msg: String::new(),
        }
    }
}

pub(crate) fn with_state<R>(f: impl FnOnce(&mut LratState) -> R) -> R {
    STATE.with(|s| f(&mut s.borrow_mut()))
}

pub(crate) fn with_siphash<R>(f: impl FnOnce(&mut SipHash) -> R) -> R {
    SIPHASH.with(|s| {
        let mut b = s.borrow_mut();
        if b.is_none() {
            *b = Some(SipHash::siphash_init(&SECRET_KEY));
        }
        f(b.as_mut().unwrap())
    })
}

pub fn reset_assignments() {
    with_state(|st| {
        for &v in st.assigned_units.iter() {
            if (v as usize) < st.var_values.len() {
                st.var_values[v as usize] = 0;
            }
        }
        st.assigned_units.clear();
    });
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
    with_state(|st| {
        // Try insert
        if st.clauses.contains_key(&id) {
            // already present
            if st.lenient {
                let old = st.clauses.get(&id).unwrap();
                if clauses_equivalent_inner(old, &cls) {
                    return true;
                }
            }
            st.msg = format!(
                "Insertion of clause {} unsuccessful - already present?",
                id
            );
            return false;
        }
        st.clauses.insert(id, cls);
        if nb_lits == 0 {
            st.unsat_proven = true;
        }
        true
    })
}

pub fn check_clause(
    base_id: u64,
    lits: &[i32],
    nb_lits: i32,
    hints: &[u64],
    nb_hints: i32,
) -> bool {
    let result = with_state(|st| -> bool {
        // Assume negation of each literal in the new clause.
        for i in 0..nb_lits as usize {
            let lit = lits[i];
            let var = if lit > 0 { lit } else { -lit } as usize;
            if var >= st.var_values.len() {
                st.var_values.resize(var + 1, 0);
            }
            st.var_values[var] = if lit > 0 { -1 } else { 1 };
            st.assigned_units.push(var as i32);
        }

        let mut ok = true;
        let mut error_msg: Option<String> = None;

        for i in 0..nb_hints as usize {
            let hint_id = hints[i];
            let cls_opt = st.clauses.get(&hint_id).cloned();
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
            let mut hint_ok = true;
            let mut local_ok = true;
            for &lit in cls.iter() {
                if lit == 0 {
                    break;
                }
                let var = if lit > 0 { lit } else { -lit } as usize;
                if var >= st.var_values.len() {
                    st.var_values.resize(var + 1, 0);
                }
                if st.var_values[var] == 0 {
                    if new_unit != 0 {
                        error_msg = Some(format!(
                            "Derivation {}: multiple literals unassigned",
                            base_id
                        ));
                        local_ok = false;
                        hint_ok = false;
                        break;
                    }
                    new_unit = lit;
                    continue;
                }
                let sign = st.var_values[var] > 0;
                if sign == (lit > 0) {
                    error_msg = Some(format!(
                        "Derivation {}: dependency {} is satisfied",
                        base_id, hint_id
                    ));
                    local_ok = false;
                    hint_ok = false;
                    break;
                }
            }
            if !local_ok {
                ok = false;
                break;
            }
            if !hint_ok {
                break;
            }

            if new_unit == 0 {
                // Empty clause derived
                if i + 1 < nb_hints as usize {
                    error_msg = Some(format!(
                        "Derivation {}: empty clause produced at non-final hint {}",
                        base_id, hint_id
                    ));
                    break;
                }
                // Final hint produced empty clause - all OK
                // Reset assignments
                for &v in st.assigned_units.iter() {
                    if (v as usize) < st.var_values.len() {
                        st.var_values[v as usize] = 0;
                    }
                }
                st.assigned_units.clear();
                return true;
            }
            // Insert derived unit
            let var = if new_unit > 0 { new_unit } else { -new_unit } as usize;
            if var >= st.var_values.len() {
                st.var_values.resize(var + 1, 0);
            }
            st.var_values[var] = if new_unit > 0 { 1 } else { -1 };
            st.assigned_units.push(var as i32);
        }

        if let Some(m) = error_msg {
            if st.msg.is_empty() {
                st.msg = m;
            }
        } else if st.msg.is_empty() {
            st.msg = format!("Derivation {}: no empty clause was produced", base_id);
        }

        // Reset assignments
        for &v in st.assigned_units.iter() {
            if (v as usize) < st.var_values.len() {
                st.var_values[v as usize] = 0;
            }
        }
        st.assigned_units.clear();
        let _ = ok;
        false
    });
    result
}

pub fn lrat_check_end_load(out_sig: &mut Option<Vec<u8>>) -> bool {
    let leftover = with_state(|st| !st.clause_to_add.is_empty());
    if leftover {
        with_state(|st| {
            st.msg = "literals left in unterminated clause".to_string();
        });
        return false;
    }
    with_siphash(|sh| {
        sh.siphash_pad(2);
        let sig = sh.siphash_digest();
        *out_sig = Some(sig);
    });
    with_state(|st| {
        st.done_loading = true;
        st.nb_loaded_clauses = st.id_to_add - 1;
    });
    true
}

pub fn lrat_check_delete_clause(ids: &[u64], nb_ids: i32) -> bool {
    with_state(|st| {
        for i in 0..nb_ids as usize {
            let id = ids[i];
            if !st.clauses.contains_key(&id) {
                st.msg = format!("Clause deletion: ID {} not found", id);
                return false;
            }
            if st.check_model && id <= st.nb_loaded_clauses {
                continue;
            }
            st.clauses.remove(&id);
        }
        true
    })
}

pub fn clauses_equivalent(left_cls: &[i32], right_cls: &[i32]) -> bool {
    clauses_equivalent_inner(left_cls, right_cls)
}

fn clauses_equivalent_inner(left_cls: &[i32], right_cls: &[i32]) -> bool {
    // Find length of left up to terminating 0 (or full length)
    let left_size = left_cls.iter().position(|&x| x == 0).unwrap_or(left_cls.len());
    let right_size = right_cls
        .iter()
        .position(|&x| x == 0)
        .unwrap_or(right_cls.len());
    for &l in &left_cls[..left_size] {
        if !right_cls[..right_size].contains(&l) {
            return false;
        }
    }
    left_size == right_size
}

pub fn lrat_check_validate_sat(model: &[i32], size: u64) -> bool {
    with_state(|st| {
        if !st.done_loading {
            st.msg =
                "SAT validation illegal - loading formula was not concluded".to_string();
            return false;
        }
        if !st.check_model {
            st.msg =
                "SAT validation illegal - not executed to explicitly support this".to_string();
            return false;
        }
        // Need a mutable copy of model for the auto-assign behavior
        let mut m = model.to_vec();
        for id in 1..=st.nb_loaded_clauses {
            let cls_opt = st.clauses.get(&id).cloned();
            let cls = match cls_opt {
                Some(c) => c,
                None => {
                    st.msg = format!("SAT validation: original ID {} not found", id);
                    return false;
                }
            };
            let mut satisfied = false;
            for &lit in cls.iter() {
                if lit == 0 {
                    break;
                }
                let var = if lit > 0 { lit } else { -lit };
                if (var - 1) as u64 >= size {
                    st.msg = format!(
                        "SAT validation: model does not cover variable {}",
                        var
                    );
                    return false;
                }
                let mut model_lit = m[(var - 1) as usize];
                if model_lit != var && model_lit != -var && model_lit != 0 {
                    st.msg = format!(
                        "SAT validation: unexpected literal {} in assignment of variable {}",
                        model_lit, var
                    );
                    return false;
                }
                if model_lit == 0 {
                    m[(var - 1) as usize] = lit;
                    model_lit = lit;
                }
                if model_lit == lit {
                    satisfied = true;
                    break;
                }
            }
            if !satisfied {
                st.msg = format!("SAT validation: original clause {} not satisfied", id);
                return false;
            }
        }
        true
    })
}

pub fn lrat_check_load(lit: i32) -> bool {
    if lit == 0 {
        // Push the assembled clause
        let (id, mut buf) = with_state(|st| {
            let id = st.id_to_add;
            let buf = std::mem::take(&mut st.clause_to_add);
            (id, buf)
        });
        let nb_lits = buf.len() as i32;
        if !lrat_check_add_axiomatic_clause(id, &buf, nb_lits) {
            // Restore buffer state
            with_state(|st| st.clause_to_add = buf);
            return false;
        }
        with_state(|st| st.id_to_add += 1);
        // Append terminating 0 then update siphash with the byte data
        buf.push(0);
        with_siphash(|sh| {
            // Convert buf to bytes
            let mut bytes = Vec::with_capacity(buf.len() * 4);
            for &v in &buf {
                bytes.extend_from_slice(&v.to_ne_bytes());
            }
            sh.siphash_update(&bytes, bytes.len() as u64);
        });
        with_state(|st| st.clause_to_add.clear());
        true
    } else {
        with_state(|st| st.clause_to_add.push(lit));
        true
    }
}

pub fn lrat_check_init(nb_vars: i32, opt_check_model: bool, opt_lenient: bool) {
    with_state(|st| {
        st.clauses.clear();
        st.var_values = vec![0i8; (nb_vars + 1) as usize];
        st.assigned_units.clear();
        st.clause_to_add.clear();
        st.check_model = opt_check_model;
        st.lenient = opt_lenient;
        st.id_to_add = 1;
        st.nb_loaded_clauses = 0;
        st.done_loading = false;
        st.unsat_proven = false;
        st.msg.clear();
    });
}

pub fn clause_init(data: &[i32], nb_lits: i32) -> Vec<i32> {
    let mut v = Vec::with_capacity((nb_lits + 1) as usize);
    for i in 0..nb_lits as usize {
        v.push(data[i]);
    }
    v.push(0);
    v
}

pub fn lrat_check_validate_unsat() -> bool {
    with_state(|st| {
        if !st.done_loading {
            st.msg =
                "UNSAT validation illegal - loading formula was not concluded".to_string();
            return false;
        }
        if !st.unsat_proven {
            st.msg =
                "UNSAT validation unsuccessful - did not derive or import empty clause"
                    .to_string();
            return false;
        }
        true
    })
}
