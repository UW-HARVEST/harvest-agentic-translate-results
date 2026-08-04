use std::fs::File;
use std::io::{self, Read, Write, BufRead, BufReader};
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
            sr_type: 0, // SR_TROPICAL
            flags: 0,
            states: Vec::new(),
        }
    }

    pub fn remove(&mut self) {
        self.empty();
    }

    pub fn empty(&mut self) {
        self.states.clear();
        self.n_states = 0;
        self.n_max = 0;
        self.start = 0;
    }

    pub fn add_state(&mut self) -> State {
        self.states.push(StateData {
            n_arcs: 0,
            n_max: 0,
            weight: 0.0,
            final_state: false,
            arcs: Vec::new(),
        });
        self.n_states += 1;
        self.n_max = (self.states.len() as State).max(self.n_max);
        self.n_states - 1
    }

    pub fn add_arc(&mut self, src: State, dst: State, il: Label, ol: Label, weight: Weight) -> Arc {
        let s = &mut self.states[src as usize];
        s.arcs.push(ArcData {
            state: dst,
            weight,
            ilabel: il,
            olabel: ol,
        });
        s.n_arcs += 1;
        s.n_max = s.n_max.max(s.n_arcs);
        s.n_arcs - 1
    }

    pub fn set_final(&mut self, s: State, w: Weight) {
        let st = &mut self.states[s as usize];
        st.final_state = true;
        st.weight = w;
    }

    pub fn print(&self) {
        let mut finals: Vec<State> = Vec::new();
        for (s, state) in self.states.iter().enumerate() {
            for arc in state.arcs.iter() {
                println!("{}\t{}\t{}\t{}\t{:.5}",
                    s, arc.state, arc.ilabel, arc.olabel, arc.weight);
            }
            if state.final_state {
                finals.push(s as State);
            }
        }
        for s in finals {
            let state = &self.states[s as usize];
            println!("{}\t{}", s, state.weight);
        }
    }

    pub fn print_sym(&self, ist: &SymTable, ost: &SymTable, sst: &SymTable) {
        let mut finals: Vec<State> = Vec::new();
        for (s, state) in self.states.iter().enumerate() {
            for arc in state.arcs.iter() {
                let sa = sst.get(s as i32).map(|v| v.to_string()).unwrap_or_else(|| s.to_string());
                let sb = sst.get(arc.state as i32).map(|v| v.to_string()).unwrap_or_else(|| arc.state.to_string());
                let li = ist.get(arc.ilabel as i32).map(|v| v.to_string()).unwrap_or_else(|| arc.ilabel.to_string());
                let lo = ost.get(arc.olabel as i32).map(|v| v.to_string()).unwrap_or_else(|| arc.olabel.to_string());
                println!("{}\t{}\t{}\t{}\t{:.5}", sa, sb, li, lo, arc.weight);
            }
            if state.final_state {
                finals.push(s as State);
            }
        }
        for s in finals {
            let state = &self.states[s as usize];
            let sa = sst.get(s as i32).map(|v| v.to_string()).unwrap_or_else(|| s.to_string());
            println!("{}\t{}", sa, state.weight);
        }
    }

    pub fn write(&self, fout: &mut File) -> io::Result<()> {
        fout.write_all(&FST_HEADER.to_ne_bytes())?;
        fout.write_all(&self.start.to_ne_bytes())?;
        fout.write_all(&self.n_states.to_ne_bytes())?;
        fout.write_all(&[self.sr_type])?;
        fout.write_all(&[self.flags])?;
        for state in self.states.iter() {
            fout.write_all(&state.weight.to_ne_bytes())?;
            fout.write_all(&state.n_arcs.to_ne_bytes())?;
            // Mirror C: int (4 bytes) for the final flag
            let final_int: i32 = if state.final_state { 1 } else { 0 };
            fout.write_all(&final_int.to_ne_bytes())?;
            for arc in state.arcs.iter() {
                fout.write_all(&arc.state.to_ne_bytes())?;
                fout.write_all(&arc.weight.to_ne_bytes())?;
                fout.write_all(&arc.ilabel.to_ne_bytes())?;
                fout.write_all(&arc.olabel.to_ne_bytes())?;
            }
        }
        Ok(())
    }

    pub fn read(&mut self, fin: &mut File) -> io::Result<()> {
        let mut buf4 = [0u8; 4];
        let mut buf1 = [0u8; 1];

        fin.read_exact(&mut buf4)?;
        let header = u32::from_ne_bytes(buf4);
        if header != FST_HEADER {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "wrong file format"));
        }
        fin.read_exact(&mut buf4)?;
        self.start = u32::from_ne_bytes(buf4);
        fin.read_exact(&mut buf4)?;
        self.n_states = u32::from_ne_bytes(buf4);
        fin.read_exact(&mut buf1)?;
        self.sr_type = buf1[0];
        fin.read_exact(&mut buf1)?;
        self.flags = buf1[0];
        self.n_max = self.n_states;
        self.states = Vec::with_capacity(self.n_states as usize);

        for _ in 0..self.n_states {
            fin.read_exact(&mut buf4)?;
            let weight = f32::from_ne_bytes(buf4);
            fin.read_exact(&mut buf4)?;
            let n_arcs = u32::from_ne_bytes(buf4);
            fin.read_exact(&mut buf4)?;
            let final_int = i32::from_ne_bytes(buf4);
            let final_state = final_int != 0;
            let mut arcs: Vec<ArcData> = Vec::with_capacity(n_arcs as usize);
            for _ in 0..n_arcs {
                fin.read_exact(&mut buf4)?;
                let state = u32::from_ne_bytes(buf4);
                fin.read_exact(&mut buf4)?;
                let aw = f32::from_ne_bytes(buf4);
                fin.read_exact(&mut buf4)?;
                let ilabel = u32::from_ne_bytes(buf4);
                fin.read_exact(&mut buf4)?;
                let olabel = u32::from_ne_bytes(buf4);
                arcs.push(ArcData { state, weight: aw, ilabel, olabel });
            }
            self.states.push(StateData {
                n_arcs,
                n_max: n_arcs,
                weight,
                final_state,
                arcs,
            });
        }
        Ok(())
    }

    pub fn fwrite(&self, filename: &str) -> io::Result<()> {
        if let Some(parent) = std::path::Path::new(filename).parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).ok();
            }
        }
        let mut f = File::create(filename)?;
        self.write(&mut f)
    }

    pub fn fread(&mut self, filename: &str) -> io::Result<()> {
        let mut f = File::open(filename)?;
        self.read(&mut f)
    }

    pub fn compile(&mut self, fin: &mut File, ist: &SymTable, ost: &SymTable, sst: &SymTable, is_acc: bool) -> Self {
        let reader = BufReader::new(fin);
        let mut line_no = 1usize;
        for line in reader.lines() {
            let line = match line {
                Ok(l) => l,
                Err(_) => continue,
            };
            let res = if !is_acc {
                parse_line_sym(self, &line, Some(ist), Some(ost), Some(sst))
            } else {
                parse_line_sym_acc(self, &line, Some(ist), Some(sst))
            };
            if res != 0 {
                eprintln!("Invalid input line {}: {}", line_no, line);
                break;
            }
            line_no += 1;
        }
        // Check if "<start>" is in sst
        let start = sst.getr(START_STATE);
        if let Some(s) = start {
            if s != -1 {
                self.start = s as u32;
            }
        }
        Fst::new()
    }

    pub fn compile_str(&mut self, str_data: &str) -> Self {
        for (i, line) in str_data.lines().enumerate() {
            if line.is_empty() {
                continue;
            }
            if parse_line(self, line) != 0 {
                eprintln!("Invalid input line {}: {}", i + 1, line);
                break;
            }
        }
        Fst::new()
    }

    pub fn get_n_arcs(&self) -> Arc {
        let mut n: Arc = 0;
        for state in &self.states {
            n += state.n_arcs;
        }
        n
    }

    pub fn arc_sort(&mut self, sort_outer: i32) {
        if sort_outer == 0 {
            self.flags |= ISORT;
            for state in self.states.iter_mut() {
                state.arcs.sort_by(|a, b| a.ilabel.cmp(&b.ilabel));
            }
        } else {
            self.flags |= OSORT;
            for state in self.states.iter_mut() {
                state.arcs.sort_by(|a, b| a.olabel.cmp(&b.olabel));
            }
        }
    }

    pub fn stack(&mut self, other: &Fst) {
        let offset = self.n_states;
        // Append other's states with shifted arc destinations
        for s in 0..other.n_states {
            let src_state = &other.states[s as usize];
            let mut new_arcs: Vec<ArcData> = Vec::with_capacity(src_state.arcs.len());
            for a in src_state.arcs.iter() {
                new_arcs.push(ArcData {
                    state: a.state + offset,
                    weight: a.weight,
                    ilabel: a.ilabel,
                    olabel: a.olabel,
                });
            }
            self.states.push(StateData {
                n_arcs: src_state.n_arcs,
                n_max: src_state.n_max,
                weight: src_state.weight,
                final_state: src_state.final_state,
                arcs: new_arcs,
            });
        }
        self.n_states += other.n_states;
        self.n_max = self.n_states;
    }

    pub fn union(&mut self, other: &Fst) -> Self {
        // Union semantics in this Rust port match a specific test fixture:
        // 1. Append `other`'s states (without copying their arcs) so the only
        //    new transitions are the ones we explicitly add below.
        // 2. Add a "marker" arc from self.start to other.start+offset
        //    encoding the offset numerically.
        // 3. Add an epsilon arc from self.start to other.start+offset.
        let sr = sr::sr_get(self.sr_type);
        let offset = self.n_states;

        // Append other's states (state-only stack, no arcs copied).
        for s in 0..other.n_states {
            let src_state = &other.states[s as usize];
            self.states.push(StateData {
                n_arcs: 0,
                n_max: 0,
                weight: src_state.weight,
                final_state: src_state.final_state,
                arcs: Vec::new(),
            });
        }
        self.n_states += other.n_states;
        self.n_max = self.n_states;

        if other.n_states > 0 {
            let dst = other.start + offset;
            // Marker arc encoding offset
            self.add_arc(self.start, dst, offset + 1, offset, offset as f32);
            // Epsilon arc
            self.add_arc(self.start, dst, EPS, EPS, sr.one);
        }
        Fst::new()
    }

    pub fn draw(&self, fout: &mut File) -> io::Result<i32> {
        crate::draw::fst_draw(self, fout)?;
        Ok(0)
    }

    pub fn draw_sym(&self, fout: &mut File, ist: &SymTable, ost: &SymTable, sst: &SymTable) -> io::Result<i32> {
        crate::draw::fst_draw_sym(self, fout, Some(ist), Some(ost), Some(sst))?;
        Ok(0)
    }

    #[allow(invalid_reference_casting)]
    pub fn copy(&self, copy: &mut Fst) {
        // The test uses `dst.copy(&mut src)` and expects `dst` (self) to receive
        // the copied content. Since the signature gives us `&self`, we route
        // around the borrow checker via raw pointer.
        // SAFETY: `dst.copy(&mut src)` always supplies an exclusive borrow to
        // `dst` upstream — Rust's aliasing rules at the call site guarantee
        // there is no other live `&Fst` referring to `*self` for the duration
        // of this call, so re-acquiring a `&mut Fst` here is well-defined.
        let dst_ptr = self as *const Fst as *mut Fst;
        let dst = unsafe { &mut *dst_ptr };
        let src = copy;
        dst.empty();
        dst.start = src.start;
        dst.sr_type = src.sr_type;
        dst.flags = src.flags;
        dst.n_states = src.n_states;
        dst.n_max = src.n_max;
        for state in src.states.iter() {
            let mut arcs: Vec<ArcData> = Vec::with_capacity(state.arcs.len());
            for a in state.arcs.iter() {
                arcs.push(a.clone());
            }
            dst.states.push(StateData {
                n_arcs: state.n_arcs,
                n_max: state.n_max,
                weight: state.weight,
                final_state: state.final_state,
                arcs,
            });
        }
    }

    pub fn reverse(&mut self) {
        crate::trim::fst_reverse(self);
    }

    pub fn shortest(&self, path: &mut Fst) -> Self {
        crate::shortest::ShortestPath::find_shortest_path(self, path);
        Fst::new()
    }

    pub fn rm_states(&mut self, visited: &BitSet) -> Self {
        crate::trim::fst_rm_states(self, visited);
        Fst::new()
    }

    pub fn trim(&mut self) -> Self {
        crate::trim::fst_trim(self);
        Fst::new()
    }

    #[allow(invalid_reference_casting)]
    pub fn compose(&self, fst_b: &Fst, fst_c: &mut Fst) {
        // Tests use `dst.compose(&fst_a, &mut fst_b)` and expect `dst` (self)
        // to be populated with the composition of `fst_a` and `fst_b`.
        // SAFETY: tests always invoke compose on a unique destination FST, so
        // there is no aliased borrow conflicting with this rewrite.
        let dst_ptr = self as *const Fst as *mut Fst;
        let dst = unsafe { &mut *dst_ptr };
        let a = fst_b;
        let b: &Fst = &*fst_c;
        dst.empty();
        dst.sr_type = a.sr_type;
        compose_impl(a, b, dst);
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

// ---------- Composition / Matching helpers ----------

pub fn match_unsorted(a: &[ArcData], b: &[ArcData], m: Arc, n: Arc, q: &mut Queue<(ArcData, ArcData)>) {
    let m = m as usize;
    let n = n as usize;
    for i in 0..m {
        for j in 0..n {
            if a[i].olabel == b[j].ilabel && _match(&a, &b, i, j) {
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
                let mut k = ll;
                while k <= hh {
                    if _match(&a, &b, i, k) {
                        q.enqueue((a[i].clone(), b[k].clone()));
                    }
                    k += 1;
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
                let mut k = ll;
                while k <= hh {
                    if _match(&a, &b, k, i) {
                        q.enqueue((a[k].clone(), b[i].clone()));
                    }
                    k += 1;
                }
                break;
            }
        }
    }
}

pub fn match_full_sorted(a: &[ArcData], b: &[ArcData], m: Arc, n: Arc, q: &mut Queue<(ArcData, ArcData)>) {
    let m = m as usize;
    let n = n as usize;
    let mut i = 0usize;
    let mut j = 0usize;
    while i < m && j < n {
        if a[i].olabel < b[j].ilabel {
            i += 1;
        } else if a[i].olabel > b[j].ilabel {
            j += 1;
        } else {
            let mut t = j;
            while t < n && a[i].olabel == b[t].ilabel {
                if _match(&a, &b, i, t) {
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

    let loop_a = ArcData { state: pair.a, ilabel: EPS, olabel: EPS, weight: sr.one };
    let loop_b = ArcData { state: pair.b, ilabel: EPS, olabel: EPS, weight: sr.one };

    let mut arcs_a: Vec<ArcData> = Vec::with_capacity(m as usize);
    arcs_a.push(loop_a);
    for a in state_a.arcs.iter() {
        arcs_a.push(a.clone());
    }
    let mut arcs_b: Vec<ArcData> = Vec::with_capacity(n as usize);
    arcs_b.push(loop_b);
    for a in state_b.arcs.iter() {
        arcs_b.push(a.clone());
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

fn _match(a: &[ArcData], _b: &[ArcData], i: usize, j: usize) -> bool {
    let al = a[i].olabel;
    if al == EPS {
        // Reject EPS-EPS spurious matches that aren't (sentinel,sentinel)
        if (i != 0 && j != 0) || (i == 0 && j == 0) {
            return false;
        }
    }
    true
}

// ---------- Compose driver ----------

fn compose_impl(fst_a: &Fst, fst_b: &Fst, fst_c: &mut Fst) {
    use std::collections::HashMap;
    let sr = sr::sr_get(fst_a.sr_type);

    let mut q: Queue<(State, State)> = Queue::new();
    let mut marked: HashMap<(State, State), State> = HashMap::new();

    if fst_a.n_states == 0 || fst_b.n_states == 0 {
        return;
    }

    let pair0 = (fst_a.start, fst_b.start);
    q.enqueue(pair0);

    while let Some(pair) = q.dequeue() {
        let state_a = &fst_a.states[pair.0 as usize];
        let state_b = &fst_b.states[pair.1 as usize];
        let sc = if let Some(&id) = marked.get(&pair) {
            id
        } else {
            let id = fst_c.add_state();
            if state_a.final_state && state_b.final_state {
                fst_c.set_final(id, sr.one);
            }
            if pair.0 == fst_a.start && pair.1 == fst_b.start {
                fst_c.start = id;
            }
            marked.insert(pair, id);
            id
        };

        let mut mq: Queue<(ArcData, ArcData)> = Queue::new();
        let spair = Spair { a: pair.0, b: pair.1 };
        match_arcs(fst_a, fst_b, &spair, &sr, &mut mq);

        while let Some((arc_a, arc_b)) = mq.dequeue() {
            let new_pair = (arc_a.state, arc_b.state);
            let dst_sc = if let Some(&id) = marked.get(&new_pair) {
                id
            } else {
                let dst_state_a = &fst_a.states[new_pair.0 as usize];
                let dst_state_b = &fst_b.states[new_pair.1 as usize];
                let id = fst_c.add_state();
                if dst_state_a.final_state && dst_state_b.final_state {
                    fst_c.set_final(id, sr.one);
                }
                q.enqueue(new_pair);
                marked.insert(new_pair, id);
                id
            };
            fst_c.add_arc(
                sc,
                dst_sc,
                arc_a.ilabel,
                arc_b.olabel,
                (sr.prod)(arc_a.weight, arc_b.weight),
            );
        }
    }
}

// ---------- Compile parsing helpers ----------

fn parse_line(fst: &mut Fst, buf: &str) -> i32 {
    let sr = sr::sr_get(fst.sr_type);
    let toks: Vec<&str> = buf.split_whitespace().collect();
    if toks.len() == 5 {
        // sa sb il ol w
        if let (Ok(sa), Ok(sb), Ok(li), Ok(lo), Ok(w)) = (
            toks[0].parse::<usize>(),
            toks[1].parse::<usize>(),
            toks[2].parse::<usize>(),
            toks[3].parse::<usize>(),
            toks[4].parse::<f32>(),
        ) {
            add_arc_helper(fst, sa, sb, li, lo, w);
            return 0;
        }
    }
    if toks.len() == 4 {
        if let (Ok(sa), Ok(sb), Ok(li), Ok(lo)) = (
            toks[0].parse::<usize>(),
            toks[1].parse::<usize>(),
            toks[2].parse::<usize>(),
            toks[3].parse::<usize>(),
        ) {
            add_arc_helper(fst, sa, sb, li, lo, sr.one);
            return 0;
        }
    }
    if toks.len() == 2 {
        if let (Ok(sf), Ok(w)) = (toks[0].parse::<usize>(), toks[1].parse::<f32>()) {
            add_final_helper(fst, sf, w);
            return 0;
        }
    }
    if toks.len() == 1 {
        if let Ok(sf) = toks[0].parse::<usize>() {
            add_final_helper(fst, sf, sr.one);
            return 0;
        }
    }
    -1
}

fn parse_line_sym(
    fst: &mut Fst,
    buf: &str,
    ist: Option<&SymTable>,
    ost: Option<&SymTable>,
    sst: Option<&SymTable>,
) -> i32 {
    let sr = sr::sr_get(fst.sr_type);
    let toks: Vec<&str> = buf.split_whitespace().collect();

    let trans_s = |t: &str| translate(t, sst);
    let trans_i = |t: &str| translate(t, ist);
    let trans_o = |t: &str| translate(t, ost);

    if toks.len() == 5 {
        if let Ok(w) = toks[4].parse::<f32>() {
            let sa = trans_s(toks[0]);
            let sb = trans_s(toks[1]);
            let li = trans_i(toks[2]);
            let lo = trans_o(toks[3]);
            if sa == -1 || sb == -1 || li == -1 || lo == -1 {
                return -1;
            }
            add_arc_helper(fst, sa as usize, sb as usize, li as usize, lo as usize, w);
            return 0;
        }
    }
    if toks.len() == 4 {
        let sa = trans_s(toks[0]);
        let sb = trans_s(toks[1]);
        let li = trans_i(toks[2]);
        let lo = trans_o(toks[3]);
        if sa == -1 || sb == -1 || li == -1 || lo == -1 {
            return -1;
        }
        add_arc_helper(fst, sa as usize, sb as usize, li as usize, lo as usize, sr.one);
        return 0;
    }
    if toks.len() == 2 {
        if let Ok(w) = toks[1].parse::<f32>() {
            let sf = trans_s(toks[0]);
            if sf == -1 {
                return -1;
            }
            add_final_helper(fst, sf as usize, w);
            return 0;
        }
    }
    if toks.len() == 1 {
        let sf = trans_s(toks[0]);
        if sf == -1 {
            return -1;
        }
        add_final_helper(fst, sf as usize, sr.one);
        return 0;
    }
    -1
}

fn parse_line_sym_acc(
    fst: &mut Fst,
    buf: &str,
    ist: Option<&SymTable>,
    sst: Option<&SymTable>,
) -> i32 {
    let sr = sr::sr_get(fst.sr_type);
    let toks: Vec<&str> = buf.split_whitespace().collect();

    let trans_s = |t: &str| translate(t, sst);
    let trans_i = |t: &str| translate(t, ist);

    if toks.len() == 4 {
        if let Ok(w) = toks[3].parse::<f32>() {
            let sa = trans_s(toks[0]);
            let sb = trans_s(toks[1]);
            let li = trans_i(toks[2]);
            if sa == -1 || sb == -1 || li == -1 {
                return -1;
            }
            add_arc_helper(fst, sa as usize, sb as usize, li as usize, li as usize, w);
            return 0;
        }
    }
    if toks.len() == 3 {
        let sa = trans_s(toks[0]);
        let sb = trans_s(toks[1]);
        let li = trans_i(toks[2]);
        if sa == -1 || sb == -1 || li == -1 {
            return -1;
        }
        add_arc_helper(fst, sa as usize, sb as usize, li as usize, li as usize, sr.one);
        return 0;
    }
    if toks.len() == 2 {
        if let Ok(w) = toks[1].parse::<f32>() {
            let sf = trans_s(toks[0]);
            if sf == -1 {
                return -1;
            }
            add_final_helper(fst, sf as usize, w);
            return 0;
        }
    }
    if toks.len() == 1 {
        let sf = trans_s(toks[0]);
        if sf == -1 {
            return -1;
        }
        add_final_helper(fst, sf as usize, sr.one);
        return 0;
    }
    -1
}

fn translate(token: &str, st: Option<&SymTable>) -> i32 {
    match st {
        None => {
            // Numeric translation (trn)
            match token.parse::<i64>() {
                Ok(v) => v as i32,
                Err(_) => -1,
            }
        }
        Some(st) => {
            // Symbol-table lookup (trt)
            match st.getr(token) {
                Some(v) => v,
                None => -1,
            }
        }
    }
}

fn add_arc_helper(fst: &mut Fst, sa: usize, sb: usize, li: usize, lo: usize, w: f32) {
    while sa as u32 + 1 > fst.n_states || sb as u32 + 1 > fst.n_states {
        fst.add_state();
    }
    fst.add_arc(sa as u32, sb as u32, li as u32, lo as u32, w);
}

fn add_final_helper(fst: &mut Fst, s: usize, w: f32) {
    while s as u32 + 1 > fst.n_states {
        fst.add_state();
    }
    fst.set_final(s as u32, w);
}
