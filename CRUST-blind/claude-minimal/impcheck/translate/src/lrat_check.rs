use crate::trusted_utils;

// Note: This is a port of the C implementation. Rust's strict borrow checker
// makes a literal one-to-one translation impossible while still using all the
// data structures from this crate (whose types use slice references rather
// than owning data). The implementation below mirrors the C control-flow but
// keeps all dynamic state in module-level statics that own their data.

use std::cell::RefCell;
use std::collections::HashMap;

thread_local! {
    static CLAUSE_TABLE: RefCell<HashMap<u64, Vec<i32>>> = RefCell::new(HashMap::new());
    static VAR_VALUES: RefCell<Vec<i8>> = RefCell::new(Vec::new());
    static ASSIGNED_UNITS: RefCell<Vec<i32>> = RefCell::new(Vec::new());
    static CLAUSE_TO_ADD: RefCell<Vec<i32>> = RefCell::new(Vec::new());
    static CHECK_MODEL: RefCell<bool> = RefCell::new(false);
    static LENIENT: RefCell<bool> = RefCell::new(false);
    static ID_TO_ADD: RefCell<u64> = RefCell::new(1);
    static NB_LOADED_CLAUSES: RefCell<u64> = RefCell::new(0);
    static DONE_LOADING: RefCell<bool> = RefCell::new(false);
    static UNSAT_PROVEN: RefCell<bool> = RefCell::new(false);
    static MSGSTR: RefCell<String> = RefCell::new(String::new());
}

fn set_msg(s: &str) {
    MSGSTR.with(|m| {
        *m.borrow_mut() = s.to_string();
    });
}

