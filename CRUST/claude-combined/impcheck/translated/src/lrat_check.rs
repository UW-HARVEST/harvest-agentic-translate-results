use crate::trusted_utils;

use std::cell::RefCell;
use std::collections::HashMap;

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
}

thread_local! {
    static STATE: RefCell<Option<LratState>> = RefCell::new(None);
}

fn with_state<R>(f: impl FnOnce(&mut LratState) -> R) -> R {
    STATE.with(|s| {
        let mut state = s.borrow_mut();
        f(state.as_mut().expect("lrat_check not initialized"))
    })
}

pub fn reset_assignments() {
    with_state(|st| {
        for &v in st.assigned_units.iter() {
            st.var_values[v as usize] = 0;
        }
        st.assigned_units.clear();
    })
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
        if st.clause_table.contains_key(&id) {
            if st.lenient {
                let old_cls = st.clause_table.get(&id).unwrap().clone();
                if clauses_equivalent(&old_cls, &cls) {
                    return true;
                }
            }
            return false;
        }
        let is_empty = nb_lits == 0;
        st.clause_table.insert(id, cls);
        if is_empty {
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
    with_state(|st| {
        st.assigned_units.reserve((nb_lits + nb_hints) as usize);
        for i in 0..nb_lits as usize {
            let lit = lits[i];
            let var = if lit > 0 { lit } else { -lit };
            st.var_values[var as usize] = if lit > 0 { -1 } else { 1 };
            st.assigned_units.push(var);
        }

        let mut ok = true;
        let mut empty_clause_derived = false;
        for i in 0..nb_hints as usize {
            let hint_id = hints[i];
            let cls = match st.clause_table.get(&hint_id) {
                Some(c) => c.clone(),
                None => {
                    ok = false;
                    break;
                }
            };

            let mut new_unit = 0;
            let mut bad = false;
            for &lit in cls.iter() {
                if lit == 0 {
                    break;
                }
                let var = if lit > 0 { lit } else { -lit };
                if st.var_values[var as usize] == 0 {
                    if new_unit != 0 {
                        ok = false;
                        bad = true;
                        break;
                    }
                    new_unit = lit;
                    continue;
                }
                let sign = st.var_values[var as usize] > 0;
                if sign == (lit > 0) {
                    ok = false;
                    bad = true;
                    break;
                }
            }
            if bad || !ok {
                break;
            }

            if new_unit == 0 {
                if i + 1 < nb_hints as usize {
                    ok = false;
                    break;
                }
                empty_clause_derived = true;
                break;
            }
            let var = if new_unit > 0 { new_unit } else { -new_unit };
            st.var_values[var as usize] = if new_unit > 0 { 1 } else { -1 };
            st.assigned_units.push(var);
        }

        for &v in st.assigned_units.iter() {
            st.var_values[v as usize] = 0;
        }
        st.assigned_units.clear();

        let _ = base_id;
        ok && empty_clause_derived
    })
}

pub fn lrat_check_end_load(out_sig: &mut Option<Vec<u8>>) -> bool {
    let cta_empty = with_state(|st| st.clause_to_add.is_empty());
    if !cta_empty {
        return false;
    }
    crate::siphash_global::with_global_siphash(|sh| {
        sh.siphash_pad(2);
        *out_sig = Some(sh.siphash_digest());
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
            if !st.clause_table.contains_key(&id) {
                return false;
            }
            if st.check_model && id <= st.nb_loaded_clauses {
                continue;
            }
            st.clause_table.remove(&id);
        }
        true
    })
}

pub fn clauses_equivalent(left_cls: &[i32], right_cls: &[i32]) -> bool {
    let mut left_size = 0;
    for &l in left_cls.iter() {
        if l == 0 {
            break;
        }
        let mut found = false;
        for &r in right_cls.iter() {
            if r == 0 {
                break;
            }
            if r == l {
                found = true;
                break;
            }
        }
        if !found {
            return false;
        }
        left_size += 1;
    }
    let mut right_size = 0;
    for &r in right_cls.iter() {
        if r == 0 {
            break;
        }
        right_size += 1;
    }
    left_size == right_size
}

pub fn lrat_check_validate_sat(model: &[i32], size: u64) -> bool {
    with_state(|st| {
        if !st.done_loading {
            return false;
        }
        if !st.check_model {
            return false;
        }
        let mut model_mut: Vec<i32> = model[..size as usize].to_vec();
        for id in 1..=st.nb_loaded_clauses {
            let cls = match st.clause_table.get(&id) {
                Some(c) => c.clone(),
                None => return false,
            };
            let mut satisfied = false;
            for &lit in cls.iter() {
                if lit == 0 {
                    break;
                }
                let var = if lit > 0 { lit } else { -lit };
                if (var - 1) as u64 >= size {
                    return false;
                }
                let mut model_lit = model_mut[(var - 1) as usize];
                if model_lit != var && model_lit != -var && model_lit != 0 {
                    return false;
                }
                if model_lit == 0 {
                    model_mut[(var - 1) as usize] = lit;
                    model_lit = lit;
                }
                if model_lit == lit {
                    satisfied = true;
                    break;
                }
            }
            if !satisfied {
                return false;
            }
        }
        true
    })
}

pub fn lrat_check_load(lit: i32) -> bool {
    if lit == 0 {
        let (id, cls) = with_state(|st| (st.id_to_add, st.clause_to_add.clone()));
        if !lrat_check_add_axiomatic_clause(id, &cls, cls.len() as i32) {
            return false;
        }
        with_state(|st| {
            st.id_to_add += 1;
            st.clause_to_add.push(0);
        });
        // Update global siphash with the clause data (including trailing zero)
        let bytes = with_state(|st| {
            let mut bytes = Vec::with_capacity(st.clause_to_add.len() * 4);
            for &v in st.clause_to_add.iter() {
                bytes.extend_from_slice(&v.to_ne_bytes());
            }
            bytes
        });
        crate::siphash_global::with_global_siphash(|sh| {
            sh.siphash_update(&bytes, bytes.len() as u64);
        });
        with_state(|st| {
            st.clause_to_add.clear();
        });
        return true;
    }
    with_state(|st| {
        st.clause_to_add.push(lit);
    });
    true
}

pub fn lrat_check_init(nb_vars: i32, opt_check_model: bool, opt_lenient: bool) {
    let st = LratState {
        clause_table: HashMap::new(),
        var_values: vec![0i8; (nb_vars + 1) as usize],
        assigned_units: Vec::new(),
        check_model: opt_check_model,
        lenient: opt_lenient,
        id_to_add: 1,
        nb_loaded_clauses: 0,
        clause_to_add: Vec::new(),
        done_loading: false,
        unsat_proven: false,
    };
    STATE.with(|s| {
        *s.borrow_mut() = Some(st);
    });
    let _ = trusted_utils::SIG_SIZE_BYTES;
}

pub fn clause_init(data: &[i32], nb_lits: i32) -> Vec<i32> {
    let mut cls = Vec::with_capacity(nb_lits as usize + 1);
    for i in 0..nb_lits as usize {
        cls.push(data[i]);
    }
    cls.push(0);
    cls
}

pub fn lrat_check_validate_unsat() -> bool {
    with_state(|st| {
        if !st.done_loading {
            return false;
        }
        if !st.unsat_proven {
            return false;
        }
        true
    })
}
