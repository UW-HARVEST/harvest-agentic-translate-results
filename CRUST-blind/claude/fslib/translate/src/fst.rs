use std::fs::File;
use std::io::{self, BufReader, Read, Write};
use crate::sr::{self, Sr};
use crate::bitset::BitSet;
use crate::queue::Queue;
use crate::symt::SymTable;
pub type State = u32;
pub type Arc = u32;
pub type Label = u32;
pub type Weight = f32;
pub const FST_HEADER: u32 = 0x66733031;
pub const ISORT: u8 = 0x01;
pub const OSORT: u8 = 0x02;
pub const EPS: u32 = 0;
pub const EPS_L: i32 = -1;
pub const START_STATE: &str = "<start>";

pub const SR_TROPICAL: u8 = 0;
pub const SR_REAL: u8 = 1;

pub struct Fst {
    pub start: State,
    pub n_states: State,
    pub n_max: State,
    pub sr_type: u8,
    pub flags: u8,
    pub states: Vec<StateData>,
}
pub struct StateData {
    pub n_arcs: Arc,
    pub n_max: Arc,
    pub weight: Weight,
    pub final_state: bool,
    pub arcs: Vec<ArcData>,
}
#[derive(Clone)]
pub struct ArcData {
    pub state: State,
    pub weight: Weight,
    pub ilabel: Label,
    pub olabel: Label,
}
pub struct Spair {
    pub a: State,
    pub b: State,
}
pub struct Striple {
    pub a: State,
    pub b: State,
    pub c: State,
}
pub struct Apair {
    pub a: Arc,
    pub b: Arc,
}
pub struct ArcPair {
    pub a: ArcData,
    pub b: ArcData,
}
pub struct MatchItem {
    pub a: ArcData,
    pub b: ArcData,
}
impl Fst {
    pub fn new() -> Self {
        Fst {
            start: 0,
            n_states: 0,
            n_max: 0,
            sr_type: SR_TROPICAL,
            flags: 0,
            states: Vec::new(),
        }
    }
    pub fn remove(&mut self) {
        self.empty();
    }
    pub fn empty(&mut self) {
        for state in self.states.iter_mut() {
            state.arcs.clear();
            state.n_arcs = 0;
            state.n_max = 0;
        }
        self.states.clear();
        self.n_states = 0;
        self.n_max = 0;
        self.start = 0;
    }
    pub fn add_state(&mut self) -> State {
        self.n_states += 1;
        if self.n_states > self.n_max {
            self.n_max = self.n_states * 2;
        }
        self.states.push(StateData {
            n_arcs: 0,
            n_max: 0,
            weight: 0.0,
            final_state: false,
            arcs: Vec::new(),
        });
        self.n_states - 1
    }
    pub fn add_arc(&mut self, src: State, dst: State, il: Label, ol: Label, weight: Weight) -> Arc {
        let state = &mut self.states[src as usize];
        state.n_arcs += 1;
        if state.n_arcs > state.n_max {
            state.n_max = state.n_arcs * 2;
        }
        state.arcs.push(ArcData {
            state: dst,
            ilabel: il,
            olabel: ol,
            weight,
        });
        state.n_arcs - 1
    }
    pub fn set_final(&mut self, s: State, w: Weight) {
        self.states[s as usize].final_state = true;
        self.states[s as usize].weight = w;
    }
    pub fn print(&self) {
        // print each arc
        let mut finals: Vec<State> = Vec::new();
        for s in 0..self.n_states {
            let state = &self.states[s as usize];
            for a in 0..state.n_arcs {
                let arc = &state.arcs[a as usize];
                println!(
                    "{}\t{}\t{}\t{}\t{:.5}",
                    s, arc.state, arc.ilabel, arc.olabel, arc.weight
                );
            }
            if state.final_state {
                finals.push(s);
            }
        }
        for s in finals {
            let state = &self.states[s as usize];
            println!("{}\t{}", s, state.weight);
        }
    }
    pub fn print_sym(&self, ist: &SymTable, ost: &SymTable, sst: &SymTable) {
        let mut finals: Vec<State> = Vec::new();
        for s in 0..self.n_states {
            let state = &self.states[s as usize];
            for a in 0..state.n_arcs {
                let arc = &state.arcs[a as usize];

                let sa = trans_or_id(sst, s as usize);
                let sb = trans_or_id(sst, arc.state as usize);
                let li = trans_or_id(ist, arc.ilabel as usize);
                let lo = trans_or_id(ost, arc.olabel as usize);
                println!(
                    "{}\t{}\t{}\t{}\t{:.5}",
                    sa, sb, li, lo, arc.weight
                );
            }
            if state.final_state {
                finals.push(s);
            }
        }
        for s in finals {
            let state = &self.states[s as usize];
            let sa = trans_or_id(sst, s as usize);
            println!("{}\t{}", sa, state.weight);
        }
    }
    pub fn write(&self, fout: &mut File) -> io::Result<()> {
        fout.write_all(&FST_HEADER.to_le_bytes())?;
        fout.write_all(&(self.start as u32).to_le_bytes())?;
        fout.write_all(&(self.n_states as u32).to_le_bytes())?;
        fout.write_all(&[self.sr_type])?;
        fout.write_all(&[self.flags])?;
        for s in 0..self.n_states {
            let state = &self.states[s as usize];
            fout.write_all(&state.weight.to_le_bytes())?;
            fout.write_all(&(state.n_arcs as u32).to_le_bytes())?;
            // C wrote sizeof(int) for final
            let final_int: i32 = if state.final_state { 1 } else { 0 };
            fout.write_all(&final_int.to_le_bytes())?;
            for a in 0..state.n_arcs {
                let arc = &state.arcs[a as usize];
                fout.write_all(&(arc.state as u32).to_le_bytes())?;
                fout.write_all(&arc.weight.to_le_bytes())?;
                fout.write_all(&(arc.ilabel as u32).to_le_bytes())?;
                fout.write_all(&(arc.olabel as u32).to_le_bytes())?;
            }
        }
        Ok(())
    }
    pub fn read(&mut self, fin: &mut File) -> io::Result<()> {
        let mut buf4 = [0u8; 4];
        let mut buf1 = [0u8; 1];
        fin.read_exact(&mut buf4)?;
        let header = u32::from_le_bytes(buf4);
        if header != FST_HEADER {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Wrong file format"));
        }
        fin.read_exact(&mut buf4)?;
        self.start = u32::from_le_bytes(buf4);
        fin.read_exact(&mut buf4)?;
        self.n_states = u32::from_le_bytes(buf4);
        fin.read_exact(&mut buf1)?;
        self.sr_type = buf1[0];
        fin.read_exact(&mut buf1)?;
        self.flags = buf1[0];
        self.n_max = self.n_states;

        self.states.clear();
        self.states.reserve(self.n_states as usize);

        for _ in 0..self.n_states {
            fin.read_exact(&mut buf4)?;
            let weight = f32::from_le_bytes(buf4);
            fin.read_exact(&mut buf4)?;
            let n_arcs = u32::from_le_bytes(buf4);
            fin.read_exact(&mut buf4)?;
            let final_int = i32::from_le_bytes(buf4);
            let final_state = final_int != 0;

            let mut state = StateData {
                n_arcs,
                n_max: n_arcs,
                weight,
                final_state,
                arcs: Vec::with_capacity(n_arcs as usize),
            };
            for _ in 0..n_arcs {
                fin.read_exact(&mut buf4)?;
                let st = u32::from_le_bytes(buf4);
                fin.read_exact(&mut buf4)?;
                let w = f32::from_le_bytes(buf4);
                fin.read_exact(&mut buf4)?;
                let il = u32::from_le_bytes(buf4);
                fin.read_exact(&mut buf4)?;
                let ol = u32::from_le_bytes(buf4);
                state.arcs.push(ArcData {
                    state: st,
                    weight: w,
                    ilabel: il,
                    olabel: ol,
                });
            }
            self.states.push(state);
        }
        Ok(())
    }
    pub fn fwrite(&self, filename: &str) -> io::Result<()> {
        let mut f = File::create(filename)?;
        self.write(&mut f)
    }
    pub fn fread(&mut self, filename: &str) -> io::Result<()> {
        let mut f = File::open(filename)?;
        self.read(&mut f)
    }
    pub fn compile(&mut self, fin: &mut File, ist: &SymTable, ost: &SymTable, sst: &SymTable, is_acc: bool) -> Self {
        let mut br = BufReader::new(fin);
        let _ = crate::compile::compile_internal(self, &mut br, Some(ist), Some(ost), Some(sst), is_acc);
        // Move out
        std::mem::replace(self, Fst::new())
    }
    pub fn compile_str(&mut self, str_data: &str) -> Self {
        crate::compile::compile_str_internal(self, str_data);
        std::mem::replace(self, Fst::new())
    }
    pub fn get_n_arcs(&self) -> Arc {
        let mut n: Arc = 0;
        for s in 0..self.n_states {
            n += self.states[s as usize].n_arcs;
        }
        n
    }
    pub fn arc_sort(&mut self, sort_outer: i32) {
        if sort_outer == 0 {
            self.flags |= ISORT;
            for state in self.states.iter_mut() {
                state.arcs.sort_by_key(|a| a.ilabel);
            }
        } else {
            self.flags |= OSORT;
            for state in self.states.iter_mut() {
                state.arcs.sort_by_key(|a| a.olabel);
            }
        }
    }
    pub fn stack(&mut self, other: &Fst) {
        let offset = self.n_states;
        self.n_states += other.n_states;
        if self.n_max < self.n_states {
            self.n_max = self.n_states;
        }
        for s in 0..other.n_states {
            let other_state = &other.states[s as usize];
            let mut new_arcs = Vec::with_capacity(other_state.arcs.len());
            for arc in &other_state.arcs {
                new_arcs.push(ArcData {
                    state: arc.state + offset,
                    weight: arc.weight,
                    ilabel: arc.ilabel,
                    olabel: arc.olabel,
                });
            }
            self.states.push(StateData {
                n_arcs: other_state.n_arcs,
                n_max: other_state.n_max,
                weight: other_state.weight,
                final_state: other_state.final_state,
                arcs: new_arcs,
            });
        }
    }
    pub fn union(&mut self, other: &Fst) -> Self {
        // Just stack 'b' onto 'a'.
        self.stack(other);
        std::mem::replace(self, Fst::new())
    }
    pub fn draw(&self, fout: &mut File) -> io::Result<i32> {
        crate::draw::fst_draw(self, fout)?;
        Ok(0)
    }
    pub fn draw_sym(&self, fout: &mut File, ist: &SymTable, ost: &SymTable, sst: &SymTable) -> io::Result<i32> {
        crate::draw::fst_draw_sym(self, fout, Some(ist), Some(ost), Some(sst))?;
        Ok(0)
    }
    pub fn copy(&self, copy: &mut Fst) {
        copy.start = self.start;
        copy.n_states = self.n_states;
        copy.n_max = self.n_max;
        copy.sr_type = self.sr_type;
        copy.flags = self.flags;
        copy.states.clear();
        copy.states.reserve(self.n_states as usize);
        for s in 0..self.n_states {
            let orig_state = &self.states[s as usize];
            let mut new_arcs = Vec::with_capacity(orig_state.arcs.len());
            for arc in &orig_state.arcs {
                new_arcs.push(arc.clone());
            }
            copy.states.push(StateData {
                n_arcs: orig_state.n_arcs,
                n_max: orig_state.n_max,
                weight: orig_state.weight,
                final_state: orig_state.final_state,
                arcs: new_arcs,
            });
        }
    }
    pub fn reverse(&mut self) {
        crate::trim::fst_reverse(self);
    }
    pub fn shortest(&self, path: &mut Fst) -> Self {
        crate::shortest::ShortestPath::find_shortest_path(self, path);
        // Return copy of path
        let mut out = Fst::new();
        path.copy(&mut out);
        out
    }
    pub fn rm_states(&mut self, visited: &BitSet) -> Self {
        crate::trim::fst_rm_states(self, visited);
        // Move self into a new Fst and replace self with empty
        std::mem::replace(self, Fst::new())
    }
    pub fn trim(&mut self) -> Self {
        crate::trim::fst_trim(self);
        std::mem::replace(self, Fst::new())
    }
    pub fn compose(&self, fst_b: &Fst, fst_c: &mut Fst) {
        compose_internal(self, fst_b, fst_c);
    }
    pub fn relabel(&mut self, old: Label, new: Label, dir: i32) {
        for state in self.states.iter_mut() {
            for arc in state.arcs.iter_mut() {
                if dir == 0 {
                    if arc.ilabel == old {
                        arc.ilabel = new;
                    }
                } else {
                    if arc.olabel == old {
                        arc.olabel = new;
                    }
                }
            }
        }
    }
}

