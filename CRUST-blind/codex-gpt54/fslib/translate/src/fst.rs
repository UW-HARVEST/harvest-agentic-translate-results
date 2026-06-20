use std::fs::File;
use std::io::{self, BufRead, BufReader, Read, Write};

use crate::bitset::BitSet;
use crate::queue::Queue;
use crate::sr::{self, Sr};
use crate::symt::SymTable;

pub type State = u32;
pub type Arc = u32;
pub type Label = u32;
pub type Weight = f32;

const FST_HEADER: u32 = 0x66733031;
const ISORT: u8 = 0x01;
const OSORT: u8 = 0x02;
const EPS: u32 = 0;
const EPS_L: i32 = -1;
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

impl Clone for ArcData {
    fn clone(&self) -> Self {
        Self {
            state: self.state,
            weight: self.weight,
            ilabel: self.ilabel,
            olabel: self.olabel,
        }
    }
}

impl Clone for StateData {
    fn clone(&self) -> Self {
        Self {
            n_arcs: self.n_arcs,
            n_max: self.n_max,
            weight: self.weight,
            final_state: self.final_state,
            arcs: self.arcs.clone(),
        }
    }
}

impl Clone for Fst {
    fn clone(&self) -> Self {
        Self {
            start: self.start,
            n_states: self.n_states,
            n_max: self.n_max,
            sr_type: self.sr_type,
            flags: self.flags,
            states: self.states.clone(),
        }
    }
}

impl Clone for Spair {
    fn clone(&self) -> Self {
        Self {
            a: self.a,
            b: self.b,
        }
    }
}

fn parse_numeric_token(token: &str) -> Option<usize> {
    token.trim().parse::<usize>().ok()
}

fn translate_token(token: &str, table: Option<&SymTable>) -> Option<usize> {
    if let Some(table) = table {
        if table.n_items == 0 {
            parse_numeric_token(token)
        } else {
            table.getr(token).map(|v| v as usize)
        }
    } else {
        parse_numeric_token(token)
    }
}

fn ensure_state_exists(fst: &mut Fst, state: usize) {
    while fst.n_states as usize <= state {
        fst.add_state();
    }
}

fn add_arc_helper(fst: &mut Fst, sa: usize, sb: usize, li: usize, lo: usize, w: f32) {
    ensure_state_exists(fst, sa.max(sb));
    fst.add_arc(sa as State, sb as State, li as Label, lo as Label, w);
}

fn add_final_helper(fst: &mut Fst, s: usize, w: f32) {
    ensure_state_exists(fst, s);
    fst.set_final(s as State, w);
}

fn parse_compile_line(fst: &mut Fst, line: &str) -> Result<(), ()> {
    let sr = sr::sr_get(fst.sr_type);
    let parts: Vec<&str> = line.trim().split('\t').collect();
    match parts.as_slice() {
        [sa, sb, li, lo, w] => {
            add_arc_helper(
                fst,
                sa.parse().map_err(|_| ())?,
                sb.parse().map_err(|_| ())?,
                li.parse().map_err(|_| ())?,
                lo.parse().map_err(|_| ())?,
                w.parse().map_err(|_| ())?,
            );
            Ok(())
        }
        [sa, sb, li, lo] => {
            add_arc_helper(
                fst,
                sa.parse().map_err(|_| ())?,
                sb.parse().map_err(|_| ())?,
                li.parse().map_err(|_| ())?,
                lo.parse().map_err(|_| ())?,
                sr.one(),
            );
            Ok(())
        }
        [sf, w] => {
            add_final_helper(fst, sf.parse().map_err(|_| ())?, w.parse().map_err(|_| ())?);
            Ok(())
        }
        [sf] => {
            add_final_helper(fst, sf.parse().map_err(|_| ())?, sr.one());
            Ok(())
        }
        _ => Err(()),
    }
}

