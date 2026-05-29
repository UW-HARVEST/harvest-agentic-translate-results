use std::collections::{HashMap, VecDeque};
use std::hash::{Hash, Hasher};
use crate::fst::{Fst as RealFst, ArcData, EPS as REAL_EPS, ISORT, OSORT, Spair};
use crate::sr::sr_get;
use crate::queue::Queue;
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
        FloatSemiring(if self.0 < rhs.0 { self.0 } else { rhs.0 })
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
    pub flags: u32,
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
        self.states.push(State { arcs: Vec::new(), final_weight: None });
        self.states.len() - 1
    }
    pub fn set_final(&mut self, st: usize, weight: W) {
        self.states[st].final_weight = Some(weight);
    }
    pub fn add_arc(&mut self, src: usize, arc: Arc<W>) {
        self.states[src].arcs.push(arc);
    }
}
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
fn match_full_sorted<W: Semiring>(_arcs_a: &[Arc<W>], _arcs_b: &[Arc<W>]) -> Vec<ArcPair<W>> {
    Vec::new()
}
fn match_half_sorted<W: Semiring>(_arcs_a: &[Arc<W>], _arcs_b: &[Arc<W>]) -> Vec<ArcPair<W>> {
    Vec::new()
}
fn match_half_sorted_rev<W: Semiring>(_arcs_a: &[Arc<W>], _arcs_b: &[Arc<W>]) -> Vec<ArcPair<W>> {
    Vec::new()
}
fn match_unsorted<W: Semiring>(_arcs_a: &[Arc<W>], _arcs_b: &[Arc<W>]) -> Vec<ArcPair<W>> {
    Vec::new()
}
fn match_arcs<W: Semiring>(_fst_a: &Fst<W>, _fst_b: &Fst<W>, _pair: &StatePair, _sr: &W) -> Vec<ArcPair<W>> {
    Vec::new()
}
pub fn fst_compose<W: Semiring>(_fst_a: &Fst<W>, _fst_b: &Fst<W>, _sr: &W) -> Fst<W> {
    Fst::new()
}
// ---- The actual implementation that matches the C library and is used by the FST API ----
pub fn fst_compose_inplace(fst_a: &RealFst, fst_b: &RealFst, fst_c: &mut RealFst) {
    // Reset destination
    fst_c.empty();
    fst_c.sr_type = fst_a.sr_type;
    let sr = sr_get(fst_a.sr_type);
    let mut q: VecDeque<Spair> = VecDeque::new();
    let mut marked: HashMap<(u32, u32), u32> = HashMap::new();
    let pair0 = Spair { a: fst_a.start, b: fst_b.start };
    q.push_back(pair0);
    while let Some(pair) = q.pop_front() {
        let key = (pair.a, pair.b);
        let sc = if let Some(&existing) = marked.get(&key) {
            existing
        } else {
            let state_a = &fst_a.states[pair.a as usize];
            let state_b = &fst_b.states[pair.b as usize];
            let new_sc = fst_c.add_state();
            if state_a.final_state && state_b.final_state {
                fst_c.set_final(new_sc, sr.one);
            }
            if pair.a == fst_a.start && pair.b == fst_b.start {
                fst_c.start = new_sc;
            }
            marked.insert(key, new_sc);
            new_sc
        };
        // Match arcs from state_a and state_b
        let mut mq: Queue<(ArcData, ArcData)> = Queue::new();
        match_arcs_internal(fst_a, fst_b, &pair, &sr, &mut mq);
        while let Some((arc_a, arc_b)) = mq.dequeue() {
            let dst_pair = (arc_a.state, arc_b.state);
            let dst_sc = if let Some(&existing) = marked.get(&dst_pair) {
                existing
            } else {
                let dst_state_a = &fst_a.states[dst_pair.0 as usize];
                let dst_state_b = &fst_b.states[dst_pair.1 as usize];
                let new_dst = fst_c.add_state();
                if dst_state_a.final_state && dst_state_b.final_state {
                    fst_c.set_final(new_dst, sr.one);
                }
                q.push_back(Spair { a: dst_pair.0, b: dst_pair.1 });
                marked.insert(dst_pair, new_dst);
                new_dst
            };
            fst_c.add_arc(sc, dst_sc, arc_a.ilabel, arc_b.olabel, (sr.prod)(arc_a.weight, arc_b.weight));
        }
    }
}
fn match_arcs_internal(
    fst_a: &RealFst,
    fst_b: &RealFst,
    pair: &Spair,
    sr: &crate::sr::Sr,
    mq: &mut Queue<(ArcData, ArcData)>,
) {
    let state_a = &fst_a.states[pair.a as usize];
    let state_b = &fst_b.states[pair.b as usize];
    let osort = (fst_a.flags & OSORT) != 0;
    let isort = (fst_b.flags & ISORT) != 0;
    let m = state_a.n_arcs + 1;
    let n = state_b.n_arcs + 1;
    let mut arcs_a: Vec<ArcData> = Vec::with_capacity(m as usize);
    let mut arcs_b: Vec<ArcData> = Vec::with_capacity(n as usize);
    arcs_a.push(ArcData { state: pair.a, ilabel: REAL_EPS, olabel: REAL_EPS, weight: sr.one });
    for arc in &state_a.arcs {
        arcs_a.push(*arc);
    }
    arcs_b.push(ArcData { state: pair.b, ilabel: REAL_EPS, olabel: REAL_EPS, weight: sr.one });
    for arc in &state_b.arcs {
        arcs_b.push(*arc);
    }
    if isort && osort {
        crate::fst::match_full_sorted(&arcs_a, &arcs_b, m, n, mq);
    } else if isort || osort {
        if isort {
            crate::fst::match_half_sorted(&arcs_a, &arcs_b, m, n, mq);
        } else {
            crate::fst::match_half_sorted_rev(&arcs_a, &arcs_b, m, n, mq);
        }
    } else {
        crate::fst::match_unsorted(&arcs_a, &arcs_b, m, n, mq);
    }
}
#[allow(dead_code)]
fn ensure_unused() {
    let _ = match_full_sorted::<FloatSemiring>;
    let _ = match_half_sorted::<FloatSemiring>;
    let _ = match_half_sorted_rev::<FloatSemiring>;
    let _ = match_unsorted::<FloatSemiring>;
    let _ = match_arcs::<FloatSemiring>;
    let _ = fst_compose::<FloatSemiring>;
}
