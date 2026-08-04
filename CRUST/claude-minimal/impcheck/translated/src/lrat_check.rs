// Direct port of c_src/src/trusted/lrat_check.c
//
// The original C code uses module-level globals. We mirror that here using a
// thread-local global state held in a `RefCell` for interior mutability.

use crate::hash::HashTable;
use crate::siphash;

use std::cell::RefCell;

// Helper: clause is a Vec<i32> ending in 0 (sentinel). All clauses are stored
// as `Box<dyn Any>` inside the hash table whose underlying object is a
// `Vec<i32>` (the C version stored an `int*` pointing at a 0-terminated array).

pub struct LratCheckState {
    pub clause_table: HashTable<Vec<i32>>,
    pub var_values: Vec<i8>,
    pub assigned_units: Vec<i32>,
    pub check_model: bool,
    pub lenient: bool,
    pub id_to_add: u64,
    pub nb_loaded_clauses: u64,
    pub clause_to_add: Vec<i32>,
    pub done_loading: bool,
    pub unsat_proven: bool,
    pub msgstr: String,
    pub siphash: Option<siphash::SipHash>,
}

impl LratCheckState {
    fn new() -> Self {
        LratCheckState {
            clause_table: HashTable::new(16),
            var_values: Vec::new(),
            assigned_units: Vec::new(),
            check_model: false,
            lenient: false,
            id_to_add: 1,
            nb_loaded_clauses: 0,
            clause_to_add: Vec::new(),
            done_loading: false,
            unsat_proven: false,
            msgstr: String::new(),
            siphash: None,
        }
    }
}

thread_local! {
    pub static STATE: RefCell<LratCheckState> = RefCell::new(LratCheckState::new());
}

pub fn clause_init(data: &[i32], nb_lits: i32) -> Vec<i32> {
    let mut cls = Vec::with_capacity(nb_lits as usize + 1);
    for i in 0..(nb_lits as usize) {
        cls.push(data[i]);
    }
    cls.push(0);
    cls
}

pub fn reset_assignments() {
    STATE.with(|s| {
        let mut st = s.borrow_mut();
        let assigned: Vec<i32> = std::mem::take(&mut st.assigned_units);
        for &v in &assigned {
            st.var_values[v as usize] = 0;
        }
        st.assigned_units = Vec::new();
    });
}

pub fn check_clause(
    base_id: u64,
    lits: &[i32],
    nb_lits: i32,
    hints: &[u64],
    nb_hints: i32,
) -> bool {
    // assume negations of all literals
    STATE.with(|s| {
        let mut st = s.borrow_mut();
        st.assigned_units.reserve((nb_lits + nb_hints) as usize);
        for i in 0..(nb_lits as usize) {
            let lit = lits[i];
            let var = if lit > 0 { lit } else { -lit } as usize;
            st.var_values[var] = if lit > 0 { -1 } else { 1 };
            st.assigned_units.push(var as i32);
        }
    });

    let mut ok = true;
    let mut error_already_set = false;

    for i in 0..(nb_hints as usize) {
        let hint_id = hints[i];

        // Find the clause for this hint. We must look it up via the hash table.
        // Since hash_table_find requires &mut self, we briefly borrow mut.
        let cls_owned: Option<Vec<i32>> = STATE.with(|s| {
            let mut st = s.borrow_mut();
            let r = st.clause_table.hash_table_find(hint_id);
            r.map(|b| b.downcast_ref::<Vec<i32>>().unwrap().clone())
        });

        let cls = match cls_owned {
            None => {
                STATE.with(|s| {
                    let mut st = s.borrow_mut();
                    st.msgstr =
                        format!("Derivation {}: hint {} not found", base_id, hint_id);
                });
                error_already_set = true;
                ok = false;
                break;
            }
            Some(c) => c,
        };

        // Interpret hint clause to derive a new unit clause
        let mut new_unit: i32 = 0;
        let mut hint_ok = true;
        for &lit in &cls {
            if lit == 0 {
                break;
            }
            let var = if lit > 0 { lit } else { -lit } as usize;
            let val: i8 = STATE.with(|s| s.borrow().var_values[var]);
            if val == 0 {
                if new_unit != 0 {
                    STATE.with(|s| {
                        let mut st = s.borrow_mut();
                        st.msgstr = format!(
                            "Derivation {}: multiple literals unassigned",
                            base_id
                        );
                    });
                    error_already_set = true;
                    ok = false;
                    hint_ok = false;
                    break;
                }
                new_unit = lit;
                continue;
            }
            // Literal is fixed
            let sign = val > 0;
            if sign == (lit > 0) {
                STATE.with(|s| {
                    let mut st = s.borrow_mut();
                    st.msgstr = format!(
                        "Derivation {}: dependency {} is satisfied",
                        base_id, hint_id
                    );
                });
                error_already_set = true;
                ok = false;
                hint_ok = false;
                break;
            }
        }
        if !hint_ok {
            break;
        }

        if new_unit == 0 {
            // empty clause derived
            if i + 1 < nb_hints as usize {
                STATE.with(|s| {
                    let mut st = s.borrow_mut();
                    st.msgstr = format!(
                        "Derivation {}: empty clause produced at non-final hint {}",
                        base_id, hint_id
                    );
                });
                error_already_set = true;
                ok = false;
                break;
            }
            reset_assignments();
            return true;
        }

        let var = if new_unit > 0 { new_unit } else { -new_unit } as usize;
        STATE.with(|s| {
            let mut st = s.borrow_mut();
            st.var_values[var] = if new_unit > 0 { 1 } else { -1 };
            st.assigned_units.push(var as i32);
        });
    }

    if !error_already_set {
        STATE.with(|s| {
            let mut st = s.borrow_mut();
            if st.msgstr.is_empty() {
                st.msgstr =
                    format!("Derivation {}: no empty clause was produced", base_id);
            }
        });
    }
    reset_assignments();
    false
}

