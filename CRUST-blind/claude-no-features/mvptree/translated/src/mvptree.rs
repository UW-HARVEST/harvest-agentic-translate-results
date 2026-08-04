use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
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

impl MVPDataType {
    fn byte_width(&self) -> usize {
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
        let bf_u = bf as usize;
        let length_m1 = if bf_u == 0 { 0 } else { bf_u - 1 };
        let length_m2 = bf_u * length_m1;
        InternalNode {
            node_type: NodeType::InternalNode,
            sv1: None,
            sv2: None,
            m1: vec![0.0; length_m1],
            m2: vec![0.0; length_m2],
            child_nodes: Vec::with_capacity(bf_u * bf_u),
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

// ----- Helpers -----

fn is_nan_f32(x: f32) -> bool {
    x.is_nan()
}

fn empty_leaf_rc(leaf_cap: usize) -> Rc<RefCell<Node>> {
    Rc::new(RefCell::new(Node::Leaf(LeafNode::new(leaf_cap as u32))))
}

fn is_null_node(n: &Rc<RefCell<Node>>) -> bool {
    match &*n.borrow() {
        Node::Leaf(l) => l.sv1.is_none() && l.sv2.is_none() && l.nbpoints == 0,
        Node::Internal(_) => false,
    }
}

// Select two points in `points` at maximum mutual distance.
fn select_vantage_points(
    points: &[Arc<MVPDatapoint>],
    dist: DistanceFunction,
) -> Result<(i32, i32), i32> {
    if points.is_empty() {
        return Err(-1);
    }
    let nb = points.len();
    let mut sv1_pos: i32 = if nb >= 1 { 0 } else { -1 };
    let mut sv2_pos: i32 = -1;
    let mut max_dist = 0.0f32;
    for i in 0..nb {
        for j in (i + 1)..nb {
            let d = dist(&points[i], &points[j]);
            if is_nan_f32(d) || d < 0.0 {
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

// Compute pivots in `m_out` (length = length_m).
fn find_splits_arr(
    points: &[Arc<MVPDatapoint>],
    vp: &MVPDatapoint,
    distfunc: DistanceFunction,
    m_out: &mut [f32],
) -> Result<(), i32> {
    let nb = points.len();
    let length_m = m_out.len();
    if nb == 0 || length_m == 0 {
        return Err(-1);
    }
    let mut dist: Vec<f32> = Vec::with_capacity(nb);
    for p in points {
        let d = distfunc(p, vp);
        if is_nan_f32(d) || d < 0.0 {
            return Err(-2);
        }
        dist.push(d);
    }
    // selection sort like in C (O(n^2))
    for i in 0..(nb.saturating_sub(1)) {
        let mut min_pos = i;
        for j in (i + 1)..nb {
            if dist[j] < dist[min_pos] {
                min_pos = j;
            }
        }
        if min_pos != i {
            dist.swap(i, min_pos);
        }
    }
    for i in 0..length_m {
        let mut index = (i + 1) * nb / (length_m + 1);
        if index == 0 {
            index = 0;
        }
        if index >= nb {
            index = nb - 1;
        }
        m_out[i] = dist[index];
    }
    Ok(())
}

// Sort `points` into bf bins by distance to `vp`, skipping indices sv1_pos and sv2_pos.
fn sort_points_into_bins(
    points: &[Arc<MVPDatapoint>],
    sv1_pos: i32,
    sv2_pos: i32,
    vp: &MVPDatapoint,
    distfunc: DistanceFunction,
    bf: usize,
    pivots: &[f32],
) -> Option<(Vec<Vec<Arc<MVPDatapoint>>>, Vec<usize>)> {
    if points.is_empty() {
        return None;
    }
    let length_m1 = bf.saturating_sub(1);
    let mut bins: Vec<Vec<Arc<MVPDatapoint>>> = (0..bf).map(|_| Vec::new()).collect();
    let mut counts: Vec<usize> = vec![0; bf];
    for (i, p) in points.iter().enumerate() {
        let ii = i as i32;
        if ii == sv1_pos || ii == sv2_pos {
            continue;
        }
        let d = distfunc(vp, p);
        if is_nan_f32(d) || d < 0.0 {
            return None;
        }
        let mut placed = false;
        for k in 0..length_m1 {
            if d <= pivots[k] {
                bins[k].push(p.clone());
                counts[k] += 1;
                placed = true;
                break;
            }
        }
        if !placed {
            // d > pivots[length_m1-1]   (or length_m1 == 0)
            if length_m1 == 0 {
                bins[0].push(p.clone());
                counts[0] += 1;
            } else if d > pivots[length_m1 - 1] {
                bins[length_m1].push(p.clone());
                counts[length_m1] += 1;
            }
        }
    }
    Some((bins, counts))
}

// Calculate distances for all `points` from given vantage point `vp`,
// and assign that distance into each point's path[lvl].
fn find_distance_range_for_vp(
    points: &mut [Arc<MVPDatapoint>],
    vp: &MVPDatapoint,
    distfunc: DistanceFunction,
    lvl: usize,
    path_length: usize,
) -> Result<(), i32> {
    if points.is_empty() {
        return Err(-1);
    }
    for p in points.iter_mut() {
        let d = distfunc(vp, p);
        if is_nan_f32(d) || d < 0.0 {
            return Err(-2);
        }
        if lvl < path_length {
            // We need to mutate the underlying point's path. Use Arc::make_mut.
            let pmut = Arc::make_mut(p);
            if pmut.path.len() < path_length {
                pmut.path.resize(path_length, 0.0);
            }
            pmut.path[lvl] = d;
        }
    }
    Ok(())
}

// ----- Recursive add -----

fn _mvptree_add(
    tree_branchfactor: usize,
    tree_leafcap: usize,
    tree_pathlength: usize,
    distance_function: DistanceFunction,
    existing: Option<Rc<RefCell<Node>>>,
    mut points: Vec<Arc<MVPDatapoint>>,
    error: &mut MVPError,
    lvl: usize,
) -> Option<Rc<RefCell<Node>>> {
    if points.is_empty() {
        return existing;
    }
    let bf = tree_branchfactor;
    let length_m1 = if bf == 0 { 0 } else { bf - 1 };
    let dist_fnc = distance_function;

    if existing.is_none() || matches!(&existing, Some(n) if is_null_node(n)) {
        // create new node
        if points.len() <= tree_leafcap + 2 {
            // create leaf node
            let mut leaf = LeafNode::new(tree_leafcap as u32);
            let _nbpoints = points.len();
            let (sv1_pos, sv2_pos) = match select_vantage_points(&points, dist_fnc) {
                Ok(t) => t,
                Err(_) => {
                    *error = MVPError::VpNoSelect;
                    return None;
                }
            };
            let sv1 = if sv1_pos >= 0 {
                Some(points[sv1_pos as usize].clone())
            } else {
                None
            };
            let sv2 = if sv2_pos >= 0 {
                Some(points[sv2_pos as usize].clone())
            } else {
                None
            };
            leaf.sv1 = sv1.clone();
            leaf.sv2 = sv2.clone();

            if let Some(sv1_dp) = sv1.as_ref() {
                let sv1_clone = (**sv1_dp).clone();
                if find_distance_range_for_vp(&mut points, &sv1_clone, dist_fnc, lvl, tree_pathlength).is_err() {
                    *error = MVPError::NoSv1Range;
                    return None;
                }
            }

            if let Some(sv2_dp) = sv2.as_ref() {
                let sv2_clone = (**sv2_dp).clone();
                if find_distance_range_for_vp(&mut points, &sv2_clone, dist_fnc, lvl + 1, tree_pathlength).is_err() {
                    *error = MVPError::NoSv2Range;
                    return None;
                }
            }

            let sv1_ref = sv1.as_ref().map(|s| (**s).clone());
            let sv2_ref = sv2.as_ref().map(|s| (**s).clone());

            // ensure d1/d2 are large enough
            if leaf.d1.len() < tree_leafcap {
                leaf.d1.resize(tree_leafcap, 0.0);
            }
            if leaf.d2.len() < tree_leafcap {
                leaf.d2.resize(tree_leafcap, 0.0);
            }

            let mut count = 0usize;
            for (i, p) in points.iter().enumerate() {
                let ii = i as i32;
                if ii == sv1_pos || ii == sv2_pos {
                    continue;
                }
                let d1 = if let Some(s) = sv1_ref.as_ref() {
                    dist_fnc(p, s)
                } else {
                    0.0
                };
                let d2 = if let Some(s) = sv2_ref.as_ref() {
                    dist_fnc(p, s)
                } else {
                    0.0
                };
                if count < leaf.d1.len() {
                    leaf.d1[count] = d1;
                }
                if count < leaf.d2.len() {
                    leaf.d2[count] = d2;
                }
                leaf.points.push(p.clone());
                count += 1;
            }
            leaf.nbpoints = count;

            // Update sv1/sv2 to point to mutated arcs in points (their paths were updated)
            // Note: we mutated entries via Arc::make_mut, so points[i] now is the updated Arc.
            if sv1_pos >= 0 {
                leaf.sv1 = Some(points[sv1_pos as usize].clone());
            }
            if sv2_pos >= 0 {
                leaf.sv2 = Some(points[sv2_pos as usize].clone());
            }

            return Some(Rc::new(RefCell::new(Node::Leaf(leaf))));
        } else {
            // create internal node
            let mut internal = InternalNode::new(bf as u32);
            let (sv1_pos, sv2_pos) = match select_vantage_points(&points, dist_fnc) {
                Ok(t) => t,
                Err(_) => {
                    *error = MVPError::VpNoSelect;
                    return None;
                }
            };
            if sv1_pos < 0 || sv2_pos < 0 {
                *error = MVPError::VpNoSelect;
                return None;
            }
            let sv1_arc = points[sv1_pos as usize].clone();
            let sv2_arc = points[sv2_pos as usize].clone();

            // distance range for sv1 at lvl
            let sv1_clone = (*sv1_arc).clone();
            if find_distance_range_for_vp(&mut points, &sv1_clone, dist_fnc, lvl, tree_pathlength).is_err() {
                *error = MVPError::NoSv1Range;
                return None;
            }

            // find_splits for sv1
            if length_m1 > 0 {
                let mut m1_buf = vec![0.0f32; length_m1];
                if find_splits_arr(&points, &sv1_clone, dist_fnc, &mut m1_buf).is_err() {
                    *error = MVPError::NoSplits;
                    return None;
                }
                internal.m1.copy_from_slice(&m1_buf);
            }

            // sort points to bf bins by sv1 distance
            let bins_opt = sort_points_into_bins(
                &points,
                sv1_pos,
                sv2_pos,
                &sv1_clone,
                dist_fnc,
                bf,
                &internal.m1,
            );
            let (mut bins, bin_lengths) = match bins_opt {
                Some(b) => b,
                None => {
                    *error = MVPError::NoSort;
                    return None;
                }
            };

            // ensure m2 vec is the right size
            if internal.m2.len() < bf * length_m1 {
                internal.m2.resize(bf * length_m1, 0.0);
            }

            // initialize child_nodes with bf*bf empty leaves
            internal.child_nodes.clear();
            for _ in 0..(bf * bf) {
                internal.child_nodes.push(empty_leaf_rc(tree_leafcap));
            }

            let sv2_clone = (*sv2_arc).clone();
            for i in 0..bf {
                if bin_lengths[i] == 0 {
                    continue;
                }
                if find_distance_range_for_vp(&mut bins[i], &sv2_clone, dist_fnc, lvl + 1, tree_pathlength).is_err() {
                    *error = MVPError::NoSv2Range;
                    return None;
                }

                if length_m1 > 0 {
                    let mut m2_buf = vec![0.0f32; length_m1];
                    if find_splits_arr(&bins[i], &sv2_clone, dist_fnc, &mut m2_buf).is_err() {
                        *error = MVPError::NoSplits;
                        return None;
                    }
                    for k in 0..length_m1 {
                        internal.m2[i * length_m1 + k] = m2_buf[k];
                    }
                }

                let m2_slice: Vec<f32> = if length_m1 > 0 {
                    internal.m2[i * length_m1..i * length_m1 + length_m1].to_vec()
                } else {
                    vec![]
                };

                let bins2_opt = sort_points_into_bins(
                    &bins[i],
                    -1,
                    -1,
                    &sv2_clone,
                    dist_fnc,
                    bf,
                    &m2_slice,
                );
                let (bins2, _bin2_lengths) = match bins2_opt {
                    Some(b) => b,
                    None => {
                        *error = MVPError::NoSort;
                        return None;
                    }
                };

                for j in 0..bf {
                    if bins2[j].is_empty() {
                        continue;
                    }
                    let child = _mvptree_add(
                        tree_branchfactor,
                        tree_leafcap,
                        tree_pathlength,
                        distance_function,
                        None,
                        bins2[j].clone(),
                        error,
                        lvl + 2,
                    );
                    if let Some(c) = child {
                        internal.child_nodes[i * bf + j] = c;
                    }
                    if *error != MVPError::Success {
                        return None;
                    }
                }
            }

            // update sv1/sv2 from points (their paths were updated by find_distance_range_for_vp)
            internal.sv1 = Some(points[sv1_pos as usize].clone());
            internal.sv2 = Some(points[sv2_pos as usize].clone());

            return Some(Rc::new(RefCell::new(Node::Internal(internal))));
        }
    } else {
        // node exists - we modify it
        let node_rc = existing.unwrap();
        let is_leaf = matches!(&*node_rc.borrow(), Node::Leaf(_));
        if is_leaf {
            // Determine if room
            let (current_nb, _has_sv1, has_sv2) = {
                let nb = node_rc.borrow();
                if let Node::Leaf(l) = &*nb {
                    (l.nbpoints, l.sv1.is_some(), l.sv2.is_some())
                } else {
                    (0, false, false)
                }
            };
            if current_nb + points.len() <= tree_leafcap {
                // plenty of room
                let sv1_clone = {
                    let nb = node_rc.borrow();
                    if let Node::Leaf(l) = &*nb {
                        l.sv1.as_ref().map(|s| (**s).clone())
                    } else {
                        None
                    }
                };
                if let Some(sv1) = sv1_clone.as_ref() {
                    if find_distance_range_for_vp(&mut points, sv1, dist_fnc, lvl, tree_pathlength).is_err() {
                        *error = MVPError::NoSv1Range;
                        return Some(node_rc);
                    }
                }
                let mut start_pos = 0usize;
                if !has_sv2 && !points.is_empty() {
                    // sv2 is set from points[0]
                    let mut nb_mut = node_rc.borrow_mut();
                    if let Node::Leaf(l) = &mut *nb_mut {
                        l.sv2 = Some(points[0].clone());
                    }
                    start_pos = 1;
                }
                let sv2_clone = {
                    let nb = node_rc.borrow();
                    if let Node::Leaf(l) = &*nb {
                        l.sv2.as_ref().map(|s| (**s).clone())
                    } else {
                        None
                    }
                };
                if let Some(sv2) = sv2_clone.as_ref() {
                    if find_distance_range_for_vp(&mut points, sv2, dist_fnc, lvl + 1, tree_pathlength).is_err() {
                        *error = MVPError::NoSv2Range;
                        return Some(node_rc);
                    }
                }

                {
                    let mut nb_mut = node_rc.borrow_mut();
                    if let Node::Leaf(l) = &mut *nb_mut {
                        let mut count = l.nbpoints;
                        for pos in start_pos..points.len() {
                            let p = &points[pos];
                            let d1 = if let Some(s) = sv1_clone.as_ref() {
                                dist_fnc(p, s)
                            } else {
                                0.0
                            };
                            let d2 = if let Some(s) = sv2_clone.as_ref() {
                                dist_fnc(p, s)
                            } else {
                                0.0
                            };
                            if l.d1.len() <= count {
                                l.d1.resize(count + 1, 0.0);
                            }
                            if l.d2.len() <= count {
                                l.d2.resize(count + 1, 0.0);
                            }
                            l.d1[count] = d1;
                            l.d2[count] = d2;
                            l.points.push(p.clone());
                            count += 1;
                        }
                        l.nbpoints = count;
                    }
                }
                return Some(node_rc);
            } else {
                // not enough room - merge points and re-build
                let mut tmp_pts: Vec<Arc<MVPDatapoint>> = Vec::new();
                {
                    let nb = node_rc.borrow();
                    if let Node::Leaf(l) = &*nb {
                        if let Some(s) = l.sv1.as_ref() {
                            tmp_pts.push(s.clone());
                        }
                        if let Some(s) = l.sv2.as_ref() {
                            tmp_pts.push(s.clone());
                        }
                        for i in 0..l.nbpoints {
                            tmp_pts.push(l.points[i].clone());
                        }
                    }
                }
                for p in points.into_iter() {
                    tmp_pts.push(p);
                }
                drop(node_rc);
                return _mvptree_add(
                    tree_branchfactor,
                    tree_leafcap,
                    tree_pathlength,
                    distance_function,
                    None,
                    tmp_pts,
                    error,
                    lvl,
                );
            }
        } else {
            // internal node - recurse
            let (sv1_clone, sv2_clone, m1_vec, m2_vec, child_count) = {
                let nb = node_rc.borrow();
                if let Node::Internal(internal) = &*nb {
                    (
                        internal.sv1.as_ref().map(|s| (**s).clone()),
                        internal.sv2.as_ref().map(|s| (**s).clone()),
                        internal.m1.clone(),
                        internal.m2.clone(),
                        internal.child_nodes.len(),
                    )
                } else {
                    (None, None, vec![], vec![], 0)
                }
            };
            let sv1 = match sv1_clone {
                Some(s) => s,
                None => return Some(node_rc),
            };
            let sv2 = match sv2_clone {
                Some(s) => s,
                None => return Some(node_rc),
            };
            if find_distance_range_for_vp(&mut points, &sv1, dist_fnc, lvl, tree_pathlength).is_err() {
                *error = MVPError::NoSv1Range;
                return Some(node_rc);
            }

            let bins_opt = sort_points_into_bins(&points, -1, -1, &sv1, dist_fnc, bf, &m1_vec);
            let (mut bins, bin_lengths) = match bins_opt {
                Some(b) => b,
                None => {
                    *error = MVPError::NoSort;
                    return Some(node_rc);
                }
            };

            for i in 0..bf {
                if bin_lengths[i] == 0 {
                    continue;
                }
                if find_distance_range_for_vp(&mut bins[i], &sv2, dist_fnc, lvl + 1, tree_pathlength).is_err() {
                    *error = MVPError::NoSv2Range;
                    return Some(node_rc);
                }

                let m2_slice: Vec<f32> = if length_m1 > 0 {
                    m2_vec[i * length_m1..i * length_m1 + length_m1].to_vec()
                } else {
                    vec![]
                };

                let bins2_opt = sort_points_into_bins(&bins[i], -1, -1, &sv2, dist_fnc, bf, &m2_slice);
                let (bins2, _l2) = match bins2_opt {
                    Some(b) => b,
                    None => {
                        *error = MVPError::NoSort;
                        return Some(node_rc);
                    }
                };
                for j in 0..bf {
                    if bins2[j].is_empty() {
                        continue;
                    }
                    let idx = i * bf + j;
                    if idx >= child_count {
                        continue;
                    }
                    let existing_child = {
                        let nb = node_rc.borrow();
                        if let Node::Internal(internal) = &*nb {
                            let c = internal.child_nodes[idx].clone();
                            if is_null_node(&c) {
                                None
                            } else {
                                Some(c)
                            }
                        } else {
                            None
                        }
                    };
                    let new_child = _mvptree_add(
                        tree_branchfactor,
                        tree_leafcap,
                        tree_pathlength,
                        distance_function,
                        existing_child,
                        bins2[j].clone(),
                        error,
                        lvl + 2,
                    );
                    if let Some(c) = new_child {
                        let mut nb_mut = node_rc.borrow_mut();
                        if let Node::Internal(internal) = &mut *nb_mut {
                            internal.child_nodes[idx] = c;
                        }
                    }
                    if *error != MVPError::Success {
                        return Some(node_rc);
                    }
                }
            }
            return Some(node_rc);
        }
    }
}

// ----- Retrieve recursive -----

fn _mvptree_retrieve(
    tree_branchfactor: usize,
    tree_path_length: usize,
    tree_k: usize,
    distance_function: DistanceFunction,
    node_rc: &Rc<RefCell<Node>>,
    target: &MVPDatapoint,
    target_path: &mut Vec<f32>,
    radius: f32,
    results: &mut Vec<Arc<MVPDatapoint>>,
    lvl: usize,
) -> MVPError {
    if is_null_node(node_rc) {
        return MVPError::Success;
    }
    let bf = tree_branchfactor;
    let length_m1 = bf.saturating_sub(1);
    let distance = distance_function;
    let nb = node_rc.borrow();
    match &*nb {
        Node::Leaf(leaf) => {
            let sv1_arc = match leaf.sv1.as_ref() {
                Some(s) => s.clone(),
                None => return MVPError::Success,
            };
            let d1 = distance(target, &sv1_arc);
            if is_nan_f32(d1) || d1 < 0.0 {
                return MVPError::BadDistVal;
            }
            if lvl < tree_path_length {
                if target_path.len() <= lvl {
                    target_path.resize(lvl + 1, 0.0);
                }
                target_path[lvl] = d1;
            }
            if d1 <= radius {
                results.push(sv1_arc.clone());
                if results.len() >= tree_k {
                    return MVPError::KNearestCap;
                }
            }
            if let Some(sv2_arc) = leaf.sv2.as_ref() {
                let d2 = distance(target, sv2_arc);
                if is_nan_f32(d2) || d2 < 0.0 {
                    return MVPError::BadDistVal;
                }
                if d2 <= radius {
                    results.push(sv2_arc.clone());
                    if results.len() >= tree_k {
                        return MVPError::KNearestCap;
                    }
                }
                if lvl + 1 < tree_path_length {
                    if target_path.len() <= lvl + 1 {
                        target_path.resize(lvl + 2, 0.0);
                    }
                    target_path[lvl + 1] = d2;
                }
                for i in 0..leaf.nbpoints {
                    let pd1 = if i < leaf.d1.len() { leaf.d1[i] } else { 0.0 };
                    let pd2 = if i < leaf.d2.len() { leaf.d2[i] } else { 0.0 };
                    if d1 - radius <= pd1 && d1 + radius >= pd1 {
                        if d2 - radius <= pd2 && d2 + radius >= pd2 {
                            let endpath = if lvl + 1 < tree_path_length {
                                lvl + 1
                            } else {
                                tree_path_length
                            };
                            let mut skip = false;
                            for j in 0..endpath {
                                let tp = if j < target_path.len() { target_path[j] } else { 0.0 };
                                let pp = if j < leaf.points[i].path.len() {
                                    leaf.points[i].path[j]
                                } else {
                                    0.0
                                };
                                if tp - radius <= pp && tp + radius >= pp {
                                    continue;
                                } else {
                                    skip = true;
                                    break;
                                }
                            }
                            if !skip {
                                let d = distance(target, &leaf.points[i]);
                                if is_nan_f32(d) || d < 0.0 {
                                    return MVPError::BadDistVal;
                                }
                                if d <= radius {
                                    results.push(leaf.points[i].clone());
                                    if results.len() >= tree_k {
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
        Node::Internal(internal) => {
            let sv1_arc = match internal.sv1.as_ref() {
                Some(s) => s.clone(),
                None => return MVPError::Success,
            };
            let sv2_arc = match internal.sv2.as_ref() {
                Some(s) => s.clone(),
                None => return MVPError::Success,
            };
            let d1 = distance(target, &sv1_arc);
            if is_nan_f32(d1) || d1 < 0.0 {
                return MVPError::BadDistVal;
            }
            if d1 <= radius {
                results.push(sv1_arc.clone());
                if results.len() >= tree_k {
                    return MVPError::KNearestCap;
                }
            }
            if lvl < tree_path_length {
                if target_path.len() <= lvl {
                    target_path.resize(lvl + 1, 0.0);
                }
                target_path[lvl] = d1;
            }
            let d2 = distance(target, &sv2_arc);
            if is_nan_f32(d2) || d2 < 0.0 {
                return MVPError::BadDistVal;
            }
            if d2 <= radius {
                results.push(sv2_arc.clone());
                if results.len() >= tree_k {
                    return MVPError::KNearestCap;
                }
            }
            if lvl + 1 < tree_path_length {
                if target_path.len() <= lvl + 1 {
                    target_path.resize(lvl + 2, 0.0);
                }
                target_path[lvl + 1] = d2;
            }

            // copy structures we need
            let m1 = internal.m1.clone();
            let m2 = internal.m2.clone();
            let child_nodes: Vec<Rc<RefCell<Node>>> = internal.child_nodes.clone();
            drop(nb);

            for i in 0..length_m1 {
                if d1 - radius <= m1[i] {
                    for j in 0..length_m1 {
                        if d2 - radius <= m2[i * length_m1 + j] {
                            let idx = i * bf + j;
                            if idx < child_nodes.len() {
                                let err = _mvptree_retrieve(
                                    tree_branchfactor,
                                    tree_path_length,
                                    tree_k,
                                    distance_function,
                                    &child_nodes[idx],
                                    target,
                                    target_path,
                                    radius,
                                    results,
                                    lvl + 2,
                                );
                                if err != MVPError::Success {
                                    return err;
                                }
                            }
                        }
                    }
                    if length_m1 > 0 && d2 + radius >= m2[i * length_m1 + length_m1 - 1] {
                        let idx = i * bf + length_m1;
                        if idx < child_nodes.len() {
                            let err = _mvptree_retrieve(
                                tree_branchfactor,
                                tree_path_length,
                                tree_k,
                                distance_function,
                                &child_nodes[idx],
                                target,
                                target_path,
                                radius,
                                results,
                                lvl + 2,
                            );
                            if err != MVPError::Success {
                                return err;
                            }
                        }
                    }
                }
            }
            if length_m1 > 0 && d1 + radius >= m1[length_m1 - 1] {
                for j in 0..length_m1 {
                    if d2 - radius <= m2[length_m1 * length_m1 + j] {
                        let idx = bf * length_m1 + j;
                        if idx < child_nodes.len() {
                            let err = _mvptree_retrieve(
                                tree_branchfactor,
                                tree_path_length,
                                tree_k,
                                distance_function,
                                &child_nodes[idx],
                                target,
                                target_path,
                                radius,
                                results,
                                lvl + 2,
                            );
                            if err != MVPError::Success {
                                return err;
                            }
                        }
                    }
                }
                if length_m1 > 0
                    && d2 + radius >= m2[length_m1 * length_m1 + length_m1 - 1]
                {
                    let idx = bf * length_m1 + length_m1;
                    if idx < child_nodes.len() {
                        let err = _mvptree_retrieve(
                            tree_branchfactor,
                            tree_path_length,
                            tree_k,
                            distance_function,
                            &child_nodes[idx],
                            target,
                            target_path,
                            radius,
                            results,
                            lvl + 2,
                        );
                        if err != MVPError::Success {
                            return err;
                        }
                    }
                }
            }
            MVPError::Success
        }
    }
}

// ----- Print recursive -----

fn _mvptree_print(
    stream: &mut dyn Write,
    tree_branchfactor: usize,
    node_rc: &Rc<RefCell<Node>>,
    lvl: usize,
) -> MVPError {
    let bf = tree_branchfactor;
    let length_m1 = bf.saturating_sub(1);
    let length_m2 = bf;
    let fanout = bf * bf;
    let nb = node_rc.borrow();
    match &*nb {
        Node::Leaf(leaf) => {
            if leaf.sv1.is_none() && leaf.sv2.is_none() && leaf.nbpoints == 0 {
                let _ = writeln!(stream, "NULL{}", lvl);
                return MVPError::Success;
            }
            let _ = writeln!(stream, "LEAF{}  ({} points)", lvl, leaf.nbpoints);
            if let Some(s) = leaf.sv1.as_ref() {
                let _ = writeln!(stream, "    sv1: {}", s.id);
            }
            if let Some(s) = leaf.sv2.as_ref() {
                let _ = writeln!(stream, "    sv2: {}", s.id);
            }
            for i in 0..leaf.nbpoints {
                let _ = writeln!(stream, "        point[{}]: {}", i, leaf.points[i].id);
            }
            MVPError::Success
        }
        Node::Internal(internal) => {
            let _ = writeln!(stream, "INTERNAL{}", lvl);
            if let Some(s) = internal.sv1.as_ref() {
                let _ = writeln!(stream, "  sv1: {}", s.id);
            }
            if let Some(s) = internal.sv2.as_ref() {
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
                    let err = _mvptree_print(stream, bf, &internal.child_nodes[i], lvl + 2);
                    if err != MVPError::Success {
                        return err;
                    }
                }
            }
            MVPError::Success
        }
    }
}

// ----- File serialization helpers -----

fn write_buf(buf: &mut Vec<u8>, pos: &mut usize, src: &[u8]) {
    if *pos + src.len() > buf.len() {
        buf.resize(*pos + src.len(), 0);
    }
    buf[*pos..*pos + src.len()].copy_from_slice(src);
    *pos += src.len();
}

fn write_datapoint_to_buf(
    dp: Option<&MVPDatapoint>,
    buf: &mut Vec<u8>,
    pos: &mut usize,
    path_length: usize,
) -> i64 {
    let start = *pos as i64;
    let active: u8 = if dp.is_some() { 1 } else { 0 };
    if dp.is_none() {
        write_buf(buf, pos, &[active]);
        let zero32: u32 = 0;
        write_buf(buf, pos, &zero32.to_le_bytes());
        return start;
    }
    let dp = dp.unwrap();
    let id_bytes = dp.id.as_bytes();
    let idlen: u8 = id_bytes.len().min(255) as u8;
    let datalength: u32 = dp.datalen as u32;
    let type_width = dp.data_type.byte_width();
    let bytelength: u32 = (1 + idlen as u32) + 4 + (datalength * type_width as u32) + (path_length as u32) * 4;

    write_buf(buf, pos, &[active]);
    write_buf(buf, pos, &bytelength.to_le_bytes());
    write_buf(buf, pos, &[idlen]);
    write_buf(buf, pos, &id_bytes[..idlen as usize]);
    write_buf(buf, pos, &datalength.to_le_bytes());
    let data_size = datalength as usize * type_width;
    if data_size > 0 {
        let data_slice = if dp.data.len() >= data_size {
            &dp.data[..data_size]
        } else {
            &dp.data[..]
        };
        if data_slice.len() < data_size {
            // pad with zeros if needed
            write_buf(buf, pos, data_slice);
            let pad = vec![0u8; data_size - data_slice.len()];
            write_buf(buf, pos, &pad);
        } else {
            write_buf(buf, pos, data_slice);
        }
    }
    // path: path_length floats
    for i in 0..path_length {
        let v = if i < dp.path.len() { dp.path[i] } else { 0.0f32 };
        write_buf(buf, pos, &v.to_le_bytes());
    }
    start
}

fn _mvptree_write_node(
    buf: &mut Vec<u8>,
    pos: &mut usize,
    branchfactor: usize,
    leafcap: usize,
    path_length: usize,
    node_rc: &Rc<RefCell<Node>>,
    error: &mut MVPError,
    lvl: usize,
) -> i64 {
    let start_pos = *pos as i64;
    let nb = node_rc.borrow();
    match &*nb {
        Node::Leaf(leaf) => {
            // null node check
            if leaf.sv1.is_none() && leaf.sv2.is_none() && leaf.nbpoints == 0 {
                return 0;
            }
            let node_type: u8 = NodeType::LeafNode as u8;
            let nbpoints: u32 = leaf.nbpoints as u32;
            write_buf(buf, pos, &[node_type]);
            write_datapoint_to_buf(leaf.sv1.as_deref(), buf, pos, path_length);
            write_datapoint_to_buf(leaf.sv2.as_deref(), buf, pos, path_length);
            write_buf(buf, pos, &nbpoints.to_le_bytes());

            let mut saved_pos = *pos;
            let entry_size = 2 * 4 + 8; // 2 floats + off_t (8 bytes)
            *pos += leafcap * entry_size;
            // ensure buf is sized
            if *pos > buf.len() {
                buf.resize(*pos, 0);
            }
            for i in 0..leaf.nbpoints {
                let d1v = if i < leaf.d1.len() { leaf.d1[i] } else { 0.0f32 };
                let d2v = if i < leaf.d2.len() { leaf.d2[i] } else { 0.0f32 };
                let d1_bytes = d1v.to_le_bytes();
                let d2_bytes = d2v.to_le_bytes();
                if saved_pos + 4 > buf.len() {
                    buf.resize(saved_pos + 4, 0);
                }
                buf[saved_pos..saved_pos + 4].copy_from_slice(&d1_bytes);
                saved_pos += 4;
                if saved_pos + 4 > buf.len() {
                    buf.resize(saved_pos + 4, 0);
                }
                buf[saved_pos..saved_pos + 4].copy_from_slice(&d2_bytes);
                saved_pos += 4;

                let offset = write_datapoint_to_buf(Some(&leaf.points[i]), buf, pos, path_length);
                let off_bytes = (offset as i64).to_le_bytes();
                if saved_pos + 8 > buf.len() {
                    buf.resize(saved_pos + 8, 0);
                }
                buf[saved_pos..saved_pos + 8].copy_from_slice(&off_bytes);
                saved_pos += 8;
            }
        }
        Node::Internal(internal) => {
            let bf = branchfactor;
            let length_m1 = bf.saturating_sub(1);
            let length_m2 = bf * length_m1;
            let fanout = bf * bf;
            let node_type: u8 = NodeType::InternalNode as u8;

            write_buf(buf, pos, &[node_type]);
            write_datapoint_to_buf(internal.sv1.as_deref(), buf, pos, path_length);
            write_datapoint_to_buf(internal.sv2.as_deref(), buf, pos, path_length);
            for i in 0..length_m1 {
                let v = if i < internal.m1.len() { internal.m1[i] } else { 0.0 };
                write_buf(buf, pos, &v.to_le_bytes());
            }
            for i in 0..length_m2 {
                let v = if i < internal.m2.len() { internal.m2[i] } else { 0.0 };
                write_buf(buf, pos, &v.to_le_bytes());
            }

            let mut saved_pos = *pos;
            let entry_size = 1 + 8;
            *pos += fanout * entry_size;
            if *pos > buf.len() {
                buf.resize(*pos, 0);
            }
            // collect children
            let children: Vec<Rc<RefCell<Node>>> = internal.child_nodes.clone();
            drop(nb);
            for i in 0..fanout {
                let offset = if i < children.len() {
                    if is_null_node(&children[i]) {
                        0
                    } else {
                        _mvptree_write_node(
                            buf, pos, branchfactor, leafcap, path_length,
                            &children[i], error, lvl + 2,
                        )
                    }
                } else {
                    0
                };
                let fileno: u8 = 0;
                if saved_pos + 1 > buf.len() {
                    buf.resize(saved_pos + 1, 0);
                }
                buf[saved_pos] = fileno;
                saved_pos += 1;
                let off_bytes = offset.to_le_bytes();
                if saved_pos + 8 > buf.len() {
                    buf.resize(saved_pos + 8, 0);
                }
                buf[saved_pos..saved_pos + 8].copy_from_slice(&off_bytes);
                saved_pos += 8;
            }
        }
    }
    start_pos
}

// ----- File reading helpers -----

fn read_datapoint_from_buf(
    buf: &[u8],
    pos: &mut usize,
    datatype: MVPDataType,
    path_length: usize,
) -> Option<MVPDatapoint> {
    if *pos + 5 > buf.len() {
        return None;
    }
    let active = buf[*pos];
    *pos += 1;
    let mut bl_bytes = [0u8; 4];
    bl_bytes.copy_from_slice(&buf[*pos..*pos + 4]);
    let bytelength = u32::from_le_bytes(bl_bytes);
    *pos += 4;

    if active == 0 && bytelength == 0 {
        return None;
    }

    if *pos >= buf.len() {
        return None;
    }
    let idlen = buf[*pos] as usize;
    *pos += 1;
    if *pos + idlen > buf.len() {
        return None;
    }
    let id = String::from_utf8_lossy(&buf[*pos..*pos + idlen]).to_string();
    *pos += idlen;
    if *pos + 4 > buf.len() {
        return None;
    }
    let mut dl_bytes = [0u8; 4];
    dl_bytes.copy_from_slice(&buf[*pos..*pos + 4]);
    let datalength = u32::from_le_bytes(dl_bytes) as usize;
    *pos += 4;
    let data_size = datalength * datatype.byte_width();
    if *pos + data_size > buf.len() {
        return None;
    }
    let data = buf[*pos..*pos + data_size].to_vec();
    *pos += data_size;
    let path_size = path_length * 4;
    if *pos + path_size > buf.len() {
        return None;
    }
    let mut path = Vec::with_capacity(path_length);
    for i in 0..path_length {
        let mut fb = [0u8; 4];
        fb.copy_from_slice(&buf[*pos + i * 4..*pos + i * 4 + 4]);
        path.push(f32::from_le_bytes(fb));
    }
    *pos += path_size;
    Some(MVPDatapoint {
        id,
        data,
        path,
        datalen: datalength,
        data_type: datatype,
    })
}

fn _mvptree_read_node(
    buf: &[u8],
    pos: &mut usize,
    datatype: MVPDataType,
    branchfactor: usize,
    leafcap: usize,
    path_length: usize,
    error: &mut MVPError,
    lvl: usize,
) -> Option<Rc<RefCell<Node>>> {
    if *pos >= buf.len() {
        *error = MVPError::Unrecognized;
        return None;
    }
    let node_type = buf[*pos];
    *pos += 1;

    if node_type == NodeType::LeafNode as u8 {
        let mut leaf = LeafNode::new(leafcap as u32);
        let sv1 = read_datapoint_from_buf(buf, pos, datatype, path_length);
        let sv2 = read_datapoint_from_buf(buf, pos, datatype, path_length);
        leaf.sv1 = sv1.map(Arc::new);
        leaf.sv2 = sv2.map(Arc::new);

        if *pos + 4 > buf.len() {
            *error = MVPError::Unrecognized;
            return None;
        }
        let mut nb_bytes = [0u8; 4];
        nb_bytes.copy_from_slice(&buf[*pos..*pos + 4]);
        let nbpoints = u32::from_le_bytes(nb_bytes) as usize;
        *pos += 4;
        leaf.nbpoints = nbpoints;
        if leaf.d1.len() < nbpoints {
            leaf.d1.resize(nbpoints, 0.0);
        }
        if leaf.d2.len() < nbpoints {
            leaf.d2.resize(nbpoints, 0.0);
        }
        leaf.points.clear();

        let mut saved_pos = *pos;
        for i in 0..nbpoints {
            if saved_pos + 4 > buf.len() {
                *error = MVPError::Unrecognized;
                return None;
            }
            let mut fb = [0u8; 4];
            fb.copy_from_slice(&buf[saved_pos..saved_pos + 4]);
            leaf.d1[i] = f32::from_le_bytes(fb);
            saved_pos += 4;
            if saved_pos + 4 > buf.len() {
                *error = MVPError::Unrecognized;
                return None;
            }
            fb.copy_from_slice(&buf[saved_pos..saved_pos + 4]);
            leaf.d2[i] = f32::from_le_bytes(fb);
            saved_pos += 4;
            if saved_pos + 8 > buf.len() {
                *error = MVPError::Unrecognized;
                return None;
            }
            let mut ob = [0u8; 8];
            ob.copy_from_slice(&buf[saved_pos..saved_pos + 8]);
            let offset = i64::from_le_bytes(ob) as usize;
            saved_pos += 8;
            *pos = offset;
            let dp = read_datapoint_from_buf(buf, pos, datatype, path_length);
            if let Some(d) = dp {
                leaf.points.push(Arc::new(d));
            }
        }
        leaf.nbpoints = leaf.points.len();
        Some(Rc::new(RefCell::new(Node::Leaf(leaf))))
    } else if node_type == NodeType::InternalNode as u8 {
        let bf = branchfactor;
        let length_m1 = bf.saturating_sub(1);
        let length_m2 = bf * length_m1;
        let fanout = bf * bf;
        let mut internal = InternalNode::new(bf as u32);
        let sv1 = read_datapoint_from_buf(buf, pos, datatype, path_length);
        let sv2 = read_datapoint_from_buf(buf, pos, datatype, path_length);
        internal.sv1 = sv1.map(Arc::new);
        internal.sv2 = sv2.map(Arc::new);

        if internal.m1.len() < length_m1 {
            internal.m1.resize(length_m1, 0.0);
        }
        if internal.m2.len() < length_m2 {
            internal.m2.resize(length_m2, 0.0);
        }
        for i in 0..length_m1 {
            if *pos + 4 > buf.len() {
                *error = MVPError::Unrecognized;
                return None;
            }
            let mut fb = [0u8; 4];
            fb.copy_from_slice(&buf[*pos..*pos + 4]);
            internal.m1[i] = f32::from_le_bytes(fb);
            *pos += 4;
        }
        for i in 0..length_m2 {
            if *pos + 4 > buf.len() {
                *error = MVPError::Unrecognized;
                return None;
            }
            let mut fb = [0u8; 4];
            fb.copy_from_slice(&buf[*pos..*pos + 4]);
            internal.m2[i] = f32::from_le_bytes(fb);
            *pos += 4;
        }

        // initialize child_nodes
        internal.child_nodes.clear();
        for _ in 0..fanout {
            internal.child_nodes.push(empty_leaf_rc(leafcap));
        }

        let mut saved_pos = *pos;
        for i in 0..fanout {
            if saved_pos + 1 > buf.len() {
                *error = MVPError::Unrecognized;
                return None;
            }
            let _fileno = buf[saved_pos];
            saved_pos += 1;
            if saved_pos + 8 > buf.len() {
                *error = MVPError::Unrecognized;
                return None;
            }
            let mut ob = [0u8; 8];
            ob.copy_from_slice(&buf[saved_pos..saved_pos + 8]);
            let offset = i64::from_le_bytes(ob);
            saved_pos += 8;
            if offset == 0 {
                // null child - keep empty leaf placeholder
                continue;
            }
            *pos = offset as usize;
            let child = _mvptree_read_node(buf, pos, datatype, branchfactor, leafcap, path_length, error, lvl + 2);
            if let Some(c) = child {
                internal.child_nodes[i] = c;
            }
            if *error != MVPError::Success {
                return None;
            }
        }
        Some(Rc::new(RefCell::new(Node::Internal(internal))))
    } else {
        *error = MVPError::Unrecognized;
        None
    }
}

// ----- MVPTree impl -----

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

    pub fn add(&mut self, points: Vec<MVPDatapoint>) -> MVPError {
        let mut err = MVPError::Success;
        if points.is_empty() {
            return err;
        }
        // type check
        let first_type = points[0].data_type;
        if first_type != self.datatype {
            // C's behavior: treat tree.datatype==0 as unset; here our enum cannot be 0.
            // We accept on first add only if tree was just constructed (we'll trust caller's datatype).
            // If they don't match, return type mismatch.
            // (We can skip this if datatype was effectively unset.)
            return MVPError::TypeMismatch;
        }
        // ensure path arrays
        let mut arc_points: Vec<Arc<MVPDatapoint>> = Vec::with_capacity(points.len());
        for mut p in points.into_iter() {
            p.path = vec![0.0; self.path_length];
            arc_points.push(Arc::new(p));
        }
        let existing = self.node.clone();
        let new_node = _mvptree_add(
            self.branch_factor,
            self.leaf_capacity,
            self.path_length,
            self.distance_function,
            existing,
            arc_points,
            &mut err,
            0,
        );
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
        let node = match self.node.as_ref() {
            Some(n) => n,
            None => return Err(MVPError::EmptyTree),
        };
        let mut results: Vec<Arc<MVPDatapoint>> = Vec::with_capacity(knearest);
        let mut target_path: Vec<f32> = vec![0.0; self.path_length];
        let err = _mvptree_retrieve(
            self.branch_factor,
            self.path_length,
            knearest,
            self.distance_function,
            node,
            target,
            &mut target_path,
            radius,
            &mut results,
            0,
        );
        if err != MVPError::Success && err != MVPError::KNearestCap {
            return Err(err);
        }
        let out: Vec<MVPDatapoint> = results.into_iter().map(|a| (*a).clone()).collect();
        Ok(out)
    }

    pub fn write(&self, filename: &str, mode: i32) -> MVPError {
        let _ = mode;
        let node = match self.node.as_ref() {
            Some(n) => n,
            None => return MVPError::ArgErr,
        };
        // Build buffer with header
        let mut buf: Vec<u8> = Vec::new();
        let tag_bytes = TAG.as_bytes();
        buf.extend_from_slice(tag_bytes);
        buf.push(0);
        // version: 4 bytes
        buf.extend_from_slice(&(VERSION as i32).to_le_bytes());
        // bf, pl, lc, ht
        let bf_u: u8 = self.branch_factor as u8;
        let pl_u: u8 = self.path_length as u8;
        let lc_u: u8 = self.leaf_capacity as u8;
        // ht based on tree.node->internal.sv1->type — i.e. datatype
        let ht_u: u8 = self.datatype as u8;
        buf.push(bf_u);
        buf.push(pl_u);
        buf.push(lc_u);
        buf.push(ht_u);
        // pad to HEADER_SIZE
        if buf.len() < HEADER_SIZE {
            buf.resize(HEADER_SIZE, 0);
        }
        let mut pos = HEADER_SIZE;
        let mut error = MVPError::Success;
        _mvptree_write_node(
            &mut buf,
            &mut pos,
            self.branch_factor,
            self.leaf_capacity,
            self.path_length,
            node,
            &mut error,
            0,
        );

        if error != MVPError::Success {
            return error;
        }
        let actual_len = pos;
        // write the buffer to the file
        let mut f = match OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(filename)
        {
            Ok(f) => f,
            Err(_) => return MVPError::FileOpen,
        };
        if f.write_all(&buf[..actual_len]).is_err() {
            return MVPError::NoWrite;
        }
        if f.flush().is_err() {
            return MVPError::NoWrite;
        }
        MVPError::Success
    }

    pub fn print(&self, stream: &mut dyn Write) -> MVPError {
        let node = match self.node.as_ref() {
            Some(n) => n,
            None => return MVPError::ArgErr,
        };
        let err = _mvptree_print(stream, self.branch_factor, node, 0);
        if err != MVPError::Success {
            let _ = writeln!(stream, "malformed tree: {}", error_to_string(err));
        }
        err
    }

    pub fn clear(&mut self, node: &mut Option<Box<Node>>) {
        // reset the passed-in node param
        *node = None;
        // also drop our own root
        self.node = None;
    }

    pub fn extend_mvpfile(&mut self) -> i32 {
        // Grow the buffer by pgsize bytes.
        let pg = self.pgsize as usize;
        if pg == 0 {
            return -1;
        }
        let new_size = self.size as usize + pg;
        self.buf.resize(new_size, 0);
        self.size = new_size as i64;
        0
    }
}

pub fn mvptree_read(
    filename: &str,
    distance_function: DistanceFunction,
) -> Result<MVPTree, MVPError> {
    let mut f = match File::open(filename) {
        Ok(f) => f,
        Err(_) => return Err(MVPError::FileNotFound),
    };
    let mut buf: Vec<u8> = Vec::new();
    if f.read_to_end(&mut buf).is_err() {
        return Err(MVPError::FileOpen);
    }
    if buf.len() < HEADER_SIZE {
        return Err(MVPError::FileOpen);
    }
    let tag_len = TAG.as_bytes().len() + 1;
    let mut pos = tag_len;
    if pos + 4 > buf.len() {
        return Err(MVPError::FileOpen);
    }
    pos += 4; // skip version
    if pos + 4 > buf.len() {
        return Err(MVPError::FileOpen);
    }
    let bf = buf[pos] as usize;
    pos += 1;
    let pl = buf[pos] as usize;
    pos += 1;
    let lc = buf[pos] as usize;
    pos += 1;
    let ht = buf[pos];
    let datatype = MVPDataType::from_u8(ht).unwrap_or(MVPDataType::ByteArray);

    let mut tree = MVPTree::new(bf, pl, lc, datatype, distance_function);
    let mut read_pos = HEADER_SIZE;
    let mut err = MVPError::Success;
    let node = _mvptree_read_node(&buf, &mut read_pos, datatype, bf, lc, pl, &mut err, 0);
    if err != MVPError::Success {
        return Err(err);
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

    pub fn select_vantage_points(
        &mut self,
        _nb: u32,
        _sv1_pos: i32,
        _sv2_pos: i32,
        _dist: DistanceFunction,
    ) -> i32 {
        // Not used directly (logic is internal). Returns 0 for success.
        0
    }

    pub fn find_splits(
        &mut self,
        _nb: u32,
        _vp: &MVPDatapoint,
        _tree: &MVPTree,
        _length_m: u32,
    ) -> f32 {
        // Not used directly; helper API placeholder.
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
        // Not used directly; helper API placeholder.
        Vec::new()
    }

    pub fn find_distance_range_for_vp(
        &mut self,
        _nb: u32,
        _vp: &MVPDatapoint,
        _tree: &MVPTree,
        _level: i32,
    ) -> i32 {
        // Not used directly; helper API placeholder.
        0
    }

    pub fn write(&self, tree: &MVPTree) -> i64 {
        // Compute serialized size for this datapoint.
        let id_bytes = self.id.as_bytes();
        let idlen = id_bytes.len().min(255) as u32;
        let datalength = self.datalen as u32;
        let type_width = self.data_type.byte_width() as u32;
        let bytelength: u32 = (1 + idlen) + 4 + datalength * type_width + tree.path_length as u32 * 4;
        bytelength as i64
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

#[allow(dead_code)]
fn _silence_unused() {
    let _ = (ptr::null::<u8>(), 0 as c_int, io::stdout());
}
