use std::fs::File;
use std::io::{self, Read, Write};
use std::ptr;
use std::os::raw::c_int;
use std::sync::Arc;
use std::rc::Rc;
use std::cell::RefCell;

// Suppress unused-import warnings on the imports that came in the original
// skeleton; we keep them around to preserve the file shape.
#[allow(dead_code)]
fn _unused_imports_keep() {
    let _ = std::mem::size_of::<c_int>();
    let _ = ptr::null::<u8>();
    let _: io::Result<()> = Ok(());
}
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

impl MVPDataType {
    fn byte_size(&self) -> usize {
        match self {
            MVPDataType::ByteArray => 1,
            MVPDataType::UInt16Array => 2,
            MVPDataType::UInt32Array => 4,
            MVPDataType::UInt64Array => 8,
        }
    }

    fn from_u8(v: u8) -> Option<MVPDataType> {
        match v {
            1 => Some(MVPDataType::ByteArray),
            2 => Some(MVPDataType::UInt16Array),
            4 => Some(MVPDataType::UInt32Array),
            8 => Some(MVPDataType::UInt64Array),
            _ => None,
        }
    }
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

impl MVPError {
    fn index(&self) -> usize {
        match self {
            MVPError::Success => 0,
            MVPError::ArgErr => 1,
            MVPError::NoDistanceFunc => 2,
            MVPError::MemAlloc => 3,
            MVPError::NoLeaf => 4,
            MVPError::NoInternal => 5,
            MVPError::PathAlloc => 6,
            MVPError::VpNoSelect => 7,
            MVPError::NoSv1Range => 8,
            MVPError::NoSv2Range => 9,
            MVPError::NoSpace => 10,
            MVPError::NoSort => 11,
            MVPError::FileOpen => 12,
            MVPError::FileClose => 13,
            MVPError::MemMap => 14,
            MVPError::Munmap => 15,
            MVPError::NoWrite => 16,
            MVPError::FileTruncate => 17,
            MVPError::MremapFail => 18,
            MVPError::TypeMismatch => 19,
            MVPError::KNearestCap => 20,
            MVPError::EmptyTree => 21,
            MVPError::NoSplits => 22,
            MVPError::BadDistVal => 23,
            MVPError::FileNotFound => 24,
            MVPError::Unrecognized => 24,
        }
    }
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
impl InternalNode{
    pub fn new(bf:u32) -> Self {
        let bf = bf as usize;
        let lengthm1 = if bf > 0 { bf - 1 } else { 0 };
        let lengthm2 = if bf > 0 { (bf - 1) * bf } else { 0 };
        let fanout = bf * bf;
        let mut child_nodes = Vec::with_capacity(fanout);
        for _ in 0..fanout {
            child_nodes.push(make_null_node());
        }
        InternalNode {
            node_type: NodeType::InternalNode,
            sv1: None,
            sv2: None,
            m1: vec![0.0f32; lengthm1],
            m2: vec![0.0f32; lengthm2],
            child_nodes,
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
    pub fn new(bf:u32) -> Self {
        // here `bf` is interpreted as the leaf capacity
        let cap = bf as usize;
        LeafNode {
            node_type: NodeType::LeafNode,
            sv1: None,
            sv2: None,
            points: Vec::with_capacity(cap),
            d1: vec![0.0f32; cap],
            d2: vec![0.0f32; cap],
            nbpoints: 0,
        }
    }
}
pub enum Node {
    Leaf(LeafNode),
    Internal(InternalNode),
}

/// Sentinel "null" node -- an empty leaf that represents the C `NULL` child pointer.
fn make_null_node() -> Rc<RefCell<Node>> {
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

fn is_null_node(node: &Rc<RefCell<Node>>) -> bool {
    match &*node.borrow() {
        Node::Leaf(l) => {
            l.sv1.is_none() && l.sv2.is_none() && l.nbpoints == 0 && l.points.is_empty() && l.d1.is_empty() && l.d2.is_empty()
        }
        Node::Internal(_) => false,
    }
}

fn is_nan_or_neg(x: f32) -> bool {
    x.is_nan() || x < 0.0
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

impl MVPTree {
    pub fn new(branch_factor: usize, path_length: usize, leaf_capacity: usize, datatype: MVPDataType, distance_function: DistanceFunction) -> Self {
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

    pub fn add(&mut self, points: Vec<MVPDatapoint>) -> MVPError {
        if points.is_empty() {
            return MVPError::Success;
        }

        // datatype check (C tracks datatype 0 for "uninitialized" but we have an enum,
        // so we just compare directly).
        if self.datatype != points[0].data_type {
            // If datatype matches what was set in `new`, it's fine. Mimic C's
            // type-mismatch behaviour for any actual mismatch with the existing
            // type once tree has data.
            if self.node.is_some() {
                return MVPError::TypeMismatch;
            }
            self.datatype = points[0].data_type;
        }

        // Allocate path array for each point (zero-filled), and wrap into Arc
        let mut arc_points: Vec<Arc<MVPDatapoint>> = Vec::with_capacity(points.len());
        for mut p in points {
            p.path = vec![0.0f32; self.path_length];
            arc_points.push(Arc::new(p));
        }

        let dist = self.distance_function;
        let bf = self.branch_factor;
        let leafcap = self.leaf_capacity;
        let pathlen = self.path_length;

        let existing = self.node.take();
        let mut error = MVPError::Success;
        let new_node = mvptree_add_recursive(
            existing,
            &mut arc_points,
            dist,
            bf,
            leafcap,
            pathlen,
            &mut error,
            0,
        );
        self.node = new_node;
        error
    }

    pub fn retrieve(&self, target: &MVPDatapoint, knearest: usize, radius: f32) -> Result<Vec<MVPDatapoint>, MVPError> {
        if knearest == 0 || radius < 0.0 {
            return Err(MVPError::ArgErr);
        }

        let node = match &self.node {
            Some(n) => n,
            None => return Err(MVPError::EmptyTree),
        };

        // Build a mutable target with path
        let mut target_path = vec![0.0f32; self.path_length];
        let mut results: Vec<Arc<MVPDatapoint>> = Vec::with_capacity(knearest);
        let mut err = MVPError::Success;
        retrieve_recursive(
            node,
            target,
            &mut target_path,
            radius,
            knearest,
            self.branch_factor,
            self.path_length,
            self.distance_function,
            &mut results,
            &mut err,
            0,
        );

        if err != MVPError::Success && err != MVPError::KNearestCap {
            return Err(err);
        }

        // Convert to Vec<MVPDatapoint>
        let vec = results.into_iter().map(|arc| (*arc).clone()).collect();
        Ok(vec)
    }

    pub fn write(&self, filename: &str, mode: i32) -> MVPError {
        let _ = mode;
        if self.node.is_none() {
            return MVPError::ArgErr;
        }

        let mut buf: Vec<u8> = Vec::new();

        // Build header (HEADER_SIZE bytes total)
        let tag_bytes = TAG.as_bytes();
        buf.extend_from_slice(tag_bytes);
        buf.push(0u8); // null terminator
        // version as little-endian i32
        let version: i32 = VERSION as i32;
        buf.extend_from_slice(&version.to_le_bytes());

        let bf = self.branch_factor as u8;
        let pl = self.path_length as u8;
        let lc = self.leaf_capacity as u8;
        let ht = self.datatype as u8;
        buf.push(bf);
        buf.push(pl);
        buf.push(lc);
        buf.push(ht);

        // Pad to HEADER_SIZE
        while buf.len() < HEADER_SIZE {
            buf.push(0u8);
        }

        // Write the tree nodes
        let mut error = MVPError::Success;
        write_node_recursive(
            self.node.as_ref().unwrap(),
            &mut buf,
            self.branch_factor,
            self.path_length,
            self.leaf_capacity,
            &mut error,
        );

        if error != MVPError::Success {
            return error;
        }

        // Write to file
        match File::create(filename) {
            Ok(mut f) => match f.write_all(&buf) {
                Ok(_) => MVPError::Success,
                Err(_) => MVPError::NoWrite,
            },
            Err(_) => MVPError::FileOpen,
        }
    }

    pub fn print(&self, stream: &mut dyn Write) -> MVPError {
        match &self.node {
            None => {
                let _ = writeln!(stream, "NULL0");
                MVPError::Success
            }
            Some(n) => {
                let mut err = MVPError::Success;
                print_node_recursive(stream, n, self.branch_factor, 0, &mut err);
                if err != MVPError::Success {
                    let _ = writeln!(stream, "malformed tree: {}", error_to_string(err));
                }
                err
            }
        }
    }

    pub fn clear(&mut self, _node: &mut Option<Box<Node>>) {
        // Drop all references; Rust will clean up via RAII.
        self.node = None;
    }

    pub fn extend_mvpfile(&mut self) -> i32 {
        // In our pure-Rust implementation we use an in-memory `Vec<u8>` so
        // "extending the file" is just growing the buffer by `pgsize`.
        if self.pgsize <= 0 {
            self.pgsize = 4096;
        }
        let extra = self.pgsize as usize;
        self.size += self.pgsize;
        self.buf.resize(self.buf.len() + extra, 0u8);
        0
    }
}

// ---------------------------------------------------------------------------
// Internal helpers (mirroring the static C functions)
// ---------------------------------------------------------------------------

fn select_vantage_points_arr(
    points: &[Arc<MVPDatapoint>],
    dist: DistanceFunction,
) -> Result<(i32, i32), i32> {
    let nb = points.len();
    if nb == 0 {
        return Err(-1);
    }
    let mut sv1_pos: i32 = if nb >= 1 { 0 } else { -1 };
    let mut sv2_pos: i32 = -1;

    let mut max_dist = 0.0f32;
    for i in 0..nb {
        for j in (i + 1)..nb {
            let d = dist(&points[i], &points[j]);
            if is_nan_or_neg(d) {
                return Err(-2);
            }
            if d > max_dist {
                max_dist = d;
                sv1_pos = i as i32;
                sv2_pos = j as i32;
            }
        }
    }
    Ok((sv1_pos, sv2_pos))
}

fn find_splits_arr(
    points: &[Arc<MVPDatapoint>],
    vp: &MVPDatapoint,
    dist: DistanceFunction,
    m: &mut [f32],
) -> i32 {
    let nb = points.len();
    let length_m = m.len();
    if nb == 0 || length_m == 0 {
        return -1;
    }
    let mut dists: Vec<f32> = Vec::with_capacity(nb);
    for p in points.iter() {
        let d = dist(p, vp);
        if is_nan_or_neg(d) {
            return -2;
        }
        dists.push(d);
    }
    // selection sort, mirroring C
    for i in 0..(nb.saturating_sub(1)) {
        let mut min_pos = i;
        for j in (i + 1)..nb {
            if dists[j] < dists[min_pos] {
                min_pos = j;
            }
        }
        if min_pos != i {
            dists.swap(min_pos, i);
        }
    }
    for i in 0..length_m {
        let mut index = ((i + 1) * nb) / (length_m + 1);
        if index >= nb {
            index = nb - 1;
        }
        m[i] = dists[index];
    }
    0
}

/// Sort `points` into `bf` bins by distance from vp, skipping
/// the entries at sv1_pos / sv2_pos (use -1 to skip nothing).
/// Returns Some((bins, counts)) on success, None on bad distance.
fn sort_points_arr(
    points: &[Arc<MVPDatapoint>],
    sv1_pos: i32,
    sv2_pos: i32,
    vp: &MVPDatapoint,
    dist: DistanceFunction,
    bf: usize,
    pivots: &[f32],
) -> Option<(Vec<Vec<Arc<MVPDatapoint>>>, Vec<usize>)> {
    let nbpoints = points.len();
    if nbpoints == 0 {
        return None;
    }
    let length_m1 = bf.saturating_sub(1);
    let mut bins: Vec<Vec<Arc<MVPDatapoint>>> = (0..bf).map(|_| Vec::new()).collect();
    let mut counts: Vec<usize> = vec![0; bf];

    for i in 0..nbpoints {
        if i as i32 == sv1_pos || i as i32 == sv2_pos {
            continue;
        }
        let d = dist(vp, &points[i]);
        if is_nan_or_neg(d) {
            return None;
        }

        let mut placed = false;
        for k in 0..length_m1 {
            if d <= pivots[k] {
                bins[k].push(Arc::clone(&points[i]));
                counts[k] += 1;
                placed = true;
                break;
            }
        }

        // Mirror the C semantics exactly: also append to the last bin if the
        // distance is greater than the last pivot.
        if length_m1 > 0 && d > pivots[length_m1 - 1] {
            bins[length_m1].push(Arc::clone(&points[i]));
            counts[length_m1] += 1;
            // (note: in some pathological cases this could double-add when the
            // pivots are equal, matching the C reference behaviour)
            let _ = placed;
        } else if length_m1 == 0 {
            bins[0].push(Arc::clone(&points[i]));
            counts[0] += 1;
        }
    }

    Some((bins, counts))
}

fn find_distance_range_for_vp_arr(
    points: &[Arc<MVPDatapoint>],
    vp: &MVPDatapoint,
    dist: DistanceFunction,
    pathlen: usize,
    lvl: usize,
) -> Result<(), i32> {
    if points.is_empty() {
        return Err(-1);
    }
    for i in 0..points.len() {
        let d = dist(vp, &points[i]);
        if is_nan_or_neg(d) {
            return Err(-2);
        }
        if lvl < pathlen {
            // Need mutable access. Each Arc<MVPDatapoint> here was built fresh
            // (single owner), so try_unwrap-then-rebuild would work but is
            // expensive. Instead, store path via interior mutation: clone the
            // datapoint, set the path, rebuild.
            //
            // Because we can't do that easily with Arc<MVPDatapoint>, we
            // require that each point in `points` has only one strong ref so
            // we can mutate via Arc::get_mut.
            //
            // Caller must guarantee unique ownership at this stage of the
            // build (which is true during _mvptree_add).
            unsafe {
                // SAFETY: We assume the caller holds the only strong reference
                // to each Arc here (true during tree construction). We avoid
                // `Arc::get_mut` through a const cast since other Arcs may have
                // been cloned into bins for routing, but the path slot at
                // `lvl` is independent across recursive calls.
                let p = &points[i] as *const Arc<MVPDatapoint> as *mut Arc<MVPDatapoint>;
                let raw = Arc::as_ptr(&*p) as *mut MVPDatapoint;
                if !raw.is_null() {
                    let path = &mut (*raw).path;
                    if lvl < path.len() {
                        path[lvl] = d;
                    }
                }
            }
        }
    }
    Ok(())
}

fn mvptree_add_recursive(
    existing: Option<Rc<RefCell<Node>>>,
    points: &mut Vec<Arc<MVPDatapoint>>,
    dist: DistanceFunction,
    bf: usize,
    leafcap: usize,
    pathlen: usize,
    error: &mut MVPError,
    lvl: usize,
) -> Option<Rc<RefCell<Node>>> {
    let nbpoints = points.len();
    if nbpoints == 0 {
        return existing;
    }
    let length_m1 = bf.saturating_sub(1);

    if existing.is_none() {
        if nbpoints <= leafcap + 2 {
            // create leaf
            let mut leaf = LeafNode::new(leafcap as u32);

            let (sv1_pos, sv2_pos) = match select_vantage_points_arr(points, dist) {
                Ok(r) => r,
                Err(_) => {
                    *error = MVPError::VpNoSelect;
                    return None;
                }
            };

            let sv1 = if sv1_pos >= 0 { Some(Arc::clone(&points[sv1_pos as usize])) } else { None };
            let sv2 = if sv2_pos >= 0 { Some(Arc::clone(&points[sv2_pos as usize])) } else { None };

            if let Some(ref sv1_dp) = sv1 {
                if find_distance_range_for_vp_arr(points, sv1_dp, dist, pathlen, lvl).is_err() {
                    *error = MVPError::NoSv1Range;
                    return None;
                }
            }
            if let Some(ref sv2_dp) = sv2 {
                if find_distance_range_for_vp_arr(points, sv2_dp, dist, pathlen, lvl + 1).is_err() {
                    *error = MVPError::NoSv2Range;
                    return None;
                }
            }

            // Reset count vectors to actual size
            leaf.d1.clear();
            leaf.d2.clear();
            leaf.points.clear();
            for i in 0..nbpoints {
                if i as i32 == sv1_pos || i as i32 == sv2_pos {
                    continue;
                }
                let d1v = if let Some(ref s) = sv1 { dist(&points[i], s) } else { 0.0 };
                let d2v = if let Some(ref s) = sv2 { dist(&points[i], s) } else { 0.0 };
                leaf.d1.push(d1v);
                leaf.d2.push(d2v);
                leaf.points.push(Arc::clone(&points[i]));
            }
            leaf.nbpoints = leaf.points.len();
            leaf.sv1 = sv1;
            leaf.sv2 = sv2;

            return Some(Rc::new(RefCell::new(Node::Leaf(leaf))));
        } else {
            // create internal node
            let mut internal = InternalNode::new(bf as u32);

            let (sv1_pos, sv2_pos) = match select_vantage_points_arr(points, dist) {
                Ok(r) => r,
                Err(_) => {
                    *error = MVPError::VpNoSelect;
                    return None;
                }
            };

            if sv1_pos < 0 || sv2_pos < 0 {
                *error = MVPError::VpNoSelect;
                return None;
            }

            let sv1 = Arc::clone(&points[sv1_pos as usize]);
            let sv2 = Arc::clone(&points[sv2_pos as usize]);

            if find_distance_range_for_vp_arr(points, &sv1, dist, pathlen, lvl).is_err() {
                *error = MVPError::NoSv1Range;
                return None;
            }

            if find_splits_arr(points, &sv1, dist, &mut internal.m1) < 0 {
                *error = MVPError::NoSplits;
                return None;
            }

            let bins_res = sort_points_arr(points, sv1_pos, sv2_pos, &sv1, dist, bf, &internal.m1);
            let (bins, _binlengths) = match bins_res {
                Some(r) => r,
                None => {
                    *error = MVPError::NoSort;
                    return None;
                }
            };

            for i in 0..bf {
                let bin = &bins[i];
                if !bin.is_empty() {
                    if find_distance_range_for_vp_arr(bin, &sv2, dist, pathlen, lvl + 1).is_err() {
                        *error = MVPError::NoSv2Range;
                        return None;
                    }

                    let m2_slice = &mut internal.m2[i * length_m1..(i + 1) * length_m1];
                    if find_splits_arr(bin, &sv2, dist, m2_slice) < 0 {
                        *error = MVPError::NoSplits;
                        return None;
                    }

                    let m2_pivots: Vec<f32> = m2_slice.to_vec();
                    let bins2_res = sort_points_arr(bin, -1, -1, &sv2, dist, bf, &m2_pivots);
                    let (bins2, _bin2lengths) = match bins2_res {
                        Some(r) => r,
                        None => {
                            *error = MVPError::NoSort;
                            return None;
                        }
                    };

                    for j in 0..bf {
                        let mut sub: Vec<Arc<MVPDatapoint>> = bins2[j].iter().map(Arc::clone).collect();
                        let child = mvptree_add_recursive(
                            None,
                            &mut sub,
                            dist,
                            bf,
                            leafcap,
                            pathlen,
                            error,
                            lvl + 2,
                        );
                        if let Some(c) = child {
                            internal.child_nodes[i * bf + j] = c;
                        }
                        if *error != MVPError::Success {
                            return Some(Rc::new(RefCell::new(Node::Internal(internal))));
                        }
                    }
                }
            }

            internal.sv1 = Some(sv1);
            internal.sv2 = Some(sv2);

            return Some(Rc::new(RefCell::new(Node::Internal(internal))));
        }
    } else {
        // Node already exists
        let node_rc = existing.unwrap();
        let is_leaf = matches!(&*node_rc.borrow(), Node::Leaf(_));
        if is_leaf {
            // Try to add to leaf
            let (current_count, _sv1_present, sv2_present) = {
                let n = node_rc.borrow();
                if let Node::Leaf(l) = &*n {
                    (l.nbpoints, l.sv1.is_some(), l.sv2.is_some())
                } else {
                    unreachable!()
                }
            };

            if current_count + nbpoints <= leafcap {
                // plenty of room
                let sv1_clone = {
                    let n = node_rc.borrow();
                    if let Node::Leaf(l) = &*n {
                        l.sv1.clone()
                    } else {
                        None
                    }
                };
                if let Some(sv1) = sv1_clone {
                    if find_distance_range_for_vp_arr(points, &sv1, dist, pathlen, lvl).is_err() {
                        *error = MVPError::NoSv1Range;
                        return Some(node_rc);
                    }

                    let mut start_pos = 0usize;
                    if !sv2_present {
                        // make first point sv2
                        let mut nb = node_rc.borrow_mut();
                        if let Node::Leaf(l) = &mut *nb {
                            l.sv2 = Some(Arc::clone(&points[0]));
                        }
                        start_pos = 1;
                    }

                    let sv2_clone = {
                        let n = node_rc.borrow();
                        if let Node::Leaf(l) = &*n { l.sv2.clone() } else { None }
                    };
                    if let Some(sv2) = sv2_clone {
                        if find_distance_range_for_vp_arr(points, &sv2, dist, pathlen, lvl + 1).is_err() {
                            *error = MVPError::NoSv2Range;
                            return Some(node_rc);
                        }

                        let mut nb = node_rc.borrow_mut();
                        if let Node::Leaf(l) = &mut *nb {
                            for pos in start_pos..nbpoints {
                                let d1v = if let Some(ref s) = l.sv1 { dist(&points[pos], s) } else { 0.0 };
                                let d2v = dist(&points[pos], &sv2);
                                if l.nbpoints < l.d1.len() {
                                    l.d1[l.nbpoints] = d1v;
                                } else {
                                    l.d1.push(d1v);
                                }
                                if l.nbpoints < l.d2.len() {
                                    l.d2[l.nbpoints] = d2v;
                                } else {
                                    l.d2.push(d2v);
                                }
                                if l.nbpoints < l.points.len() {
                                    l.points[l.nbpoints] = Arc::clone(&points[pos]);
                                } else {
                                    l.points.push(Arc::clone(&points[pos]));
                                }
                                l.nbpoints += 1;
                            }
                        }
                    }
                }
                Some(node_rc)
            } else {
                // not enough room - merge & rebuild from scratch
                let mut tmp_pts: Vec<Arc<MVPDatapoint>> = Vec::new();
                {
                    let n = node_rc.borrow();
                    if let Node::Leaf(l) = &*n {
                        if let Some(ref s) = l.sv1 { tmp_pts.push(Arc::clone(s)); }
                        if let Some(ref s) = l.sv2 { tmp_pts.push(Arc::clone(s)); }
                        for p in l.points.iter().take(l.nbpoints) {
                            tmp_pts.push(Arc::clone(p));
                        }
                    }
                }
                for p in points.iter() {
                    tmp_pts.push(Arc::clone(p));
                }
                drop(node_rc);
                mvptree_add_recursive(None, &mut tmp_pts, dist, bf, leafcap, pathlen, error, lvl)
            }
        } else {
            // Internal node: bin and recurse
            let (sv1_clone, sv2_clone, m1_clone) = {
                let n = node_rc.borrow();
                if let Node::Internal(i) = &*n {
                    (i.sv1.clone(), i.sv2.clone(), i.m1.clone())
                } else {
                    unreachable!()
                }
            };
            let sv1 = match sv1_clone { Some(s) => s, None => { *error = MVPError::ArgErr; return Some(node_rc); }};
            let sv2 = match sv2_clone { Some(s) => s, None => { *error = MVPError::ArgErr; return Some(node_rc); }};

            if find_distance_range_for_vp_arr(points, &sv1, dist, pathlen, lvl).is_err() {
                *error = MVPError::NoSv1Range;
                return Some(node_rc);
            }

            let bins_res = sort_points_arr(points, -1, -1, &sv1, dist, bf, &m1_clone);
            let (bins, _binlengths) = match bins_res {
                Some(r) => r,
                None => {
                    *error = MVPError::NoSort;
                    return Some(node_rc);
                }
            };

            for i in 0..bf {
                let bin = &bins[i];
                if bin.is_empty() {
                    continue;
                }
                if find_distance_range_for_vp_arr(bin, &sv2, dist, pathlen, lvl + 1).is_err() {
                    *error = MVPError::NoSv2Range;
                    return Some(node_rc);
                }

                let m2_pivots: Vec<f32> = {
                    let n = node_rc.borrow();
                    if let Node::Internal(internal) = &*n {
                        internal.m2[i * length_m1..(i + 1) * length_m1].to_vec()
                    } else {
                        unreachable!()
                    }
                };

                let bins2_res = sort_points_arr(bin, -1, -1, &sv2, dist, bf, &m2_pivots);
                let (bins2, _bin2lengths) = match bins2_res {
                    Some(r) => r,
                    None => {
                        *error = MVPError::NoSort;
                        return Some(node_rc);
                    }
                };

                for j in 0..bf {
                    if bins2[j].is_empty() { continue; }
                    let child_idx = i * bf + j;
                    let child_rc = {
                        let n = node_rc.borrow();
                        if let Node::Internal(internal) = &*n {
                            let c = &internal.child_nodes[child_idx];
                            if is_null_node(c) { None } else { Some(Rc::clone(c)) }
                        } else {
                            unreachable!()
                        }
                    };

                    let mut sub: Vec<Arc<MVPDatapoint>> = bins2[j].iter().map(Arc::clone).collect();
                    let new_child = mvptree_add_recursive(
                        child_rc,
                        &mut sub,
                        dist,
                        bf,
                        leafcap,
                        pathlen,
                        error,
                        lvl + 2,
                    );

                    {
                        let mut nb = node_rc.borrow_mut();
                        if let Node::Internal(internal) = &mut *nb {
                            if let Some(c) = new_child {
                                internal.child_nodes[child_idx] = c;
                            }
                        }
                    }

                    if *error != MVPError::Success {
                        return Some(node_rc);
                    }
                }
            }

            Some(node_rc)
        }
    }
}

fn retrieve_recursive(
    node_rc: &Rc<RefCell<Node>>,
    target: &MVPDatapoint,
    target_path: &mut Vec<f32>,
    radius: f32,
    knearest: usize,
    bf: usize,
    pathlen: usize,
    dist: DistanceFunction,
    results: &mut Vec<Arc<MVPDatapoint>>,
    err: &mut MVPError,
    lvl: usize,
) {
    if is_null_node(node_rc) {
        return;
    }

    let length_m1 = bf.saturating_sub(1);
    let n = node_rc.borrow();

    match &*n {
        Node::Leaf(leaf) => {
            let sv1 = match &leaf.sv1 {
                Some(s) => s,
                None => return,
            };
            let d1 = dist(target, sv1);
            if is_nan_or_neg(d1) {
                *err = MVPError::BadDistVal;
                return;
            }
            if lvl < pathlen { target_path[lvl] = d1; }
            if d1 <= radius {
                results.push(Arc::clone(sv1));
                if results.len() >= knearest {
                    *err = MVPError::KNearestCap;
                    return;
                }
            }

            if let Some(sv2) = &leaf.sv2 {
                let d2 = dist(target, sv2);
                if is_nan_or_neg(d2) {
                    *err = MVPError::BadDistVal;
                    return;
                }
                if d2 <= radius {
                    results.push(Arc::clone(sv2));
                    if results.len() >= knearest {
                        *err = MVPError::KNearestCap;
                        return;
                    }
                }
                if lvl + 1 < pathlen { target_path[lvl + 1] = d2; }

                for i in 0..leaf.nbpoints {
                    let d1i = leaf.d1.get(i).copied().unwrap_or(0.0);
                    let d2i = leaf.d2.get(i).copied().unwrap_or(0.0);
                    if d1 - radius <= d1i && d1 + radius >= d1i
                        && d2 - radius <= d2i && d2 + radius >= d2i
                    {
                        let endpath = if lvl + 1 < pathlen { lvl + 1 } else { pathlen };
                        let mut skip = false;
                        let pi = &leaf.points[i];
                        for j in 0..endpath {
                            if j >= pi.path.len() || j >= target_path.len() { break; }
                            if target_path[j] - radius <= pi.path[j]
                                && target_path[j] + radius >= pi.path[j]
                            {
                                continue;
                            } else {
                                skip = true;
                                break;
                            }
                        }
                        if !skip {
                            let d = dist(target, pi);
                            if is_nan_or_neg(d) {
                                *err = MVPError::BadDistVal;
                                return;
                            }
                            if d <= radius {
                                results.push(Arc::clone(pi));
                                if results.len() >= knearest {
                                    *err = MVPError::KNearestCap;
                                    return;
                                }
                            }
                        }
                    }
                }
            }
        }
        Node::Internal(internal) => {
            let sv1 = match &internal.sv1 { Some(s) => s, None => return };
            let sv2 = match &internal.sv2 { Some(s) => s, None => return };
            let d1 = dist(target, sv1);
            if is_nan_or_neg(d1) { *err = MVPError::BadDistVal; return; }
            if d1 <= radius {
                results.push(Arc::clone(sv1));
                if results.len() >= knearest { *err = MVPError::KNearestCap; return; }
            }
            if lvl < pathlen { target_path[lvl] = d1; }
            let d2 = dist(target, sv2);
            if is_nan_or_neg(d2) { *err = MVPError::BadDistVal; return; }
            if d2 <= radius {
                results.push(Arc::clone(sv2));
                if results.len() >= knearest { *err = MVPError::KNearestCap; return; }
            }
            if lvl + 1 < pathlen { target_path[lvl + 1] = d2; }

            for i in 0..length_m1 {
                if d1 - radius <= internal.m1[i] {
                    for j in 0..length_m1 {
                        let m2_idx = i * length_m1 + j;
                        if m2_idx < internal.m2.len() && d2 - radius <= internal.m2[m2_idx] {
                            let child_idx = i * bf + j;
                            if child_idx < internal.child_nodes.len() {
                                retrieve_recursive(
                                    &internal.child_nodes[child_idx],
                                    target,
                                    target_path,
                                    radius,
                                    knearest,
                                    bf,
                                    pathlen,
                                    dist,
                                    results,
                                    err,
                                    lvl + 2,
                                );
                                if *err != MVPError::Success { return; }
                            }
                        }
                    }
                    if length_m1 > 0 {
                        let m2_last = i * length_m1 + length_m1 - 1;
                        if m2_last < internal.m2.len() && d2 + radius >= internal.m2[m2_last] {
                            let child_idx = i * bf + length_m1;
                            if child_idx < internal.child_nodes.len() {
                                retrieve_recursive(
                                    &internal.child_nodes[child_idx],
                                    target,
                                    target_path,
                                    radius,
                                    knearest,
                                    bf,
                                    pathlen,
                                    dist,
                                    results,
                                    err,
                                    lvl + 2,
                                );
                                if *err != MVPError::Success { return; }
                            }
                        }
                    }
                }
            }

            if length_m1 > 0 && d1 + radius >= internal.m1[length_m1 - 1] {
                for j in 0..length_m1 {
                    let m2_idx = length_m1 * length_m1 + j;
                    if m2_idx < internal.m2.len() && d2 - radius <= internal.m2[m2_idx] {
                        let child_idx = bf * length_m1 + j;
                        if child_idx < internal.child_nodes.len() {
                            retrieve_recursive(
                                &internal.child_nodes[child_idx],
                                target,
                                target_path,
                                radius,
                                knearest,
                                bf,
                                pathlen,
                                dist,
                                results,
                                err,
                                lvl + 2,
                            );
                            if *err != MVPError::Success { return; }
                        }
                    }
                }
                let m2_last = length_m1 * length_m1 + length_m1 - 1;
                if m2_last < internal.m2.len() && d2 + radius >= internal.m2[m2_last] {
                    let child_idx = bf * length_m1 + length_m1;
                    if child_idx < internal.child_nodes.len() {
                        retrieve_recursive(
                            &internal.child_nodes[child_idx],
                            target,
                            target_path,
                            radius,
                            knearest,
                            bf,
                            pathlen,
                            dist,
                            results,
                            err,
                            lvl + 2,
                        );
                        if *err != MVPError::Success { return; }
                    }
                }
            }
        }
    }
}

fn write_datapoint_buf(
    dp: Option<&MVPDatapoint>,
    buf: &mut Vec<u8>,
    pathlen: usize,
) -> u64 {
    let start = buf.len() as u64;
    match dp {
        None => {
            // write inactive marker: u8 0 + u32 0
            buf.push(0u8);
            buf.extend_from_slice(&0u32.to_le_bytes());
        }
        Some(dp) => {
            let active: u8 = 1;
            let id_bytes = dp.id.as_bytes();
            let idlen: u8 = id_bytes.len().min(255) as u8;
            let datalength: u32 = dp.datalen as u32;
            let type_size = dp.data_type.byte_size();
            let bytelength: u32 = (1u32) + (idlen as u32) + 4u32
                + (datalength * type_size as u32)
                + (pathlen as u32) * 4u32;

            buf.push(active);
            buf.extend_from_slice(&bytelength.to_le_bytes());
            buf.push(idlen);
            buf.extend_from_slice(&id_bytes[..idlen as usize]);
            buf.extend_from_slice(&datalength.to_le_bytes());
            // data
            let total_data_bytes = (datalength as usize) * type_size;
            if dp.data.len() >= total_data_bytes {
                buf.extend_from_slice(&dp.data[..total_data_bytes]);
            } else {
                buf.extend_from_slice(&dp.data);
                let pad = total_data_bytes - dp.data.len();
                buf.extend(std::iter::repeat(0u8).take(pad));
            }
            // path
            let mut path_iter = dp.path.iter();
            for _ in 0..pathlen {
                let v = path_iter.next().copied().unwrap_or(0.0f32);
                buf.extend_from_slice(&v.to_le_bytes());
            }
        }
    }
    start
}

fn write_node_recursive(
    node_rc: &Rc<RefCell<Node>>,
    buf: &mut Vec<u8>,
    bf: usize,
    pathlen: usize,
    leafcap: usize,
    error: &mut MVPError,
) -> u64 {
    if is_null_node(node_rc) {
        return 0;
    }
    let start_pos = buf.len() as u64;
    let n = node_rc.borrow();

    match &*n {
        Node::Leaf(leaf) => {
            buf.push(NodeType::LeafNode as u8);
            write_datapoint_buf(leaf.sv1.as_deref(), buf, pathlen);
            write_datapoint_buf(leaf.sv2.as_deref(), buf, pathlen);
            let nbpoints = leaf.nbpoints as u32;
            buf.extend_from_slice(&nbpoints.to_le_bytes());

            // Reserve space for leafcap entries of (f32, f32, u64)
            let entry_size = 4 + 4 + 8;
            let table_offset = buf.len();
            let table_bytes = leafcap * entry_size;
            buf.extend(std::iter::repeat(0u8).take(table_bytes));

            for i in 0..leaf.nbpoints {
                let d1 = leaf.d1.get(i).copied().unwrap_or(0.0);
                let d2 = leaf.d2.get(i).copied().unwrap_or(0.0);
                let off = write_datapoint_buf(Some(&leaf.points[i]), buf, pathlen);

                let entry_off = table_offset + i * entry_size;
                buf[entry_off..entry_off + 4].copy_from_slice(&d1.to_le_bytes());
                buf[entry_off + 4..entry_off + 8].copy_from_slice(&d2.to_le_bytes());
                buf[entry_off + 8..entry_off + 16].copy_from_slice(&off.to_le_bytes());
            }
        }
        Node::Internal(internal) => {
            buf.push(NodeType::InternalNode as u8);
            write_datapoint_buf(internal.sv1.as_deref(), buf, pathlen);
            write_datapoint_buf(internal.sv2.as_deref(), buf, pathlen);

            for v in internal.m1.iter() {
                buf.extend_from_slice(&v.to_le_bytes());
            }
            for v in internal.m2.iter() {
                buf.extend_from_slice(&v.to_le_bytes());
            }

            let fanout = bf * bf;
            let entry_size = 1 + 8;
            let table_offset = buf.len();
            buf.extend(std::iter::repeat(0u8).take(fanout * entry_size));

            for i in 0..fanout {
                let off = if i < internal.child_nodes.len() {
                    write_node_recursive(&internal.child_nodes[i], buf, bf, pathlen, leafcap, error)
                } else {
                    0u64
                };
                let eo = table_offset + i * entry_size;
                buf[eo] = 0u8;
                buf[eo + 1..eo + 9].copy_from_slice(&off.to_le_bytes());
            }
        }
    }

    start_pos
}

fn print_node_recursive(
    stream: &mut dyn Write,
    node_rc: &Rc<RefCell<Node>>,
    bf: usize,
    lvl: usize,
    err: &mut MVPError,
) {
    if is_null_node(node_rc) {
        let _ = writeln!(stream, "NULL{}", lvl);
        return;
    }
    let length_m1 = bf.saturating_sub(1);
    let length_m2 = bf;
    let fanout = bf * bf;
    let n = node_rc.borrow();

    match &*n {
        Node::Leaf(leaf) => {
            let _ = writeln!(stream, "LEAF{}  ({} points)", lvl, leaf.nbpoints);
            if let Some(s) = &leaf.sv1 {
                let _ = writeln!(stream, "    sv1: {}", s.id);
            }
            if let Some(s) = &leaf.sv2 {
                let _ = writeln!(stream, "    sv2: {}", s.id);
            }
            for i in 0..leaf.nbpoints {
                let _ = writeln!(stream, "        point[{}]: {}", i, leaf.points[i].id);
            }
        }
        Node::Internal(internal) => {
            let _ = writeln!(stream, "INTERNAL{}", lvl);
            if let Some(s) = &internal.sv1 {
                let _ = writeln!(stream, "  sv1: {}", s.id);
            }
            if let Some(s) = &internal.sv2 {
                let _ = writeln!(stream, "  sv2: {}", s.id);
            }
            for i in 0..length_m1 {
                if i < internal.m1.len() {
                    let _ = write!(stream, "  M1[{}] = {:.4};", i, internal.m1[i]);
                }
            }
            for i in 0..length_m2 {
                if i < internal.m2.len() {
                    let _ = write!(stream, "  M2[{}] = {:.4};", i, internal.m2[i]);
                }
            }
            let _ = writeln!(stream);

            for i in 0..fanout {
                if i < internal.child_nodes.len() {
                    print_node_recursive(stream, &internal.child_nodes[i], bf, lvl + 2, err);
                    if *err != MVPError::Success { break; }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Reading
// ---------------------------------------------------------------------------

struct ReadCtx<'a> {
    buf: &'a [u8],
    pos: usize,
    pathlen: usize,
    datatype: MVPDataType,
}

fn read_u8(ctx: &mut ReadCtx, pos: &mut usize) -> u8 {
    let v = ctx.buf[*pos];
    *pos += 1;
    v
}
fn read_u32(ctx: &mut ReadCtx, pos: &mut usize) -> u32 {
    let mut b = [0u8; 4];
    b.copy_from_slice(&ctx.buf[*pos..*pos + 4]);
    *pos += 4;
    u32::from_le_bytes(b)
}
fn read_u64(ctx: &mut ReadCtx, pos: &mut usize) -> u64 {
    let mut b = [0u8; 8];
    b.copy_from_slice(&ctx.buf[*pos..*pos + 8]);
    *pos += 8;
    u64::from_le_bytes(b)
}
fn read_f32(ctx: &mut ReadCtx, pos: &mut usize) -> f32 {
    let mut b = [0u8; 4];
    b.copy_from_slice(&ctx.buf[*pos..*pos + 4]);
    *pos += 4;
    f32::from_le_bytes(b)
}

fn read_datapoint_buf(ctx: &mut ReadCtx) -> Option<MVPDatapoint> {
    let mut p = ctx.pos;
    let active = read_u8(ctx, &mut p);
    let bytelength = read_u32(ctx, &mut p);
    if active == 0 && bytelength == 0 {
        ctx.pos = p;
        return None;
    }
    let idlen = read_u8(ctx, &mut p);
    let id_bytes = &ctx.buf[p..p + idlen as usize];
    let id = String::from_utf8_lossy(id_bytes).into_owned();
    p += idlen as usize;
    let datalength = read_u32(ctx, &mut p) as usize;
    let type_size = ctx.datatype.byte_size();
    let total = datalength * type_size;
    let data = ctx.buf[p..p + total].to_vec();
    p += total;
    let mut path = Vec::with_capacity(ctx.pathlen);
    for _ in 0..ctx.pathlen {
        path.push(read_f32(ctx, &mut p));
    }
    ctx.pos = p;
    Some(MVPDatapoint {
        id,
        data,
        path,
        datalen: datalength,
        data_type: ctx.datatype,
    })
}

fn read_node_recursive(
    ctx: &mut ReadCtx,
    bf: usize,
    leafcap: usize,
    error: &mut MVPError,
) -> Option<Rc<RefCell<Node>>> {
    if ctx.pos >= ctx.buf.len() {
        return None;
    }
    let mut p = ctx.pos;
    let node_type = read_u8(ctx, &mut p);
    ctx.pos = p;

    if node_type == NodeType::LeafNode as u8 {
        let mut leaf = LeafNode::new(leafcap as u32);
        let sv1 = read_datapoint_buf(ctx);
        let sv2 = read_datapoint_buf(ctx);
        leaf.sv1 = sv1.map(Arc::new);
        leaf.sv2 = sv2.map(Arc::new);

        let mut p = ctx.pos;
        let nbpoints = read_u32(ctx, &mut p) as usize;
        ctx.pos = p;
        leaf.nbpoints = nbpoints;
        leaf.d1.clear();
        leaf.d2.clear();
        leaf.points.clear();
        for _ in 0..nbpoints {
            leaf.d1.push(0.0);
            leaf.d2.push(0.0);
        }

        let mut saved_pos = ctx.pos;
        for i in 0..nbpoints {
            let mut p = saved_pos;
            let d1 = read_f32(ctx, &mut p);
            let d2 = read_f32(ctx, &mut p);
            let offset = read_u64(ctx, &mut p) as usize;
            saved_pos = p;
            leaf.d1[i] = d1;
            leaf.d2[i] = d2;
            ctx.pos = offset;
            let dp = read_datapoint_buf(ctx);
            if let Some(dp) = dp {
                leaf.points.push(Arc::new(dp));
            }
        }
        Some(Rc::new(RefCell::new(Node::Leaf(leaf))))
    } else if node_type == NodeType::InternalNode as u8 {
        let mut internal = InternalNode::new(bf as u32);
        let sv1 = read_datapoint_buf(ctx);
        let sv2 = read_datapoint_buf(ctx);
        internal.sv1 = sv1.map(Arc::new);
        internal.sv2 = sv2.map(Arc::new);

        let length_m1 = bf.saturating_sub(1);
        let length_m2 = (bf.saturating_sub(1)) * bf;
        let fanout = bf * bf;
        let mut p = ctx.pos;
        for i in 0..length_m1 {
            internal.m1[i] = read_f32(ctx, &mut p);
        }
        for i in 0..length_m2 {
            internal.m2[i] = read_f32(ctx, &mut p);
        }
        ctx.pos = p;

        let mut saved_pos = ctx.pos;
        for i in 0..fanout {
            let mut p = saved_pos;
            let _fileno = read_u8(ctx, &mut p);
            let offset = read_u64(ctx, &mut p) as usize;
            saved_pos = p;
            if offset != 0 {
                ctx.pos = offset;
                let child = read_node_recursive(ctx, bf, leafcap, error);
                if let Some(c) = child {
                    internal.child_nodes[i] = c;
                }
                if *error != MVPError::Success { break; }
            }
        }
        Some(Rc::new(RefCell::new(Node::Internal(internal))))
    } else {
        *error = MVPError::Unrecognized;
        None
    }
}

pub fn mvptree_read(filename: &str, distance_function: DistanceFunction) -> Result<MVPTree, MVPError> {
    let mut f = match File::open(filename) {
        Ok(f) => f,
        Err(_) => return Err(MVPError::FileNotFound),
    };
    let mut buf = Vec::new();
    if f.read_to_end(&mut buf).is_err() {
        return Err(MVPError::FileOpen);
    }
    if buf.len() < HEADER_SIZE {
        return Err(MVPError::FileOpen);
    }

    let tag_bytes = TAG.as_bytes();
    let mut pos = tag_bytes.len() + 1; // includes null
    pos += 4; // version
    let bf = buf[pos] as usize; pos += 1;
    let pl = buf[pos] as usize; pos += 1;
    let lc = buf[pos] as usize; pos += 1;
    let ht = buf[pos]; pos += 1;
    let _ = pos;

    let datatype = MVPDataType::from_u8(ht).unwrap_or(MVPDataType::ByteArray);

    let mut tree = MVPTree::new(bf, pl, lc, datatype, distance_function);

    let mut ctx = ReadCtx {
        buf: &buf,
        pos: HEADER_SIZE,
        pathlen: pl,
        datatype,
    };

    let mut error = MVPError::Success;
    let node = read_node_recursive(&mut ctx, bf, lc, &mut error);
    if error != MVPError::Success {
        return Err(error);
    }
    tree.node = node;
    Ok(tree)
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

    /// Single-datapoint convenience: sets the path slot at level `lvl` to the
    /// distance between this datapoint and `vp`. Returns 0 on success, -2 on
    /// bad distance.
    pub fn select_vantage_points(
        &mut self,
        nb: u32,
        sv1_pos: i32,
        sv2_pos: i32,
        dist: DistanceFunction,
    ) -> i32 {
        let _ = (nb, sv1_pos, sv2_pos, dist);
        // No-op for a single datapoint. The actual vantage-point selection
        // happens inside `MVPTree::add` over arrays of points.
        0
    }

    pub fn find_splits(
        &mut self,
        nb: u32,
        vp: &MVPDatapoint,
        tree: &MVPTree,
        length_m: u32,
    ) -> f32 {
        let _ = (nb, length_m);
        let d = (tree.distance_function)(self, vp);
        d
    }

    pub fn sort_points(
        &mut self,
        _nb: u32,
        _sv1_pos: i32,
        _sv2_pos: i32,
        _vp: &MVPDatapoint,
        tree: &MVPTree,
        _counts: &mut Vec<Vec<i32>>,
        _pivots: Vec<f32>,
    ) -> Vec<Vec<Vec<Arc<MVPDatapoint>>>> {
        // No-op for a single datapoint -- return an empty 3D vec sized by bf.
        let bf = tree.branch_factor;
        (0..bf).map(|_| (0..bf).map(|_| Vec::new()).collect()).collect()
    }

    pub fn find_distance_range_for_vp(
        &mut self,
        _nb: u32,
        vp: &MVPDatapoint,
        tree: &MVPTree,
        level: i32,
    ) -> i32 {
        let d = (tree.distance_function)(self, vp);
        if d.is_nan() || d < 0.0 {
            return -2;
        }
        let lvl = level as usize;
        if lvl < tree.path_length {
            if self.path.len() <= lvl {
                self.path.resize(tree.path_length, 0.0);
            }
            self.path[lvl] = d;
        }
        0
    }

    pub fn write(&self, tree: &MVPTree) -> i64 {
        // Serialise a single datapoint to a fresh buffer and return the size.
        let mut buf = Vec::new();
        let _ = write_datapoint_buf(Some(self), &mut buf, tree.path_length);
        buf.len() as i64
    }
}

pub fn error_to_string(error: MVPError) -> &'static str {
    let idx = error.index();
    if idx < ERROR_MSGS.len() {
        ERROR_MSGS[idx]
    } else {
        "unknown error"
    }
}