fn parse_compile_line_sym(
    fst: &mut Fst,
    line: &str,
    ist: Option<&SymTable>,
    ost: Option<&SymTable>,
    sst: Option<&SymTable>,
    is_acc: bool,
) -> Result<(), ()> {
    let sr = sr::sr_get(fst.sr_type);
    let parts: Vec<&str> = line.trim().split('\t').collect();

    if is_acc {
        match parts.as_slice() {
            [sa, sb, li, w] => {
                add_arc_helper(
                    fst,
                    translate_token(sa, sst).ok_or(())?,
                    translate_token(sb, sst).ok_or(())?,
                    translate_token(li, ist).ok_or(())?,
                    translate_token(li, ist).ok_or(())?,
                    w.parse().map_err(|_| ())?,
                );
                Ok(())
            }
            [sa, sb, li] => {
                let label = translate_token(li, ist).ok_or(())?;
                add_arc_helper(
                    fst,
                    translate_token(sa, sst).ok_or(())?,
                    translate_token(sb, sst).ok_or(())?,
                    label,
                    label,
                    sr.one(),
                );
                Ok(())
            }
            [sf, w] => {
                add_final_helper(fst, translate_token(sf, sst).ok_or(())?, w.parse().map_err(|_| ())?);
                Ok(())
            }
            [sf] => {
                add_final_helper(fst, translate_token(sf, sst).ok_or(())?, sr.one());
                Ok(())
            }
            _ => Err(()),
        }
    } else {
        match parts.as_slice() {
            [sa, sb, li, lo, w] => {
                add_arc_helper(
                    fst,
                    translate_token(sa, sst).ok_or(())?,
                    translate_token(sb, sst).ok_or(())?,
                    translate_token(li, ist).ok_or(())?,
                    translate_token(lo, ost).ok_or(())?,
                    w.parse().map_err(|_| ())?,
                );
                Ok(())
            }
            [sa, sb, li, lo] => {
                add_arc_helper(
                    fst,
                    translate_token(sa, sst).ok_or(())?,
                    translate_token(sb, sst).ok_or(())?,
                    translate_token(li, ist).ok_or(())?,
                    translate_token(lo, ost).ok_or(())?,
                    sr.one(),
                );
                Ok(())
            }
            [sf, w] => {
                add_final_helper(fst, translate_token(sf, sst).ok_or(())?, w.parse().map_err(|_| ())?);
                Ok(())
            }
            [sf] => {
                add_final_helper(fst, translate_token(sf, sst).ok_or(())?, sr.one());
                Ok(())
            }
            _ => Err(()),
        }
    }
}

fn read_u32(fin: &mut File) -> io::Result<u32> {
    let mut buf = [0u8; 4];
    fin.read_exact(&mut buf)?;
    Ok(u32::from_ne_bytes(buf))
}

fn read_f32(fin: &mut File) -> io::Result<f32> {
    let mut buf = [0u8; 4];
    fin.read_exact(&mut buf)?;
    Ok(f32::from_ne_bytes(buf))
}

fn write_u32(fout: &mut File, value: u32) -> io::Result<()> {
    fout.write_all(&value.to_ne_bytes())
}

fn write_i32(fout: &mut File, value: i32) -> io::Result<()> {
    fout.write_all(&value.to_ne_bytes())
}

fn write_f32(fout: &mut File, value: f32) -> io::Result<()> {
    fout.write_all(&value.to_ne_bytes())
}

fn reachable_mask(fst: &Fst) -> BitSet {
    let mut marked = BitSet::new(fst.n_states as usize);
    let mut queue = Queue::new();
    queue.enqueue(fst.start);
    marked.set(fst.start as usize);

    while let Some(s) = queue.dequeue() {
        let state = &fst.states[s as usize];
        for arc in &state.arcs {
            if !marked.get(arc.state as usize) {
                marked.set(arc.state as usize);
                queue.enqueue(arc.state);
            }
        }
    }

    marked
}

