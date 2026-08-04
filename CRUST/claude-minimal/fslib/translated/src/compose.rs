use std::collections::{HashMap, VecDeque};
use std::hash::{Hash, Hasher};
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

fn arc_match<W: Semiring>(a: &[Arc<W>], _b: &[Arc<W>], i: usize, j: usize) -> bool {
    let al = a[i].olabel;
    if al == EPS {
        if (i != 0 && j != 0) || (i == 0 && j == 0) {
            return false;
        }
    }
    true
}

fn match_full_sorted<W: Semiring>(arcs_a: &[Arc<W>], arcs_b: &[Arc<W>]) -> Vec<ArcPair<W>> {
    let mut out: Vec<ArcPair<W>> = Vec::new();
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
                    out.push(ArcPair { a: arcs_a[i].clone(), b: arcs_b[t].clone() });
                }
                t += 1;
            }
            i += 1;
        }
    }
    out
}
fn match_half_sorted<W: Semiring>(arcs_a: &[Arc<W>], arcs_b: &[Arc<W>]) -> Vec<ArcPair<W>> {
    let mut out: Vec<ArcPair<W>> = Vec::new();
    let m = arcs_a.len() as i64;
    let n = arcs_b.len() as i64;
    for i in 0..m as usize {
        let mut l: i64 = 0;
        let mut h: i64 = n - 1;
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
                while ll <= hh {
                    if arc_match(arcs_a, arcs_b, i, ll) {
                        out.push(ArcPair { a: arcs_a[i].clone(), b: arcs_b[ll].clone() });
                    }
                    ll += 1;
                }
                break;
            }
        }
    }
    out
}
fn match_half_sorted_rev<W: Semiring>(arcs_a: &[Arc<W>], arcs_b: &[Arc<W>]) -> Vec<ArcPair<W>> {
    let mut out: Vec<ArcPair<W>> = Vec::new();
    let m = arcs_a.len() as i64;
    let n = arcs_b.len() as i64;
    for i in 0..n as usize {
        let mut l: i64 = 0;
        let mut h: i64 = m - 1;
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
                while ll <= hh {
                    if arc_match(arcs_a, arcs_b, ll, i) {
                        out.push(ArcPair { a: arcs_a[ll].clone(), b: arcs_b[i].clone() });
                    }
                    ll += 1;
                }
                break;
            }
        }
    }
    out
}
fn match_unsorted<W: Semiring>(arcs_a: &[Arc<W>], arcs_b: &[Arc<W>]) -> Vec<ArcPair<W>> {
    let mut out: Vec<ArcPair<W>> = Vec::new();
    let m = arcs_a.len();
    let n = arcs_b.len();
    for i in 0..m {
        for j in 0..n {
            if arcs_a[i].olabel == arcs_b[j].ilabel && arc_match(arcs_a, arcs_b, i, j) {
                out.push(ArcPair { a: arcs_a[i].clone(), b: arcs_b[j].clone() });
            }
        }
    }
    out
}
fn match_arcs<W: Semiring>(
    fst_a: &Fst<W>,
    fst_b: &Fst<W>,
    pair: &StatePair,
    _sr: &W,
) -> Vec<ArcPair<W>> {
    let state_a = &fst_a.states[pair.a];
    let state_b = &fst_b.states[pair.b];

    let osort = (fst_a.flags & OSORT) != 0;
    let isort = (fst_b.flags & ISORT) != 0;

    // Prepend epsilon self-loop arcs
    let mut arcs_a: Vec<Arc<W>> = Vec::with_capacity(state_a.arcs.len() + 1);
    arcs_a.push(Arc { state: pair.a, ilabel: EPS, olabel: EPS, weight: W::one() });
    for arc in &state_a.arcs {
        arcs_a.push(arc.clone());
    }
    let mut arcs_b: Vec<Arc<W>> = Vec::with_capacity(state_b.arcs.len() + 1);
    arcs_b.push(Arc { state: pair.b, ilabel: EPS, olabel: EPS, weight: W::one() });
    for arc in &state_b.arcs {
        arcs_b.push(arc.clone());
    }

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
    let mut q: VecDeque<StatePair> = VecDeque::new();
    let mut marked: HashMap<StatePair, usize> = HashMap::new();

    let init = StatePair { a: fst_a.start, b: fst_b.start };
    q.push_back(init);

    while let Some(pair) = q.pop_front() {
        let state_a = &fst_a.states[pair.a];
        let state_b = &fst_b.states[pair.b];

        let sc = if let Some(&existing) = marked.get(&pair) {
            existing
        } else {
            let new_sc = fst_c.add_state();
            if state_a.final_weight.is_some() && state_b.final_weight.is_some() {
                fst_c.set_final(new_sc, W::one());
            }
            if pair.a == fst_a.start && pair.b == fst_b.start {
                fst_c.start = new_sc;
            }
            marked.insert(pair, new_sc);
            new_sc
        };

        let pairs = match_arcs(fst_a, fst_b, &pair, sr);
        for ap in pairs {
            let new_pair = StatePair { a: ap.a.state, b: ap.b.state };
            let dst_sc = if let Some(&existing) = marked.get(&new_pair) {
                existing
            } else {
                let dst_state_a = &fst_a.states[new_pair.a];
                let dst_state_b = &fst_b.states[new_pair.b];
                let new_dst_sc = fst_c.add_state();
                if dst_state_a.final_weight.is_some() && dst_state_b.final_weight.is_some() {
                    fst_c.set_final(new_dst_sc, W::one());
                }
                q.push_back(new_pair);
                marked.insert(new_pair, new_dst_sc);
                new_dst_sc
            };
            let combined = ap.a.weight.prod(&ap.b.weight);
            fst_c.add_arc(sc, Arc {
                state: dst_sc,
                ilabel: ap.a.ilabel,
                olabel: ap.b.olabel,
                weight: combined,
            });
        }
    }

    fst_c
}
