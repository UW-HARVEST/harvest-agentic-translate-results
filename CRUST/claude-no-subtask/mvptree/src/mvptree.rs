use std::fs::File;
use std::io::{Read as IoRead, Write};
use std::sync::Arc;
use std::rc::Rc;
use std::cell::RefCell;
use std::convert::TryInto;

pub const TAG: &str = "phashmvp2010";
pub const VERSION: u32 = 0x01000000;
pub const HEADER_SIZE: usize = 32;
pub const FILE_OFFSET_BITS: usize = 64;
pub const ERROR_MSGS: [&str; 25] = [
    "no error",
    "bad argument",
    "no distance function found",
    "mem alloc error",
    "no leaf node created",
    "no internal node created",
    "no path array alloc'd",
    "could not select vantage points",
    "could not calculate range from an sv1",
    "could not calculate range from an sv2",
    "points too compact",
    "could not sort points",
    "could not open file",
    "could not close file",
    "mmap error",
    "unmap error",
    "no write",
    "could not extend file",
    "could not remap file",
    "datatypes in conflict",
    "no. retrieved exceeds k",
    "empty tree",
    "distance value either NaN or less than zero",
    "could not open file",
    "unrecognized node",
];
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MVPDataType {
    ByteArray = 1,
    UInt16Array = 2,
    UInt32Array = 4,
    UInt64Array = 8,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeType {
    InternalNode = 1,
    LeafNode,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MVPError {
    Success,
    ArgErr,
    NoDistanceFunc,
    MemAlloc,
    NoLeaf,
    NoInternal,
    PathAlloc,
    VpNoSelect,
    NoSv1Range,
    NoSv2Range,
    NoSpace,
    NoSort,
    FileOpen,
    FileClose,
    MemMap,
    Munmap,
    NoWrite,
    FileTruncate,
    MremapFail,
    TypeMismatch,
    KNearestCap,
    EmptyTree,
    NoSplits,
    BadDistVal,
    FileNotFound,
    Unrecognized,
}
#[derive(Debug, Clone)]
pub struct MVPDatapoint {
    pub id: String,
    pub data: Vec<u8>,
    pub path: Vec<f32>,
    pub datalen: usize,
    pub data_type: MVPDataType,
}
pub type DistanceFunction = fn(&MVPDatapoint, &MVPDatapoint) -> f32;
pub struct InternalNode {
    pub node_type: NodeType,
    pub sv1: Option<Arc<MVPDatapoint>>,
    pub sv2: Option<Arc<MVPDatapoint>>,
    pub m1: Vec<f32>,
    pub m2: Vec<f32>,
    pub child_nodes: Vec<Rc<RefCell<Node>>>,
}
impl InternalNode {
    pub fn new(bf: u32) -> Self {
        let bf = bf as usize;
        let length_m1 = if bf > 0 { bf - 1 } else { 0 };
        let length_m2 = if bf > 0 { (bf - 1) * bf } else { 0 };
        let fanout = bf * bf;
        InternalNode {
            node_type: NodeType::InternalNode,
            sv1: None,
            sv2: None,
            m1: vec![0.0; length_m1],
            m2: vec![0.0; length_m2],
            child_nodes: (0..fanout).map(|_| empty_node()).collect(),
        }
    }
}
pub struct LeafNode {
    pub node_type: NodeType,
    pub sv1: Option<Arc<MVPDatapoint>>,
    pub sv2: Option<Arc<MVPDatapoint>>,
    pub points: Vec<Arc<MVPDatapoint>>,
    pub d1: Vec<f32>,
    pub d2: Vec<f32>,
    pub nbpoints: usize,
}
impl LeafNode {
    pub fn new(bf: u32) -> Self {
        let cap = bf as usize;
        LeafNode {
            node_type: NodeType::LeafNode,
            sv1: None,
            sv2: None,
            points: Vec::with_capacity(cap),
            d1: Vec::with_capacity(cap),
            d2: Vec::with_capacity(cap),
            nbpoints: 0,
        }
    }
}
pub enum Node {
    Leaf(LeafNode),
    Internal(InternalNode),
}
pub struct MVPTree {
    pub branch_factor: usize,
    pub path_length: usize,
    pub leaf_capacity: usize,
    pub datatype: MVPDataType,
    pub pos: i64,
    pub size: i64,
    pub pgsize: i64,
    pub buf: Vec<u8>,
    pub node: Option<Rc<RefCell<Node>>>,
    pub distance_function: DistanceFunction,
}

// ---------- helpers ----------

fn empty_node() -> Rc<RefCell<Node>> {
    Rc::new(RefCell::new(Node::Leaf(LeafNode {
        node_type: NodeType::LeafNode,
        sv1: None,
        sv2: None,
        points: Vec::new(),
        d1: Vec::new(),
        d2: Vec::new(),
        nbpoints: 0,
    })))
}

fn is_empty_node(rc: &Rc<RefCell<Node>>) -> bool {
    let b = rc.borrow();
    match &*b {
        Node::Leaf(l) => l.sv1.is_none() && l.nbpoints == 0,
        _ => false,
    }
}

fn arc_into_owned<T: Clone>(a: Arc<T>) -> T {
    Arc::try_unwrap(a).unwrap_or_else(|arc| (*arc).clone())
}

fn select_vps(points: &[MVPDatapoint], dist: DistanceFunction) -> Result<(i32, i32), ()> {
    if points.is_empty() {
        return Err(());
    }
    let mut sv1: i32 = 0;
    let mut sv2: i32 = -1;
    let mut max_d: f32 = 0.0;
    for i in 0..points.len() {
        for j in (i + 1)..points.len() {
            let d = dist(&points[i], &points[j]);
            if d.is_nan() || d < 0.0 {
                return Err(());
            }
            if d > max_d {
                max_d = d;
                sv1 = i as i32;
                sv2 = j as i32;
            }
        }
    }
    Ok((sv1, sv2))
}

fn find_splits_helper(
    points: &[MVPDatapoint],
    vp: &MVPDatapoint,
    dist: DistanceFunction,
    length_m: usize,
) -> Option<Vec<f32>> {
    let nb = points.len();
    if nb == 0 || length_m == 0 {
        return None;
    }
    let mut dists: Vec<f32> = Vec::with_capacity(nb);
    for p in points {
        let d = dist(p, vp);
        if d.is_nan() || d < 0.0 {
            return None;
        }
        dists.push(d);
    }
    dists.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mut m = vec![0.0f32; length_m];
    for i in 0..length_m {
        let mut index = (i + 1) * nb / (length_m + 1);
        if index >= nb {
            index = nb - 1;
        }
        m[i] = dists[index];
    }
    Some(m)
}

fn sort_points_helper(
    points: Vec<MVPDatapoint>,
    vp: &MVPDatapoint,
    bf: usize,
    pivots: &[f32],
    dist: DistanceFunction,
) -> Option<Vec<Vec<MVPDatapoint>>> {
    let length_m1 = if bf == 0 { 0 } else { bf - 1 };
    let mut bins: Vec<Vec<MVPDatapoint>> = (0..bf).map(|_| Vec::new()).collect();
    for p in points.into_iter() {
        let d = dist(&p, vp);
        if d.is_nan() || d < 0.0 {
            return None;
        }
        let mut target_bin: usize = length_m1;
        for k in 0..length_m1 {
            if d <= pivots[k] {
                target_bin = k;
                break;
            }
        }
        bins[target_bin].push(p);
    }
    Some(bins)
}

fn dt_to_u8(t: MVPDataType) -> u8 {
    match t {
        MVPDataType::ByteArray => 1,
        MVPDataType::UInt16Array => 2,
        MVPDataType::UInt32Array => 4,
        MVPDataType::UInt64Array => 8,
    }
}

fn u8_to_dt(b: u8) -> Option<MVPDataType> {
    match b {
        1 => Some(MVPDataType::ByteArray),
        2 => Some(MVPDataType::UInt16Array),
        4 => Some(MVPDataType::UInt32Array),
        8 => Some(MVPDataType::UInt64Array),
        _ => None,
    }
}

// ---------- MVPTree impl ----------

impl MVPTree {
    pub fn new(
        branch_factor: usize,
        path_length: usize,
        leaf_capacity: usize,
        datatype: MVPDataType,
        distance_function: DistanceFunction,
    ) -> Self {
        MVPTree {
            branch_factor,
            path_length,
            leaf_capacity,
            datatype,
            pos: 0,
            size: 0,
            pgsize: 4096,
            buf: Vec::new(),
            node: None,
            distance_function,
        }
    }

    pub fn add(&mut self, mut points: Vec<MVPDatapoint>) -> MVPError {
        if points.is_empty() {
            return MVPError::Success;
        }

        // Type check vs the tree's datatype.
        if points[0].data_type != self.datatype {
            return MVPError::TypeMismatch;
        }

        // Initialize paths for all incoming points.
        for p in &mut points {
            p.path = vec![0.0f32; self.path_length];
        }

        let mut err = MVPError::Success;
        let existing = self.node.take();
        let new_node = self._mvptree_add(existing, points, &mut err, 0);
        self.node = new_node;
        err
    }

    pub fn retrieve(
        &self,
        target: &MVPDatapoint,
        knearest: usize,
        radius: f32,
    ) -> Result<Vec<MVPDatapoint>, MVPError> {
        if knearest == 0 || radius < 0.0 {
            return Err(MVPError::ArgErr);
        }

        let root = match &self.node {
            Some(r) => r.clone(),
            None => return Err(MVPError::EmptyTree),
        };

        let mut results: Vec<Arc<MVPDatapoint>> = Vec::new();
        let mut path = vec![0.0f32; self.path_length];
        let err = self.retrieve_recursive(&root, target, &mut path, radius, knearest, &mut results, 0);
        match err {
            MVPError::Success | MVPError::KNearestCap => {
                Ok(results.into_iter().map(|a| (*a).clone()).collect())
            }
            e => Err(e),
        }
    }

    pub fn write(&self, filename: &str, mode: i32) -> MVPError {
        let _ = mode;
        let root = match &self.node {
            Some(r) => r.clone(),
            None => return MVPError::ArgErr,
        };

        let mut buf: Vec<u8> = Vec::new();

        // Header (32 bytes, zero-padded).
        let tag_bytes = TAG.as_bytes();
        buf.extend_from_slice(tag_bytes);
        buf.push(0u8); // null terminator
        let version_i32: i32 = VERSION as i32;
        buf.extend_from_slice(&version_i32.to_le_bytes());
        buf.push(self.branch_factor as u8);
        buf.push(self.path_length as u8);
        buf.push(self.leaf_capacity as u8);
        buf.push(dt_to_u8(self.datatype));
        while buf.len() < HEADER_SIZE {
            buf.push(0);
        }

        // Write the tree starting from this offset.
        write_node(&root, &mut buf, self.branch_factor, self.path_length, self.leaf_capacity);

        match File::create(filename) {
            Ok(mut f) => {
                if f.write_all(&buf).is_err() {
                    return MVPError::NoWrite;
                }
            }
            Err(_) => return MVPError::FileOpen,
        }

        MVPError::Success
    }

    pub fn print(&self, stream: &mut dyn Write) -> MVPError {
        let node = match &self.node {
            Some(n) => n.clone(),
            None => {
                let _ = writeln!(stream, "NULL0");
                return MVPError::Success;
            }
        };
        let err = print_recursive(stream, self, &node, 0);
        if err != MVPError::Success {
            let _ = writeln!(stream, "malformed tree: {}", error_to_string(err));
        }
        err
    }

    pub fn clear(&mut self, _node: &mut Option<Box<Node>>) {
        self.node = None;
    }

    pub fn extend_mvpfile(&mut self) -> i32 {
        // In-memory: pretend to extend size by pgsize.
        self.size += self.pgsize;
        self.buf.resize(self.size as usize, 0);
        0
    }

    // ---------- internal recursion ----------

    fn _mvptree_add(
        &self,
        node: Option<Rc<RefCell<Node>>>,
        points: Vec<MVPDatapoint>,
        err: &mut MVPError,
        lvl: usize,
    ) -> Option<Rc<RefCell<Node>>> {
        if points.is_empty() {
            return node;
        }

        let node_rc = match node {
            None => return self.create_new_node(points, err, lvl),
            Some(rc) => rc,
        };

        let is_leaf = matches!(&*node_rc.borrow(), Node::Leaf(_));

        if is_leaf {
            let needs_split = {
                let b = node_rc.borrow();
                if let Node::Leaf(l) = &*b {
                    l.nbpoints + points.len() > self.leaf_capacity
                } else {
                    unreachable!()
                }
            };

            if !needs_split {
                self.add_to_leaf(&node_rc, points, err, lvl);
                Some(node_rc)
            } else {
                let leaf = match Rc::try_unwrap(node_rc) {
                    Ok(rc) => match rc.into_inner() {
                        Node::Leaf(l) => l,
                        _ => unreachable!(),
                    },
                    Err(_) => {
                        *err = MVPError::MemAlloc;
                        return None;
                    }
                };
                let mut all_points: Vec<MVPDatapoint> =
                    Vec::with_capacity(leaf.nbpoints + 2 + points.len());
                if let Some(sv1) = leaf.sv1 {
                    all_points.push(arc_into_owned(sv1));
                }
                if let Some(sv2) = leaf.sv2 {
                    all_points.push(arc_into_owned(sv2));
                }
                for p in leaf.points {
                    all_points.push(arc_into_owned(p));
                }
                all_points.extend(points);
                self.create_new_node(all_points, err, lvl)
            }
        } else {
            self.add_to_internal(&node_rc, points, err, lvl);
            Some(node_rc)
        }
    }

    fn add_to_leaf(
        &self,
        node_rc: &Rc<RefCell<Node>>,
        mut points: Vec<MVPDatapoint>,
        err: &mut MVPError,
        lvl: usize,
    ) {
        let dist = self.distance_function;
        let mut b = node_rc.borrow_mut();
        let leaf = match &mut *b {
            Node::Leaf(l) => l,
            _ => return,
        };
        let sv1 = match &leaf.sv1 {
            Some(s) => s.clone(),
            None => return,
        };

        // If sv2 is None and there is at least one point, promote points[0] to sv2.
        if leaf.sv2.is_none() && !points.is_empty() {
            let mut new_sv2 = points.remove(0);
            // path[lvl] for sv2 is dist to sv1, path[lvl+1] is 0.
            let d = dist(&sv1, &new_sv2);
            if lvl < self.path_length {
                new_sv2.path[lvl] = d;
            }
            if lvl + 1 < self.path_length {
                new_sv2.path[lvl + 1] = 0.0;
            }
            leaf.sv2 = Some(Arc::new(new_sv2));
        }

        let sv2 = match &leaf.sv2 {
            Some(s) => s.clone(),
            None => return, // only the original sv1 — nothing to add
        };

        for mut p in points.into_iter() {
            let d1 = dist(&sv1, &p);
            if d1.is_nan() || d1 < 0.0 {
                *err = MVPError::NoSv1Range;
                return;
            }
            let d2 = dist(&sv2, &p);
            if d2.is_nan() || d2 < 0.0 {
                *err = MVPError::NoSv2Range;
                return;
            }
            if lvl < self.path_length {
                p.path[lvl] = d1;
            }
            if lvl + 1 < self.path_length {
                p.path[lvl + 1] = d2;
            }
            leaf.d1.push(d1);
            leaf.d2.push(d2);
            leaf.points.push(Arc::new(p));
            leaf.nbpoints += 1;
        }
    }

    fn add_to_internal(
        &self,
        node_rc: &Rc<RefCell<Node>>,
        mut points: Vec<MVPDatapoint>,
        err: &mut MVPError,
        lvl: usize,
    ) {
        let bf = self.branch_factor;
        let length_m1 = if bf == 0 { 0 } else { bf - 1 };
        let dist = self.distance_function;

        let (sv1, sv2, m1, m2) = {
            let b = node_rc.borrow();
            if let Node::Internal(int) = &*b {
                (
                    int.sv1.clone().expect("internal node sv1"),
                    int.sv2.clone().expect("internal node sv2"),
                    int.m1.clone(),
                    int.m2.clone(),
                )
            } else {
                return;
            }
        };

        // path[lvl] for all incoming points
        for p in &mut points {
            let d = dist(&sv1, p);
            if d.is_nan() || d < 0.0 {
                *err = MVPError::NoSv1Range;
                return;
            }
            if lvl < self.path_length {
                p.path[lvl] = d;
            }
        }

        let bins = match sort_points_helper(points, &sv1, bf, &m1, dist) {
            Some(b) => b,
            None => {
                *err = MVPError::NoSort;
                return;
            }
        };

        for (i, mut bin) in bins.into_iter().enumerate() {
            if bin.is_empty() {
                continue;
            }
            // path[lvl+1] for points in this bin via sv2
            for p in &mut bin {
                let d = dist(&sv2, p);
                if d.is_nan() || d < 0.0 {
                    *err = MVPError::NoSv2Range;
                    return;
                }
                if lvl + 1 < self.path_length {
                    p.path[lvl + 1] = d;
                }
            }

            let m2_i: Vec<f32> = m2[i * length_m1..(i + 1) * length_m1].to_vec();

            let bins2 = match sort_points_helper(bin, &sv2, bf, &m2_i, dist) {
                Some(b) => b,
                None => {
                    *err = MVPError::NoSort;
                    return;
                }
            };

            for (j, bin2) in bins2.into_iter().enumerate() {
                let child_idx = i * bf + j;
                let placeholder = empty_node();
                let original_child = {
                    let mut b = node_rc.borrow_mut();
                    if let Node::Internal(int) = &mut *b {
                        std::mem::replace(&mut int.child_nodes[child_idx], placeholder)
                    } else {
                        return;
                    }
                };

                let child_arg = if is_empty_node(&original_child) {
                    None
                } else {
                    Some(original_child)
                };

                let new_child = self._mvptree_add(child_arg, bin2, err, lvl + 2);
                let new_child_rc = new_child.unwrap_or_else(empty_node);
                {
                    let mut b = node_rc.borrow_mut();
                    if let Node::Internal(int) = &mut *b {
                        int.child_nodes[child_idx] = new_child_rc;
                    }
                }
                if !matches!(*err, MVPError::Success) {
                    return;
                }
            }
        }
    }

    fn create_new_node(
        &self,
        points: Vec<MVPDatapoint>,
        err: &mut MVPError,
        lvl: usize,
    ) -> Option<Rc<RefCell<Node>>> {
        let nb = points.len();
        if nb == 0 {
            return None;
        }
        if nb <= self.leaf_capacity + 2 {
            return self.create_leaf_node(points, err, lvl);
        }
        // Probe first: if all points are identical (sv2 cannot be selected),
        // fall back to a leaf to avoid an undefined / unsplit internal node.
        match select_vps(&points, self.distance_function) {
            Ok((_, sv2_pos)) if sv2_pos < 0 => self.create_leaf_node(points, err, lvl),
            Ok(_) => self.create_internal_node(points, err, lvl),
            Err(_) => {
                *err = MVPError::VpNoSelect;
                None
            }
        }
    }

    fn create_leaf_node(
        &self,
        mut points: Vec<MVPDatapoint>,
        err: &mut MVPError,
        lvl: usize,
    ) -> Option<Rc<RefCell<Node>>> {
        let dist = self.distance_function;
        let (sv1_pos, sv2_pos) = match select_vps(&points, dist) {
            Ok(v) => v,
            Err(_) => {
                *err = MVPError::VpNoSelect;
                return None;
            }
        };

        // Extract sv2 first (sv2_pos > sv1_pos guarantees stability of sv1_pos after).
        let sv2_owned = if sv2_pos >= 0 {
            Some(points.swap_remove(sv2_pos as usize))
        } else {
            None
        };
        let sv1_owned = if sv1_pos >= 0 {
            Some(points.swap_remove(sv1_pos as usize))
        } else {
            None
        };

        let mut sv1_owned = sv1_owned;
        let mut sv2_owned = sv2_owned;

        if let Some(s) = sv1_owned.as_mut() {
            if lvl < self.path_length {
                s.path[lvl] = 0.0;
            }
        }
        if let (Some(ref s1), Some(s2)) = (sv1_owned.as_ref(), sv2_owned.as_mut()) {
            let d = dist(s1, s2);
            if lvl < self.path_length {
                s2.path[lvl] = d;
            }
            if lvl + 1 < self.path_length {
                s2.path[lvl + 1] = 0.0;
            }
        }

        let mut d1_vec: Vec<f32> = Vec::with_capacity(points.len());
        let mut d2_vec: Vec<f32> = Vec::with_capacity(points.len());
        let mut new_points: Vec<Arc<MVPDatapoint>> = Vec::with_capacity(points.len());

        for mut p in points.into_iter() {
            let d1 = if let Some(s) = sv1_owned.as_ref() {
                let d = dist(s, &p);
                if d.is_nan() || d < 0.0 {
                    *err = MVPError::NoSv1Range;
                    return None;
                }
                d
            } else {
                0.0
            };
            let d2 = if let Some(s) = sv2_owned.as_ref() {
                let d = dist(s, &p);
                if d.is_nan() || d < 0.0 {
                    *err = MVPError::NoSv2Range;
                    return None;
                }
                d
            } else {
                0.0
            };
            if lvl < self.path_length {
                p.path[lvl] = d1;
            }
            if lvl + 1 < self.path_length {
                p.path[lvl + 1] = d2;
            }
            d1_vec.push(d1);
            d2_vec.push(d2);
            new_points.push(Arc::new(p));
        }

        let nbpoints = new_points.len();
        let leaf = LeafNode {
            node_type: NodeType::LeafNode,
            sv1: sv1_owned.map(Arc::new),
            sv2: sv2_owned.map(Arc::new),
            points: new_points,
            d1: d1_vec,
            d2: d2_vec,
            nbpoints,
        };

        Some(Rc::new(RefCell::new(Node::Leaf(leaf))))
    }

    fn create_internal_node(
        &self,
        mut points: Vec<MVPDatapoint>,
        err: &mut MVPError,
        lvl: usize,
    ) -> Option<Rc<RefCell<Node>>> {
        let bf = self.branch_factor;
        let length_m1 = if bf == 0 { 0 } else { bf - 1 };
        let dist = self.distance_function;

        let (sv1_pos, sv2_pos) = match select_vps(&points, dist) {
            Ok(v) => v,
            Err(_) => {
                *err = MVPError::VpNoSelect;
                return None;
            }
        };

        if sv2_pos < 0 {
            // not enough variety to split
            *err = MVPError::VpNoSelect;
            return None;
        }

        let mut sv2 = points.swap_remove(sv2_pos as usize);
        let mut sv1 = points.swap_remove(sv1_pos as usize);

        if lvl < self.path_length {
            sv1.path[lvl] = 0.0;
        }
        let d_sv1_sv2 = dist(&sv1, &sv2);
        if lvl < self.path_length {
            sv2.path[lvl] = d_sv1_sv2;
        }
        if lvl + 1 < self.path_length {
            sv2.path[lvl + 1] = 0.0;
        }

        // path[lvl] for the rest
        for p in &mut points {
            let d = dist(&sv1, p);
            if d.is_nan() || d < 0.0 {
                *err = MVPError::NoSv1Range;
                return None;
            }
            if lvl < self.path_length {
                p.path[lvl] = d;
            }
        }

        let m1 = match find_splits_helper(&points, &sv1, dist, length_m1) {
            Some(m) => m,
            None => {
                *err = MVPError::NoSplits;
                return None;
            }
        };

        let bins = match sort_points_helper(points, &sv1, bf, &m1, dist) {
            Some(b) => b,
            None => {
                *err = MVPError::NoSort;
                return None;
            }
        };

        let mut m2 = vec![0.0f32; length_m1 * bf];
        let mut child_nodes: Vec<Rc<RefCell<Node>>> = Vec::with_capacity(bf * bf);

        for (i, mut bin) in bins.into_iter().enumerate() {
            // path[lvl+1] for the points in this bin
            for p in &mut bin {
                let d = dist(&sv2, p);
                if d.is_nan() || d < 0.0 {
                    *err = MVPError::NoSv2Range;
                    return None;
                }
                if lvl + 1 < self.path_length {
                    p.path[lvl + 1] = d;
                }
            }

            let m2_i = if !bin.is_empty() {
                match find_splits_helper(&bin, &sv2, dist, length_m1) {
                    Some(m) => m,
                    None => {
                        *err = MVPError::NoSplits;
                        return None;
                    }
                }
            } else {
                vec![0.0f32; length_m1]
            };

            for k in 0..length_m1 {
                m2[i * length_m1 + k] = m2_i[k];
            }

            let bins2 = if !bin.is_empty() {
                match sort_points_helper(bin, &sv2, bf, &m2_i, dist) {
                    Some(b) => b,
                    None => {
                        *err = MVPError::NoSort;
                        return None;
                    }
                }
            } else {
                (0..bf).map(|_| Vec::new()).collect()
            };

            for bin2 in bins2.into_iter() {
                if bin2.is_empty() {
                    child_nodes.push(empty_node());
                } else {
                    let child = self._mvptree_add(None, bin2, err, lvl + 2);
                    child_nodes.push(child.unwrap_or_else(empty_node));
                }
            }
        }

        let internal = InternalNode {
            node_type: NodeType::InternalNode,
            sv1: Some(Arc::new(sv1)),
            sv2: Some(Arc::new(sv2)),
            m1,
            m2,
            child_nodes,
        };

        Some(Rc::new(RefCell::new(Node::Internal(internal))))
    }

    fn retrieve_recursive(
        &self,
        node_rc: &Rc<RefCell<Node>>,
        target: &MVPDatapoint,
        path: &mut Vec<f32>,
        radius: f32,
        knearest: usize,
        results: &mut Vec<Arc<MVPDatapoint>>,
        lvl: usize,
    ) -> MVPError {
        if is_empty_node(node_rc) {
            return MVPError::Success;
        }
        let dist = self.distance_function;
        let bf = self.branch_factor;
        let length_m1 = if bf == 0 { 0 } else { bf - 1 };

        let kind_is_leaf = matches!(&*node_rc.borrow(), Node::Leaf(_));

        if kind_is_leaf {
            let b = node_rc.borrow();
            let leaf = if let Node::Leaf(l) = &*b {
                l
            } else {
                return MVPError::Unrecognized;
            };
            let sv1 = match &leaf.sv1 {
                Some(s) => s.clone(),
                None => return MVPError::Success,
            };
            let d1 = dist(target, &sv1);
            if d1.is_nan() || d1 < 0.0 {
                return MVPError::BadDistVal;
            }
            if lvl < self.path_length {
                path[lvl] = d1;
            }
            if d1 <= radius {
                results.push(sv1.clone());
                if results.len() >= knearest {
                    return MVPError::KNearestCap;
                }
            }
            if let Some(sv2) = &leaf.sv2 {
                let d2 = dist(target, sv2);
                if d2.is_nan() || d2 < 0.0 {
                    return MVPError::BadDistVal;
                }
                if d2 <= radius {
                    results.push(sv2.clone());
                    if results.len() >= knearest {
                        return MVPError::KNearestCap;
                    }
                }
                if lvl + 1 < self.path_length {
                    path[lvl + 1] = d2;
                }

                for i in 0..leaf.nbpoints {
                    if d1 - radius <= leaf.d1[i] && d1 + radius >= leaf.d1[i] {
                        if d2 - radius <= leaf.d2[i] && d2 + radius >= leaf.d2[i] {
                            let endpath = if lvl + 1 < self.path_length {
                                lvl + 1
                            } else {
                                self.path_length
                            };
                            let mut skip = false;
                            for j in 0..endpath {
                                if j >= leaf.points[i].path.len() {
                                    skip = true;
                                    break;
                                }
                                let pp = leaf.points[i].path[j];
                                if path[j] - radius <= pp && path[j] + radius >= pp {
                                    continue;
                                } else {
                                    skip = true;
                                    break;
                                }
                            }
                            if !skip {
                                let d = dist(target, &leaf.points[i]);
                                if d.is_nan() || d < 0.0 {
                                    return MVPError::BadDistVal;
                                }
                                if d <= radius {
                                    results.push(leaf.points[i].clone());
                                    if results.len() >= knearest {
                                        return MVPError::KNearestCap;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            MVPError::Success
        } else {
            // Internal node — extract everything we need so we can drop the borrow.
            let (sv1, sv2, m1, m2, children) = {
                let b = node_rc.borrow();
                if let Node::Internal(int) = &*b {
                    (
                        int.sv1.clone().expect("sv1"),
                        int.sv2.clone().expect("sv2"),
                        int.m1.clone(),
                        int.m2.clone(),
                        int.child_nodes.clone(),
                    )
                } else {
                    return MVPError::Unrecognized;
                }
            };

            let d1 = dist(target, &sv1);
            if d1.is_nan() || d1 < 0.0 {
                return MVPError::BadDistVal;
            }
            if d1 <= radius {
                results.push(sv1.clone());
                if results.len() >= knearest {
                    return MVPError::KNearestCap;
                }
            }
            if lvl < self.path_length {
                path[lvl] = d1;
            }
            let d2 = dist(target, &sv2);
            if d2.is_nan() || d2 < 0.0 {
                return MVPError::BadDistVal;
            }
            if d2 <= radius {
                results.push(sv2.clone());
                if results.len() >= knearest {
                    return MVPError::KNearestCap;
                }
            }
            if lvl + 1 < self.path_length {
                path[lvl + 1] = d2;
            }

            // First-level bins 0..lengthM1
            for i in 0..length_m1 {
                if d1 - radius <= m1[i] {
                    for j in 0..length_m1 {
                        if d2 - radius <= m2[i * length_m1 + j] {
                            let err = self.retrieve_recursive(
                                &children[i * bf + j],
                                target,
                                path,
                                radius,
                                knearest,
                                results,
                                lvl + 2,
                            );
                            if !matches!(err, MVPError::Success) {
                                return err;
                            }
                        }
                    }
                    if length_m1 > 0 && d2 + radius >= m2[i * length_m1 + length_m1 - 1] {
                        let err = self.retrieve_recursive(
                            &children[i * bf + length_m1],
                            target,
                            path,
                            radius,
                            knearest,
                            results,
                            lvl + 2,
                        );
                        if !matches!(err, MVPError::Success) {
                            return err;
                        }
                    }
                }
            }

            // Last first-level bin
            if length_m1 > 0 && d1 + radius >= m1[length_m1 - 1] {
                for j in 0..length_m1 {
                    if d2 - radius <= m2[length_m1 * length_m1 + j] {
                        let err = self.retrieve_recursive(
                            &children[bf * length_m1 + j],
                            target,
                            path,
                            radius,
                            knearest,
                            results,
                            lvl + 2,
                        );
                        if !matches!(err, MVPError::Success) {
                            return err;
                        }
                    }
                }
                if d2 + radius >= m2[length_m1 * length_m1 + length_m1 - 1] {
                    let err = self.retrieve_recursive(
                        &children[bf * length_m1 + length_m1],
                        target,
                        path,
                        radius,
                        knearest,
                        results,
                        lvl + 2,
                    );
                    if !matches!(err, MVPError::Success) {
                        return err;
                    }
                }
            }

            MVPError::Success
        }
    }
}

// ---------- write helpers ----------

fn write_datapoint(
    dp: Option<&MVPDatapoint>,
    buf: &mut Vec<u8>,
    pathlength: usize,
) -> usize {
    let start = buf.len();
    match dp {
        None => {
            buf.push(0u8);
            buf.extend_from_slice(&0u32.to_le_bytes());
        }
        Some(dp) => {
            let active = 1u8;
            let idlen = dp.id.len() as u8;
            let datalength = dp.datalen as u32;
            let dtype = dt_to_u8(dp.data_type);
            let bytelength: u32 = 1u32
                + idlen as u32
                + 4
                + datalength * dtype as u32
                + (pathlength as u32) * 4;
            buf.push(active);
            buf.extend_from_slice(&bytelength.to_le_bytes());
            buf.push(idlen);
            buf.extend_from_slice(dp.id.as_bytes());
            buf.extend_from_slice(&datalength.to_le_bytes());
            // data is exactly datalen*dtype bytes (Vec<u8>)
            let expected = (datalength as usize) * (dtype as usize);
            if dp.data.len() >= expected {
                buf.extend_from_slice(&dp.data[..expected]);
            } else {
                buf.extend_from_slice(&dp.data);
                for _ in dp.data.len()..expected {
                    buf.push(0);
                }
            }
            // path
            for k in 0..pathlength {
                let v = if k < dp.path.len() { dp.path[k] } else { 0.0 };
                buf.extend_from_slice(&v.to_le_bytes());
            }
        }
    }
    start
}

fn write_node(
    rc: &Rc<RefCell<Node>>,
    buf: &mut Vec<u8>,
    bf: usize,
    pathlength: usize,
    leafcap: usize,
) -> usize {
    if is_empty_node(rc) {
        return 0;
    }
    let start = buf.len();
    let b = rc.borrow();
    match &*b {
        Node::Leaf(leaf) => {
            buf.push(NodeType::LeafNode as u8);
            // Need to clone Arcs to drop the borrow before recursion (although datapoint
            // writes do not recurse — still safe).
            let sv1 = leaf.sv1.clone();
            let sv2 = leaf.sv2.clone();
            write_datapoint(sv1.as_deref(), buf, pathlength);
            write_datapoint(sv2.as_deref(), buf, pathlength);
            let nbpoints = leaf.nbpoints as u32;
            buf.extend_from_slice(&nbpoints.to_le_bytes());

            let placeholder_pos = buf.len();
            buf.resize(buf.len() + leafcap * 16, 0);

            for i in 0..leaf.nbpoints {
                let entry_pos = placeholder_pos + i * 16;
                buf[entry_pos..entry_pos + 4].copy_from_slice(&leaf.d1[i].to_le_bytes());
                buf[entry_pos + 4..entry_pos + 8].copy_from_slice(&leaf.d2[i].to_le_bytes());
                let offset =
                    write_datapoint(Some(&leaf.points[i]), buf, pathlength) as u64;
                buf[entry_pos + 8..entry_pos + 16].copy_from_slice(&offset.to_le_bytes());
            }
        }
        Node::Internal(int) => {
            buf.push(NodeType::InternalNode as u8);
            let sv1 = int.sv1.clone();
            let sv2 = int.sv2.clone();
            write_datapoint(sv1.as_deref(), buf, pathlength);
            write_datapoint(sv2.as_deref(), buf, pathlength);

            for v in &int.m1 {
                buf.extend_from_slice(&v.to_le_bytes());
            }
            for v in &int.m2 {
                buf.extend_from_slice(&v.to_le_bytes());
            }

            let fanout = bf * bf;
            let placeholder_pos = buf.len();
            buf.resize(buf.len() + fanout * 9, 0);

            // We need to drop the borrow before recursing (children are Rc<RefCell>...).
            let children: Vec<Rc<RefCell<Node>>> = int.child_nodes.clone();
            drop(b);

            for i in 0..fanout {
                let entry_pos = placeholder_pos + i * 9;
                let offset = write_node(&children[i], buf, bf, pathlength, leafcap) as u64;
                buf[entry_pos] = 0;
                buf[entry_pos + 1..entry_pos + 9].copy_from_slice(&offset.to_le_bytes());
            }
        }
    }
    start
}

// ---------- read ----------

pub fn mvptree_read(
    filename: &str,
    distance_function: DistanceFunction,
) -> Result<MVPTree, MVPError> {
    let mut file = match File::open(filename) {
        Ok(f) => f,
        Err(_) => return Err(MVPError::FileNotFound),
    };
    let mut buf: Vec<u8> = Vec::new();
    if file.read_to_end(&mut buf).is_err() {
        return Err(MVPError::FileOpen);
    }
    if buf.len() < HEADER_SIZE {
        return Err(MVPError::ArgErr);
    }

    let tag_len = TAG.len();
    let mut hpos = tag_len + 1;
    if hpos + 4 + 4 > buf.len() {
        return Err(MVPError::ArgErr);
    }
    let _v = i32::from_le_bytes(buf[hpos..hpos + 4].try_into().unwrap());
    hpos += 4;
    let bf = buf[hpos] as usize;
    hpos += 1;
    let pl = buf[hpos] as usize;
    hpos += 1;
    let lc = buf[hpos] as usize;
    hpos += 1;
    let ht = buf[hpos];
    let datatype = match u8_to_dt(ht) {
        Some(d) => d,
        None => return Err(MVPError::TypeMismatch),
    };

    let mut tree = MVPTree::new(bf, pl, lc, datatype, distance_function);

    let mut pos = HEADER_SIZE;
    let mut err = MVPError::Success;
    let node = read_node(&buf, &mut pos, bf, pl, lc, datatype, &mut err);
    if !matches!(err, MVPError::Success) {
        return Err(err);
    }
    tree.node = node;
    Ok(tree)
}

fn read_datapoint(
    buf: &[u8],
    pos: &mut usize,
    pathlength: usize,
    datatype: MVPDataType,
) -> Option<MVPDatapoint> {
    if *pos + 5 > buf.len() {
        return None;
    }
    let active = buf[*pos];
    *pos += 1;
    let bytelength = u32::from_le_bytes(buf[*pos..*pos + 4].try_into().unwrap());
    *pos += 4;

    if active == 0 && bytelength == 0 {
        return None;
    }

    let idlen = buf[*pos] as usize;
    *pos += 1;
    let id = String::from_utf8_lossy(&buf[*pos..*pos + idlen]).into_owned();
    *pos += idlen;
    let datalength = u32::from_le_bytes(buf[*pos..*pos + 4].try_into().unwrap()) as usize;
    *pos += 4;

    let type_size = dt_to_u8(datatype) as usize;
    let data_bytes = datalength * type_size;
    let data = buf[*pos..*pos + data_bytes].to_vec();
    *pos += data_bytes;

    let mut path = Vec::with_capacity(pathlength);
    for _ in 0..pathlength {
        let v = f32::from_le_bytes(buf[*pos..*pos + 4].try_into().unwrap());
        path.push(v);
        *pos += 4;
    }

    Some(MVPDatapoint {
        id,
        data,
        path,
        datalen: datalength,
        data_type: datatype,
    })
}

fn read_node(
    buf: &[u8],
    pos: &mut usize,
    bf: usize,
    pl: usize,
    lc: usize,
    datatype: MVPDataType,
    err: &mut MVPError,
) -> Option<Rc<RefCell<Node>>> {
    if *pos >= buf.len() {
        return None;
    }
    let node_type = buf[*pos];
    *pos += 1;

    if node_type == NodeType::LeafNode as u8 {
        let sv1 = read_datapoint(buf, pos, pl, datatype);
        let sv2 = read_datapoint(buf, pos, pl, datatype);
        let nbpoints = u32::from_le_bytes(buf[*pos..*pos + 4].try_into().unwrap()) as usize;
        *pos += 4;

        let entries_pos = *pos;

        let mut d1: Vec<f32> = Vec::with_capacity(nbpoints);
        let mut d2: Vec<f32> = Vec::with_capacity(nbpoints);
        let mut points: Vec<Arc<MVPDatapoint>> = Vec::with_capacity(nbpoints);

        for i in 0..nbpoints {
            let entry_off = entries_pos + i * 16;
            let v1 = f32::from_le_bytes(buf[entry_off..entry_off + 4].try_into().unwrap());
            let v2 = f32::from_le_bytes(buf[entry_off + 4..entry_off + 8].try_into().unwrap());
            let offset = u64::from_le_bytes(
                buf[entry_off + 8..entry_off + 16].try_into().unwrap(),
            ) as usize;
            d1.push(v1);
            d2.push(v2);

            let mut p_pos = offset;
            if let Some(dp) = read_datapoint(buf, &mut p_pos, pl, datatype) {
                points.push(Arc::new(dp));
            }
        }

        let leaf = LeafNode {
            node_type: NodeType::LeafNode,
            sv1: sv1.map(Arc::new),
            sv2: sv2.map(Arc::new),
            points,
            d1,
            d2,
            nbpoints,
        };

        Some(Rc::new(RefCell::new(Node::Leaf(leaf))))
    } else if node_type == NodeType::InternalNode as u8 {
        let sv1 = read_datapoint(buf, pos, pl, datatype);
        let sv2 = read_datapoint(buf, pos, pl, datatype);
        let length_m1 = if bf == 0 { 0 } else { bf - 1 };
        let length_m2 = if bf == 0 { 0 } else { (bf - 1) * bf };
        let fanout = bf * bf;

        let mut m1: Vec<f32> = Vec::with_capacity(length_m1);
        for _ in 0..length_m1 {
            let v = f32::from_le_bytes(buf[*pos..*pos + 4].try_into().unwrap());
            m1.push(v);
            *pos += 4;
        }
        let mut m2: Vec<f32> = Vec::with_capacity(length_m2);
        for _ in 0..length_m2 {
            let v = f32::from_le_bytes(buf[*pos..*pos + 4].try_into().unwrap());
            m2.push(v);
            *pos += 4;
        }

        let entries_pos = *pos;
        let mut child_nodes: Vec<Rc<RefCell<Node>>> = Vec::with_capacity(fanout);
        for i in 0..fanout {
            let entry_off = entries_pos + i * 9;
            let _fileno = buf[entry_off];
            let offset = u64::from_le_bytes(
                buf[entry_off + 1..entry_off + 9].try_into().unwrap(),
            ) as usize;
            if offset == 0 {
                child_nodes.push(empty_node());
            } else {
                let mut c_pos = offset;
                let child = read_node(buf, &mut c_pos, bf, pl, lc, datatype, err);
                child_nodes.push(child.unwrap_or_else(empty_node));
                if !matches!(*err, MVPError::Success) {
                    break;
                }
            }
        }

        let internal = InternalNode {
            node_type: NodeType::InternalNode,
            sv1: sv1.map(Arc::new),
            sv2: sv2.map(Arc::new),
            m1,
            m2,
            child_nodes,
        };
        Some(Rc::new(RefCell::new(Node::Internal(internal))))
    } else {
        *err = MVPError::Unrecognized;
        None
    }
}

fn print_recursive(
    stream: &mut dyn Write,
    tree: &MVPTree,
    rc: &Rc<RefCell<Node>>,
    lvl: usize,
) -> MVPError {
    if is_empty_node(rc) {
        let _ = writeln!(stream, "NULL{}", lvl);
        return MVPError::Success;
    }
    let bf = tree.branch_factor;
    let length_m1 = if bf == 0 { 0 } else { bf - 1 };
    let length_m2 = bf;
    let fanout = bf * bf;
    let b = rc.borrow();
    match &*b {
        Node::Leaf(leaf) => {
            let _ = writeln!(stream, "LEAF{}  ({} points)", lvl, leaf.nbpoints);
            if let Some(sv1) = &leaf.sv1 {
                let _ = writeln!(stream, "    sv1: {}", sv1.id);
            }
            if let Some(sv2) = &leaf.sv2 {
                let _ = writeln!(stream, "    sv2: {}", sv2.id);
            }
            for i in 0..leaf.nbpoints {
                let _ = writeln!(stream, "        point[{}]: {}", i, leaf.points[i].id);
            }
            MVPError::Success
        }
        Node::Internal(int) => {
            let _ = writeln!(stream, "INTERNAL{}", lvl);
            if let Some(sv1) = &int.sv1 {
                let _ = writeln!(stream, "  sv1: {}", sv1.id);
            }
            if let Some(sv2) = &int.sv2 {
                let _ = writeln!(stream, "  sv2: {}", sv2.id);
            }
            for i in 0..length_m1 {
                let _ = write!(stream, "  M1[{}] = {:.4};", i, int.m1[i]);
            }
            for i in 0..length_m2 {
                if i < int.m2.len() {
                    let _ = write!(stream, "  M2[{}] = {:.4};", i, int.m2[i]);
                }
            }
            let _ = writeln!(stream);
            let children = int.child_nodes.clone();
            drop(b);
            let mut err = MVPError::Success;
            for i in 0..fanout {
                err = print_recursive(stream, tree, &children[i], lvl + 2);
                if err != MVPError::Success {
                    break;
                }
            }
            err
        }
    }
}

impl MVPDatapoint {
    pub fn new(id: String, data: Vec<u8>, data_type: MVPDataType) -> Self {
        let datalen = data.len();
        MVPDatapoint {
            id,
            data,
            path: vec![],
            datalen,
            data_type,
        }
    }
    pub fn select_vantage_points(
        &mut self,
        _nb: u32,
        _sv1_pos: i32,
        _sv2_pos: i32,
        _dist: DistanceFunction,
    ) -> i32 {
        0
    }
    pub fn find_splits(
        &mut self,
        _nb: u32,
        _vp: &MVPDatapoint,
        _tree: &MVPTree,
        _length_m: u32,
    ) -> f32 {
        0.0
    }
    pub fn sort_points(
        &mut self,
        _nb: u32,
        _sv1_pos: i32,
        _sv2_pos: i32,
        _vp: &MVPDatapoint,
        _tree: &MVPTree,
        _counts: &mut Vec<Vec<i32>>,
        _pivots: Vec<f32>,
    ) -> Vec<Vec<Vec<Arc<MVPDatapoint>>>> {
        Vec::new()
    }
    pub fn find_distance_range_for_vp(
        &mut self,
        _nb: u32,
        _vp: &MVPDatapoint,
        _tree: &MVPTree,
        _level: i32,
    ) -> i32 {
        0
    }
    pub fn write(&self, _tree: &MVPTree) -> i64 {
        0
    }
}
pub fn error_to_string(error: MVPError) -> &'static str {
    let idx = error as usize;
    if idx < ERROR_MSGS.len() {
        ERROR_MSGS[idx]
    } else {
        "unknown error"
    }
}