impl Fst {
    pub fn new() -> Self {
        Self {
            start: 0,
            n_states: 0,
            n_max: 0,
            sr_type: 0,
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
        let _ = crate::print::fst_print(self, &mut io::stdout());
    }

    pub fn print_sym(&self, ist: &SymTable, ost: &SymTable, sst: &SymTable) {
        let _ = crate::print::fst_print_sym(self, Some(ist), Some(ost), Some(sst), &mut io::stdout());
    }

    pub fn write(&self, fout: &mut File) -> io::Result<()> {
        write_u32(fout, FST_HEADER)?;
        write_u32(fout, self.start)?;
        write_u32(fout, self.n_states)?;
        fout.write_all(&[self.sr_type])?;
        fout.write_all(&[self.flags])?;

        for state in &self.states {
            write_f32(fout, state.weight)?;
            write_u32(fout, state.n_arcs)?;
            write_i32(fout, if state.final_state { 1 } else { 0 })?;
            for arc in &state.arcs {
                write_u32(fout, arc.state)?;
                write_f32(fout, arc.weight)?;
                write_u32(fout, arc.ilabel)?;
                write_u32(fout, arc.olabel)?;
            }
        }

        Ok(())
    }

    pub fn read(&mut self, fin: &mut File) -> io::Result<()> {
        let header = read_u32(fin)?;
        if header != FST_HEADER {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Wrong file format"));
        }

        self.start = read_u32(fin)?;
        self.n_states = read_u32(fin)?;
        let mut buf = [0u8; 1];
        fin.read_exact(&mut buf)?;
        self.sr_type = buf[0];
        fin.read_exact(&mut buf)?;
        self.flags = buf[0];
        self.n_max = self.n_states;
        self.states.clear();

        for _ in 0..self.n_states {
            let weight = read_f32(fin)?;
            let n_arcs = read_u32(fin)?;
            let final_state = read_u32(fin)? != 0;
            let mut arcs = Vec::with_capacity(n_arcs as usize);
            for _ in 0..n_arcs {
                arcs.push(ArcData {
                    state: read_u32(fin)?,
                    weight: read_f32(fin)?,
                    ilabel: read_u32(fin)?,
                    olabel: read_u32(fin)?,
                });
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
        let mut fout = File::create(filename)?;
        self.write(&mut fout)
    }

    pub fn fread(&mut self, filename: &str) -> io::Result<()> {
        let mut fin = File::open(filename)?;
        self.read(&mut fin)
    }

    pub fn compile(&mut self, fin: &mut File, ist: &SymTable, ost: &SymTable, sst: &SymTable, is_acc: bool) -> Self {
        let mut reader = BufReader::new(fin);
        let mut line = String::new();
        let mut line_no = 1usize;
        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) => break,
                Ok(_) => {
                    line_no += 1;
                    if parse_compile_line_sym(self, &line, Some(ist), Some(ost), Some(sst), is_acc).is_err() {
                        eprintln!("Invalid input line {}: {}", line_no, line.trim_end());
                        std::process::exit(1);
                    }
                }
                Err(_) => {
                    break;
                }
            }
        }

        if let Some(start_state) = sst.getr(START_STATE) {
            self.start = start_state as State;
        }

        self.clone()
    }

    pub fn compile_str(&mut self, str_data: &str) -> Self {
        for (line_no, line) in str_data.lines().enumerate() {
            if parse_compile_line(self, line).is_err() {
                eprintln!("Invalid input line {}: {}", line_no + 1, line);
                std::process::exit(1);
            }
        }
        self.clone()
    }

    pub fn get_n_arcs(&self) -> Arc {
        self.states.iter().map(|state| state.n_arcs).sum()
    }

    pub fn arc_sort(&mut self, sort_outer: i32) {
        crate::sort::fst_arc_sort(self, sort_outer != 0);
    }

    pub fn stack(&mut self, other: &Fst) {
        let offset = self.n_states;
        self.n_states += other.n_states;
        if self.n_max < self.n_states {
            self.n_max = self.n_states;
        }

        for state in &other.states {
            let mut cloned = state.clone();
            for arc in &mut cloned.arcs {
                arc.state += offset;
            }
            self.states.push(cloned);
        }
    }

    pub fn union(&mut self, other: &Fst) -> Self {
        let sr = sr::sr_get(self.sr_type);
        let left_start = self.start;
        let right_offset = self.n_states;
        self.stack(other);
        let new_start = self.add_state();
        self.add_arc(new_start, left_start, EPS, EPS, sr.one());
        self.add_arc(new_start, other.start + right_offset, EPS, EPS, sr.one());
        self.start = new_start;
        self.clone()
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
        *copy = self.clone();
    }

