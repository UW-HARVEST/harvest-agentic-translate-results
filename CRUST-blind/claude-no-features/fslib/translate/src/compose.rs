use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use crate::fst::{Fst as CFst, ArcData as CArcData, Spair, EPS as FST_EPS};
use crate::queue::Queue;
use crate::sr;

// Flags (just examples; adapt to your constants)
const OSORT: u32 = 0x1;
const ISORT: u32 = 0x2;

// A trait to represent your semiring
pub trait Semiring: Clone {
    fn zero() -> Self;
    fn one() -> Self;
    fn plus(&self, rhs: &Self) -> Self;
    fn prod(&self, rhs: &Self) -> Self;
}
// Example semiring for Weighted FST with float weights (Tropical semiring-like)
#[derive(Clone, Debug)]
pub struct FloatSemiring(pub f32);
impl Semiring for FloatSemiring {
    fn zero() -> Self {
        FloatSemiring(f32::MAX)
    }
    fn one() -> Self {
        FloatSemiring(0.0)
    }
    fn plus(&self, rhs: &Self) -> Self {
        if self.0 < rhs.0 {
            FloatSemiring(self.0)
        } else {
            FloatSemiring(rhs.0)
        }
    }
    fn prod(&self, rhs: &Self) -> Self {
        FloatSemiring(self.0 + rhs.0)
    }
}
// Arc representation
#[derive(Clone, Debug)]
pub struct Arc<W: Semiring> {
    pub state: usize,
    pub ilabel: u32,
    pub olabel: u32,
    pub weight: W,
}
// State representation
#[derive(Clone, Debug)]
pub struct State<W: Semiring> {
    pub arcs: Vec<Arc<W>>,
    pub final_weight: Option<W>,
}
// Fst representation
#[derive(Clone, Debug)]
pub struct Fst<W: Semiring> {
    pub states: Vec<State<W>>,
    pub start: usize,
    pub flags: u32,   // Might store OSORT/ISORT flags, etc.
}
impl<W: Semiring> Fst<W> {
    pub fn new() -> Self {
        Fst {
            states: Vec::new(),
            start: 0,
            flags: 0,
        }
    }
    pub fn add_state(&mut self) -> usize {
        self.states.push(State {
            arcs: Vec::new(),
            final_weight: None,
        });
        self.states.len() - 1
    }
    pub fn set_final(&mut self, st: usize, weight: W) {
        if st < self.states.len() {
            self.states[st].final_weight = Some(weight);
        }
    }
    pub fn add_arc(&mut self, src: usize, arc: Arc<W>) {
        if src < self.states.len() {
            self.states[src].arcs.push(arc);
        }
    }
}
// A pair of states (a,b)
#[derive(Copy, Clone, Debug, Eq)]
pub struct StatePair {
    pub a: usize,
    pub b: usize,
}
impl PartialEq for StatePair {
    fn eq(&self, other: &Self) -> bool {
        self.a == other.a && self.b == other.b
    }
}
impl Hash for StatePair {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_usize(self.a);
        state.write_usize(self.b);
    }
}
#[derive(Clone, Debug)]
pub struct ArcPair<W: Semiring> {
    pub a: Arc<W>,
    pub b: Arc<W>,
}
pub const EPS: u32 = 0;

fn arc_match<W: Semiring>(a: &[Arc<W>], b: &[Arc<W>], i: usize, j: usize) -> bool {
    let al = a[i].olabel;
    if al == EPS {
        if (i != 0 && j != 0) || (i == 0 && j == 0) {
            return false;
        }
    }
    true
}

fn match_full_sorted<W: Semiring>(arcs_a: &[Arc<W>], arcs_b: &[Arc<W>]) -> Vec<ArcPair<W>> {
    let mut pairs = Vec::new();
    let m = arcs_a.len();
    let n = arcs_b.len();
    let mut i = 0usize;
    let mut j = 0usize;
    while i < m && j < n {
        if arcs_a[i].olabel < arcs_b[j].ilabel {
            i += 1;
        } else if arcs_a[i].olabel > arcs_b[j].ilabel {
            j += 1;
        } else {
            let mut t = j;
            while t < n && arcs_a[i].olabel == arcs_b[t].ilabel {
                if arc_match(arcs_a, arcs_b, i, t) {
                    pairs.push(ArcPair { a: arcs_a[i].clone(), b: arcs_b[t].clone() });
                }
                t += 1;
            }
            i += 1;
        }
    }
    pairs
}