pub fn clauses_equivalent(left_cls: &[i32], right_cls: &[i32]) -> bool {
    let mut lit_idx = 0usize;
    while left_cls[lit_idx] != 0 {
        let left_lit = left_cls[lit_idx];
        let mut found = false;
        let mut ridx = 0usize;
        while right_cls[ridx] != 0 {
            if right_cls[ridx] == left_lit {
                found = true;
                break;
            }
            ridx += 1;
        }
        if !found {
            return false;
        }
        lit_idx += 1;
    }
    let left_size = lit_idx;
    let mut right_size = 0usize;
    while right_cls[right_size] != 0 {
        right_size += 1;
    }
    left_size == right_size
}

pub fn lrat_check_add_axiomatic_clause(id: u64, lits: &[i32], nb_lits: i32) -> bool {
    let cls = clause_init(lits, nb_lits);
    let mut ok = STATE.with(|s| {
        let mut st = s.borrow_mut();
        st.clause_table.hash_table_insert(id, Box::new(cls.clone()))
    });
    if !ok {
        let lenient = STATE.with(|s| s.borrow().lenient);
        if lenient {
            let equivalent = STATE.with(|s| {
                let mut st = s.borrow_mut();
                let opt = st.clause_table.hash_table_find(id);
                if let Some(b) = opt {
                    let old_cls = b.downcast_ref::<Vec<i32>>().unwrap().clone();
                    clauses_equivalent(&old_cls, &cls)
                } else {
                    false
                }
            });
            if equivalent {
                ok = true;
            }
        }
        if !ok {
            STATE.with(|s| {
                let mut st = s.borrow_mut();
                st.msgstr = format!(
                    "Insertion of clause {} unsuccessful - already present?",
                    id
                );
            });
        }
    } else if nb_lits == 0 {
        STATE.with(|s| s.borrow_mut().unsat_proven = true);
    }
    ok
}

pub fn lrat_check_init(nb_vars: i32, opt_check_model: bool, opt_lenient: bool) {
    STATE.with(|s| {
        let mut st = s.borrow_mut();
        *st = LratCheckState::new();
        st.clause_table = HashTable::new(16);
        st.clause_to_add = Vec::with_capacity(512);
        st.var_values = vec![0i8; (nb_vars + 1) as usize];
        st.assigned_units = Vec::with_capacity(512);
        st.check_model = opt_check_model;
        st.lenient = opt_lenient;
    });
}

pub fn lrat_check_load(lit: i32) -> bool {
    if lit == 0 {
        let (id, cls_data, cls_size) = STATE.with(|s| {
            let st = s.borrow();
            let v = st.clause_to_add.clone();
            (st.id_to_add, v.clone(), v.len() as i32)
        });
        if !lrat_check_add_axiomatic_clause(id, &cls_data, cls_size) {
            return false;
        }
        STATE.with(|s| {
            let mut st = s.borrow_mut();
            st.id_to_add += 1;
            st.clause_to_add.push(0);
            // siphash update with the bytes of clause_to_add
            let bytes_per_int = std::mem::size_of::<i32>();
            let nb_bytes = (st.clause_to_add.len() * bytes_per_int) as u64;
            // Convert the vector to bytes
            let mut bytes: Vec<u8> = Vec::with_capacity(nb_bytes as usize);
            for &val in &st.clause_to_add {
                bytes.extend_from_slice(&val.to_ne_bytes());
            }
            if let Some(sh) = st.siphash.as_mut() {
                sh.siphash_update(&bytes, nb_bytes);
            }
            st.clause_to_add.clear();
        });
        return true;
    }
    STATE.with(|s| s.borrow_mut().clause_to_add.push(lit));
    true
}

