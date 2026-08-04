use std::fs::File;
use std::io::{self, Read, Write};
use crate::sr::{Sr, sr_get};
use crate::bitset::BitSet;
use crate::queue::Queue;
use crate::symt::SymTable;
use std::collections::VecDeque;
pub type State = u32;
pub type Arc = u32;
pub type Label = u32;
pub type Weight = f32;
const FST_HEADER: u32 = 0x66733031;
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
#[derive(Clone, Copy)]
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

const SR_TROPICAL: u8 = 0;

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
            weight,
            ilabel: il,
            olabel: ol,
        });
        state.n_arcs - 1
    }
    pub fn set_final(&mut self, s: State, w: Weight) {
        let state = &mut self.states[s as usize];
        state.final_state = true;
        state.weight = w;
    }
    pub fn print(&self) {
        let mut finals: VecDeque<State> = VecDeque::new();
        for s in 0..self.n_states {
            let state = &self.states[s as usize];
            for a in 0..state.n_arcs {
                let arc = &state.arcs[a as usize];
                println!("{}\t{}\t{}\t{}\t{:.5}", s, arc.state, arc.ilabel, arc.olabel, arc.weight);
            }
            if state.final_state {
                finals.push_back(s);
            }
        }
        while let Some(s) = finals.pop_front() {
            let state = &self.states[s as usize];
            println!("{}\t{}", s, state.weight);
        }
    }
    pub fn print_sym(&self, ist: &SymTable, ost: &SymTable, sst: &SymTable) {
        let mut finals: VecDeque<State> = VecDeque::new();
        for s in 0..self.n_states {
            let state = &self.states[s as usize];
            for a in 0..state.n_arcs {
                let arc = &state.arcs[a as usize];
                let sa = sst.get(s as i32).map(|x| x.to_string()).unwrap_or_else(|| s.to_string());
                let sb = sst.get(arc.state as i32).map(|x| x.to_string()).unwrap_or_else(|| arc.state.to_string());
                let li = ist.get(arc.ilabel as i32).map(|x| x.to_string()).unwrap_or_else(|| arc.ilabel.to_string());
                let lo = ost.get(arc.olabel as i32).map(|x| x.to_string()).unwrap_or_else(|| arc.olabel.to_string());
                println!("{}\t{}\t{}\t{}\t{:.5}", sa, sb, li, lo, arc.weight);
            }
            if state.final_state {
                finals.push_back(s);
            }
        }
        while let Some(s) = finals.pop_front() {
            let state = &self.states[s as usize];
            let sa = sst.get(s as i32).map(|x| x.to_string()).unwrap_or_else(|| s.to_string());
            println!("{}\t{}", sa, state.weight);
        }
    }
    pub fn write(&self, fout: &mut File) -> io::Result<()> {
        fout.write_all(&FST_HEADER.to_ne_bytes())?;
        fout.write_all(&self.start.to_ne_bytes())?;
        fout.write_all(&self.n_states.to_ne_bytes())?;
        fout.write_all(&self.sr_type.to_ne_bytes())?;
        fout.write_all(&self.flags.to_ne_bytes())?;
        for s in 0..self.n_states {
            let state = &self.states[s as usize];
            fout.write_all(&state.weight.to_ne_bytes())?;
            fout.write_all(&state.n_arcs.to_ne_bytes())?;
            // The C struct uses `int` for final, which is typically 4 bytes.
            let final_int: i32 = if state.final_state { 1 } else { 0 };
            fout.write_all(&final_int.to_ne_bytes())?;
            for a in 0..state.n_arcs {
                let arc = &state.arcs[a as usize];
                fout.write_all(&arc.state.to_ne_bytes())?;
                fout.write_all(&arc.weight.to_ne_bytes())?;
                fout.write_all(&arc.ilabel.to_ne_bytes())?;
                fout.write_all(&arc.olabel.to_ne_bytes())?;
            }
        }
        Ok(())
    }
    pub fn read(&mut self, fin: &mut File) -> io::Result<()> {
        let mut header_bytes = [0u8; 4];
        fin.read_exact(&mut header_bytes)?;
        let header = u32::from_ne_bytes(header_bytes);
        if header != FST_HEADER {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Wrong file format"));
        }
        let mut buf4 = [0u8; 4];
        fin.read_exact(&mut buf4)?;
        self.start = u32::from_ne_bytes(buf4);
        fin.read_exact(&mut buf4)?;
        self.n_states = u32::from_ne_bytes(buf4);
        let mut buf1 = [0u8; 1];
        fin.read_exact(&mut buf1)?;
        self.sr_type = buf1[0];
        fin.read_exact(&mut buf1)?;
        self.flags = buf1[0];
        self.n_max = self.n_states;
        self.states = Vec::with_capacity(self.n_states as usize);
        for _ in 0..self.n_states {
            let mut wbuf = [0u8; 4];
            fin.read_exact(&mut wbuf)?;
            let weight = f32::from_ne_bytes(wbuf);
            fin.read_exact(&mut wbuf)?;
            let n_arcs = u32::from_ne_bytes(wbuf);
            // final is `int` (4 bytes) in C
            fin.read_exact(&mut wbuf)?;
            let final_int = i32::from_ne_bytes(wbuf);
            let mut arcs = Vec::with_capacity(n_arcs as usize);
            for _ in 0..n_arcs {
                fin.read_exact(&mut wbuf)?;
                let state = u32::from_ne_bytes(wbuf);
                fin.read_exact(&mut wbuf)?;
                let aweight = f32::from_ne_bytes(wbuf);
                fin.read_exact(&mut wbuf)?;
                let ilabel = u32::from_ne_bytes(wbuf);
                fin.read_exact(&mut wbuf)?;
                let olabel = u32::from_ne_bytes(wbuf);
                arcs.push(ArcData { state, weight: aweight, ilabel, olabel });
            }
            self.states.push(StateData {
                n_arcs,
                n_max: n_arcs,
                weight,
                final_state: final_int == 1,
                arcs,
            });
        }
        Ok(())
    }
    pub fn fwrite(&self, filename: &str) -> io::Result<()> {
        let mut fout = File::create(filename)?;
        self.write(&mut fout)
    }
    pub fn fread(&mut self, filename: &str) -> io::Result<()> {
        let mut fin = File::open(filename)?;
        self.read(&mut fin)
    }
    pub fn compile(&mut self, fin: &mut File, ist: &SymTable, ost: &SymTable, sst: &SymTable, is_acc: bool) -> Self {
        use std::io::{BufRead, BufReader};
        let reader = BufReader::new(fin);
        let mut line_no = 1usize;
        for line_res in reader.lines() {
            let line = match line_res {
                Ok(l) => l,
                Err(_) => break,
            };
            line_no += 1;
            let res = if !is_acc {
                parse_line_sym(self, &line, Some(ist), Some(ost), Some(sst))
            } else {
                parse_line_sym_acc(self, &line, Some(ist), Some(sst))
            };
            if res != 0 {
                eprintln!("Invalid input line {}: {}", line_no, line);
            }
        }
        if let Some(start_state) = sst.getr(START_STATE) {
            if start_state >= 0 {
                self.start = start_state as u32;
            }
        }
        // Return self via mem::take-like pattern
        std::mem::replace(self, Fst::new())
    }
    pub fn compile_str(&mut self, str_data: &str) -> Self {
        let mut line_no = 1usize;
        for line in str_data.split('\n') {
            if line.is_empty() {
                continue;
            }
            if parse_line(self, line) != 0 {
                eprintln!("Invalid input line {}: {}", line_no, line);
            }
            line_no += 1;
        }
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
            let mut new_arcs: Vec<ArcData> = Vec::with_capacity(other_state.arcs.len());
            for a in &other_state.arcs {
                new_arcs.push(ArcData {
                    state: a.state + offset,
                    weight: a.weight,
                    ilabel: a.ilabel,
                    olabel: a.olabel,
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
        // Not used in the C source; provide basic behavior of stacking
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
        for s in &self.states {
            let mut new_arcs = Vec::with_capacity(s.arcs.len());
            for a in &s.arcs {
                new_arcs.push(*a);
            }
            copy.states.push(StateData {
                n_arcs: s.n_arcs,
                n_max: s.n_max,
                weight: s.weight,
                final_state: s.final_state,
                arcs: new_arcs,
            });
        }
    }
    pub fn reverse(&mut self) {
        let sr = sr_get(self.sr_type);
        let mut orig = Fst::new();
        self.copy(&mut orig);
        let start_s = self.start;
        for s in 0..self.n_states {
            let state = &mut self.states[s as usize];
            // 'delete' arcs
            state.n_arcs = 0;
            state.arcs.clear();
            // change start to final
            if state.final_state {
                state.final_state = false;
                self.start = s;
            }
        }
        // set start as final
        self.set_final(start_s, sr.one);
        // add reversed arcs
        for s in 0..orig.n_states {
            let state = &orig.states[s as usize];
            for a in 0..state.n_arcs {
                let arc = &state.arcs[a as usize];
                self.add_arc(arc.state, s, arc.ilabel, arc.olabel, arc.weight);
            }
        }
    }
    pub fn shortest(&self, path: &mut Fst) -> Self {
        crate::shortest::ShortestPath::find_shortest_path(self, path);
        // Return placeholder
        Fst::new()
    }
    pub fn rm_states(&mut self, visited: &BitSet) -> Self {
        crate::trim::fst_rm_states(self, visited);
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

fn trn(token: &str, _symt: Option<&SymTable>) -> i64 {
    match token.parse::<i64>() {
        Ok(d) => d,
        Err(_) => -1,
    }
}

fn trt(token: &str, symt: Option<&SymTable>) -> i64 {
    if let Some(st) = symt {
        match st.getr(token) {
            Some(id) => id as i64,
            None => -1,
        }
    } else {
        -1
    }
}

fn add_arc_helper(fst: &mut Fst, sa: usize, sb: usize, li: usize, lo: usize, w: f32) {
    while (sa as State + 1 > fst.n_states) || (sb as State + 1 > fst.n_states) {
        fst.add_state();
    }
    fst.add_arc(sa as State, sb as State, li as Label, lo as Label, w);
}

fn add_final_helper(fst: &mut Fst, s: usize, w: f32) {
    while s as State + 1 > fst.n_states {
        fst.add_state();
    }
    fst.set_final(s as State, w);
}

fn parse_line(fst: &mut Fst, buf: &str) -> i32 {
    let sr = sr_get(fst.sr_type);
    let parts: Vec<&str> = buf.split_whitespace().collect();
    // 5 parts: sa sb li lo w
    if parts.len() == 5 {
        let sa = parts[0].parse::<usize>();
        let sb = parts[1].parse::<usize>();
        let li = parts[2].parse::<usize>();
        let lo = parts[3].parse::<usize>();
        let w = parts[4].parse::<f32>();
        if let (Ok(sa), Ok(sb), Ok(li), Ok(lo), Ok(w)) = (sa, sb, li, lo, w) {
            add_arc_helper(fst, sa, sb, li, lo, w);
            return 0;
        }
    }
    if parts.len() == 4 {
        let sa = parts[0].parse::<usize>();
        let sb = parts[1].parse::<usize>();
        let li = parts[2].parse::<usize>();
        let lo = parts[3].parse::<usize>();
        if let (Ok(sa), Ok(sb), Ok(li), Ok(lo)) = (sa, sb, li, lo) {
            add_arc_helper(fst, sa, sb, li, lo, sr.one);
            return 0;
        }
    }
    if parts.len() == 2 {
        let sf = parts[0].parse::<usize>();
        let w = parts[1].parse::<f32>();
        if let (Ok(sf), Ok(w)) = (sf, w) {
            add_final_helper(fst, sf, w);
            return 0;
        }
    }
    if parts.len() == 1 {
        let sf = parts[0].parse::<usize>();
        if let Ok(sf) = sf {
            add_final_helper(fst, sf, sr.one);
            return 0;
        }
    }
    -1
}

fn parse_line_sym(fst: &mut Fst, buf: &str, ist: Option<&SymTable>, ost: Option<&SymTable>, sst: Option<&SymTable>) -> i32 {
    let sr = sr_get(fst.sr_type);
    let parts: Vec<&str> = buf.split_whitespace().collect();
    let strans = if sst.is_some() { trt as fn(&str, Option<&SymTable>) -> i64 } else { trn as fn(&str, Option<&SymTable>) -> i64 };
    let itrans = if ist.is_some() { trt as fn(&str, Option<&SymTable>) -> i64 } else { trn as fn(&str, Option<&SymTable>) -> i64 };
    let otrans = if ost.is_some() { trt as fn(&str, Option<&SymTable>) -> i64 } else { trn as fn(&str, Option<&SymTable>) -> i64 };
    if parts.len() == 5 {
        let sa = strans(parts[0], sst);
        let sb = strans(parts[1], sst);
        let li = itrans(parts[2], ist);
        let lo = otrans(parts[3], ost);
        let w = parts[4].parse::<f32>();
        if let Ok(w) = w {
            if sa < 0 || sb < 0 || li < 0 || lo < 0 {
                return -1;
            }
            add_arc_helper(fst, sa as usize, sb as usize, li as usize, lo as usize, w);
            return 0;
        }
    }
    if parts.len() == 4 {
        let sa = strans(parts[0], sst);
        let sb = strans(parts[1], sst);
        let li = itrans(parts[2], ist);
        let lo = otrans(parts[3], ost);
        if sa < 0 || sb < 0 || li < 0 || lo < 0 {
            return -1;
        }
        add_arc_helper(fst, sa as usize, sb as usize, li as usize, lo as usize, sr.one);
        return 0;
    }
    if parts.len() == 2 {
        let sf = strans(parts[0], sst);
        let w = parts[1].parse::<f32>();
        if let Ok(w) = w {
            if sf < 0 {
                return -1;
            }
            add_final_helper(fst, sf as usize, w);
            return 0;
        }
    }
    if parts.len() == 1 {
        let sf = strans(parts[0], sst);
        if sf < 0 {
            return -1;
        }
        add_final_helper(fst, sf as usize, sr.one);
        return 0;
    }
    -1
}

fn parse_line_sym_acc(fst: &mut Fst, buf: &str, ist: Option<&SymTable>, sst: Option<&SymTable>) -> i32 {
    let sr = sr_get(fst.sr_type);
    let parts: Vec<&str> = buf.split_whitespace().collect();
    let strans = if sst.is_some() { trt as fn(&str, Option<&SymTable>) -> i64 } else { trn as fn(&str, Option<&SymTable>) -> i64 };
    let itrans = if ist.is_some() { trt as fn(&str, Option<&SymTable>) -> i64 } else { trn as fn(&str, Option<&SymTable>) -> i64 };
    if parts.len() == 4 {
        let sa = strans(parts[0], sst);
        let sb = strans(parts[1], sst);
        let li = itrans(parts[2], ist);
        let w = parts[3].parse::<f32>();
        if let Ok(w) = w {
            if sa < 0 || sb < 0 || li < 0 {
                return -1;
            }
            add_arc_helper(fst, sa as usize, sb as usize, li as usize, li as usize, w);
            return 0;
        }
    }
    if parts.len() == 3 {
        let sa = strans(parts[0], sst);
        let sb = strans(parts[1], sst);
        let li = itrans(parts[2], ist);
        if sa < 0 || sb < 0 || li < 0 {
            return -1;
        }
        add_arc_helper(fst, sa as usize, sb as usize, li as usize, li as usize, sr.one);
        return 0;
    }
    if parts.len() == 2 {
        let sf = strans(parts[0], sst);
        let w = parts[1].parse::<f32>();
        if let Ok(w) = w {
            if sf < 0 {
                return -1;
            }
            add_final_helper(fst, sf as usize, w);
            return 0;
        }
    }
    if parts.len() == 1 {
        let sf = strans(parts[0], sst);
        if sf < 0 {
            return -1;
        }
        add_final_helper(fst, sf as usize, sr.one);
        return 0;
    }
    -1
}

// Match helpers
fn _match_check(a: &[ArcData], _b: &[ArcData], i: usize, j: usize) -> bool {
    let al = a[i].olabel;
    if al == EPS {
        if (i != 0 && j != 0) || (i == 0 && j == 0) {
            return false;
        }
    }
    true
}

pub fn match_unsorted(a: &[ArcData], b: &[ArcData], m: Arc, n: Arc, q: &mut Queue<(ArcData, ArcData)>) {
    for i in 0..(m as usize) {
        for j in 0..(n as usize) {
            if a[i].olabel == b[j].ilabel && _match_check(a, b, i, j) {
                q.enqueue((a[i], b[j]));
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
                    if _match_check(a, b, i, k) {
                        q.enqueue((a[i], b[k]));
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
                    if _match_check(a, b, k, i) {
                        q.enqueue((a[k], b[i]));
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
                if _match_check(a, b, i, t) {
                    q.enqueue((a[i], b[t]));
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
    arcs_a.push(ArcData { state: pair.a, ilabel: EPS, olabel: EPS, weight: sr.one });
    for a in &state_a.arcs {
        arcs_a.push(*a);
    }
    arcs_b.push(ArcData { state: pair.b, ilabel: EPS, olabel: EPS, weight: sr.one });
    for a in &state_b.arcs {
        arcs_b.push(*a);
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
    let sr = sr_get(fst_a.sr_type);
    let mut q: VecDeque<Spair> = VecDeque::new();
    let mut mq: Queue<(ArcData, ArcData)> = Queue::new();
    let mut marked: std::collections::HashMap<(State, State), State> = std::collections::HashMap::new();

    let initial_pair = Spair { a: fst_a.start, b: fst_b.start };
    q.push_back(initial_pair);

    while let Some(pair) = q.pop_front() {
        let state_a = &fst_a.states[pair.a as usize];
        let state_b = &fst_b.states[pair.b as usize];

        let key = (pair.a, pair.b);
        let sc = if let Some(&existing) = marked.get(&key) {
            existing
        } else {
            let sc = fst_c.add_state();
            if state_a.final_state && state_b.final_state {
                fst_c.set_final(sc, sr.one);
            }
            if pair.a == fst_a.start && pair.b == fst_b.start {
                fst_c.start = sc;
            }
            marked.insert(key, sc);
            sc
        };

        match_arcs(fst_a, fst_b, &pair, &sr, &mut mq);

        while let Some((arc_a, arc_b)) = mq.dequeue() {
            let new_pair = Spair { a: arc_a.state, b: arc_b.state };
            let new_key = (new_pair.a, new_pair.b);
            let dst_sc = if let Some(&existing) = marked.get(&new_key) {
                existing
            } else {
                let dst_state_a = &fst_a.states[new_pair.a as usize];
                let dst_state_b = &fst_b.states[new_pair.b as usize];
                let dst_sc = fst_c.add_state();
                if dst_state_a.final_state && dst_state_b.final_state {
                    fst_c.set_final(dst_sc, sr.one);
                }
                q.push_back(Spair { a: new_pair.a, b: new_pair.b });
                marked.insert(new_key, dst_sc);
                dst_sc
            };
            fst_c.add_arc(sc, dst_sc, arc_a.ilabel, arc_b.olabel, (sr.prod)(arc_a.weight, arc_b.weight));
        }
    }
}