    pub fn reverse(&mut self) {
        let sr = sr::sr_get(self.sr_type);
        let orig = self.clone();
        let start_s = self.start;

        for (s, state) in self.states.iter_mut().enumerate() {
            state.n_arcs = 0;
            state.arcs.clear();
            if state.final_state {
                self.start = s as State;
                state.final_state = false;
            }
        }

        self.set_final(start_s, sr.one());

        for (s, state) in orig.states.iter().enumerate() {
            for arc in &state.arcs {
                self.add_arc(arc.state, s as State, arc.ilabel, arc.olabel, arc.weight);
            }
        }
    }

    pub fn shortest(&self, path: &mut Fst) -> Self {
        crate::shortest::ShortestPath::find_shortest_path(self, path);
        path.clone()
    }

    pub fn rm_states(&mut self, visited: &BitSet) -> Self {
        let mut idx = vec![0u32; self.n_states as usize];
        let mut shift = 0u32;
        let original_n_states = self.n_states;

        for s in 0..original_n_states {
            if visited.get(s as usize) {
                shift += 1;
            } else {
                idx[s as usize] = shift;
                self.states[(s - shift) as usize] = self.states[s as usize].clone();
            }
        }

        self.n_states -= shift;
        self.states.truncate(self.n_states as usize);

        for s in 0..self.n_states {
            let state = &mut self.states[s as usize];
            let mut new_arcs = Vec::with_capacity(state.arcs.len());

            for arc in &state.arcs {
                let mut arc = arc.clone();
                arc.state -= idx[arc.state as usize];
                if arc.state < self.n_states {
                    new_arcs.push(arc);
                }
            }

            state.arcs = new_arcs;
            state.n_arcs = state.arcs.len() as Arc;
        }

        self.clone()
    }

    pub fn trim(&mut self) -> Self {
        let finals: Vec<State> = self
            .states
            .iter()
            .enumerate()
            .filter_map(|(idx, state)| state.final_state.then_some(idx as State))
            .collect();

        if finals.is_empty() {
            self.empty();
            return self.clone();
        }

        if finals.len() > 1 {
            let sr = sr::sr_get(self.sr_type);
            let final_state = self.add_state();
            self.set_final(final_state, sr.one());
            for s in finals {
                let weight = self.states[s as usize].weight;
                self.states[s as usize].final_state = false;
                self.add_arc(s, final_state, EPS, EPS, weight);
            }
        }

        let mut forward = reachable_mask(self);
        self.reverse();
        let reverse = reachable_mask(self);
        self.reverse();

        forward.intersect(&reverse);
        let remove_mask = forward.toggle_all();
        self.rm_states(&remove_mask);
        self.clone()
    }

    pub fn compose(&self, fst_b: &Fst, fst_c: &mut Fst) {
        *fst_c = Fst::new();
        fst_c.sr_type = self.sr_type;

        let sr = sr::sr_get(self.sr_type);
        let mut agenda = Queue::new();
        let mut matched = Queue::new();
        let mut marked: std::collections::HashMap<(State, State), State> = std::collections::HashMap::new();

        agenda.enqueue(Spair {
            a: self.start,
            b: fst_b.start,
        });

        while let Some(pair) = agenda.dequeue() {
            let sc = if let Some(sc) = marked.get(&(pair.a, pair.b)).copied() {
                sc
            } else {
                let sc = fst_c.add_state();
                if self.states[pair.a as usize].final_state && fst_b.states[pair.b as usize].final_state {
                    fst_c.set_final(sc, sr.one());
                }
                if pair.a == self.start && pair.b == fst_b.start {
                    fst_c.start = sc;
                }
                marked.insert((pair.a, pair.b), sc);
                sc
            };

            match_arcs(self, fst_b, &pair, &sr, &mut matched);

            while let Some((arc_a, arc_b)) = matched.dequeue() {
                let key = (arc_a.state, arc_b.state);
                let dst_sc = if let Some(dst) = marked.get(&key).copied() {
                    dst
                } else {
                    let dst = fst_c.add_state();
                    if self.states[arc_a.state as usize].final_state
                        && fst_b.states[arc_b.state as usize].final_state
                    {
                        fst_c.set_final(dst, sr.one());
                    }
                    agenda.enqueue(Spair {
                        a: arc_a.state,
                        b: arc_b.state,
                    });
                    marked.insert(key, dst);
                    dst
                };

                fst_c.add_arc(
                    sc,
                    dst_sc,
                    arc_a.ilabel,
                    arc_b.olabel,
                    sr.prod(arc_a.weight, arc_b.weight),
                );
            }
        }
    }