pub fn lrat_check_end_load(out_sig: &mut Option<Vec<u8>>) -> bool {
    let size_pre = STATE.with(|s| s.borrow().clause_to_add.len());
    if size_pre > 0 {
        STATE.with(|s| {
            let mut st = s.borrow_mut();
            st.msgstr = "literals left in unterminated clause".to_string();
        });
        return false;
    }
    let sig = STATE.with(|s| {
        let mut st = s.borrow_mut();
        if let Some(sh) = st.siphash.as_mut() {
            sh.siphash_pad(2);
            sh.siphash_digest()
        } else {
            vec![0u8; 16]
        }
    });
    *out_sig = Some(sig);
    STATE.with(|s| {
        let mut st = s.borrow_mut();
        st.done_loading = true;
        st.nb_loaded_clauses = st.id_to_add - 1;
    });
    true
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

pub fn lrat_check_delete_clause(ids: &[u64], nb_ids: i32) -> bool {
    for i in 0..(nb_ids as usize) {
        let id = ids[i];
        let exists = STATE.with(|s| {
            let mut st = s.borrow_mut();
            st.clause_table.hash_table_find(id).is_some()
        });
        if !exists {
            STATE.with(|s| {
                let mut st = s.borrow_mut();
                st.msgstr = format!("Clause deletion: ID {} not found", id);
            });
            return false;
        }
        let nb_loaded = STATE.with(|s| s.borrow().nb_loaded_clauses);
        let check_model = STATE.with(|s| s.borrow().check_model);
        if check_model && id <= nb_loaded {
            continue;
        }
        let ok = STATE.with(|s| {
            let mut st = s.borrow_mut();
            st.clause_table.hash_table_delete_last_found()
        });
        if !ok {
            STATE.with(|s| {
                let mut st = s.borrow_mut();
                st.msgstr = format!("Clause deletion: Hash table error for ID {}", id);
            });
            return false;
        }
    }
    true
}

pub fn lrat_check_validate_unsat() -> bool {
    let (done, unsat) = STATE.with(|s| {
        let st = s.borrow();
        (st.done_loading, st.unsat_proven)
    });
    if !done {
        STATE.with(|s| {
            let mut st = s.borrow_mut();
            st.msgstr =
                "UNSAT validation illegal - loading formula was not concluded".to_string();
        });
        return false;
    }
    if !unsat {
        STATE.with(|s| {
            let mut st = s.borrow_mut();
            st.msgstr =
                "UNSAT validation unsuccessful - did not derive or import empty clause"
                    .to_string();
        });
        return false;
    }
    true
}

pub fn lrat_check_validate_sat(model: &[i32], size: u64) -> bool {
    let (done, check_model_flag, nb_loaded) = STATE.with(|s| {
        let st = s.borrow();
        (st.done_loading, st.check_model, st.nb_loaded_clauses)
    });
    if !done {
        STATE.with(|s| {
            let mut st = s.borrow_mut();
            st.msgstr =
                "SAT validation illegal - loading formula was not concluded".to_string();
        });
        return false;
    }
    if !check_model_flag {
        STATE.with(|s| {
            let mut st = s.borrow_mut();
            st.msgstr =
                "SAT validation illegal - not executed to explicitly support this"
                    .to_string();
        });
        return false;
    }
    let mut model_mut: Vec<i32> = model[..(size as usize)].to_vec();
    for id in 1..=nb_loaded {
        let cls_opt: Option<Vec<i32>> = STATE.with(|s| {
            let mut st = s.borrow_mut();
            st.clause_table
                .hash_table_find(id)
                .map(|b| b.downcast_ref::<Vec<i32>>().unwrap().clone())
        });
        let cls = match cls_opt {
            None => {
                STATE.with(|s| {
                    let mut st = s.borrow_mut();
                    st.msgstr = format!("SAT validation: original ID {} not found", id);
                });
                return false;
            }
            Some(v) => v,
        };
        let mut satisfied = false;
        for &lit in &cls {
            if lit == 0 {
                break;
            }
            let var = if lit > 0 { lit } else { -lit };
            if (var as u64 - 1) >= size {
                STATE.with(|s| {
                    let mut st = s.borrow_mut();
                    st.msgstr = format!(
                        "SAT validation: model does not cover variable {}",
                        var
                    );
                });
                return false;
            }
            let mut model_lit = model_mut[(var - 1) as usize];
            if model_lit != var && model_lit != -var && model_lit != 0 {
                STATE.with(|s| {
                    let mut st = s.borrow_mut();
                    st.msgstr = format!(
                        "SAT validation: unexpected literal {} in assignment of variable {}",
                        model_lit, var
                    );
                });
                return false;
            }
            if model_lit == 0 {
                model_lit = lit;
                model_mut[(var - 1) as usize] = lit;
            }
            if model_lit == lit {
                satisfied = true;
                break;
            }
        }
        if !satisfied {
            STATE.with(|s| {
                let mut st = s.borrow_mut();
                st.msgstr =
                    format!("SAT validation: original clause {} not satisfied", id);
            });
            return false;
        }
    }
    true
}