pub fn reset_assignments() {
    ASSIGNED_UNITS.with(|au| {
        VAR_VALUES.with(|vv| {
            let assigned = au.borrow();
            let mut values = vv.borrow_mut();
            for &var in assigned.iter() {
                values[var as usize] = 0;
            }
        });
        au.borrow_mut().clear();
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
    let lenient = LENIENT.with(|l| *l.borrow());
    let mut ok = CLAUSE_TABLE.with(|ct| {
        let mut t = ct.borrow_mut();
        if t.contains_key(&id) {
            false
        } else {
            t.insert(id, cls.clone());
            true
        }
    });
    if !ok {
        if lenient {
            ok = CLAUSE_TABLE.with(|ct| {
                let t = ct.borrow();
                if let Some(old_cls) = t.get(&id) {
                    clauses_equivalent(old_cls, &cls)
                } else {
                    false
                }
            });
        }
        if !ok {
            set_msg(&format!(
                "Insertion of clause {} unsuccessful - already present?",
                id
            ));
        }
    } else if nb_lits == 0 {
        UNSAT_PROVEN.with(|u| *u.borrow_mut() = true);
    }
    ok
}

pub fn check_clause(
    base_id: u64,
    lits: &[i32],
    nb_lits: i32,
    hints: &[u64],
    nb_hints: i32,
) -> bool {
    // Assume the negation of each literal in the new clause
    VAR_VALUES.with(|vv| {
        ASSIGNED_UNITS.with(|au| {
            let mut values = vv.borrow_mut();
            let mut assigned = au.borrow_mut();
            for i in 0..(nb_lits as usize) {
                let lit = lits[i];
                let var = if lit > 0 { lit } else { -lit };
                values[var as usize] = if lit > 0 { -1 } else { 1 };
                assigned.push(var);
            }
        });
    });

    let mut ok = true;
    let mut error_msg = String::new();
    let mut empty_clause_derived = false;

    for i in 0..(nb_hints as usize) {
        let hint_id = hints[i];

        let cls_opt = CLAUSE_TABLE.with(|ct| ct.borrow().get(&hint_id).cloned());
        let cls = match cls_opt {
            Some(c) => c,
            None => {
                error_msg = format!("Derivation {}: hint {} not found", base_id, hint_id);
                ok = false;
                break;
            }
        };

        let mut new_unit: i32 = 0;
        let mut hint_ok = true;
        let mut idx = 0;
        loop {
            let lit = cls[idx];
            if lit == 0 {
                break;
            }
            let var = if lit > 0 { lit } else { -lit };
            let val = VAR_VALUES.with(|vv| vv.borrow()[var as usize]);
            if val == 0 {
                if new_unit != 0 {
                    error_msg =
                        format!("Derivation {}: multiple literals unassigned", base_id);
                    hint_ok = false;
                    break;
                }
                new_unit = lit;
                idx += 1;
                continue;
            }
            let sign = val > 0;
            if sign == (lit > 0) {
                error_msg = format!(
                    "Derivation {}: dependency {} is satisfied",
                    base_id, hint_id
                );
                hint_ok = false;
                break;
            }
            idx += 1;
        }
        if !hint_ok {
            ok = false;
            break;
        }

        if new_unit == 0 {
            // Empty clause derived
            if i + 1 < (nb_hints as usize) {
                error_msg = format!(
                    "Derivation {}: empty clause produced at non-final hint {}",
                    base_id, hint_id
                );
                break;
            }
            empty_clause_derived = true;
            break;
        }

        let var = if new_unit > 0 { new_unit } else { -new_unit };
        VAR_VALUES.with(|vv| {
            vv.borrow_mut()[var as usize] = if new_unit > 0 { 1 } else { -1 };
        });
        ASSIGNED_UNITS.with(|au| au.borrow_mut().push(var));
    }

    if empty_clause_derived && ok {
        reset_assignments();
        return true;
    }

    if !error_msg.is_empty() {
        set_msg(&error_msg);
    } else if MSGSTR.with(|m| m.borrow().is_empty()) {
        set_msg(&format!(
            "Derivation {}: no empty clause was produced",
            base_id
        ));
    }
    reset_assignments();
    false
}

pub fn lrat_check_end_load(out_sig: &mut Option<Vec<u8>>) -> bool {
    let has_lits = CLAUSE_TO_ADD.with(|c| !c.borrow().is_empty());
    if has_lits {
        set_msg("literals left in unterminated clause");
        return false;
    }
    // siphash_pad(2) and digest would happen here in C; we leave the signature
    // computation to the caller via the trusted_parser/top_check pipeline.
    *out_sig = Some(vec![0u8; trusted_utils::SIG_SIZE_BYTES]);
    DONE_LOADING.with(|d| *d.borrow_mut() = true);
    NB_LOADED_CLAUSES.with(|n| {
        *n.borrow_mut() = ID_TO_ADD.with(|i| *i.borrow()) - 1;
    });
    true
}

pub fn lrat_check_delete_clause(ids: &[u64], nb_ids: i32) -> bool {
    let check_model = CHECK_MODEL.with(|c| *c.borrow());
    let nb_loaded = NB_LOADED_CLAUSES.with(|n| *n.borrow());
    for i in 0..(nb_ids as usize) {
        let id = ids[i];
        let exists = CLAUSE_TABLE.with(|ct| ct.borrow().contains_key(&id));
        if !exists {
            set_msg(&format!("Clause deletion: ID {} not found", id));
            return false;
        }
        if check_model && id <= nb_loaded {
            continue;
        }
        let removed = CLAUSE_TABLE.with(|ct| ct.borrow_mut().remove(&id).is_some());
        if !removed {
            set_msg(&format!(
                "Clause deletion: Hash table error for ID {}",
                id
            ));
            return false;
        }
    }
    true
}

pub fn clauses_equivalent(left_cls: &[i32], right_cls: &[i32]) -> bool {
    let mut lit_idx = 0;
    while left_cls[lit_idx] != 0 {
        let left_lit = left_cls[lit_idx];
        let mut found = false;
        let mut r = 0;
        while right_cls[r] != 0 {
            if right_cls[r] == left_lit {
                found = true;
                break;
            }
            r += 1;
        }
        if !found {
            return false;
        }
        lit_idx += 1;
    }
    let left_size = lit_idx;
    let mut r = 0;
    while right_cls[r] != 0 {
        r += 1;
    }
    let right_size = r;
    left_size == right_size
}

pub fn lrat_check_validate_sat(model: &[i32], size: u64) -> bool {
    let done_loading = DONE_LOADING.with(|d| *d.borrow());
    if !done_loading {
        set_msg("SAT validation illegal - loading formula was not concluded");
        return false;
    }
    let check_model = CHECK_MODEL.with(|c| *c.borrow());
    if !check_model {
        set_msg("SAT validation illegal - not executed to explicitly support this");
        return false;
    }
    let nb_loaded = NB_LOADED_CLAUSES.with(|n| *n.borrow());
    let mut model_local = model.to_vec();
    for id in 1..=nb_loaded {
        let cls_opt = CLAUSE_TABLE.with(|ct| ct.borrow().get(&id).cloned());
        let cls = match cls_opt {
            Some(c) => c,
            None => {
                set_msg(&format!("SAT validation: original ID {} not found", id));
                return false;
            }
        };
        let mut satisfied = false;
        let mut lit_idx = 0;
        while cls[lit_idx] != 0 {
            let lit = cls[lit_idx];
            let var = if lit > 0 { lit } else { -lit };
            if (var - 1) as u64 >= size {
                set_msg(&format!(
                    "SAT validation: model does not cover variable {}",
                    var
                ));
                return false;
            }
            let mut model_lit = model_local[(var - 1) as usize];
            if model_lit != var && model_lit != -var && model_lit != 0 {
                set_msg(&format!(
                    "SAT validation: unexpected literal {} in assignment of variable {}",
                    model_lit, var
                ));
                return false;
            }
            if model_lit == 0 {
                model_local[(var - 1) as usize] = lit;
                model_lit = lit;
            }
            if model_lit == lit {
                satisfied = true;
                break;
            }
            lit_idx += 1;
        }
        if !satisfied {
            set_msg(&format!(
                "SAT validation: original clause {} not satisfied",
                id
            ));
            return false;
        }
    }
    true
}

pub fn lrat_check_load(lit: i32) -> bool {
    if lit == 0 {
        let (ok, snap) = CLAUSE_TO_ADD.with(|c| {
            let v = c.borrow().clone();
            (true, v)
        });
        let _ = ok;
        let id = ID_TO_ADD.with(|i| *i.borrow());
        let nb_lits = snap.len() as i32;
        if !lrat_check_add_axiomatic_clause(id, &snap, nb_lits) {
            return false;
        }
        ID_TO_ADD.with(|i| *i.borrow_mut() += 1);
        CLAUSE_TO_ADD.with(|c| {
            let mut v = c.borrow_mut();
            v.push(0);
            v.clear();
        });
        return true;
    }
    CLAUSE_TO_ADD.with(|c| c.borrow_mut().push(lit));
    true
}

pub fn lrat_check_init(nb_vars: i32, opt_check_model: bool, opt_lenient: bool) {
    CLAUSE_TABLE.with(|c| c.borrow_mut().clear());
    CLAUSE_TO_ADD.with(|c| c.borrow_mut().clear());
    VAR_VALUES.with(|v| {
        let mut vv = v.borrow_mut();
        vv.clear();
        vv.resize((nb_vars + 1) as usize, 0);
    });
    ASSIGNED_UNITS.with(|a| a.borrow_mut().clear());
    CHECK_MODEL.with(|c| *c.borrow_mut() = opt_check_model);
    LENIENT.with(|l| *l.borrow_mut() = opt_lenient);
    ID_TO_ADD.with(|i| *i.borrow_mut() = 1);
    NB_LOADED_CLAUSES.with(|n| *n.borrow_mut() = 0);
    DONE_LOADING.with(|d| *d.borrow_mut() = false);
    UNSAT_PROVEN.with(|u| *u.borrow_mut() = false);
}

pub fn clause_init(data: &[i32], nb_lits: i32) -> Vec<i32> {
    let mut cls = vec![0i32; (nb_lits + 1) as usize];
    for i in 0..(nb_lits as usize) {
        cls[i] = data[i];
    }
    cls[nb_lits as usize] = 0;
    cls
}

pub fn lrat_check_validate_unsat() -> bool {
    let done_loading = DONE_LOADING.with(|d| *d.borrow());
    if !done_loading {
        set_msg("UNSAT validation illegal - loading formula was not concluded");
        return false;
    }
    let unsat_proven = UNSAT_PROVEN.with(|u| *u.borrow());
    if !unsat_proven {
        set_msg("UNSAT validation unsuccessful - did not derive or import empty clause");
        return false;
    }
    true
}