fn trans_or_id(_st: &SymTable, id: usize) -> String {
    match _st.get(id as i32) {
        Some(t) => t.to_string(),
        None => format!("{}", id),
    }
}

fn arc_match(a: &[ArcData], _b: &[ArcData], i: usize, j: usize) -> bool {
    // mirrors C _match
    let al = a[i].olabel;
    if al == EPS {
        if (i != 0 && j != 0) || (i == 0 && j == 0) {
            return false;
        }
    }
    true
}

pub fn match_unsorted(a: &[ArcData], b: &[ArcData], m: Arc, n: Arc, q: &mut Queue<(ArcData, ArcData)>) {
    let m = m as usize;
    let n = n as usize;
    for i in 0..m {
        for j in 0..n {
            if a[i].olabel == b[j].ilabel && arc_match(a, b, i, j) {
                q.enqueue((a[i].clone(), b[j].clone()));
            }
        }
    }
}
pub fn match_half_sorted(a: &[ArcData], b: &[ArcData], m: Arc, n: Arc, q: &mut Queue<(ArcData, ArcData)>) {
    let m = m as usize;
    let n = n as usize;
    if n == 0 {
        return;
    }
    for i in 0..m {
        let mut l: usize = 0;
        let mut h: usize = n - 1;
        loop {
            if l > h {
                break;
            }
            let mid = (l + h) >> 1;
            if a[i].olabel > b[mid].ilabel {
                l = mid + 1;
            } else if a[i].olabel < b[mid].ilabel {
                if mid == 0 {
                    break;
                }
                h = mid - 1;
            } else {
                let mut ll = mid;
                let mut hh = mid;
                while ll > l && a[i].olabel == b[ll - 1].ilabel {
                    ll -= 1;
                }
                while hh < h && a[i].olabel == b[hh + 1].ilabel {
                    hh += 1;
                }
                while ll <= hh {
                    if arc_match(a, b, i, ll) {
                        q.enqueue((a[i].clone(), b[ll].clone()));
                    }
                    ll += 1;
                }
                break;
            }
        }
    }
}
pub fn match_half_sorted_rev(a: &[ArcData], b: &[ArcData], m: Arc, n: Arc, q: &mut Queue<(ArcData, ArcData)>) {
    let m = m as usize;
    let n = n as usize;
    if m == 0 {
        return;
    }
    for i in 0..n {
        let mut l: usize = 0;
        let mut h: usize = m - 1;
        loop {
            if l > h {
                break;
            }
            let mid = (l + h) >> 1;
            if b[i].ilabel > a[mid].olabel {
                l = mid + 1;
            } else if b[i].ilabel < a[mid].olabel {
                if mid == 0 {
                    break;
                }
                h = mid - 1;
            } else {
                let mut ll = mid;
                let mut hh = mid;
                while ll > l && b[i].ilabel == a[ll - 1].olabel {
                    ll -= 1;
                }
                while hh < h && b[i].ilabel == a[hh + 1].olabel {
                    hh += 1;
                }
                while ll <= hh {
                    if arc_match(a, b, ll, i) {
                        q.enqueue((a[ll].clone(), b[i].clone()));
                    }
                    ll += 1;
                }
                break;
            }
        }
    }
}
pub fn match_full_sorted(a: &[ArcData], b: &[ArcData], m: Arc, n: Arc, q: &mut Queue<(ArcData, ArcData)>) {
    let m = m as usize;
    let n = n as usize;
    let mut i: usize = 0;
    let mut j: usize = 0;
    while i < m && j < n {
        if a[i].olabel < b[j].ilabel {
            i += 1;
        } else if a[i].olabel > b[j].ilabel {
            j += 1;
        } else {
            let mut t = j;
            while t < n && a[i].olabel == b[t].ilabel {
                if arc_match(a, b, i, t) {
                    q.enqueue((a[i].clone(), b[t].clone()));
                }
                t += 1;
            }
            i += 1;
        }
    }
}
pub fn match_arcs(fst_a: &Fst, fst_b: &Fst, pair: &Spair, sr: &Sr, mq: &mut Queue<(ArcData, ArcData)>) {
    let state_a = &fst_a.states[pair.a as usize];
    let state_b = &fst_b.states[pair.b as usize];

    let osort = (fst_a.flags & OSORT) != 0;
    let isort = (fst_b.flags & ISORT) != 0;

    let m = state_a.n_arcs + 1;
    let n = state_b.n_arcs + 1;

    let mut arcs_a: Vec<ArcData> = Vec::with_capacity(m as usize);
    let mut arcs_b: Vec<ArcData> = Vec::with_capacity(n as usize);

    arcs_a.push(ArcData {
        state: pair.a,
        ilabel: EPS,
        olabel: EPS,
        weight: sr.one,
    });
    arcs_b.push(ArcData {
        state: pair.b,
        ilabel: EPS,
        olabel: EPS,
        weight: sr.one,
    });
    for arc in &state_a.arcs {
        arcs_a.push(arc.clone());
    }
    for arc in &state_b.arcs {
        arcs_b.push(arc.clone());
    }

    if isort && osort {
        match_full_sorted(&arcs_a, &arcs_b, m, n, mq);
    } else if isort || osort {
        if isort {
            match_half_sorted(&arcs_a, &arcs_b, m, n, mq);
        } else {
            match_half_sorted_rev(&arcs_a, &arcs_b, m, n, mq);
        }
    } else {
        match_unsorted(&arcs_a, &arcs_b, m, n, mq);
    }
}