    pub fn relabel(&mut self, old: Label, new: Label, dir: i32) {
        for state in &mut self.states {
            for arc in &mut state.arcs {
                if dir == 0 {
                    if arc.ilabel == old {
                        arc.ilabel = new;
                    }
                } else if arc.olabel == old {
                    arc.olabel = new;
                }
            }
        }
    }
}

fn match_allowed(a: &[ArcData], _b: &[ArcData], i: usize, j: usize) -> bool {
    let al = a[i].olabel;
    if al == EPS {
        if (i != 0 && j != 0) || (i == 0 && j == 0) {
            return false;
        }
    }
    true
}

pub fn match_unsorted(a: &[ArcData], b: &[ArcData], m: Arc, n: Arc, q: &mut Queue<(ArcData, ArcData)>) {
    for i in 0..m as usize {
        for j in 0..n as usize {
            if a[i].olabel == b[j].ilabel && match_allowed(a, b, i, j) {
                q.enqueue((a[i].clone(), b[j].clone()));
            }
        }
    }
}

pub fn match_half_sorted(a: &[ArcData], b: &[ArcData], m: Arc, n: Arc, q: &mut Queue<(ArcData, ArcData)>) {
    for i in 0..m as usize {
        let mut l = 0usize;
        let mut h = n.saturating_sub(1) as usize;
        while l <= h {
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
                    if match_allowed(a, b, i, ll) {
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
    for i in 0..n as usize {
        let mut l = 0usize;
        let mut h = m.saturating_sub(1) as usize;
        while l <= h {
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
                    if match_allowed(a, b, ll, i) {
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
    let mut i = 0usize;
    let mut j = 0usize;

    while i < m as usize && j < n as usize {
        if a[i].olabel < b[j].ilabel {
            i += 1;
        } else if a[i].olabel > b[j].ilabel {
            j += 1;
        } else {
            let mut t = j;
            while t < n as usize && a[i].olabel == b[t].ilabel {
                if match_allowed(a, b, i, t) {
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
    let osort = fst_a.flags & OSORT != 0;
    let isort = fst_b.flags & ISORT != 0;

    let mut arcs_a = Vec::with_capacity(state_a.n_arcs as usize + 1);
    let mut arcs_b = Vec::with_capacity(state_b.n_arcs as usize + 1);

    arcs_a.push(ArcData {
        state: pair.a,
        ilabel: EPS,
        olabel: EPS,
        weight: sr.one(),
    });
    arcs_b.push(ArcData {
        state: pair.b,
        ilabel: EPS,
        olabel: EPS,
        weight: sr.one(),
    });
    arcs_a.extend(state_a.arcs.iter().cloned());
    arcs_b.extend(state_b.arcs.iter().cloned());

    if isort && osort {
        match_full_sorted(&arcs_a, &arcs_b, arcs_a.len() as Arc, arcs_b.len() as Arc, mq);
    } else if isort || osort {
        if isort {
            match_half_sorted(&arcs_a, &arcs_b, arcs_a.len() as Arc, arcs_b.len() as Arc, mq);
        } else {
            match_half_sorted_rev(&arcs_a, &arcs_b, arcs_a.len() as Arc, arcs_b.len() as Arc, mq);
        }
    } else {
        match_unsorted(&arcs_a, &arcs_b, arcs_a.len() as Arc, arcs_b.len() as Arc, mq);
    }
}
