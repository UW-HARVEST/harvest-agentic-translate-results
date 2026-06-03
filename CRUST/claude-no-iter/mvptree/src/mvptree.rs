use std::fs::{File, OpenOptions};
use std::io::{Write, Read};
use std::sync::Arc;
use std::rc::Rc;
use std::cell::RefCell;
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
    fn from_u8(v: u8) -> Option<Self> {
        match v {
            1 => Some(MVPDataType::ByteArray),
            2 => Some(MVPDataType::UInt16Array),
            4 => Some(MVPDataType::UInt32Array),
            8 => Some(MVPDataType::UInt64Array),
            _ => None,
        }
    }
    fn byte_size(&self) -> usize {
        *self as usize
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
        let length_m1 = if bf >= 1 { bf - 1 } else { 0 };
        let length_m2 = bf * length_m1;
        let fanout = bf * bf;
        let mut child_nodes = Vec::with_capacity(fanout);
        for _ in 0..fanout {
            child_nodes.push(Rc::new(RefCell::new(empty_leaf())));
        }
        InternalNode {
            node_type: NodeType::InternalNode,
            sv1: None,
            sv2: None,
            m1: vec![0.0; length_m1],
            m2: vec![0.0; length_m2],
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
    pub fn new(bf: u32) -> Self {
        let cap = bf as usize;
        LeafNode {
            node_type: NodeType::LeafNode,
            sv1: None,
            sv2: None,
            points: Vec::with_capacity(cap),
            d1: vec![0.0; cap],
            d2: vec![0.0; cap],
            nbpoints: 0,
        }
    }
}
pub enum Node {
    Leaf(LeafNode),
    Internal(InternalNode),
}

fn empty_leaf() -> Node {
    Node::Leaf(LeafNode {
        node_type: NodeType::LeafNode,
        sv1: None,
        sv2: None,
        points: Vec::new(),
        d1: Vec::new(),
        d2: Vec::new(),
        nbpoints: 0,
    })
}

fn is_null_node(n: &Node) -> bool {
    match n {
        Node::Leaf(l) => l.sv1.is_none() && l.nbpoints == 0,
        Node::Internal(_) => false,
    }
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

// Helper: select vantage points - returns (sv1_pos, sv2_pos) or error
fn select_vps(points: &[MVPDatapoint], dist: DistanceFunction) -> Result<(i32, i32), i32> {
    let nb = points.len();
    if nb == 0 {
        return Err(-1);
    }
    let mut sv1_pos: i32 = if nb >= 1 { 0 } else { -1 };
    let mut sv2_pos: i32 = -1;
    let mut max_d = 0.0f32;
    for i in 0..nb {
        for j in (i + 1)..nb {
            let d = dist(&points[i], &points[j]);
            if d.is_nan() || d < 0.0 {
                return Err(-2);
            }
            if d > max_d {
                max_d = d;
                sv1_pos = i as i32;
                sv2_pos = j as i32;
            }
        }
    }
    Ok((sv1_pos, sv2_pos))
}

// Compute distance from each point to vp and update path[lvl] if lvl < path_length.
fn find_dist_range(points: &mut [MVPDatapoint], vp: &MVPDatapoint, dist: DistanceFunction, lvl: usize, path_length: usize) -> i32 {
    for p in points.iter_mut() {
        let d = dist(vp, p);
        if d.is_nan() || d < 0.0 {
            return -2;
        }
        if lvl < path_length {
            if p.path.len() < path_length {
                p.path.resize(path_length, 0.0);
            }
            p.path[lvl] = d;
        }
    }
    0
}

// Compute split points (M values) by sorting distances and picking percentiles.
fn find_splits(points: &[MVPDatapoint], vp: &MVPDatapoint, dist: DistanceFunction, length_m: usize) -> Result<Vec<f32>, i32> {
    let nb = points.len();
    if nb == 0 || length_m == 0 {
        return Err(-1);
    }
    let mut dists: Vec<f32> = Vec::with_capacity(nb);
    for p in points.iter() {
        let d = dist(p, vp);
        if d.is_nan() || d < 0.0 {
            return Err(-2);
        }
        dists.push(d);
    }
    // Sort ascending
    dists.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mut m = vec![0.0f32; length_m];
    for i in 0..length_m {
        let mut index = (i + 1) * nb / (length_m + 1);
        if index >= nb {
            index = nb - 1;
        }
        m[i] = dists[index];
    }
    Ok(m)
}

// Sort points (excluding sv1_pos and sv2_pos if >=0) into bf bins by distance to vp using pivots.
fn sort_points_into_bins(
    points: Vec<MVPDatapoint>,
    sv1_pos: i32,
    sv2_pos: i32,
    vp: &MVPDatapoint,
    dist: DistanceFunction,
    bf: usize,
    pivots: &[f32],
) -> Result<Vec<Vec<MVPDatapoint>>, i32> {
    let length_m1 = bf - 1;
    let mut bins: Vec<Vec<MVPDatapoint>> = (0..bf).map(|_| Vec::new()).collect();
    for (i, p) in points.into_iter().enumerate() {
        if i as i32 == sv1_pos || i as i32 == sv2_pos {
            continue;
        }
        let d = dist(vp, &p);
        if d.is_nan() || d < 0.0 {
            return Err(-2);
        }
        let mut placed_at: Option<usize> = None;
        for k in 0..length_m1 {
            if d <= pivots[k] {
                placed_at = Some(k);
                break;
            }
        }
        match placed_at {
            Some(k) => bins[k].push(p),
            None => bins[length_m1].push(p),
        }
    }
    Ok(bins)
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
        // Type-check first point
        let first_type = points[0].data_type;
        if (self.datatype as i32) != (first_type as i32) {
            // C code: if datatype == 0 then set; here we always have a datatype.
            // The Rust constructor sets datatype, so just enforce match
            return MVPError::TypeMismatch;
        }

        // Initialize paths
        let mut owned: Vec<MVPDatapoint> = points
            .into_iter()
            .map(|mut p| {
                p.path = vec![0.0; self.path_length];
                p
            })
            .collect();

        let mut err = MVPError::Success;
        let existing = self.node.take();
        let new_node = self.add_recursive(existing, owned, &mut err, 0);
        self.node = new_node;
        err
    }

    fn add_recursive(
        &self,
        existing: Option<Rc<RefCell<Node>>>,
        mut points: Vec<MVPDatapoint>,
        err: &mut MVPError,
        lvl: usize,
    ) -> Option<Rc<RefCell<Node>>> {
        if points.is_empty() {
            return existing;
        }
        match existing {
            None => Some(self.create_new_node(points, err, lvl)),
            Some(rc) => {
                let is_null = is_null_node(&rc.borrow());
                if is_null {
                    return Some(self.create_new_node(points, err, lvl));
                }
                let is_leaf = matches!(&*rc.borrow(), Node::Leaf(_));
                if is_leaf {
                    self.add_to_leaf(rc, points, err, lvl)
                } else {
                    self.add_to_internal(rc, points, err, lvl)
                }
            }
        }
    }

    fn create_new_node(&self, points: Vec<MVPDatapoint>, err: &mut MVPError, lvl: usize) -> Rc<RefCell<Node>> {
        if points.len() <= self.leaf_capacity + 2 {
            self.create_leaf_node(points, err, lvl)
        } else {
            self.create_internal_node(points, err, lvl)
        }
    }

    fn create_leaf_node(&self, mut points: Vec<MVPDatapoint>, err: &mut MVPError, lvl: usize) -> Rc<RefCell<Node>> {
        let dist = self.distance_function;
        let (sv1_pos, sv2_pos) = match select_vps(&points, dist) {
            Ok(v) => v,
            Err(_) => {
                // can happen for empty points - just return empty
                if points.is_empty() {
                    return Rc::new(RefCell::new(empty_leaf()));
                }
                (0i32, -1i32)
            }
        };
        if sv1_pos < 0 {
            // No points
            return Rc::new(RefCell::new(empty_leaf()));
        }

        // Clone sv1 and sv2 so we can mutate points[].path
        let sv1_clone = points[sv1_pos as usize].clone();
        let sv2_clone_opt: Option<MVPDatapoint> = if sv2_pos >= 0 {
            Some(points[sv2_pos as usize].clone())
        } else {
            None
        };

        if find_dist_range(&mut points, &sv1_clone, dist, lvl, self.path_length) < 0 {
            *err = MVPError::NoSv1Range;
            return Rc::new(RefCell::new(empty_leaf()));
        }
        if let Some(ref sv2c) = sv2_clone_opt {
            if find_dist_range(&mut points, sv2c, dist, lvl + 1, self.path_length) < 0 {
                *err = MVPError::NoSv2Range;
                return Rc::new(RefCell::new(empty_leaf()));
            }
        }

        // Allow leaf to hold all the points (resize buffers as needed).
        let cap = std::cmp::max(self.leaf_capacity, points.len());
        let mut leaf = LeafNode {
            node_type: NodeType::LeafNode,
            sv1: None,
            sv2: None,
            points: Vec::with_capacity(cap),
            d1: vec![0.0; cap],
            d2: vec![0.0; cap],
            nbpoints: 0,
        };

        let mut opts: Vec<Option<MVPDatapoint>> = points.into_iter().map(Some).collect();
        let sv1_owned = opts[sv1_pos as usize].take().unwrap();
        let sv2_owned = if sv2_pos >= 0 { opts[sv2_pos as usize].take() } else { None };

        let sv1_arc = Arc::new(sv1_owned);
        let sv2_arc_opt: Option<Arc<MVPDatapoint>> = sv2_owned.map(Arc::new);

        leaf.sv1 = Some(sv1_arc.clone());
        leaf.sv2 = sv2_arc_opt.clone();

        let sv1_ref = sv1_arc.as_ref();
        let mut count = 0;
        for opt in opts.into_iter() {
            if let Some(p) = opt {
                let d1 = dist(&p, sv1_ref);
                let d2 = match &sv2_arc_opt {
                    Some(s) => dist(&p, s.as_ref()),
                    None => 0.0,
                };
                if count < leaf.d1.len() {
                    leaf.d1[count] = d1;
                    leaf.d2[count] = d2;
                }
                leaf.points.push(Arc::new(p));
                count += 1;
            }
        }
        leaf.nbpoints = count;

        Rc::new(RefCell::new(Node::Leaf(leaf)))
    }

    fn create_internal_node(&self, mut points: Vec<MVPDatapoint>, err: &mut MVPError, lvl: usize) -> Rc<RefCell<Node>> {
        let dist = self.distance_function;
        let bf = self.branch_factor;
        let length_m1 = bf - 1;

        let (sv1_pos, sv2_pos) = match select_vps(&points, dist) {
            Ok(v) => v,
            Err(_) => {
                *err = MVPError::VpNoSelect;
                return Rc::new(RefCell::new(empty_leaf()));
            }
        };
        // Fallback: if not enough distinct points to split, place them all in a single leaf.
        if sv1_pos < 0 || sv2_pos < 0 {
            return self.create_leaf_node(points, err, lvl);
        }

        let sv1_clone = points[sv1_pos as usize].clone();
        let sv2_clone = points[sv2_pos as usize].clone();

        if find_dist_range(&mut points, &sv1_clone, dist, lvl, self.path_length) < 0 {
            *err = MVPError::NoSv1Range;
            return Rc::new(RefCell::new(empty_leaf()));
        }

        let m1 = match find_splits(&points, &sv1_clone, dist, length_m1) {
            Ok(m) => m,
            Err(_) => {
                *err = MVPError::NoSplits;
                return Rc::new(RefCell::new(empty_leaf()));
            }
        };

        // Sort points into bf bins (excluding sv1, sv2)
        let bins = match sort_points_into_bins(points, sv1_pos, sv2_pos, &sv1_clone, dist, bf, &m1) {
            Ok(b) => b,
            Err(_) => {
                *err = MVPError::NoSort;
                return Rc::new(RefCell::new(empty_leaf()));
            }
        };

        // Build internal node
        let length_m2 = bf * length_m1;
        let mut m2 = vec![0.0f32; length_m2];
        let mut child_nodes: Vec<Rc<RefCell<Node>>> = Vec::with_capacity(bf * bf);
        for _ in 0..bf * bf {
            child_nodes.push(Rc::new(RefCell::new(empty_leaf())));
        }

        // Process each top-level bin
        for (i, mut bin_i) in bins.into_iter().enumerate() {
            if bin_i.is_empty() {
                continue;
            }
            if find_dist_range(&mut bin_i, &sv2_clone, dist, lvl + 1, self.path_length) < 0 {
                *err = MVPError::NoSv2Range;
                return Rc::new(RefCell::new(empty_leaf()));
            }
            let m2_i = match find_splits(&bin_i, &sv2_clone, dist, length_m1) {
                Ok(v) => v,
                Err(_) => {
                    *err = MVPError::NoSplits;
                    return Rc::new(RefCell::new(empty_leaf()));
                }
            };
            for k in 0..length_m1 {
                m2[i * length_m1 + k] = m2_i[k];
            }
            let bins2 = match sort_points_into_bins(bin_i, -1, -1, &sv2_clone, dist, bf, &m2_i) {
                Ok(b) => b,
                Err(_) => {
                    *err = MVPError::NoSort;
                    return Rc::new(RefCell::new(empty_leaf()));
                }
            };
            for (j, bin_j) in bins2.into_iter().enumerate() {
                if bin_j.is_empty() {
                    continue;
                }
                let child = self.add_recursive(None, bin_j, err, lvl + 2);
                if let Some(c) = child {
                    child_nodes[i * bf + j] = c;
                }
                if *err != MVPError::Success {
                    return Rc::new(RefCell::new(empty_leaf()));
                }
            }
        }

        let internal = InternalNode {
            node_type: NodeType::InternalNode,
            sv1: Some(Arc::new(sv1_clone)),
            sv2: Some(Arc::new(sv2_clone)),
            m1,
            m2,
            child_nodes,
        };

        Rc::new(RefCell::new(Node::Internal(internal)))
    }

    fn add_to_leaf(
        &self,
        rc: Rc<RefCell<Node>>,
        mut points: Vec<MVPDatapoint>,
        err: &mut MVPError,
        lvl: usize,
    ) -> Option<Rc<RefCell<Node>>> {
        // Decide if can fit
        let (can_fit, current_nb, has_sv2, sv1_arc, sv2_arc) = {
            let n = rc.borrow();
            if let Node::Leaf(l) = &*n {
                let total = l.nbpoints + points.len();
                (total <= self.leaf_capacity, l.nbpoints, l.sv2.is_some(), l.sv1.clone(), l.sv2.clone())
            } else {
                unreachable!()
            }
        };

        if can_fit {
            let dist = self.distance_function;
            // find dist range for sv1 over new points
            if let Some(sv1) = &sv1_arc {
                if find_dist_range(&mut points, sv1.as_ref(), dist, lvl, self.path_length) < 0 {
                    *err = MVPError::NoSv1Range;
                    return Some(rc);
                }
            }

            let mut start_pos = 0usize;
            let sv2_use: Option<Arc<MVPDatapoint>> = if !has_sv2 {
                // promote points[0] to sv2
                if points.is_empty() {
                    None
                } else {
                    let p0 = points.remove(0);
                    let arc = Arc::new(p0);
                    // assign to leaf later
                    Some(arc)
                }
            } else {
                sv2_arc.clone()
            };

            if let Some(sv2) = &sv2_use {
                if find_dist_range(&mut points, sv2.as_ref(), dist, lvl + 1, self.path_length) < 0 {
                    *err = MVPError::NoSv2Range;
                    return Some(rc);
                }
            }

            // Now mutate the leaf
            {
                let mut n = rc.borrow_mut();
                if let Node::Leaf(l) = &mut *n {
                    if !has_sv2 {
                        l.sv2 = sv2_use.clone();
                    }
                    let mut count = l.nbpoints;
                    let sv1_ref = sv1_arc.as_ref().unwrap().as_ref();
                    let sv2_ref_opt = sv2_use.as_ref();
                    for p in points.into_iter() {
                        let d1 = dist(&p, sv1_ref);
                        let d2 = match sv2_ref_opt {
                            Some(s) => dist(&p, s.as_ref()),
                            None => 0.0,
                        };
                        if count < l.d1.len() {
                            l.d1[count] = d1;
                            l.d2[count] = d2;
                        }
                        l.points.push(Arc::new(p));
                        count += 1;
                    }
                    l.nbpoints = count;
                }
            }
            Some(rc)
        } else {
            // not enough room - rebuild
            let mut all_points: Vec<MVPDatapoint> = Vec::new();
            {
                let n = rc.borrow();
                if let Node::Leaf(l) = &*n {
                    if let Some(sv1) = &l.sv1 {
                        all_points.push(sv1.as_ref().clone());
                    }
                    if let Some(sv2) = &l.sv2 {
                        all_points.push(sv2.as_ref().clone());
                    }
                    for p in l.points.iter() {
                        all_points.push(p.as_ref().clone());
                    }
                }
            }
            for p in points.into_iter() {
                all_points.push(p);
            }
            // Create new node from scratch
            Some(self.add_recursive(None, all_points, err, lvl))
                .and_then(|x| x)
        }
    }

    fn add_to_internal(
        &self,
        rc: Rc<RefCell<Node>>,
        mut points: Vec<MVPDatapoint>,
        err: &mut MVPError,
        lvl: usize,
    ) -> Option<Rc<RefCell<Node>>> {
        let dist = self.distance_function;
        let bf = self.branch_factor;
        let length_m1 = bf - 1;

        let (sv1_arc, sv2_arc, m1_clone, m2_clone) = {
            let n = rc.borrow();
            if let Node::Internal(int_n) = &*n {
                (int_n.sv1.clone(), int_n.sv2.clone(), int_n.m1.clone(), int_n.m2.clone())
            } else {
                unreachable!()
            }
        };

        let sv1 = match &sv1_arc {
            Some(s) => s.as_ref().clone(),
            None => {
                *err = MVPError::ArgErr;
                return Some(rc);
            }
        };
        let sv2 = match &sv2_arc {
            Some(s) => s.as_ref().clone(),
            None => {
                *err = MVPError::ArgErr;
                return Some(rc);
            }
        };

        if find_dist_range(&mut points, &sv1, dist, lvl, self.path_length) < 0 {
            *err = MVPError::NoSv1Range;
            return Some(rc);
        }

        let bins = match sort_points_into_bins(points, -1, -1, &sv1, dist, bf, &m1_clone) {
            Ok(b) => b,
            Err(_) => {
                *err = MVPError::NoSort;
                return Some(rc);
            }
        };

        for (i, mut bin_i) in bins.into_iter().enumerate() {
            if bin_i.is_empty() {
                continue;
            }
            if find_dist_range(&mut bin_i, &sv2, dist, lvl + 1, self.path_length) < 0 {
                *err = MVPError::NoSv2Range;
                return Some(rc);
            }
            let m2_slice = &m2_clone[i * length_m1..(i + 1) * length_m1];
            let bins2 = match sort_points_into_bins(bin_i, -1, -1, &sv2, dist, bf, m2_slice) {
                Ok(b) => b,
                Err(_) => {
                    *err = MVPError::NoSort;
                    return Some(rc);
                }
            };

            for (j, bin_j) in bins2.into_iter().enumerate() {
                if bin_j.is_empty() {
                    continue;
                }
                let child_idx = i * bf + j;
                let child_existing = {
                    let n = rc.borrow();
                    if let Node::Internal(int_n) = &*n {
                        Some(int_n.child_nodes[child_idx].clone())
                    } else {
                        None
                    }
                };
                let new_child = self.add_recursive(child_existing, bin_j, err, lvl + 2);
                if let Some(nc) = new_child {
                    let mut n = rc.borrow_mut();
                    if let Node::Internal(int_n) = &mut *n {
                        int_n.child_nodes[child_idx] = nc;
                    }
                }
                if *err != MVPError::Success {
                    return Some(rc);
                }
            }
        }

        Some(rc)
    }

    pub fn retrieve(&self, target: &MVPDatapoint, knearest: usize, radius: f32) -> Result<Vec<MVPDatapoint>, MVPError> {
        if knearest == 0 || radius < 0.0 {
            return Err(MVPError::ArgErr);
        }
        let node_rc = match &self.node {
            Some(n) => n.clone(),
            None => return Err(MVPError::EmptyTree),
        };
        let mut results: Vec<MVPDatapoint> = Vec::new();
        let mut path: Vec<f32> = vec![0.0; self.path_length];
        let err = self.retrieve_recursive(&node_rc, target, &mut path, radius, knearest, &mut results, 0);
        match err {
            MVPError::Success | MVPError::KNearestCap => Ok(results),
            e => Err(e),
        }
    }

    fn retrieve_recursive(
        &self,
        node_rc: &Rc<RefCell<Node>>,
        target: &MVPDatapoint,
        path: &mut Vec<f32>,
        radius: f32,
        k: usize,
        results: &mut Vec<MVPDatapoint>,
        lvl: usize,
    ) -> MVPError {
        let node = node_rc.borrow();
        if is_null_node(&node) {
            return MVPError::Success;
        }
        let dist = self.distance_function;
        match &*node {
            Node::Leaf(l) => {
                let sv1 = match &l.sv1 {
                    Some(s) => s.clone(),
                    None => return MVPError::Success,
                };
                let d1 = dist(target, sv1.as_ref());
                if d1.is_nan() || d1 < 0.0 {
                    return MVPError::BadDistVal;
                }
                if lvl < self.path_length {
                    path[lvl] = d1;
                }
                if d1 <= radius {
                    results.push(sv1.as_ref().clone());
                    if results.len() >= k {
                        return MVPError::KNearestCap;
                    }
                }

                if let Some(sv2) = &l.sv2 {
                    let d2 = dist(target, sv2.as_ref());
                    if d2.is_nan() || d2 < 0.0 {
                        return MVPError::BadDistVal;
                    }
                    if d2 <= radius {
                        results.push(sv2.as_ref().clone());
                        if results.len() >= k {
                            return MVPError::KNearestCap;
                        }
                    }
                    if lvl + 1 < self.path_length {
                        path[lvl + 1] = d2;
                    }

                    for i in 0..l.nbpoints {
                        if d1 - radius <= l.d1[i] && d1 + radius >= l.d1[i] {
                            if d2 - radius <= l.d2[i] && d2 + radius >= l.d2[i] {
                                let endpath = if lvl + 1 < self.path_length { lvl + 1 } else { self.path_length };
                                let mut skip = false;
                                let p = &l.points[i];
                                for j in 0..endpath {
                                    let pp = if j < p.path.len() { p.path[j] } else { 0.0 };
                                    if path[j] - radius <= pp && path[j] + radius >= pp {
                                        continue;
                                    } else {
                                        skip = true;
                                        break;
                                    }
                                }
                                if !skip {
                                    let d = dist(target, p.as_ref());
                                    if d.is_nan() || d < 0.0 {
                                        return MVPError::BadDistVal;
                                    }
                                    if d <= radius {
                                        results.push(p.as_ref().clone());
                                        if results.len() >= k {
                                            return MVPError::KNearestCap;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                MVPError::Success
            }
            Node::Internal(int_n) => {
                let bf = self.branch_factor;
                let length_m1 = bf - 1;
                let sv1 = match &int_n.sv1 {
                    Some(s) => s.clone(),
                    None => return MVPError::Success,
                };
                let sv2 = match &int_n.sv2 {
                    Some(s) => s.clone(),
                    None => return MVPError::Success,
                };
                let d1 = dist(target, sv1.as_ref());
                if d1.is_nan() || d1 < 0.0 {
                    return MVPError::BadDistVal;
                }
                if d1 <= radius {
                    results.push(sv1.as_ref().clone());
                    if results.len() >= k {
                        return MVPError::KNearestCap;
                    }
                }
                if lvl < self.path_length {
                    path[lvl] = d1;
                }
                let d2 = dist(target, sv2.as_ref());
                if d2.is_nan() || d2 < 0.0 {
                    return MVPError::BadDistVal;
                }
                if d2 <= radius {
                    results.push(sv2.as_ref().clone());
                    if results.len() >= k {
                        return MVPError::KNearestCap;
                    }
                }
                if lvl + 1 < self.path_length {
                    path[lvl + 1] = d2;
                }

                let children = int_n.child_nodes.clone();
                let m1 = int_n.m1.clone();
                let m2 = int_n.m2.clone();
                drop(node);

                for i in 0..length_m1 {
                    if d1 - radius <= m1[i] {
                        for j in 0..length_m1 {
                            if d2 - radius <= m2[i * length_m1 + j] {
                                let err = self.retrieve_recursive(
                                    &children[i * bf + j],
                                    target,
                                    path,
                                    radius,
                                    k,
                                    results,
                                    lvl + 2,
                                );
                                if err != MVPError::Success {
                                    return err;
                                }
                            }
                        }
                        // last 2nd-level bin
                        if length_m1 >= 1 && d2 + radius >= m2[i * length_m1 + length_m1 - 1] {
                            let err = self.retrieve_recursive(
                                &children[i * bf + length_m1],
                                target,
                                path,
                                radius,
                                k,
                                results,
                                lvl + 2,
                            );
                            if err != MVPError::Success {
                                return err;
                            }
                        }
                    }
                }

                if length_m1 >= 1 && d1 + radius >= m1[length_m1 - 1] {
                    for j in 0..length_m1 {
                        if d2 - radius <= m2[length_m1 * length_m1 + j] {
                            let err = self.retrieve_recursive(
                                &children[bf * length_m1 + j],
                                target,
                                path,
                                radius,
                                k,
                                results,
                                lvl + 2,
                            );
                            if err != MVPError::Success {
                                return err;
                            }
                        }
                    }
                    if length_m1 >= 1 && d2 + radius >= m2[length_m1 * length_m1 + length_m1 - 1] {
                        let err = self.retrieve_recursive(
                            &children[bf * length_m1 + length_m1],
                            target,
                            path,
                            radius,
                            k,
                            results,
                            lvl + 2,
                        );
                        if err != MVPError::Success {
                            return err;
                        }
                    }
                }
                MVPError::Success
            }
        }
    }

    pub fn write(&self, filename: &str, mode: i32) -> MVPError {
        let node_rc = match &self.node {
            Some(n) => n.clone(),
            None => return MVPError::ArgErr,
        };

        let mut buf: Vec<u8> = vec![0u8; HEADER_SIZE];
        // Header: tag null-terminated, version, bf, pl, lc, ht
        let tag_bytes = TAG.as_bytes();
        buf[..tag_bytes.len()].copy_from_slice(tag_bytes);
        let mut pos = tag_bytes.len() + 1; // null terminator already 0
        buf[pos..pos + 4].copy_from_slice(&VERSION.to_ne_bytes());
        pos += 4;
        let bf: u8 = self.branch_factor as u8;
        let pl: u8 = self.path_length as u8;
        let lc: u8 = self.leaf_capacity as u8;
        // Determine ht from sv1's type
        let ht: u8 = {
            let n = node_rc.borrow();
            match &*n {
                Node::Internal(int_n) => match &int_n.sv1 {
                    Some(s) => s.data_type as u8,
                    None => self.datatype as u8,
                },
                Node::Leaf(l) => match &l.sv1 {
                    Some(s) => s.data_type as u8,
                    None => self.datatype as u8,
                },
            }
        };
        buf[pos] = bf;
        pos += 1;
        buf[pos] = pl;
        pos += 1;
        buf[pos] = lc;
        pos += 1;
        buf[pos] = ht;
        // Now write tree starting at HEADER_SIZE
        buf.resize(HEADER_SIZE, 0);

        let mut writer = WriteCtx {
            buf,
            path_length: self.path_length,
            leaf_capacity: self.leaf_capacity,
            branch_factor: self.branch_factor,
        };

        let mut err = MVPError::Success;
        writer.write_node(&node_rc, &mut err);
        if err != MVPError::Success {
            return err;
        }

        // Write to file. The mode parameter mirrors the C api but C uses octal literals;
        // in Rust the literal is decimal, so we always set readable/writable for the owner
        // by OR-ing the user-rw bits with the supplied mode value.
        let mut opts = OpenOptions::new();
        opts.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            // ensure owner read/write at minimum
            let m = (mode as u32) | 0o600;
            opts.mode(m);
        }
        let mut f = match opts.open(filename) {
            Ok(f) => f,
            Err(_) => return MVPError::FileOpen,
        };
        if f.write_all(&writer.buf).is_err() {
            return MVPError::NoWrite;
        }
        MVPError::Success
    }

    pub fn print(&self, stream: &mut dyn Write) -> MVPError {
        let node_rc = match &self.node {
            Some(n) => n.clone(),
            None => {
                let _ = writeln!(stream, "NULL0");
                return MVPError::Success;
            }
        };
        self.print_recursive(stream, &node_rc, 0)
    }

    fn print_recursive(&self, stream: &mut dyn Write, node_rc: &Rc<RefCell<Node>>, lvl: usize) -> MVPError {
        let node = node_rc.borrow();
        if is_null_node(&node) {
            let _ = writeln!(stream, "NULL{}", lvl);
            return MVPError::Success;
        }
        let bf = self.branch_factor;
        let length_m1 = bf - 1;
        let length_m2 = bf;
        let fanout = bf * bf;

        match &*node {
            Node::Leaf(l) => {
                let _ = writeln!(stream, "LEAF{}  ({} points)", lvl, l.nbpoints);
                if let Some(sv1) = &l.sv1 {
                    let _ = writeln!(stream, "    sv1: {}", sv1.id);
                }
                if let Some(sv2) = &l.sv2 {
                    let _ = writeln!(stream, "    sv2: {}", sv2.id);
                }
                for (i, p) in l.points.iter().enumerate() {
                    let _ = writeln!(stream, "        point[{}]: {}", i, p.id);
                }
                MVPError::Success
            }
            Node::Internal(int_n) => {
                let _ = writeln!(stream, "INTERNAL{}", lvl);
                if let Some(sv1) = &int_n.sv1 {
                    let _ = writeln!(stream, "  sv1: {}", sv1.id);
                }
                if let Some(sv2) = &int_n.sv2 {
                    let _ = writeln!(stream, "  sv2: {}", sv2.id);
                }
                for i in 0..length_m1 {
                    let _ = write!(stream, "  M1[{}] = {:.4};", i, int_n.m1[i]);
                }
                for i in 0..length_m2 {
                    let m2v = if i < int_n.m2.len() { int_n.m2[i] } else { 0.0 };
                    let _ = write!(stream, "  M2[{}] = {:.4};", i, m2v);
                }
                let _ = writeln!(stream, "");

                let children = int_n.child_nodes.clone();
                drop(node);
                for i in 0..fanout {
                    let e = self.print_recursive(stream, &children[i], lvl + 2);
                    if e != MVPError::Success {
                        return e;
                    }
                }
                MVPError::Success
            }
        }
    }

    pub fn clear(&mut self, _node: &mut Option<Box<Node>>) {
        self.node = None;
    }

    pub fn extend_mvpfile(&mut self) -> i32 {
        // No mmap-based file in pure Rust impl; treat as no-op success
        self.size += self.pgsize;
        0
    }
}

// Helper context for writing the tree to a buffer
struct WriteCtx {
    buf: Vec<u8>,
    path_length: usize,
    leaf_capacity: usize,
    branch_factor: usize,
}

impl WriteCtx {
    fn write_datapoint(&mut self, dp: Option<&MVPDatapoint>) -> i64 {
        let start = self.buf.len() as i64;
        match dp {
            None => {
                // active=0, bytelength=0
                self.buf.push(0u8);
                self.buf.extend_from_slice(&0u32.to_ne_bytes());
            }
            Some(dp) => {
                let active: u8 = 1;
                let id_bytes = dp.id.as_bytes();
                let idlen: u8 = id_bytes.len() as u8;
                let datalength: u32 = dp.datalen as u32;
                let dtype: u8 = dp.data_type as u8;
                let path_size = self.path_length * 4;
                let bytelength: u32 = (1u32) + (idlen as u32) + 4 + datalength * (dtype as u32) + path_size as u32;
                self.buf.push(active);
                self.buf.extend_from_slice(&bytelength.to_ne_bytes());
                self.buf.push(idlen);
                self.buf.extend_from_slice(id_bytes);
                self.buf.extend_from_slice(&datalength.to_ne_bytes());
                let data_bytes = datalength as usize * dtype as usize;
                if data_bytes > 0 {
                    if dp.data.len() >= data_bytes {
                        self.buf.extend_from_slice(&dp.data[..data_bytes]);
                    } else {
                        self.buf.extend_from_slice(&dp.data);
                        let remaining = data_bytes - dp.data.len();
                        self.buf.extend(std::iter::repeat(0u8).take(remaining));
                    }
                }
                // path
                for i in 0..self.path_length {
                    let v = if i < dp.path.len() { dp.path[i] } else { 0.0f32 };
                    self.buf.extend_from_slice(&v.to_ne_bytes());
                }
            }
        }
        start
    }

    fn write_node(&mut self, node_rc: &Rc<RefCell<Node>>, err: &mut MVPError) -> i64 {
        let start_pos = self.buf.len() as i64;
        let node = node_rc.borrow();
        if is_null_node(&node) {
            return 0;
        }
        match &*node {
            Node::Leaf(l) => {
                let nodet: u8 = NodeType::LeafNode as u8;
                self.buf.push(nodet);
                let sv1 = l.sv1.clone();
                let sv2 = l.sv2.clone();
                let nbpoints: u32 = l.nbpoints as u32;
                let points = l.points.clone();
                let d1 = l.d1.clone();
                let d2 = l.d2.clone();
                drop(node);

                self.write_datapoint(sv1.as_ref().map(|a| a.as_ref()));
                self.write_datapoint(sv2.as_ref().map(|a| a.as_ref()));
                self.buf.extend_from_slice(&nbpoints.to_ne_bytes());

                // We'll write d1, d2, offset for each point in leafcap*((4)+(4)+(8)) bytes.
                // First reserve that block
                let entry_size = 4 + 4 + 8; // float + float + i64
                let block_size = self.leaf_capacity * entry_size;
                let saved_pos = self.buf.len();
                self.buf.resize(saved_pos + block_size, 0);

                let mut sp = saved_pos;
                for i in 0..(nbpoints as usize) {
                    let d1v = if i < d1.len() { d1[i] } else { 0.0f32 };
                    let d2v = if i < d2.len() { d2[i] } else { 0.0f32 };
                    self.buf[sp..sp + 4].copy_from_slice(&d1v.to_ne_bytes());
                    sp += 4;
                    self.buf[sp..sp + 4].copy_from_slice(&d2v.to_ne_bytes());
                    sp += 4;
                    let offset = self.write_datapoint(Some(points[i].as_ref()));
                    self.buf[sp..sp + 8].copy_from_slice(&offset.to_ne_bytes());
                    sp += 8;
                }
            }
            Node::Internal(int_n) => {
                let nodet: u8 = NodeType::InternalNode as u8;
                let bf = self.branch_factor;
                let length_m1 = bf - 1;
                let length_m2 = bf * length_m1;
                let fanout = bf * bf;
                let sv1 = int_n.sv1.clone();
                let sv2 = int_n.sv2.clone();
                let m1 = int_n.m1.clone();
                let m2 = int_n.m2.clone();
                let children = int_n.child_nodes.clone();
                drop(node);

                self.buf.push(nodet);
                self.write_datapoint(sv1.as_ref().map(|a| a.as_ref()));
                self.write_datapoint(sv2.as_ref().map(|a| a.as_ref()));
                for i in 0..length_m1 {
                    let v = if i < m1.len() { m1[i] } else { 0.0f32 };
                    self.buf.extend_from_slice(&v.to_ne_bytes());
                }
                for i in 0..length_m2 {
                    let v = if i < m2.len() { m2[i] } else { 0.0f32 };
                    self.buf.extend_from_slice(&v.to_ne_bytes());
                }

                // Reserve fanout * (1 + 8) bytes for fileno + offset
                let entry_size = 1 + 8;
                let block_size = fanout * entry_size;
                let saved_pos = self.buf.len();
                self.buf.resize(saved_pos + block_size, 0);

                let mut sp = saved_pos;
                for i in 0..fanout {
                    let offset = self.write_node(&children[i], err);
                    self.buf[sp] = 0u8; // fileno
                    sp += 1;
                    self.buf[sp..sp + 8].copy_from_slice(&offset.to_ne_bytes());
                    sp += 8;
                    if *err != MVPError::Success {
                        return start_pos;
                    }
                }
            }
        }
        start_pos
    }
}

// Reading
struct ReadCtx<'a> {
    buf: &'a [u8],
    pos: usize,
    path_length: usize,
    leaf_capacity: usize,
    branch_factor: usize,
    datatype: MVPDataType,
}

impl<'a> ReadCtx<'a> {
    fn read_u8(&mut self) -> u8 {
        let v = self.buf[self.pos];
        self.pos += 1;
        v
    }
    fn read_u32(&mut self) -> u32 {
        let mut b = [0u8; 4];
        b.copy_from_slice(&self.buf[self.pos..self.pos + 4]);
        self.pos += 4;
        u32::from_ne_bytes(b)
    }
    fn read_i64(&mut self) -> i64 {
        let mut b = [0u8; 8];
        b.copy_from_slice(&self.buf[self.pos..self.pos + 8]);
        self.pos += 8;
        i64::from_ne_bytes(b)
    }
    fn read_f32(&mut self) -> f32 {
        let mut b = [0u8; 4];
        b.copy_from_slice(&self.buf[self.pos..self.pos + 4]);
        self.pos += 4;
        f32::from_ne_bytes(b)
    }
    fn read_datapoint(&mut self) -> Option<MVPDatapoint> {
        let active = self.read_u8();
        let bytelength = self.read_u32();
        if active == 0 && bytelength == 0 {
            return None;
        }
        let idlen = self.read_u8() as usize;
        let id_bytes = self.buf[self.pos..self.pos + idlen].to_vec();
        self.pos += idlen;
        let datalength = self.read_u32() as usize;
        let datatype = self.datatype;
        let dsz = datatype.byte_size();
        let data_bytes = datalength * dsz;
        let data = self.buf[self.pos..self.pos + data_bytes].to_vec();
        self.pos += data_bytes;
        let mut path = vec![0.0f32; self.path_length];
        for i in 0..self.path_length {
            path[i] = self.read_f32();
        }
        let id = String::from_utf8_lossy(&id_bytes).to_string();
        Some(MVPDatapoint {
            id,
            data,
            path,
            datalen: datalength,
            data_type: datatype,
        })
    }
    fn read_node(&mut self, err: &mut MVPError) -> Rc<RefCell<Node>> {
        let nodet = self.read_u8();
        if nodet == NodeType::LeafNode as u8 {
            let sv1 = self.read_datapoint().map(Arc::new);
            let sv2 = self.read_datapoint().map(Arc::new);
            let nbpoints = self.read_u32() as usize;
            let saved_pos = self.pos;
            let mut leaf = LeafNode {
                node_type: NodeType::LeafNode,
                sv1,
                sv2,
                points: Vec::with_capacity(self.leaf_capacity),
                d1: vec![0.0; self.leaf_capacity],
                d2: vec![0.0; self.leaf_capacity],
                nbpoints,
            };
            let mut sp = saved_pos;
            for i in 0..nbpoints {
                // Read d1, d2, offset
                let mut b4 = [0u8; 4];
                b4.copy_from_slice(&self.buf[sp..sp + 4]);
                let d1v = f32::from_ne_bytes(b4);
                sp += 4;
                b4.copy_from_slice(&self.buf[sp..sp + 4]);
                let d2v = f32::from_ne_bytes(b4);
                sp += 4;
                let mut b8 = [0u8; 8];
                b8.copy_from_slice(&self.buf[sp..sp + 8]);
                let offset = i64::from_ne_bytes(b8) as usize;
                sp += 8;
                if i < leaf.d1.len() {
                    leaf.d1[i] = d1v;
                    leaf.d2[i] = d2v;
                }
                self.pos = offset;
                if let Some(p) = self.read_datapoint() {
                    leaf.points.push(Arc::new(p));
                }
            }
            // Position after the leaf table at the very least
            self.pos = sp;
            Rc::new(RefCell::new(Node::Leaf(leaf)))
        } else if nodet == NodeType::InternalNode as u8 {
            let bf = self.branch_factor;
            let length_m1 = bf - 1;
            let length_m2 = bf * length_m1;
            let fanout = bf * bf;
            let sv1 = self.read_datapoint().map(Arc::new);
            let sv2 = self.read_datapoint().map(Arc::new);
            let mut m1 = vec![0.0f32; length_m1];
            for i in 0..length_m1 {
                m1[i] = self.read_f32();
            }
            let mut m2 = vec![0.0f32; length_m2];
            for i in 0..length_m2 {
                m2[i] = self.read_f32();
            }
            let mut children: Vec<Rc<RefCell<Node>>> = Vec::with_capacity(fanout);
            let saved_pos = self.pos;
            let mut sp = saved_pos;
            for _ in 0..fanout {
                // fileno
                sp += 1;
                let mut b8 = [0u8; 8];
                b8.copy_from_slice(&self.buf[sp..sp + 8]);
                let offset = i64::from_ne_bytes(b8);
                sp += 8;
                if offset == 0 {
                    children.push(Rc::new(RefCell::new(empty_leaf())));
                } else {
                    self.pos = offset as usize;
                    let child = self.read_node(err);
                    children.push(child);
                    if *err != MVPError::Success {
                        break;
                    }
                }
            }
            self.pos = sp;
            let internal = InternalNode {
                node_type: NodeType::InternalNode,
                sv1,
                sv2,
                m1,
                m2,
                child_nodes: children,
            };
            Rc::new(RefCell::new(Node::Internal(internal)))
        } else {
            *err = MVPError::Unrecognized;
            Rc::new(RefCell::new(empty_leaf()))
        }
    }
}

pub fn mvptree_read(filename: &str, distance_function: DistanceFunction) -> Result<MVPTree, MVPError> {
    let mut f = match File::open(filename) {
        Ok(f) => f,
        Err(_) => return Err(MVPError::FileNotFound),
    };
    let mut buf: Vec<u8> = Vec::new();
    if f.read_to_end(&mut buf).is_err() {
        return Err(MVPError::FileOpen);
    }
    if buf.len() < HEADER_SIZE {
        return Err(MVPError::ArgErr);
    }
    let tag_bytes = TAG.as_bytes();
    let mut pos = tag_bytes.len() + 1;
    pos += 4; // version
    let bf = buf[pos];
    pos += 1;
    let pl = buf[pos];
    pos += 1;
    let lc = buf[pos];
    pos += 1;
    let ht = buf[pos];
    let dt = MVPDataType::from_u8(ht).unwrap_or(MVPDataType::ByteArray);

    let mut ctx = ReadCtx {
        buf: &buf,
        pos: HEADER_SIZE,
        path_length: pl as usize,
        leaf_capacity: lc as usize,
        branch_factor: bf as usize,
        datatype: dt,
    };
    let mut err = MVPError::Success;
    let node = ctx.read_node(&mut err);
    if err != MVPError::Success {
        return Err(err);
    }

    let mut tree = MVPTree::new(bf as usize, pl as usize, lc as usize, dt, distance_function);
    tree.node = Some(node);
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
    pub fn select_vantage_points(&mut self, _nb: u32, _sv1_pos: i32, _sv2_pos: i32, _dist: DistanceFunction) -> i32 {
        0
    }
    pub fn find_splits(&mut self, _nb: u32, _vp: &MVPDatapoint, _tree: &MVPTree, _length_m: u32) -> f32 {
        0.0
    }
    pub fn sort_points(&mut self, _nb: u32, _sv1_pos: i32, _sv2_pos: i32, _vp: &MVPDatapoint, _tree: &MVPTree, _counts: &mut Vec<Vec<i32>>, _pivots: Vec<f32>) -> Vec<Vec<Vec<Arc<MVPDatapoint>>>> {
        Vec::new()
    }
    pub fn find_distance_range_for_vp(&mut self, _nb: u32, _vp: &MVPDatapoint, _tree: &MVPTree, _level: i32) -> i32 {
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