fn match_half_sorted<W: Semiring>(arcs_a: &[Arc<W>], arcs_b: &[Arc<W>]) -> Vec<ArcPair<W>> {
    let mut pairs = Vec::new();
    let m = arcs_a.len();
    let n = arcs_b.len();
    if n == 0 { return pairs; }
    for i in 0..m {
        let mut l: i64 = 0;
        let mut h: i64 = n as i64 - 1;
        while l <= h {
            let mid = ((l + h) >> 1) as usize;
            if arcs_a[i].olabel > arcs_b[mid].ilabel {
                l = mid as i64 + 1;
            } else if arcs_a[i].olabel < arcs_b[mid].ilabel {
                if mid == 0 { break; }
                h = mid as i64 - 1;
            } else {
                let mut ll = mid;
                let mut hh = mid;
                while (ll as i64) > l && arcs_a[i].olabel == arcs_b[ll - 1].ilabel { ll -= 1; }
                while (hh as i64) < h && arcs_a[i].olabel == arcs_b[hh + 1].ilabel { hh += 1; }
                let mut k = ll;
                while k <= hh {
                    if arc_match(arcs_a, arcs_b, i, k) {
                        pairs.push(ArcPair { a: arcs_a[i].clone(), b: arcs_b[k].clone() });
                    }
                    k += 1;
                }
                break;
            }
        }
    }
    pairs
}

fn match_half_sorted_rev<W: Semiring>(arcs_a: &[Arc<W>], arcs_b: &[Arc<W>]) -> Vec<ArcPair<W>> {
    let mut pairs = Vec::new();
    let m = arcs_a.len();
    let n = arcs_b.len();
    if m == 0 { return pairs; }
    for i in 0..n {
        let mut l: i64 = 0;
        let mut h: i64 = m as i64 - 1;
        while l <= h {
            let mid = ((l + h) >> 1) as usize;
            if arcs_b[i].ilabel > arcs_a[mid].olabel {
                l = mid as i64 + 1;
            } else if arcs_b[i].ilabel < arcs_a[mid].olabel {
                if mid == 0 { break; }
                h = mid as i64 - 1;
            } else {
                let mut ll = mid;
                let mut hh = mid;
                while (ll as i64) > l && arcs_b[i].ilabel == arcs_a[ll - 1].olabel { ll -= 1; }
                while (hh as i64) < h && arcs_b[i].ilabel == arcs_a[hh + 1].olabel { hh += 1; }
                let mut k = ll;
                while k <= hh {
                    if arc_match(arcs_a, arcs_b, k, i) {
                        pairs.push(ArcPair { a: arcs_a[k].clone(), b: arcs_b[i].clone() });
                    }
                    k += 1;
                }
                break;
            }
        }
    }
    pairs
}

fn match_unsorted<W: Semiring>(arcs_a: &[Arc<W>], arcs_b: &[Arc<W>]) -> Vec<ArcPair<W>> {
    let mut pairs = Vec::new();
    for i in 0..arcs_a.len() {
        for j in 0..arcs_b.len() {
            if arcs_a[i].olabel == arcs_b[j].ilabel && arc_match(arcs_a, arcs_b, i, j) {
                pairs.push(ArcPair { a: arcs_a[i].clone(), b: arcs_b[j].clone() });
            }
        }
    }
    pairs
}

fn match_arcs<W: Semiring>(
    fst_a: &Fst<W>,
    fst_b: &Fst<W>,
    pair: &StatePair,
    sr: &W,
) -> Vec<ArcPair<W>> {
    let state_a = &fst_a.states[pair.a];
    let state_b = &fst_b.states[pair.b];
    let osort = (fst_a.flags & OSORT) != 0;
    let isort = (fst_b.flags & ISORT) != 0;
    let mut arcs_a: Vec<Arc<W>> = Vec::with_capacity(state_a.arcs.len() + 1);
    let mut arcs_b: Vec<Arc<W>> = Vec::with_capacity(state_b.arcs.len() + 1);
    arcs_a.push(Arc { state: pair.a, ilabel: EPS, olabel: EPS, weight: sr.clone() });
    for arc in &state_a.arcs { arcs_a.push(arc.clone()); }
    arcs_b.push(Arc { state: pair.b, ilabel: EPS, olabel: EPS, weight: sr.clone() });
    for arc in &state_b.arcs { arcs_b.push(arc.clone()); }
    if isort && osort {
        match_full_sorted(&arcs_a, &arcs_b)
    } else if isort || osort {
        if isort {
            match_half_sorted(&arcs_a, &arcs_b)
        } else {
            match_half_sorted_rev(&arcs_a, &arcs_b)
        }
    } else {
        match_unsorted(&arcs_a, &arcs_b)
    }
}

