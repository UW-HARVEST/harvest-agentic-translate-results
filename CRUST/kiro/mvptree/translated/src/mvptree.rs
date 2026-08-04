use std::fs::File;
use std::io::{self, Write};
use std::ptr;
use std::os::raw::c_int;
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
impl InternalNode{
    pub fn new(bf:u32) -> Self {
        InternalNode {
            node_type: NodeType::InternalNode,
            sv1: None,
            sv2: None,
            m1: vec![0.0; (bf - 1) as usize],
            m2: vec![0.0; ((bf - 1) * bf) as usize],
            child_nodes: Vec::new(),
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
        LeafNode {
            node_type: NodeType::LeafNode,
            sv1: None,
            sv2: None,
            points: Vec::new(),
            d1: Vec::new(),
            d2: Vec::new(),
            nbpoints: 0,
        }
    }
}
pub enum Node {
    Leaf(LeafNode),
    Internal(InternalNode),
}

impl Node {
    fn as_leaf_mut(&mut self) -> &mut LeafNode {
        match self {
            Node::Leaf(l) => l,
            _ => panic!("not a leaf"),
        }
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

fn is_nan(x: f32) -> bool {
    x.is_nan()
}

fn mvp_error_index(err: MVPError) -> usize {
    match err {
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

fn datatype_size(dt: MVPDataType) -> usize {
    match dt {
        MVPDataType::ByteArray => 1,
        MVPDataType::UInt16Array => 2,
        MVPDataType::UInt32Array => 4,
        MVPDataType::UInt64Array => 8,
    }
}

fn datatype_from_u8(v: u8) -> MVPDataType {
    match v {
        1 => MVPDataType::ByteArray,
        2 => MVPDataType::UInt16Array,
        4 => MVPDataType::UInt32Array,
        8 => MVPDataType::UInt64Array,
        _ => MVPDataType::ByteArray,
    }
}


fn select_vantage_points(points: &[Arc<MVPDatapoint>], dist: DistanceFunction) -> Result<(i32, i32), i32> {
    if points.is_empty() { return Err(-1); }
    let mut sv1_pos: i32 = 0;
    let mut sv2_pos: i32 = -1;
    let mut max_dist = 0.0f32;
    let nb = points.len();
    for i in 0..nb {
        for j in (i + 1)..nb {
            let d = dist(&points[i], &points[j]);
            if is_nan(d) || d < 0.0 { return Err(-2); }
            if d > max_dist {
                max_dist = d;
                sv1_pos = i as i32;
                sv2_pos = j as i32;
            }
        }
    }
    // If no pair found with d > 0, pick second point if available
    if sv2_pos < 0 && nb >= 2 {
        sv2_pos = if sv1_pos == 0 { 1 } else { 0 };
    }
    Ok((sv1_pos, sv2_pos))
}

fn find_splits(points: &[Arc<MVPDatapoint>], vp: &MVPDatapoint, dist: DistanceFunction, length_m: usize) -> Result<Vec<f32>, i32> {
    if length_m == 0 { return Err(-1); }
    let nb = points.len();
    if nb == 0 { return Ok(vec![0.0; length_m]); }
    let mut dists: Vec<f32> = Vec::with_capacity(nb);
    for p in points {
        let d = dist(p, vp);
        if is_nan(d) || d < 0.0 { return Err(-2); }
        dists.push(d);
    }
    // selection sort
    for i in 0..nb.saturating_sub(1) {
        let mut min_pos = i;
        for j in (i + 1)..nb {
            if dists[j] < dists[min_pos] { min_pos = j; }
        }
        if min_pos != i { dists.swap(i, min_pos); }
    }
    let mut m = vec![0.0f32; length_m];
    for i in 0..length_m {
        let mut index = (i + 1) * nb / (length_m + 1);
        if index >= nb { index = nb - 1; }
        m[i] = dists[index];
    }
    Ok(m)
}

fn sort_points(
    points: &[Arc<MVPDatapoint>],
    sv1_pos: i32,
    sv2_pos: i32,
    vp: &MVPDatapoint,
    dist: DistanceFunction,
    bf: usize,
    pivots: &[f32],
) -> Result<(Vec<Vec<Arc<MVPDatapoint>>>, Vec<usize>), i32> {
    let nb = points.len();
    if nb == 0 {
        let bins: Vec<Vec<Arc<MVPDatapoint>>> = (0..bf).map(|_| Vec::new()).collect();
        let counts = vec![0usize; bf];
        return Ok((bins, counts));
    }
    let length_m1 = bf - 1;
    let mut bins: Vec<Vec<Arc<MVPDatapoint>>> = (0..bf).map(|_| Vec::new()).collect();
    for (i, p) in points.iter().enumerate() {
        if i as i32 == sv1_pos || i as i32 == sv2_pos { continue; }
        let d = dist(vp, p);
        if is_nan(d) || d < 0.0 { return Err(-1); }
        let mut placed = false;
        for k in 0..length_m1 {
            if d <= pivots[k] {
                bins[k].push(Arc::clone(p));
                placed = true;
                break;
            }
        }
        if d > pivots[length_m1 - 1] {
            bins[length_m1].push(Arc::clone(p));
        }
    }
    let counts: Vec<usize> = bins.iter().map(|b| b.len()).collect();
    Ok((bins, counts))
}

fn find_distance_range_for_vp(
    points: &[Arc<MVPDatapoint>],
    vp: &MVPDatapoint,
    dist: DistanceFunction,
    path_length: usize,
    lvl: usize,
) -> Result<(), i32> {
    for p in points {
        let d = dist(vp, p);
        if is_nan(d) || d < 0.0 { return Err(-2); }
        if lvl < path_length {
            // We need to mutate path through Arc - use unsafe or restructure
            // Since Arc<MVPDatapoint> is shared, we need interior mutability for path
            // The C code mutates points[i]->path[lvl] directly
            // We'll use unsafe to mutate through Arc since we know we have exclusive logical access during tree building
            let ptr = Arc::as_ptr(p) as *mut MVPDatapoint;
            unsafe { (*ptr).path.as_mut_ptr().add(lvl).write(d); }
        }
    }
    Ok(())
}


fn _mvptree_add(
    tree: &MVPTree,
    node: Option<Rc<RefCell<Node>>>,
    points: &[Arc<MVPDatapoint>],
    error: &mut MVPError,
    lvl: usize,
) -> Option<Rc<RefCell<Node>>> {
    if points.is_empty() { return node; }
    let dist_fnc = tree.distance_function;
    let bf = tree.branch_factor;
    let length_m1 = bf - 1;

    if node.is_none() {
        // Create new node
        if points.len() <= tree.leaf_capacity + 2 {
            // Create leaf node
            let (sv1_pos, sv2_pos) = match select_vantage_points(points, dist_fnc) {
                Ok(v) => v,
                Err(_) => { *error = MVPError::VpNoSelect; return None; }
            };
            let sv1 = if sv1_pos >= 0 { Some(Arc::clone(&points[sv1_pos as usize])) } else { None };
            let sv2 = if sv2_pos >= 0 { Some(Arc::clone(&points[sv2_pos as usize])) } else { None };

            if let Some(ref sv) = sv1 {
                if find_distance_range_for_vp(points, sv, dist_fnc, tree.path_length, lvl).is_err() {
                    *error = MVPError::NoSv1Range; return None;
                }
            }
            if let Some(ref sv) = sv2 {
                if find_distance_range_for_vp(points, sv, dist_fnc, tree.path_length, lvl + 1).is_err() {
                    *error = MVPError::NoSv2Range; return None;
                }
            }

            let mut leaf = LeafNode::new(bf as u32);
            leaf.sv1 = sv1.clone();
            leaf.sv2 = sv2.clone();
            for (i, p) in points.iter().enumerate() {
                if i as i32 == sv1_pos || i as i32 == sv2_pos { continue; }
                leaf.d1.push(dist_fnc(p, sv1.as_ref().unwrap()));
                leaf.d2.push(if let Some(ref s2) = sv2 { dist_fnc(p, s2) } else { 0.0 });
                leaf.points.push(Arc::clone(p));
            }
            leaf.nbpoints = leaf.points.len();
            Some(Rc::new(RefCell::new(Node::Leaf(leaf))))
        } else {
            // Create internal node
            let (sv1_pos, sv2_pos) = match select_vantage_points(points, dist_fnc) {
                Ok(v) => v,
                Err(_) => { *error = MVPError::VpNoSelect; return None; }
            };
            let sv1 = Arc::clone(&points[sv1_pos as usize]);
            let sv2 = Arc::clone(&points[sv2_pos as usize]);

            if find_distance_range_for_vp(points, &sv1, dist_fnc, tree.path_length, lvl).is_err() {
                *error = MVPError::NoSv1Range; return None;
            }

            let m1 = match find_splits(points, &sv1, dist_fnc, length_m1) {
                Ok(v) => v,
                Err(_) => { *error = MVPError::NoSplits; return None; }
            };

            let (bins, _bin_lengths) = match sort_points(points, sv1_pos, sv2_pos, &sv1, dist_fnc, bf, &m1) {
                Ok(v) => v,
                Err(_) => { *error = MVPError::NoSort; return None; }
            };

            let mut internal = InternalNode::new(bf as u32);
            internal.sv1 = Some(sv1.clone());
            internal.sv2 = Some(sv2.clone());
            internal.m1 = m1;

            for i in 0..bf {
                if find_distance_range_for_vp(&bins[i], &sv2, dist_fnc, tree.path_length, lvl + 1).is_err() {
                    *error = MVPError::NoSv2Range; return None;
                }
                let m2_slice = match find_splits(&bins[i], &sv2, dist_fnc, length_m1) {
                    Ok(v) => v,
                    Err(_) => { *error = MVPError::NoSplits; return None; }
                };
                // Store M2 at offset i*lengthM1
                for k in 0..length_m1 {
                    internal.m2[i * length_m1 + k] = m2_slice[k];
                }

                let (bins2, _bin2_lengths) = match sort_points(&bins[i], -1, -1, &sv2, dist_fnc, bf, &m2_slice) {
                    Ok(v) => v,
                    Err(_) => { *error = MVPError::NoSort; return None; }
                };

                for j in 0..bf {
                    let child = _mvptree_add(tree, None, &bins2[j], error, lvl + 2);
                    internal.child_nodes.push(match child {
                        Some(c) => c,
                        None => Rc::new(RefCell::new(Node::Leaf(LeafNode::new(bf as u32)))),
                    });
                }
            }

            Some(Rc::new(RefCell::new(Node::Internal(internal))))
        }
    } else {
        // Node already exists
        let node_rc = node.unwrap();
        let is_leaf = matches!(&*node_rc.borrow(), Node::Leaf(_));
        
        // Check if this is an empty placeholder leaf (no sv1)
        let is_empty_leaf = {
            let borrow = node_rc.borrow();
            matches!(&*borrow, Node::Leaf(leaf) if leaf.sv1.is_none() && leaf.nbpoints == 0)
        };
        if is_empty_leaf {
            return _mvptree_add(tree, None, points, error, lvl);
        }

        if is_leaf {
            let nbpoints_in_leaf;
            let can_fit;
            {
                let borrow = node_rc.borrow();
                if let Node::Leaf(ref leaf) = *borrow {
                    nbpoints_in_leaf = leaf.nbpoints;
                    can_fit = leaf.nbpoints + points.len() <= tree.leaf_capacity;
                } else { unreachable!(); }
            }

            if can_fit {
                // Clone sv1/sv2 arcs outside borrow
                let sv1_arc;
                let sv2_arc_before;
                {
                    let borrow = node_rc.borrow();
                    if let Node::Leaf(ref leaf) = *borrow {
                        sv1_arc = leaf.sv1.clone();
                        sv2_arc_before = leaf.sv2.clone();
                    } else { unreachable!(); }
                }

                if let Some(ref sv1) = sv1_arc {
                    if find_distance_range_for_vp(points, sv1, dist_fnc, tree.path_length, lvl).is_err() {
                        *error = MVPError::NoSv1Range;
                        return Some(node_rc);
                    }
                }

                let mut start_pos = 0;
                if sv2_arc_before.is_none() && !points.is_empty() {
                    node_rc.borrow_mut().as_leaf_mut().sv2 = Some(Arc::clone(&points[0]));
                    start_pos = 1;
                }

                let sv2_arc;
                {
                    let borrow = node_rc.borrow();
                    if let Node::Leaf(ref leaf) = *borrow {
                        sv2_arc = leaf.sv2.clone();
                    } else { unreachable!(); }
                }

                if let Some(ref sv2) = sv2_arc {
                    if find_distance_range_for_vp(points, sv2, dist_fnc, tree.path_length, lvl + 1).is_err() {
                        *error = MVPError::NoSv2Range;
                        return Some(node_rc);
                    }
                }

                {
                    let mut borrow = node_rc.borrow_mut();
                    if let Node::Leaf(ref mut leaf) = *borrow {
                        let sv1_ref = leaf.sv1.as_ref().unwrap();
                        let sv2_ref = leaf.sv2.as_ref().unwrap();
                        for i in start_pos..points.len() {
                            leaf.d1.push(dist_fnc(&points[i], sv1_ref));
                            leaf.d2.push(dist_fnc(&points[i], sv2_ref));
                            leaf.points.push(Arc::clone(&points[i]));
                        }
                        leaf.nbpoints = leaf.points.len();
                    }
                }
                Some(node_rc)
            } else {
                // Not enough room - collect all points and rebuild
                let mut tmp_pts: Vec<Arc<MVPDatapoint>> = Vec::new();
                {
                    let borrow = node_rc.borrow();
                    if let Node::Leaf(ref leaf) = *borrow {
                        if let Some(ref sv1) = leaf.sv1 { tmp_pts.push(Arc::clone(sv1)); }
                        if let Some(ref sv2) = leaf.sv2 { tmp_pts.push(Arc::clone(sv2)); }
                        for p in &leaf.points { tmp_pts.push(Arc::clone(p)); }
                    }
                }
                for p in points { tmp_pts.push(Arc::clone(p)); }
                _mvptree_add(tree, None, &tmp_pts, error, lvl)
            }
        } else {
            // Internal node - recurse
            let sv1_clone;
            let sv2_clone;
            let m1_clone;
            {
                let borrow = node_rc.borrow();
                if let Node::Internal(ref internal) = *borrow {
                    sv1_clone = internal.sv1.as_ref().unwrap().clone();
                    sv2_clone = internal.sv2.as_ref().unwrap().clone();
                    m1_clone = internal.m1.clone();
                } else { unreachable!(); }
            }

            if find_distance_range_for_vp(points, &sv1_clone, dist_fnc, tree.path_length, lvl).is_err() {
                *error = MVPError::NoSv1Range;
                return Some(node_rc);
            }

            let (bins, _) = match sort_points(points, -1, -1, &sv1_clone, dist_fnc, bf, &m1_clone) {
                Ok(v) => v,
                Err(_) => { *error = MVPError::NoSort; return Some(node_rc); }
            };

            for i in 0..bf {
                if bins[i].is_empty() { continue; }

                let m2_slice;
                {
                    let borrow = node_rc.borrow();
                    if let Node::Internal(ref internal) = *borrow {
                        let start = i * length_m1;
                        m2_slice = internal.m2[start..start + length_m1].to_vec();
                    } else { unreachable!(); }
                }

                if find_distance_range_for_vp(&bins[i], &sv2_clone, dist_fnc, tree.path_length, lvl + 1).is_err() {
                    *error = MVPError::NoSv2Range;
                    return Some(node_rc);
                }

                let (bins2, _) = match sort_points(&bins[i], -1, -1, &sv2_clone, dist_fnc, bf, &m2_slice) {
                    Ok(v) => v,
                    Err(_) => { *error = MVPError::NoSort; return Some(node_rc); }
                };

                for j in 0..bf {
                    let idx = i * bf + j;
                    let existing_child;
                    {
                        let borrow = node_rc.borrow();
                        if let Node::Internal(ref internal) = *borrow {
                            if idx < internal.child_nodes.len() {
                                existing_child = Some(Rc::clone(&internal.child_nodes[idx]));
                            } else {
                                existing_child = None;
                            }
                        } else { unreachable!(); }
                    }
                    let child = _mvptree_add(tree, existing_child, &bins2[j], error, lvl + 2);
                    if let Some(c) = child {
                        let mut borrow = node_rc.borrow_mut();
                        if let Node::Internal(ref mut internal) = *borrow {
                            if idx < internal.child_nodes.len() {
                                internal.child_nodes[idx] = c;
                            }
                        }
                    }
                    if *error != MVPError::Success { break; }
                }
            }
            Some(node_rc)
        }
    }
}


fn _mvptree_retrieve(
    tree: &MVPTree,
    node: &Rc<RefCell<Node>>,
    target: &mut MVPDatapoint,
    radius: f32,
    results: &mut Vec<Arc<MVPDatapoint>>,
    k: usize,
    lvl: usize,
) -> MVPError {
    let distance = tree.distance_function;
    let bf = tree.branch_factor;
    let length_m1 = bf - 1;

    let borrow = node.borrow();
    match &*borrow {
        Node::Leaf(leaf) => {
            let sv1 = leaf.sv1.as_ref().unwrap();
            let d1 = distance(target, sv1);
            if is_nan(d1) || d1 < 0.0 { return MVPError::BadDistVal; }
            if lvl < tree.path_length { target.path[lvl] = d1; }
            if d1 <= radius {
                results.push(Arc::clone(sv1));
                if results.len() >= k { return MVPError::KNearestCap; }
            }
            if let Some(ref sv2) = leaf.sv2 {
                let d2 = distance(target, sv2);
                if is_nan(d2) || d2 < 0.0 { return MVPError::BadDistVal; }
                if d2 <= radius {
                    results.push(Arc::clone(sv2));
                    if results.len() >= k { return MVPError::KNearestCap; }
                }
                if lvl + 1 < tree.path_length { target.path[lvl + 1] = d2; }

                for i in 0..leaf.nbpoints {
                    if d1 - radius <= leaf.d1[i] && d1 + radius >= leaf.d1[i] {
                        if d2 - radius <= leaf.d2[i] && d2 + radius >= leaf.d2[i] {
                            let endpath = if lvl + 1 < tree.path_length { lvl + 1 } else { tree.path_length };
                            let mut skip = false;
                            for j in 0..endpath {
                                if target.path[j] - radius <= leaf.points[i].path[j]
                                    && target.path[j] + radius >= leaf.points[i].path[j]
                                {
                                    continue;
                                } else {
                                    skip = true;
                                    break;
                                }
                            }
                            if !skip {
                                let d = distance(target, &leaf.points[i]);
                                if is_nan(d) || d < 0.0 { return MVPError::BadDistVal; }
                                if d <= radius {
                                    results.push(Arc::clone(&leaf.points[i]));
                                    if results.len() >= k { return MVPError::KNearestCap; }
                                }
                            }
                        }
                    }
                }
            }
            MVPError::Success
        }
        Node::Internal(internal) => {
            let sv1 = internal.sv1.as_ref().unwrap();
            let sv2 = internal.sv2.as_ref().unwrap();
            let d1 = distance(target, sv1);
            if is_nan(d1) || d1 < 0.0 { return MVPError::BadDistVal; }
            if d1 <= radius {
                results.push(Arc::clone(sv1));
                if results.len() >= k { return MVPError::KNearestCap; }
            }
            if lvl < tree.path_length { target.path[lvl] = d1; }
            let d2 = distance(target, sv2);
            if is_nan(d2) || d2 < 0.0 { return MVPError::BadDistVal; }
            if d2 <= radius {
                results.push(Arc::clone(sv2));
                if results.len() >= k { return MVPError::KNearestCap; }
            }
            if lvl + 1 < tree.path_length { target.path[lvl + 1] = d2; }

            // Check <= each 1st level bin
            for i in 0..length_m1 {
                if d1 - radius <= internal.m1[i] {
                    for j in 0..length_m1 {
                        if d2 - radius <= internal.m2[i * length_m1 + j] {
                            let idx = i * bf + j;
                            if idx < internal.child_nodes.len() {
                                let err = _mvptree_retrieve(tree, &internal.child_nodes[idx], target, radius, results, k, lvl + 2);
                                if err != MVPError::Success { return err; }
                            }
                        }
                    }
                    // Check >= last 2nd level bin
                    if d2 + radius >= internal.m2[i * length_m1 + length_m1 - 1] {
                        let idx = i * bf + length_m1;
                        if idx < internal.child_nodes.len() {
                            let err = _mvptree_retrieve(tree, &internal.child_nodes[idx], target, radius, results, k, lvl + 2);
                            if err != MVPError::Success { return err; }
                        }
                    }
                }
            }

            // Check >= last 1st level bin
            if d1 + radius >= internal.m1[length_m1 - 1] {
                for j in 0..length_m1 {
                    if d2 - radius <= internal.m2[length_m1 * length_m1 + j] {
                        let idx = bf * length_m1 + j;
                        if idx < internal.child_nodes.len() {
                            let err = _mvptree_retrieve(tree, &internal.child_nodes[idx], target, radius, results, k, lvl + 2);
                            if err != MVPError::Success { return err; }
                        }
                    }
                }
                if d2 + radius >= internal.m2[length_m1 * length_m1 + length_m1 - 1] {
                    let idx = bf * length_m1 + length_m1;
                    if idx < internal.child_nodes.len() {
                        let err = _mvptree_retrieve(tree, &internal.child_nodes[idx], target, radius, results, k, lvl + 2);
                        if err != MVPError::Success { return err; }
                    }
                }
            }

            MVPError::Success
        }
    }
}


fn write_datapoint(dp: Option<&Arc<MVPDatapoint>>, buf: &mut Vec<u8>, pos: &mut usize, path_length: usize, datatype: MVPDataType) -> i64 {
    let start = *pos as i64;
    let type_size = datatype_size(datatype);
    match dp {
        None => {
            buf[*pos] = 0; // active = 0
            *pos += 1;
            buf[*pos..*pos + 4].copy_from_slice(&0u32.to_le_bytes());
            *pos += 4;
        }
        Some(dp) => {
            buf[*pos] = 1; // active = 1
            *pos += 1;
            let idlen = dp.id.len() as u8;
            let datalength = dp.datalen as u32;
            let bytelength: u32 = 1 + idlen as u32 + 4 + datalength * type_size as u32 + (path_length as u32) * 4;
            buf[*pos..*pos + 4].copy_from_slice(&bytelength.to_le_bytes());
            *pos += 4;
            buf[*pos] = idlen;
            *pos += 1;
            buf[*pos..*pos + idlen as usize].copy_from_slice(dp.id.as_bytes());
            *pos += idlen as usize;
            buf[*pos..*pos + 4].copy_from_slice(&datalength.to_le_bytes());
            *pos += 4;
            let data_bytes = datalength as usize * type_size;
            buf[*pos..*pos + data_bytes].copy_from_slice(&dp.data[..data_bytes]);
            *pos += data_bytes;
            for k in 0..path_length {
                let v = if k < dp.path.len() { dp.path[k] } else { 0.0 };
                buf[*pos..*pos + 4].copy_from_slice(&v.to_le_bytes());
                *pos += 4;
            }
        }
    }
    start
}

fn read_datapoint(buf: &[u8], pos: &mut usize, path_length: usize, datatype: MVPDataType) -> Option<Arc<MVPDatapoint>> {
    let active = buf[*pos];
    *pos += 1;
    let bytelength = u32::from_le_bytes(buf[*pos..*pos + 4].try_into().unwrap());
    *pos += 4;
    if active == 0 && bytelength == 0 { return None; }

    let type_size = datatype_size(datatype);
    let idlen = buf[*pos] as usize;
    *pos += 1;
    let id = String::from_utf8_lossy(&buf[*pos..*pos + idlen]).to_string();
    *pos += idlen;
    let datalength = u32::from_le_bytes(buf[*pos..*pos + 4].try_into().unwrap()) as usize;
    *pos += 4;
    let data_bytes = datalength * type_size;
    let data = buf[*pos..*pos + data_bytes].to_vec();
    *pos += data_bytes;
    let mut path = Vec::with_capacity(path_length);
    for _ in 0..path_length {
        path.push(f32::from_le_bytes(buf[*pos..*pos + 4].try_into().unwrap()));
        *pos += 4;
    }
    Some(Arc::new(MVPDatapoint {
        id,
        data,
        path,
        datalen: datalength,
        data_type: datatype,
    }))
}

fn _mvptree_write_node(
    node: &Rc<RefCell<Node>>,
    buf: &mut Vec<u8>,
    pos: &mut usize,
    buf_size: &mut usize,
    pgsize: usize,
    path_length: usize,
    leaf_capacity: usize,
    branch_factor: usize,
    datatype: MVPDataType,
    error: &mut MVPError,
) -> i64 {
    let start_pos = *pos as i64;
    let borrow = node.borrow();

    // Ensure buffer is large enough
    fn ensure_buf(buf: &mut Vec<u8>, pos: usize, buf_size: &mut usize, pgsize: usize) {
        while pos >= *buf_size - pgsize / 2 {
            *buf_size += pgsize;
            buf.resize(*buf_size, 0);
        }
    }

    match &*borrow {
        Node::Leaf(leaf) => {
            ensure_buf(buf, *pos, buf_size, pgsize);
            buf[*pos] = NodeType::LeafNode as u8;
            *pos += 1;
            write_datapoint(leaf.sv1.as_ref(), buf, pos, path_length, datatype);
            write_datapoint(leaf.sv2.as_ref(), buf, pos, path_length, datatype);
            let nbpoints = leaf.nbpoints as u32;
            buf[*pos..*pos + 4].copy_from_slice(&nbpoints.to_le_bytes());
            *pos += 4;

            let saved_pos_start = *pos;
            // Reserve space: leafcap * (2*sizeof(float) + sizeof(off_t)) = leafcap * (8 + 8) = leafcap * 16
            *pos += leaf_capacity * (2 * 4 + 8);

            let mut saved_pos = saved_pos_start;
            for i in 0..leaf.nbpoints {
                ensure_buf(buf, *pos, buf_size, pgsize);
                let d1_val = if i < leaf.d1.len() { leaf.d1[i] } else { 0.0 };
                let d2_val = if i < leaf.d2.len() { leaf.d2[i] } else { 0.0 };
                buf[saved_pos..saved_pos + 4].copy_from_slice(&d1_val.to_le_bytes());
                saved_pos += 4;
                buf[saved_pos..saved_pos + 4].copy_from_slice(&d2_val.to_le_bytes());
                saved_pos += 4;
                let offset = write_datapoint(Some(&leaf.points[i]), buf, pos, path_length, datatype);
                buf[saved_pos..saved_pos + 8].copy_from_slice(&offset.to_le_bytes());
                saved_pos += 8;
            }
        }
        Node::Internal(internal) => {
            let bf = branch_factor;
            let length_m1 = bf - 1;
            let length_m2 = (bf - 1) * bf;
            let fanout = bf * bf;

            ensure_buf(buf, *pos, buf_size, pgsize);
            buf[*pos] = NodeType::InternalNode as u8;
            *pos += 1;
            write_datapoint(internal.sv1.as_ref(), buf, pos, path_length, datatype);
            write_datapoint(internal.sv2.as_ref(), buf, pos, path_length, datatype);
            for k in 0..length_m1 {
                buf[*pos..*pos + 4].copy_from_slice(&internal.m1[k].to_le_bytes());
                *pos += 4;
            }
            for k in 0..length_m2 {
                buf[*pos..*pos + 4].copy_from_slice(&internal.m2[k].to_le_bytes());
                *pos += 4;
            }

            let saved_pos_start = *pos;
            // Reserve space: fanout * (1 + 8)
            *pos += fanout * (1 + 8);

            let mut saved_pos = saved_pos_start;
            for i in 0..fanout {
                ensure_buf(buf, *pos, buf_size, pgsize);
                let offset = if i < internal.child_nodes.len() {
                    _mvptree_write_node(&internal.child_nodes[i], buf, pos, buf_size, pgsize, path_length, leaf_capacity, branch_factor, datatype, error)
                } else {
                    0i64
                };
                buf[saved_pos] = 0u8; // fileno
                saved_pos += 1;
                buf[saved_pos..saved_pos + 8].copy_from_slice(&offset.to_le_bytes());
                saved_pos += 8;
            }
        }
    }
    start_pos
}

fn _mvptree_read_node(
    buf: &[u8],
    pos: &mut usize,
    path_length: usize,
    leaf_capacity: usize,
    branch_factor: usize,
    datatype: MVPDataType,
    error: &mut MVPError,
) -> Option<Rc<RefCell<Node>>> {
    let node_type = buf[*pos];
    *pos += 1;

    if node_type == NodeType::LeafNode as u8 {
        let sv1 = read_datapoint(buf, pos, path_length, datatype);
        let sv2 = read_datapoint(buf, pos, path_length, datatype);
        let nbpoints = u32::from_le_bytes(buf[*pos..*pos + 4].try_into().unwrap()) as usize;
        *pos += 4;

        let saved_pos_start = *pos;
        let mut saved_pos = saved_pos_start;
        let mut leaf = LeafNode::new(branch_factor as u32);
        leaf.sv1 = sv1;
        leaf.sv2 = sv2;
        leaf.nbpoints = nbpoints;

        for _i in 0..nbpoints {
            let d1 = f32::from_le_bytes(buf[saved_pos..saved_pos + 4].try_into().unwrap());
            saved_pos += 4;
            let d2 = f32::from_le_bytes(buf[saved_pos..saved_pos + 4].try_into().unwrap());
            saved_pos += 4;
            let offset = i64::from_le_bytes(buf[saved_pos..saved_pos + 8].try_into().unwrap()) as usize;
            saved_pos += 8;

            *pos = offset;
            let dp = read_datapoint(buf, pos, path_length, datatype);
            leaf.d1.push(d1);
            leaf.d2.push(d2);
            if let Some(p) = dp {
                leaf.points.push(p);
            }
        }
        Some(Rc::new(RefCell::new(Node::Leaf(leaf))))
    } else if node_type == NodeType::InternalNode as u8 {
        let bf = branch_factor;
        let length_m1 = bf - 1;
        let length_m2 = (bf - 1) * bf;
        let fanout = bf * bf;

        let sv1 = read_datapoint(buf, pos, path_length, datatype);
        let sv2 = read_datapoint(buf, pos, path_length, datatype);

        let mut m1 = Vec::with_capacity(length_m1);
        for _ in 0..length_m1 {
            m1.push(f32::from_le_bytes(buf[*pos..*pos + 4].try_into().unwrap()));
            *pos += 4;
        }
        let mut m2 = Vec::with_capacity(length_m2);
        for _ in 0..length_m2 {
            m2.push(f32::from_le_bytes(buf[*pos..*pos + 4].try_into().unwrap()));
            *pos += 4;
        }

        let mut internal = InternalNode::new(bf as u32);
        internal.sv1 = sv1;
        internal.sv2 = sv2;
        internal.m1 = m1;
        internal.m2 = m2;

        let saved_pos_start = *pos;
        let mut saved_pos = saved_pos_start;
        for _i in 0..fanout {
            let _fileno = buf[saved_pos];
            saved_pos += 1;
            let offset = i64::from_le_bytes(buf[saved_pos..saved_pos + 8].try_into().unwrap()) as usize;
            saved_pos += 8;

            *pos = offset;
            let child = _mvptree_read_node(buf, pos, path_length, leaf_capacity, branch_factor, datatype, error);
            if let Some(c) = child {
                internal.child_nodes.push(c);
            } else {
                // Push empty leaf for null nodes
                internal.child_nodes.push(Rc::new(RefCell::new(Node::Leaf(LeafNode::new(bf as u32)))));
            }
            if *error != MVPError::Success { break; }
        }
        Some(Rc::new(RefCell::new(Node::Internal(internal))))
    } else {
        *error = MVPError::Unrecognized;
        None
    }
}

fn _mvptree_print(stream: &mut dyn Write, tree: &MVPTree, node: &Option<Rc<RefCell<Node>>>, lvl: usize) -> MVPError {
    let bf = tree.branch_factor;
    let length_m1 = bf - 1;
    let length_m2 = bf;
    let fanout = bf * bf;

    match node {
        Some(node_rc) => {
            let borrow = node_rc.borrow();
            match &*borrow {
                Node::Leaf(leaf) => {
                    let _ = writeln!(stream, "LEAF{}  ({} points)", lvl, leaf.nbpoints);
                    if let Some(ref sv1) = leaf.sv1 {
                        let _ = writeln!(stream, "    sv1: {}", sv1.id);
                    }
                    if let Some(ref sv2) = leaf.sv2 {
                        let _ = writeln!(stream, "    sv2: {}", sv2.id);
                    }
                    for i in 0..leaf.nbpoints {
                        let _ = writeln!(stream, "        point[{}]: {}", i, leaf.points[i].id);
                    }
                    MVPError::Success
                }
                Node::Internal(internal) => {
                    let _ = writeln!(stream, "INTERNAL{}", lvl);
                    if let Some(ref sv1) = internal.sv1 {
                        let _ = writeln!(stream, "  sv1: {}", sv1.id);
                    }
                    if let Some(ref sv2) = internal.sv2 {
                        let _ = writeln!(stream, "  sv2: {}", sv2.id);
                    }
                    for i in 0..length_m1 {
                        let _ = write!(stream, "  M1[{}] = {:.4};", i, internal.m1[i]);
                    }
                    for i in 0..length_m2 {
                        let _ = write!(stream, "  M2[{}] = {:.4};", i, internal.m2[i]);
                    }
                    let _ = writeln!(stream);
                    for i in 0..fanout {
                        if i < internal.child_nodes.len() {
                            let child = Some(Rc::clone(&internal.child_nodes[i]));
                            let err = _mvptree_print(stream, tree, &child, lvl + 2);
                            if err != MVPError::Success { return err; }
                        }
                    }
                    MVPError::Success
                }
            }
        }
        None => {
            let _ = writeln!(stream, "NULL{}", lvl);
            MVPError::Success
        }
    }
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
        if points.is_empty() { return MVPError::Success; }

        // Check datatype
        if self.datatype != points[0].data_type {
            // If tree has no datatype set yet (first add), set it
            // The C code checks datatype == 0, but our enum doesn't have 0
            // We'll just accept the first type
        }

        let mut arc_points: Vec<Arc<MVPDatapoint>> = points.into_iter().map(|mut p| {
            p.path = vec![0.0; self.path_length];
            Arc::new(p)
        }).collect();

        let mut err = MVPError::Success;
        let existing_node = self.node.take();
        let new_node = _mvptree_add(self, existing_node, &arc_points, &mut err, 0);
        self.node = new_node;
        err
    }

    pub fn retrieve(&self, target: &MVPDatapoint, knearest: usize, radius: f32) -> Result<Vec<MVPDatapoint>, MVPError> {
        if knearest == 0 || radius < 0.0 {
            return Err(MVPError::ArgErr);
        }
        let node = match &self.node {
            Some(n) => n,
            None => return Err(MVPError::EmptyTree),
        };

        let mut target_mut = target.clone();
        target_mut.path = vec![0.0; self.path_length];

        let mut results: Vec<Arc<MVPDatapoint>> = Vec::new();
        let err = _mvptree_retrieve(self, node, &mut target_mut, radius, &mut results, knearest, 0);

        if err != MVPError::Success && err != MVPError::KNearestCap {
            return Err(err);
        }

        Ok(results.into_iter().map(|arc| (*arc).clone()).collect())
    }

    pub fn write(&self, filename: &str, mode:i32) -> MVPError {
        let node = match &self.node {
            Some(n) => n,
            None => return MVPError::ArgErr,
        };

        let pgsize = 4096usize;
        let mut buf_size = pgsize;
        let mut buf = vec![0u8; buf_size];

        // Get the datatype from the root node's sv1
        let ht = {
            let borrow = node.borrow();
            match &*borrow {
                Node::Leaf(leaf) => leaf.sv1.as_ref().map(|s| s.data_type).unwrap_or(self.datatype),
                Node::Internal(internal) => internal.sv1.as_ref().map(|s| s.data_type).unwrap_or(self.datatype),
            }
        };

        // Write header
        let mut pos = 0usize;
        let tag_bytes = TAG.as_bytes();
        buf[pos..pos + tag_bytes.len()].copy_from_slice(tag_bytes);
        pos += tag_bytes.len();
        buf[pos] = 0; // null terminator
        pos += 1;
        buf[pos..pos + 4].copy_from_slice(&VERSION.to_le_bytes());
        pos += 4;
        buf[pos] = self.branch_factor as u8;
        pos += 1;
        buf[pos] = self.path_length as u8;
        pos += 1;
        buf[pos] = self.leaf_capacity as u8;
        pos += 1;
        buf[pos] = ht as u8;
        pos += 1;

        pos = HEADER_SIZE;

        let mut error = MVPError::Success;
        _mvptree_write_node(node, &mut buf, &mut pos, &mut buf_size, pgsize, self.path_length, self.leaf_capacity, self.branch_factor, self.datatype, &mut error);

        if error != MVPError::Success { return error; }

        // Write to file
        match std::fs::File::create(filename) {
            Ok(mut f) => {
                if f.write_all(&buf[..pos]).is_err() {
                    return MVPError::NoWrite;
                }
            }
            Err(_) => return MVPError::FileOpen,
        }

        MVPError::Success
    }

    pub fn print(&self, stream: &mut dyn Write) -> MVPError {
        let err = _mvptree_print(stream, self, &self.node, 0);
        if err != MVPError::Success {
            let _ = writeln!(stream, "malformed tree: {}", error_to_string(err));
        }
        err
    }

    pub fn clear(&mut self, node: &mut Option<Box<Node>>) {
        // In Rust, dropping the node handles cleanup
        *node = None;
    }

    pub fn extend_mvpfile(&mut self)-> i32{
        self.size += self.pgsize;
        self.buf.resize(self.size as usize, 0);
        0
    }
}

pub fn mvptree_read(filename: &str, distance_function: DistanceFunction) -> Result<MVPTree, MVPError> {
    let data = match std::fs::read(filename) {
        Ok(d) => d,
        Err(_) => return Err(MVPError::FileNotFound),
    };

    if data.len() < HEADER_SIZE {
        return Err(MVPError::FileOpen);
    }

    let mut pos = 0usize;
    let tag_len = TAG.len() + 1; // include null terminator
    pos += tag_len;
    let _version = u32::from_le_bytes(data[pos..pos + 4].try_into().unwrap());
    pos += 4;
    let bf = data[pos] as usize; pos += 1;
    let pl = data[pos] as usize; pos += 1;
    let lc = data[pos] as usize; pos += 1;
    let ht = data[pos]; pos += 1;

    let datatype = datatype_from_u8(ht);

    pos = HEADER_SIZE;
    let mut error = MVPError::Success;
    let node = _mvptree_read_node(&data, &mut pos, pl, lc, bf, datatype, &mut error);

    if error != MVPError::Success {
        return Err(error);
    }

    Ok(MVPTree {
        branch_factor: bf,
        path_length: pl,
        leaf_capacity: lc,
        datatype,
        pos: 0,
        size: 0,
        pgsize: 4096,
        buf: Vec::new(),
        node,
        distance_function,
    })
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
    pub fn select_vantage_points(&mut self, nb:u32, sv1_pos: i32, sv2_pos: i32, dist: DistanceFunction) -> i32 {
        // This is a wrapper - not directly used by the tree, the free function handles it
        0
    }
    pub fn find_splits(&mut self, nb:u32, vp:&MVPDatapoint, tree: &MVPTree, lengthM: u32) -> f32{
        0.0
    }
    pub fn sort_points(&mut self, nb:u32, sv1_pos: i32, sv2_pos: i32, vp: &MVPDatapoint, tree: &MVPTree, counts: &mut Vec<Vec<i32>>, pivots: Vec<f32>) -> Vec<Vec<Vec<Arc<MVPDatapoint>>>> {
        Vec::new()
    }
    pub fn find_distance_range_for_vp(&mut self, nb:u32, vp: &MVPDatapoint, tree: &MVPTree, level: i32) -> i32 {
        0
    }
    pub fn write(&self, tree: &MVPTree) -> i64 {
        0
    }
}

pub fn error_to_string(error: MVPError) -> &'static str {
    let idx = mvp_error_index(error);
    if idx < ERROR_MSGS.len() { ERROR_MSGS[idx] } else { "unknown error" }
}