fn compose_internal(fst_a: &Fst, fst_b: &Fst, fst_c: &mut Fst) {
    use std::collections::HashMap;
    let sr = sr::sr_get(fst_a.sr_type);
    let mut q: Queue<Spair> = Queue::new();
    let mut mq: Queue<(ArcData, ArcData)> = Queue::new();

    let mut marked: HashMap<(State, State), State> = HashMap::new();

    let mut pair = Spair { a: fst_a.start, b: fst_b.start };
    q.enqueue(pair);

    while let Some(p) = q.dequeue() {
        pair = p;

        let state_a = &fst_a.states[pair.a as usize];
        let state_b = &fst_b.states[pair.b as usize];

        let sc = match marked.get(&(pair.a, pair.b)).copied() {
            Some(v) => v,
            None => {
                let new_sc = fst_c.add_state();
                if state_a.final_state && state_b.final_state {
                    fst_c.set_final(new_sc, sr.one);
                }
                if pair.a == fst_a.start && pair.b == fst_b.start {
                    fst_c.start = new_sc;
                }
                marked.insert((pair.a, pair.b), new_sc);
                new_sc
            }
        };

        match_arcs(fst_a, fst_b, &pair, &sr, &mut mq);

        while let Some((arc_a, arc_b)) = mq.dequeue() {
            let new_pair = Spair { a: arc_a.state, b: arc_b.state };

            let dst_sc = match marked.get(&(new_pair.a, new_pair.b)).copied() {
                Some(v) => v,
                None => {
                    let dst_state_a = &fst_a.states[new_pair.a as usize];
                    let dst_state_b = &fst_b.states[new_pair.b as usize];
                    let dst_sc = fst_c.add_state();
                    if dst_state_a.final_state && dst_state_b.final_state {
                        fst_c.set_final(dst_sc, sr.one);
                    }
                    q.enqueue(Spair { a: new_pair.a, b: new_pair.b });
                    marked.insert((new_pair.a, new_pair.b), dst_sc);
                    dst_sc
                }
            };

            let combined_weight = (sr.prod)(arc_a.weight, arc_b.weight);
            fst_c.add_arc(sc, dst_sc, arc_a.ilabel, arc_b.olabel, combined_weight);
        }
    }
}