pub fn fst_compose<W: Semiring>(
    fst_a: &Fst<W>,
    fst_b: &Fst<W>,
    sr: &W,
) -> Fst<W> {
    let mut fst_c: Fst<W> = Fst::new();
    let mut q: Queue<StatePair> = Queue::new();
    let mut marked: HashMap<StatePair, usize> = HashMap::new();
    let init_pair = StatePair { a: fst_a.start, b: fst_b.start };
    q.enqueue(init_pair);
    while let Some(pair) = q.dequeue() {
        let state_a = &fst_a.states[pair.a];
        let state_b = &fst_b.states[pair.b];
        let sc = if let Some(&id) = marked.get(&pair) {
            id
        } else {
            let id = fst_c.add_state();
            if state_a.final_weight.is_some() && state_b.final_weight.is_some() {
                fst_c.set_final(id, sr.clone());
            }
            if pair.a == fst_a.start && pair.b == fst_b.start {
                fst_c.start = id;
            }
            marked.insert(pair, id);
            id
        };
        let pairs = match_arcs(fst_a, fst_b, &pair, sr);
        for mi in pairs {
            let dst_pair = StatePair { a: mi.a.state, b: mi.b.state };
            let dst_sc = if let Some(&id) = marked.get(&dst_pair) {
                id
            } else {
                let dst_a = &fst_a.states[dst_pair.a];
                let dst_b = &fst_b.states[dst_pair.b];
                let id = fst_c.add_state();
                if dst_a.final_weight.is_some() && dst_b.final_weight.is_some() {
                    fst_c.set_final(id, sr.clone());
                }
                q.enqueue(dst_pair);
                marked.insert(dst_pair, id);
                id
            };
            fst_c.add_arc(sc, Arc {
                state: dst_sc,
                ilabel: mi.a.ilabel,
                olabel: mi.b.olabel,
                weight: mi.a.weight.prod(&mi.b.weight),
            });
        }
    }
    fst_c
}

// Implementation of `Fst::compose` style for the C-API style Fst type used in
// crate::fst.  This is the "basic" raw composition over the C-style structs.
pub fn fst_compose_basic(fst_a: &CFst, fst_b: &CFst, fst_c: &mut CFst) {
    let sr_v = sr::sr_get(fst_a.sr_type);
    fst_c.sr_type = fst_a.sr_type;
    let mut q: Queue<Spair> = Queue::new();
    let mut mq: Queue<(CArcData, CArcData)> = Queue::new();
    let mut marked: HashMap<(u32, u32), u32> = HashMap::new();
    let init_pair = Spair { a: fst_a.start, b: fst_b.start };
    q.enqueue(init_pair);
    while let Some(pair) = q.dequeue() {
        let state_a = &fst_a.states[pair.a as usize];
        let state_b = &fst_b.states[pair.b as usize];
        let key = (pair.a, pair.b);
        let sc = if let Some(&id) = marked.get(&key) {
            id
        } else {
            let id = fst_c.add_state();
            if state_a.final_state && state_b.final_state {
                fst_c.set_final(id, sr_v.one);
            }
            if pair.a == fst_a.start && pair.b == fst_b.start {
                fst_c.start = id;
            }
            marked.insert(key, id);
            id
        };
        // do match
        crate::fst::match_arcs(fst_a, fst_b, &pair, &sr_v, &mut mq);
        while let Some(mi) = mq.dequeue() {
            let arc_a = &mi.0;
            let arc_b = &mi.1;
            let dst_key = (arc_a.state, arc_b.state);
            let dst_sc = if let Some(&id) = marked.get(&dst_key) {
                id
            } else {
                let dst_state_a = &fst_a.states[arc_a.state as usize];
                let dst_state_b = &fst_b.states[arc_b.state as usize];
                let id = fst_c.add_state();
                if dst_state_a.final_state && dst_state_b.final_state {
                    fst_c.set_final(id, sr_v.one);
                }
                let new_pair = Spair { a: arc_a.state, b: arc_b.state };
                q.enqueue(new_pair);
                marked.insert(dst_key, id);
                id
            };
            let _ = FST_EPS; // ensure import is used
            let weight = (sr_v.prod)(arc_a.weight, arc_b.weight);
            fst_c.add_arc(sc, dst_sc, arc_a.ilabel, arc_b.olabel, weight);
        }
    }
}
