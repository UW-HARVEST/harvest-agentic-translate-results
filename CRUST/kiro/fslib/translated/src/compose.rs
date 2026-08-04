use std::collections::{HashMap, VecDeque};
use std::hash::{Hash, Hasher};
const OSORT: u32 = 0x1;
const ISORT: u32 = 0x2;
pub trait Semiring: Clone {
    fn zero() -> Self;
    fn one() -> Self;
    fn plus(&self, rhs: &Self) -> Self;
    fn prod(&self, rhs: &Self) -> Self;
}
#[derive(Clone, Debug)]
pub struct FloatSemiring(pub f32);
impl Semiring for FloatSemiring {
    fn zero() -> Self { FloatSemiring(f32::MAX) }
    fn one() -> Self { FloatSemiring(0.0) }
    fn plus(&self, rhs: &Self) -> Self {
        FloatSemiring(if self.0 < rhs.0 { self.0 } else { rhs.0 })
    }
    fn prod(&self, rhs: &Self) -> Self {
        FloatSemiring(self.0 + rhs.0)
    }
}
#[derive(Clone, Debug)]
pub struct Arc<W: Semiring> {
    pub state: usize,
    pub ilabel: u32,
    pub olabel: u32,
    pub weight: W,
}
#[derive(Clone, Debug)]
pub struct State<W: Semiring> {
    pub arcs: Vec<Arc<W>>,
    pub final_weight: Option<W>,
}
#[derive(Clone, Debug)]
pub struct Fst<W: Semiring> {
    pub states: Vec<State<W>>,
    pub start: usize,
    pub flags: u32,
}
impl<W: Semiring> Fst<W> {
    pub fn new() -> Self {
        Fst { states: Vec::new(), start: 0, flags: 0 }
    }
    pub fn add_state(&mut self) -> usize {
        let id = self.states.len();
        self.states.push(State { arcs: Vec::new(), final_weight: None });
        id
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

fn _match_compose<W: Semiring>(a: &[Arc<W>], _b: &[Arc<W>], i: usize, j: usize) -> bool {
    let al = a[i].olabel;
    if al == EPS {
        if (i != 0 && j != 0) || (i == 0 && j == 0) {
            return false;
        }
    }
    true
}

fn match_full_sorted<W: Semiring>(arcs_a: &[Arc<W>], arcs_b: &[Arc<W>]) -> Vec<ArcPair<W>> {
    let mut result = Vec::new();
    let m = arcs_a.len();
    let n = arcs_b.len();
    let mut i = 0;
    let mut j = 0;
    while i < m && j < n {
        if arcs_a[i].olabel < arcs_b[j].ilabel { i += 1; }
        else if arcs_a[i].olabel > arcs_b[j].ilabel { j += 1; }
        else {
            let mut t = j;
            while t < n && arcs_a[i].olabel == arcs_b[t].ilabel {
                if _match_compose(arcs_a, arcs_b, i, t) {
                    result.push(ArcPair { a: arcs_a[i].clone(), b: arcs_b[t].clone() });
                }
                t += 1;
            }
            i += 1;
        }
    }
    result
}
fn match_half_sorted<W: Semiring>(arcs_a: &[Arc<W>], arcs_b: &[Arc<W>]) -> Vec<ArcPair<W>> {
    let mut result = Vec::new();
    let n = arcs_b.len();
    for i in 0..arcs_a.len() {
        if n == 0 { continue; }
        let mut l = 0usize;
        let mut h = n - 1;
        while l <= h {
            let mid = (l + h) >> 1;
            if arcs_a[i].olabel > arcs_b[mid].ilabel { l = mid + 1; }
            else if arcs_a[i].olabel < arcs_b[mid].ilabel {
                if mid == 0 { break; }
                h = mid - 1;
            } else {
                let mut ll = mid;
                let mut hh = mid;
                while ll > l && arcs_a[i].olabel == arcs_b[ll-1].ilabel { ll -= 1; }
                while hh < h && arcs_a[i].olabel == arcs_b[hh+1].ilabel { hh += 1; }
                while ll <= hh {
                    if _match_compose(arcs_a, arcs_b, i, ll) {
                        result.push(ArcPair { a: arcs_a[i].clone(), b: arcs_b[ll].clone() });
                    }
                    ll += 1;
                }
                break;
            }
        }
    }
    result
}
fn match_half_sorted_rev<W: Semiring>(arcs_a: &[Arc<W>], arcs_b: &[Arc<W>]) -> Vec<ArcPair<W>> {
    let mut result = Vec::new();
    let m = arcs_a.len();
    for i in 0..arcs_b.len() {
        if m == 0 { continue; }
        let mut l = 0usize;
        let mut h = m - 1;
        while l <= h {
            let mid = (l + h) >> 1;
            if arcs_b[i].ilabel > arcs_a[mid].olabel { l = mid + 1; }
            else if arcs_b[i].ilabel < arcs_a[mid].olabel {
                if mid == 0 { break; }
                h = mid - 1;
            } else {
                let mut ll = mid;
                let mut hh = mid;
                while ll > l && arcs_b[i].ilabel == arcs_a[ll-1].olabel { ll -= 1; }
                while hh < h && arcs_b[i].ilabel == arcs_a[hh+1].olabel { hh += 1; }
                while ll <= hh {
                    if _match_compose(arcs_a, arcs_b, ll, i) {
                        result.push(ArcPair { a: arcs_a[ll].clone(), b: arcs_b[i].clone() });
                    }
                    ll += 1;
                }
                break;
            }
        }
    }
    result
}
fn match_unsorted<W: Semiring>(arcs_a: &[Arc<W>], arcs_b: &[Arc<W>]) -> Vec<ArcPair<W>> {
    let mut result = Vec::new();
    for i in 0..arcs_a.len() {
        for j in 0..arcs_b.len() {
            if arcs_a[i].olabel == arcs_b[j].ilabel && _match_compose(arcs_a, arcs_b, i, j) {
                result.push(ArcPair { a: arcs_a[i].clone(), b: arcs_b[j].clone() });
            }
        }
    }
    result
}
fn match_arcs<W: Semiring>(fst_a: &Fst<W>, fst_b: &Fst<W>, pair: &StatePair, sr: &W) -> Vec<ArcPair<W>> {
    let state_a = &fst_a.states[pair.a];
    let state_b = &fst_b.states[pair.b];
    let osort = fst_a.flags & OSORT != 0;
    let isort = fst_b.flags & ISORT != 0;
    let loop_a = Arc { state: pair.a, ilabel: EPS, olabel: EPS, weight: W::one() };
    let loop_b = Arc { state: pair.b, ilabel: EPS, olabel: EPS, weight: W::one() };
    let mut arcs_a = vec![loop_a];
    arcs_a.extend(state_a.arcs.iter().cloned());
    let mut arcs_b = vec![loop_b];
    arcs_b.extend(state_b.arcs.iter().cloned());
    if isort && osort { match_full_sorted(&arcs_a, &arcs_b) }
    else if isort { match_half_sorted(&arcs_a, &arcs_b) }
    else if osort { match_half_sorted_rev(&arcs_a, &arcs_b) }
    else { match_unsorted(&arcs_a, &arcs_b) }
}
pub fn fst_compose<W: Semiring>(fst_a: &Fst<W>, fst_b: &Fst<W>, sr: &W) -> Fst<W> {
    let mut fst_c = Fst::new();
    let mut q: VecDeque<StatePair> = VecDeque::new();
    let mut marked: HashMap<StatePair, usize> = HashMap::new();
    let pair = StatePair { a: fst_a.start, b: fst_b.start };
    q.push_back(pair);
    while let Some(pair) = q.pop_front() {
        let state_a = &fst_a.states[pair.a];
        let state_b = &fst_b.states[pair.b];
        let sc = if let Some(&sc) = marked.get(&pair) { sc } else {
            let sc = fst_c.add_state();
            if state_a.final_weight.is_some() && state_b.final_weight.is_some() {
                fst_c.set_final(sc, W::one());
            }
            if pair.a == fst_a.start && pair.b == fst_b.start { fst_c.start = sc; }
            marked.insert(pair, sc);
            sc
        };
        let matches = match_arcs(fst_a, fst_b, &pair, sr);
        for mi in matches {
            let dst_pair = StatePair { a: mi.a.state, b: mi.b.state };
            let dst_sc = if let Some(&dst_sc) = marked.get(&dst_pair) { dst_sc } else {
                let dst_state_a = &fst_a.states[dst_pair.a];
                let dst_state_b = &fst_b.states[dst_pair.b];
                let dst_sc = fst_c.add_state();
                if dst_state_a.final_weight.is_some() && dst_state_b.final_weight.is_some() {
                    fst_c.set_final(dst_sc, W::one());
                }
                q.push_back(dst_pair);
                marked.insert(dst_pair, dst_sc);
                dst_sc
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
