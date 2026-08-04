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

impl MVPDataType {
    fn from_u8(v: u8) -> MVPDataType {
        match v {
            1 => MVPDataType::ByteArray,
            2 => MVPDataType::UInt16Array,
            4 => MVPDataType::UInt32Array,
            8 => MVPDataType::UInt64Array,
            _ => MVPDataType::ByteArray,
        }
    }
    fn width(self) -> usize {
        match self {
            MVPDataType::ByteArray => 1,
            MVPDataType::UInt16Array => 2,
            MVPDataType::UInt32Array => 4,
            MVPDataType::UInt64Array => 8,
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
impl InternalNode{
    pub fn new(bf:u32) -> Self {
        let bf = bf as usize;
        InternalNode {
            node_type: NodeType::InternalNode,
            sv1: None,
            sv2: None,
            m1: vec![0.0; bf.saturating_sub(1)],
            m2: vec![0.0; bf * bf.saturating_sub(1)],
            child_nodes: Vec::with_capacity(bf * bf),
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
        let bf = bf as usize;
        LeafNode {
            node_type: NodeType::LeafNode,
            sv1: None,
            sv2: None,
            points: Vec::with_capacity(bf),
            d1: Vec::with_capacity(bf),
            d2: Vec::with_capacity(bf),
            nbpoints: 0,
        }
    }
}
pub enum Node {
    Leaf(LeafNode),
    Internal(InternalNode),
}

impl Node {
    fn is_empty_placeholder(&self) -> bool {
        match self {
            Node::Leaf(l) => l.sv1.is_none() && l.sv2.is_none() && l.nbpoints == 0,
            _ => false,
        }
    }
}

fn empty_leaf() -> Rc<RefCell<Node>> {
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

fn is_nan_f32(x: f32) -> bool {
    x != x
}

// Helper to compute select_vantage_points on a slice of Arc<MVPDatapoint>
fn select_vantage_points_helper(points: &[Arc<MVPDatapoint>], dist: DistanceFunction) -> Result<(i32, i32), i32> {
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

// find_splits helper - sorts distances and computes split points
fn find_splits_helper(
    points: &[Arc<MVPDatapoint>],
    vp: &MVPDatapoint,
    distfunc: DistanceFunction,
    m: &mut [f32],
) -> i32 {
    let nb = points.len();
    let length_m = m.len();
    if nb == 0 || length_m == 0 {
        return -1;
    }
    let mut dist_arr: Vec<f32> = Vec::with_capacity(nb);
    for p in points.iter() {
        let d = distfunc(p, vp);
        if is_nan_f32(d) || d < 0.0 {
            return -2;
        }
        dist_arr.push(d);
    }
    dist_arr.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    for i in 0..length_m {
        let mut index = (i + 1) * nb / (length_m + 1);
        if index >= nb {
            index = nb - 1;
        }
        m[i] = dist_arr[index];
    }
    0
}

// find_distance_range_for_vp helper - assigns path values
fn find_distance_range_for_vp_helper(
    points: &[Arc<MVPDatapoint>],
    vp: &MVPDatapoint,
    distfunc: DistanceFunction,
    pathlength: usize,
    lvl: i32,
) -> i32 {
    if points.is_empty() {
        return -1;
    }
    for p in points.iter() {
        let d = distfunc(vp, p);
        if is_nan_f32(d) || d < 0.0 {
            return -2;
        }
        if (lvl as usize) < pathlength {
            // we need mutable access to path
            // SAFETY: We have an Arc<MVPDatapoint>; to modify path, we need to use unsafe
            // since the field path is Vec<f32>. But we need shared mutability.
            // Alternative: make the path a RefCell/Mutex, or directly use the underlying vector.
            // We'll use a raw pointer cast via Arc::as_ptr and then to mut.
            unsafe {
                let raw = Arc::as_ptr(p) as *mut MVPDatapoint;
                let path_ptr = ptr::addr_of_mut!((*raw).path);
                let path_len = (*path_ptr).len();
                if path_len > lvl as usize {
                    let elem = (*path_ptr).as_mut_ptr().add(lvl as usize);
                    *elem = d;
                }
            }
        }
    }
    0
}

// sort_points helper
fn sort_points_helper(
    points: &[Arc<MVPDatapoint>],
    sv1_pos: i32,
    sv2_pos: i32,
    vp: &MVPDatapoint,
    distfunc: DistanceFunction,
    bf: usize,
    pivots: &[f32],
) -> Option<(Vec<Vec<Arc<MVPDatapoint>>>, Vec<i32>)> {
    let nbpoints = points.len();
    if nbpoints == 0 {
        return None;
    }
    let length_m1 = bf - 1;
    let mut bins: Vec<Vec<Arc<MVPDatapoint>>> = (0..bf).map(|_| Vec::new()).collect();
    let mut counts: Vec<i32> = vec![0; bf];

    for i in 0..nbpoints {
        if i as i32 == sv1_pos || i as i32 == sv2_pos {
            continue;
        }
        let d = distfunc(vp, &points[i]);
        if is_nan_f32(d) || d < 0.0 {
            return None;
        }
        let mut placed = false;
        for k in 0..length_m1 {
            if d <= pivots[k] {
                bins[k].push(points[i].clone());
                counts[k] += 1;
                placed = true;
                break;
            }
        }
        if !placed && length_m1 > 0 && d > pivots[length_m1 - 1] {
            bins[length_m1].push(points[i].clone());
            counts[length_m1] += 1;
        } else if length_m1 == 0 {
            // bf == 1 - all points go into bin 0
            bins[0].push(points[i].clone());
            counts[0] += 1;
        }
    }

    Some((bins, counts))
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

// Internal recursive function to add points to the tree
fn _mvptree_add(
    tree_branch_factor: usize,
    tree_pathlength: usize,
    tree_leafcap: usize,
    tree_dist: DistanceFunction,
    node: Option<Rc<RefCell<Node>>>,
    points: Vec<Arc<MVPDatapoint>>,
    error: &mut MVPError,
    lvl: i32,
) -> Option<Rc<RefCell<Node>>> {
    let nbpoints = points.len();
    if nbpoints == 0 {
        return node;
    }
    let bf = tree_branch_factor;
    let length_m1 = if bf > 0 { bf - 1 } else { 0 };

    if node.is_none() {
        // Create new node
        // First, check if we can find two distinct vantage points
        let force_leaf = match select_vantage_points_helper(&points, tree_dist) {
            Ok((_, sv2)) => sv2 < 0,
            Err(_) => false,
        };
        if force_leaf || nbpoints <= tree_leafcap + 2 {
            // Create leaf node
            let mut leaf = LeafNode {
                node_type: NodeType::LeafNode,
                sv1: None,
                sv2: None,
                points: Vec::new(),
                d1: Vec::new(),
                d2: Vec::new(),
                nbpoints: 0,
            };
            let (sv1_pos, sv2_pos) = match select_vantage_points_helper(&points, tree_dist) {
                Ok(v) => v,
                Err(_) => {
                    *error = MVPError::VpNoSelect;
                    return None;
                }
            };
            leaf.sv1 = if sv1_pos >= 0 { Some(points[sv1_pos as usize].clone()) } else { None };
            leaf.sv2 = if sv2_pos >= 0 { Some(points[sv2_pos as usize].clone()) } else { None };

            if let Some(ref sv1_arc) = leaf.sv1 {
                if find_distance_range_for_vp_helper(&points, sv1_arc, tree_dist, tree_pathlength, lvl) < 0 {
                    *error = MVPError::NoSv1Range;
                    return None;
                }
            }
            if let Some(ref sv2_arc) = leaf.sv2 {
                if find_distance_range_for_vp_helper(&points, sv2_arc, tree_dist, tree_pathlength, lvl + 1) < 0 {
                    *error = MVPError::NoSv2Range;
                    return None;
                }
            }

            for i in 0..nbpoints {
                if i as i32 == sv1_pos || i as i32 == sv2_pos {
                    continue;
                }
                let d1 = if let Some(ref sv1_arc) = leaf.sv1 {
                    tree_dist(&points[i], sv1_arc)
                } else {
                    0.0
                };
                let d2 = if let Some(ref sv2_arc) = leaf.sv2 {
                    tree_dist(&points[i], sv2_arc)
                } else {
                    0.0
                };
                leaf.d1.push(d1);
                leaf.d2.push(d2);
                leaf.points.push(points[i].clone());
            }
            leaf.nbpoints = leaf.points.len();
            return Some(Rc::new(RefCell::new(Node::Leaf(leaf))));
        } else {
            // Create internal node
            let mut internal = InternalNode {
                node_type: NodeType::InternalNode,
                sv1: None,
                sv2: None,
                m1: vec![0.0; length_m1],
                m2: vec![0.0; bf * length_m1],
                child_nodes: Vec::with_capacity(bf * bf),
            };
            let (sv1_pos, sv2_pos) = match select_vantage_points_helper(&points, tree_dist) {
                Ok(v) => v,
                Err(_) => {
                    *error = MVPError::VpNoSelect;
                    return None;
                }
            };
            internal.sv1 = Some(points[sv1_pos as usize].clone());
            internal.sv2 = Some(points[sv2_pos as usize].clone());

            let sv1_arc = internal.sv1.clone().unwrap();
            let sv2_arc = internal.sv2.clone().unwrap();

            if find_distance_range_for_vp_helper(&points, &sv1_arc, tree_dist, tree_pathlength, lvl) < 0 {
                *error = MVPError::NoSv1Range;
                return None;
            }

            if find_splits_helper(&points, &sv1_arc, tree_dist, &mut internal.m1) < 0 {
                *error = MVPError::NoSplits;
                return None;
            }

            let bins_result = sort_points_helper(&points, sv1_pos, sv2_pos, &sv1_arc, tree_dist, bf, &internal.m1);
            let (bins, _binlengths) = match bins_result {
                Some(v) => v,
                None => {
                    *error = MVPError::NoSort;
                    return None;
                }
            };

            // initialize children with None placeholders
            for _ in 0..(bf * bf) {
                internal.child_nodes.push(empty_leaf());
            }

            for i in 0..bf {
                let bin_i = &bins[i];
                if find_distance_range_for_vp_helper(bin_i, &sv2_arc, tree_dist, tree_pathlength, lvl + 1) < 0 {
                    if !bin_i.is_empty() {
                        *error = MVPError::NoSv2Range;
                        return None;
                    }
                }

                let m2_slice = &mut internal.m2[i * length_m1..(i + 1) * length_m1];
                if !bin_i.is_empty() {
                    if find_splits_helper(bin_i, &sv2_arc, tree_dist, m2_slice) < 0 {
                        *error = MVPError::NoSplits;
                        return None;
                    }
                }

                let bins2_opt = if !bin_i.is_empty() {
                    sort_points_helper(bin_i, -1, -1, &sv2_arc, tree_dist, bf, m2_slice)
                } else {
                    Some(((0..bf).map(|_| Vec::<Arc<MVPDatapoint>>::new()).collect(), vec![0; bf]))
                };
                let (bins2, _bin2lengths) = match bins2_opt {
                    Some(v) => v,
                    None => {
                        *error = MVPError::NoSort;
                        return None;
                    }
                };

                for (j, bin2j) in bins2.into_iter().enumerate() {
                    let child_idx = i * bf + j;
                    let child = _mvptree_add(
                        tree_branch_factor,
                        tree_pathlength,
                        tree_leafcap,
                        tree_dist,
                        None,
                        bin2j,
                        error,
                        lvl + 2,
                    );
                    if let Some(c) = child {
                        internal.child_nodes[child_idx] = c;
                    } else {
                        internal.child_nodes[child_idx] = empty_leaf();
                    }
                }
            }
            return Some(Rc::new(RefCell::new(Node::Internal(internal))));
        }
    } else {
        // Node already exists
        let n = node.unwrap();
        let is_leaf;
        let is_empty_placeholder;
        let leaf_nbpoints;
        {
            let borrow = n.borrow();
            is_leaf = matches!(&*borrow, Node::Leaf(_));
            is_empty_placeholder = borrow.is_empty_placeholder();
            leaf_nbpoints = match &*borrow {
                Node::Leaf(l) => l.nbpoints,
                _ => 0,
            };
        }

        if is_empty_placeholder {
            // Treat as None
            return _mvptree_add(
                tree_branch_factor,
                tree_pathlength,
                tree_leafcap,
                tree_dist,
                None,
                points,
                error,
                lvl,
            );
        }

        if is_leaf {
            if leaf_nbpoints + nbpoints <= tree_leafcap {
                // Add points into leaf - plenty of room
                let mut borrow = n.borrow_mut();
                if let Node::Leaf(ref mut leaf) = *borrow {
                    let sv1_arc = leaf.sv1.clone();
                    if let Some(ref sv1) = sv1_arc {
                        if find_distance_range_for_vp_helper(&points, sv1, tree_dist, tree_pathlength, lvl) < 0 {
                            *error = MVPError::NoSv1Range;
                            drop(borrow);
                            return Some(n);
                        }
                    }

                    let mut pos = 0usize;
                    if leaf.sv2.is_none() {
                        leaf.sv2 = Some(points[0].clone());
                        pos = 1;
                    }
                    let sv2_arc = leaf.sv2.clone();
                    if let Some(ref sv2) = sv2_arc {
                        if find_distance_range_for_vp_helper(&points, sv2, tree_dist, tree_pathlength, lvl + 1) < 0 {
                            *error = MVPError::NoSv2Range;
                            drop(borrow);
                            return Some(n);
                        }
                    }
                    while pos < nbpoints {
                        let d1 = if let Some(ref sv1) = sv1_arc {
                            tree_dist(&points[pos], sv1)
                        } else {
                            0.0
                        };
                        let d2 = if let Some(ref sv2) = sv2_arc {
                            tree_dist(&points[pos], sv2)
                        } else {
                            0.0
                        };
                        leaf.d1.push(d1);
                        leaf.d2.push(d2);
                        leaf.points.push(points[pos].clone());
                        pos += 1;
                    }
                    leaf.nbpoints = leaf.points.len();
                }
                drop(borrow);
                return Some(n);
            } else {
                // Not enough room - create new node
                let mut tmp_pts: Vec<Arc<MVPDatapoint>> = Vec::new();
                {
                    let borrow = n.borrow();
                    if let Node::Leaf(ref leaf) = *borrow {
                        if let Some(ref sv1) = leaf.sv1 {
                            tmp_pts.push(sv1.clone());
                        }
                        if let Some(ref sv2) = leaf.sv2 {
                            tmp_pts.push(sv2.clone());
                        }
                        for p in leaf.points.iter() {
                            tmp_pts.push(p.clone());
                        }
                    }
                }
                for p in points.iter() {
                    tmp_pts.push(p.clone());
                }
                drop(n);
                return _mvptree_add(
                    tree_branch_factor,
                    tree_pathlength,
                    tree_leafcap,
                    tree_dist,
                    None,
                    tmp_pts,
                    error,
                    lvl,
                );
            }
        } else {
            // Internal node - recurse
            let internal_data = {
                let borrow = n.borrow();
                if let Node::Internal(ref internal) = *borrow {
                    (
                        internal.sv1.clone(),
                        internal.sv2.clone(),
                        internal.m1.clone(),
                        internal.m2.clone(),
                        internal.child_nodes.clone(),
                    )
                } else {
                    unreachable!()
                }
            };
            let (sv1_opt, sv2_opt, m1, m2, child_nodes) = internal_data;
            let sv1_arc = sv1_opt.unwrap();
            let sv2_arc = sv2_opt.unwrap();

            if find_distance_range_for_vp_helper(&points, &sv1_arc, tree_dist, tree_pathlength, lvl) < 0 {
                *error = MVPError::NoSv1Range;
                return Some(n);
            }

            let bins_result = sort_points_helper(&points, -1, -1, &sv1_arc, tree_dist, bf, &m1);
            let (bins, binlengths) = match bins_result {
                Some(v) => v,
                None => {
                    *error = MVPError::NoSort;
                    return Some(n);
                }
            };

            let mut new_children: Vec<Rc<RefCell<Node>>> = child_nodes.clone();

            for i in 0..bf {
                let bin_i = &bins[i];
                if binlengths[i] <= 0 {
                    continue;
                }
                if find_distance_range_for_vp_helper(bin_i, &sv2_arc, tree_dist, tree_pathlength, lvl + 1) < 0 {
                    *error = MVPError::NoSv2Range;
                    return Some(n);
                }
                let m2_slice = &m2[i * length_m1..(i + 1) * length_m1];
                let bins2_result = sort_points_helper(bin_i, -1, -1, &sv2_arc, tree_dist, bf, m2_slice);
                let (bins2, _bin2lengths) = match bins2_result {
                    Some(v) => v,
                    None => {
                        *error = MVPError::NoSort;
                        return Some(n);
                    }
                };

                for (j, bin2j) in bins2.into_iter().enumerate() {
                    let child_idx = i * bf + j;
                    let existing_child = new_children[child_idx].clone();
                    let is_placeholder = existing_child.borrow().is_empty_placeholder();
                    let child_to_pass = if is_placeholder {
                        None
                    } else {
                        Some(existing_child)
                    };
                    let child = _mvptree_add(
                        tree_branch_factor,
                        tree_pathlength,
                        tree_leafcap,
                        tree_dist,
                        child_to_pass,
                        bin2j,
                        error,
                        lvl + 2,
                    );
                    if let Some(c) = child {
                        new_children[child_idx] = c;
                    }
                    if *error != MVPError::Success {
                        break;
                    }
                }
            }

            // Update the node's children
            {
                let mut borrow = n.borrow_mut();
                if let Node::Internal(ref mut internal) = *borrow {
                    internal.child_nodes = new_children;
                }
            }
            return Some(n);
        }
    }
}

// Recursive retrieve helper
fn _mvptree_retrieve(
    bf: usize,
    pathlength: usize,
    distance: DistanceFunction,
    k: usize,
    node: &Rc<RefCell<Node>>,
    target: &mut MVPDatapoint,
    radius: f32,
    results: &mut Vec<Arc<MVPDatapoint>>,
    lvl: i32,
) -> MVPError {
    let length_m1 = if bf > 0 { bf - 1 } else { 0 };
    let borrow = node.borrow();
    match &*borrow {
        Node::Leaf(leaf) => {
            let sv1_arc = match &leaf.sv1 {
                Some(s) => s.clone(),
                None => return MVPError::Success,
            };
            let d1 = distance(target, &sv1_arc);
            if is_nan_f32(d1) || d1 < 0.0 {
                return MVPError::BadDistVal;
            }
            if (lvl as usize) < pathlength && (target.path.len() > lvl as usize) {
                target.path[lvl as usize] = d1;
            }
            if d1 <= radius {
                results.push(sv1_arc.clone());
                if results.len() >= k {
                    return MVPError::KNearestCap;
                }
            }
            if let Some(ref sv2_arc) = leaf.sv2 {
                let d2 = distance(target, sv2_arc);
                if is_nan_f32(d2) || d2 < 0.0 {
                    return MVPError::BadDistVal;
                }
                if d2 <= radius {
                    results.push(sv2_arc.clone());
                    if results.len() >= k {
                        return MVPError::KNearestCap;
                    }
                }
                if (lvl as usize + 1) < pathlength && (target.path.len() > lvl as usize + 1) {
                    target.path[lvl as usize + 1] = d2;
                }

                for i in 0..leaf.nbpoints {
                    if d1 - radius <= leaf.d1[i] && d1 + radius >= leaf.d1[i] {
                        if d2 - radius <= leaf.d2[i] && d2 + radius >= leaf.d2[i] {
                            let endpath = if (lvl as usize + 1) < pathlength {
                                lvl as usize + 1
                            } else {
                                pathlength
                            };
                            let mut skip = false;
                            for j in 0..endpath {
                                let target_pj = if target.path.len() > j { target.path[j] } else { 0.0 };
                                let pt_path = &leaf.points[i].path;
                                let pt_pj = if pt_path.len() > j { pt_path[j] } else { 0.0 };
                                if target_pj - radius <= pt_pj && target_pj + radius >= pt_pj {
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
        Node::Internal(internal) => {
            let sv1_arc = internal.sv1.clone().unwrap();
            let sv2_arc = internal.sv2.clone().unwrap();

            let d1 = distance(target, &sv1_arc);
            if is_nan_f32(d1) || d1 < 0.0 {
                return MVPError::BadDistVal;
            }
            if d1 <= radius {
                results.push(sv1_arc.clone());
                if results.len() >= k {
                    return MVPError::KNearestCap;
                }
            }
            if (lvl as usize) < pathlength && (target.path.len() > lvl as usize) {
                target.path[lvl as usize] = d1;
            }
            let d2 = distance(target, &sv2_arc);
            if is_nan_f32(d2) || d2 < 0.0 {
                return MVPError::BadDistVal;
            }
            if d2 <= radius {
                results.push(sv2_arc.clone());
                if results.len() >= k {
                    return MVPError::KNearestCap;
                }
            }
            if (lvl as usize + 1) < pathlength && (target.path.len() > lvl as usize + 1) {
                target.path[lvl as usize + 1] = d2;
            }

            // Children copy for recursion (avoid borrow conflict)
            let children: Vec<Rc<RefCell<Node>>> = internal.child_nodes.clone();
            let m1 = internal.m1.clone();
            let m2 = internal.m2.clone();
            drop(borrow);

            for i in 0..length_m1 {
                if d1 - radius <= m1[i] {
                    for j in 0..length_m1 {
                        if d2 - radius <= m2[i * length_m1 + j] {
                            let child = &children[i * bf + j];
                            if !child.borrow().is_empty_placeholder() {
                                let err = _mvptree_retrieve(bf, pathlength, distance, k, child, target, radius, results, lvl + 2);
                                if err != MVPError::Success {
                                    return err;
                                }
                            }
                        }
                    }
                    if length_m1 > 0 && d2 + radius >= m2[i * length_m1 + length_m1 - 1] {
                        let child = &children[i * bf + length_m1];
                        if !child.borrow().is_empty_placeholder() {
                            let err = _mvptree_retrieve(bf, pathlength, distance, k, child, target, radius, results, lvl + 2);
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
                        let child = &children[bf * length_m1 + j];
                        if !child.borrow().is_empty_placeholder() {
                            let err = _mvptree_retrieve(bf, pathlength, distance, k, child, target, radius, results, lvl + 2);
                            if err != MVPError::Success {
                                return err;
                            }
                        }
                    }
                }
                if length_m1 > 0 && d2 + radius >= m2[length_m1 * length_m1 + length_m1 - 1] {
                    let child = &children[bf * length_m1 + length_m1];
                    if !child.borrow().is_empty_placeholder() {
                        let err = _mvptree_retrieve(bf, pathlength, distance, k, child, target, radius, results, lvl + 2);
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

// Write helpers - operate on a buffer (Vec<u8>) instead of mmap
fn write_u8(buf: &mut Vec<u8>, pos: &mut usize, val: u8) {
    if buf.len() < *pos + 1 {
        buf.resize(*pos + 1, 0);
    }
    buf[*pos] = val;
    *pos += 1;
}
fn write_u32(buf: &mut Vec<u8>, pos: &mut usize, val: u32) {
    let bytes = val.to_le_bytes();
    if buf.len() < *pos + 4 {
        buf.resize(*pos + 4, 0);
    }
    buf[*pos..*pos + 4].copy_from_slice(&bytes);
    *pos += 4;
}
fn write_u64(buf: &mut Vec<u8>, pos: &mut usize, val: u64) {
    let bytes = val.to_le_bytes();
    if buf.len() < *pos + 8 {
        buf.resize(*pos + 8, 0);
    }
    buf[*pos..*pos + 8].copy_from_slice(&bytes);
    *pos += 8;
}
fn write_i64(buf: &mut Vec<u8>, pos: &mut usize, val: i64) {
    let bytes = val.to_le_bytes();
    if buf.len() < *pos + 8 {
        buf.resize(*pos + 8, 0);
    }
    buf[*pos..*pos + 8].copy_from_slice(&bytes);
    *pos += 8;
}
fn write_f32(buf: &mut Vec<u8>, pos: &mut usize, val: f32) {
    let bytes = val.to_le_bytes();
    if buf.len() < *pos + 4 {
        buf.resize(*pos + 4, 0);
    }
    buf[*pos..*pos + 4].copy_from_slice(&bytes);
    *pos += 4;
}
fn write_bytes(buf: &mut Vec<u8>, pos: &mut usize, data: &[u8]) {
    if buf.len() < *pos + data.len() {
        buf.resize(*pos + data.len(), 0);
    }
    buf[*pos..*pos + data.len()].copy_from_slice(data);
    *pos += data.len();
}

fn read_u8(buf: &[u8], pos: &mut usize) -> u8 {
    let v = buf[*pos];
    *pos += 1;
    v
}
fn read_u32(buf: &[u8], pos: &mut usize) -> u32 {
    let mut a = [0u8; 4];
    a.copy_from_slice(&buf[*pos..*pos + 4]);
    *pos += 4;
    u32::from_le_bytes(a)
}
fn read_i64(buf: &[u8], pos: &mut usize) -> i64 {
    let mut a = [0u8; 8];
    a.copy_from_slice(&buf[*pos..*pos + 8]);
    *pos += 8;
    i64::from_le_bytes(a)
}
fn read_f32(buf: &[u8], pos: &mut usize) -> f32 {
    let mut a = [0u8; 4];
    a.copy_from_slice(&buf[*pos..*pos + 4]);
    *pos += 4;
    f32::from_le_bytes(a)
}

// Write a datapoint and return the start offset
fn write_datapoint_helper(
    dp_opt: Option<&MVPDatapoint>,
    buf: &mut Vec<u8>,
    pos: &mut usize,
    pathlength: usize,
) -> i64 {
    let start = *pos as i64;
    match dp_opt {
        None => {
            write_u8(buf, pos, 0); // active = 0
            write_u32(buf, pos, 0); // bytelength = 0
            start
        }
        Some(dp) => {
            let active: u8 = 1;
            let id_bytes = dp.id.as_bytes();
            let idlen: u8 = id_bytes.len() as u8;
            let datalength: u32 = dp.datalen as u32;
            let dtype = dp.data_type as u8;
            let bytelength: u32 = (1u32 + idlen as u32 + 4 + (datalength * dtype as u32) + (pathlength as u32) * 4) as u32;
            write_u8(buf, pos, active);
            write_u32(buf, pos, bytelength);
            write_u8(buf, pos, idlen);
            write_bytes(buf, pos, id_bytes);
            write_u32(buf, pos, datalength);
            write_bytes(buf, pos, &dp.data);
            // path
            for i in 0..pathlength {
                let v = if dp.path.len() > i { dp.path[i] } else { 0.0 };
                write_f32(buf, pos, v);
            }
            start
        }
    }
}

fn read_datapoint_helper(
    buf: &[u8],
    pos: &mut usize,
    pathlength: usize,
    datatype: MVPDataType,
) -> Option<MVPDatapoint> {
    let active = read_u8(buf, pos);
    let bytelength = read_u32(buf, pos);
    if active == 0 && bytelength == 0 {
        return None;
    }
    let idlen = read_u8(buf, pos);
    let id_start = *pos;
    let id_end = id_start + idlen as usize;
    let id_str = String::from_utf8_lossy(&buf[id_start..id_end]).to_string();
    *pos = id_end;
    let datalength = read_u32(buf, pos);
    let dtype_width = datatype as u8 as usize;
    let data_bytes = (datalength as usize) * dtype_width;
    let mut data = vec![0u8; data_bytes];
    data.copy_from_slice(&buf[*pos..*pos + data_bytes]);
    *pos += data_bytes;
    let mut path = vec![0.0f32; pathlength];
    for i in 0..pathlength {
        path[i] = read_f32(buf, pos);
    }
    Some(MVPDatapoint {
        id: id_str,
        data,
        path,
        datalen: datalength as usize,
        data_type: datatype,
    })
}

fn _mvptree_write(
    bf: usize,
    pathlength: usize,
    leafcap: usize,
    node: &Rc<RefCell<Node>>,
    buf: &mut Vec<u8>,
    pos: &mut usize,
    error: &mut MVPError,
    lvl: i32,
) -> i64 {
    let start_pos = *pos as i64;
    let length_m1 = if bf > 0 { bf - 1 } else { 0 };
    let length_m2 = (bf - 1) * bf;
    let fanout = bf * bf;

    let borrow = node.borrow();
    match &*borrow {
        Node::Leaf(leaf) => {
            let node_type: u8 = NodeType::LeafNode as u8;
            write_u8(buf, pos, node_type);
            write_datapoint_helper(leaf.sv1.as_deref(), buf, pos, pathlength);
            write_datapoint_helper(leaf.sv2.as_deref(), buf, pos, pathlength);
            write_u32(buf, pos, leaf.nbpoints as u32);

            let mut saved_pos = *pos;
            // reserve space for leafcap*(2*sizeof(float) + sizeof(off_t))
            *pos += leafcap * (2 * 4 + 8);

            // Need to drop borrow before recursing
            let nbpoints = leaf.nbpoints;
            let d1 = leaf.d1.clone();
            let d2 = leaf.d2.clone();
            let points: Vec<Arc<MVPDatapoint>> = leaf.points.iter().cloned().collect();
            drop(borrow);

            for i in 0..nbpoints {
                write_f32(buf, &mut saved_pos, d1[i]);
                write_f32(buf, &mut saved_pos, d2[i]);
                let offset = write_datapoint_helper(Some(&points[i]), buf, pos, pathlength);
                write_i64(buf, &mut saved_pos, offset);
            }
        }
        Node::Internal(internal) => {
            let node_type: u8 = NodeType::InternalNode as u8;
            write_u8(buf, pos, node_type);
            write_datapoint_helper(internal.sv1.as_deref(), buf, pos, pathlength);
            write_datapoint_helper(internal.sv2.as_deref(), buf, pos, pathlength);
            // M1
            for i in 0..length_m1 {
                let v = if internal.m1.len() > i { internal.m1[i] } else { 0.0 };
                write_f32(buf, pos, v);
            }
            // M2
            for i in 0..length_m2 {
                let v = if internal.m2.len() > i { internal.m2[i] } else { 0.0 };
                write_f32(buf, pos, v);
            }
            let mut saved_pos = *pos;
            *pos += fanout * (1 + 8); // 1 byte fileno + 8 byte offset
            let children = internal.child_nodes.clone();
            drop(borrow);
            for i in 0..fanout {
                let child = &children[i];
                let is_placeholder = child.borrow().is_empty_placeholder();
                let offset = if is_placeholder {
                    0i64
                } else {
                    _mvptree_write(bf, pathlength, leafcap, child, buf, pos, error, lvl + 2)
                };
                write_u8(buf, &mut saved_pos, 0); // fileno
                write_i64(buf, &mut saved_pos, offset);
            }
        }
    }
    start_pos
}

fn _mvptree_read_node(
    buf: &[u8],
    pos: &mut usize,
    bf: usize,
    pathlength: usize,
    leafcap: usize,
    datatype: MVPDataType,
    error: &mut MVPError,
    lvl: i32,
) -> Option<Rc<RefCell<Node>>> {
    let node_type = read_u8(buf, pos);
    let length_m1 = if bf > 0 { bf - 1 } else { 0 };
    let length_m2 = (bf - 1) * bf;
    let fanout = bf * bf;

    if node_type == NodeType::LeafNode as u8 {
        let mut leaf = LeafNode {
            node_type: NodeType::LeafNode,
            sv1: None,
            sv2: None,
            points: Vec::new(),
            d1: Vec::new(),
            d2: Vec::new(),
            nbpoints: 0,
        };
        leaf.sv1 = read_datapoint_helper(buf, pos, pathlength, datatype).map(Arc::new);
        leaf.sv2 = read_datapoint_helper(buf, pos, pathlength, datatype).map(Arc::new);
        let nbpoints = read_u32(buf, pos) as usize;
        leaf.nbpoints = nbpoints;
        let mut saved_pos = *pos;
        for _ in 0..nbpoints {
            let d1 = read_f32(buf, &mut saved_pos);
            let d2 = read_f32(buf, &mut saved_pos);
            let offset = read_i64(buf, &mut saved_pos);
            let mut p = offset as usize;
            let dp = read_datapoint_helper(buf, &mut p, pathlength, datatype);
            if let Some(dp) = dp {
                leaf.d1.push(d1);
                leaf.d2.push(d2);
                leaf.points.push(Arc::new(dp));
            }
        }
        leaf.nbpoints = leaf.points.len();
        Some(Rc::new(RefCell::new(Node::Leaf(leaf))))
    } else if node_type == NodeType::InternalNode as u8 {
        let mut internal = InternalNode {
            node_type: NodeType::InternalNode,
            sv1: None,
            sv2: None,
            m1: vec![0.0; length_m1],
            m2: vec![0.0; length_m2],
            child_nodes: Vec::with_capacity(fanout),
        };
        internal.sv1 = read_datapoint_helper(buf, pos, pathlength, datatype).map(Arc::new);
        internal.sv2 = read_datapoint_helper(buf, pos, pathlength, datatype).map(Arc::new);
        for i in 0..length_m1 {
            internal.m1[i] = read_f32(buf, pos);
        }
        for i in 0..length_m2 {
            internal.m2[i] = read_f32(buf, pos);
        }
        let mut saved_pos = *pos;
        for _ in 0..fanout {
            let _fileno = read_u8(buf, &mut saved_pos);
            let offset = read_i64(buf, &mut saved_pos);
            if offset == 0 {
                internal.child_nodes.push(empty_leaf());
            } else {
                let mut p = offset as usize;
                let child = _mvptree_read_node(buf, &mut p, bf, pathlength, leafcap, datatype, error, lvl + 2);
                if let Some(c) = child {
                    internal.child_nodes.push(c);
                } else {
                    internal.child_nodes.push(empty_leaf());
                }
            }
            if *error != MVPError::Success {
                break;
            }
        }
        // Pad children to fanout if break
        while internal.child_nodes.len() < fanout {
            internal.child_nodes.push(empty_leaf());
        }
        Some(Rc::new(RefCell::new(Node::Internal(internal))))
    } else {
        *error = MVPError::Unrecognized;
        None
    }
}

fn _mvptree_print(
    stream: &mut dyn Write,
    bf: usize,
    node_opt: &Option<Rc<RefCell<Node>>>,
    lvl: i32,
) -> MVPError {
    let length_m1 = bf - 1;
    let length_m2 = bf;
    let fanout = bf * bf;

    if let Some(node) = node_opt {
        let borrow = node.borrow();
        match &*borrow {
            Node::Leaf(leaf) => {
                let _ = writeln!(stream, "LEAF{}  ({} points)", lvl, leaf.nbpoints);
                if let Some(ref sv1) = leaf.sv1 {
                    let _ = writeln!(stream, "    sv1: {}", sv1.id);
                }
                if let Some(ref sv2) = leaf.sv2 {
                    let _ = writeln!(stream, "    sv2: {}", sv2.id);
                }
                for (i, p) in leaf.points.iter().enumerate() {
                    let _ = writeln!(stream, "        point[{}]: {}", i, p.id);
                }
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
                    let v = if internal.m2.len() > i { internal.m2[i] } else { 0.0 };
                    let _ = write!(stream, "  M2[{}] = {:.4};", i, v);
                }
                let _ = writeln!(stream);
                let children = internal.child_nodes.clone();
                drop(borrow);
                for i in 0..fanout {
                    let child = &children[i];
                    let is_placeholder = child.borrow().is_empty_placeholder();
                    if is_placeholder {
                        let _ = writeln!(stream, "NULL{}", lvl + 2);
                    } else {
                        let err = _mvptree_print(stream, bf, &Some(child.clone()), lvl + 2);
                        if err != MVPError::Success {
                            return err;
                        }
                    }
                }
            }
        }
    } else {
        let _ = writeln!(stream, "NULL{}", lvl);
    }
    MVPError::Success
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
        // Check datatype
        let first_type = points[0].data_type;
        if self.node.is_none() {
            self.datatype = first_type;
        }
        if self.datatype != first_type {
            return MVPError::TypeMismatch;
        }

        // Initialize path arrays for each point
        let pathlen = self.path_length;
        let arc_points: Vec<Arc<MVPDatapoint>> = points.into_iter().map(|mut p| {
            p.path = vec![0.0; pathlen];
            Arc::new(p)
        }).collect();

        let mut err = MVPError::Success;
        let cur_node = self.node.clone();
        let new_node = _mvptree_add(
            self.branch_factor,
            self.path_length,
            self.leaf_capacity,
            self.distance_function,
            cur_node,
            arc_points,
            &mut err,
            0,
        );
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
        let mut target_clone = target.clone();
        target_clone.path = vec![0.0; self.path_length];
        let mut results: Vec<Arc<MVPDatapoint>> = Vec::new();
        let err = _mvptree_retrieve(
            self.branch_factor,
            self.path_length,
            self.distance_function,
            knearest,
            node,
            &mut target_clone,
            radius,
            &mut results,
            0,
        );
        match err {
            MVPError::Success | MVPError::KNearestCap => {
                Ok(results.into_iter().map(|a| (*a).clone()).collect())
            }
            e => Err(e),
        }
    }

    pub fn write(&self, filename: &str, _mode: i32) -> MVPError {
        let node = match &self.node {
            Some(n) => n,
            None => return MVPError::ArgErr,
        };
        let mut buf: Vec<u8> = Vec::new();
        let mut pos = 0usize;
        // Write header - tag (null terminated)
        let tag_bytes = TAG.as_bytes();
        write_bytes(&mut buf, &mut pos, tag_bytes);
        write_u8(&mut buf, &mut pos, 0); // null terminator
        // version (i32 LE)
        let v_bytes = (VERSION as i32).to_le_bytes();
        write_bytes(&mut buf, &mut pos, &v_bytes);

        write_u8(&mut buf, &mut pos, self.branch_factor as u8);
        write_u8(&mut buf, &mut pos, self.path_length as u8);
        write_u8(&mut buf, &mut pos, self.leaf_capacity as u8);
        let ht = self.datatype as u8;
        write_u8(&mut buf, &mut pos, ht);
        // Pad to HEADER_SIZE
        if buf.len() < HEADER_SIZE {
            buf.resize(HEADER_SIZE, 0);
        }
        pos = HEADER_SIZE;

        let mut error = MVPError::Success;
        _mvptree_write(
            self.branch_factor,
            self.path_length,
            self.leaf_capacity,
            node,
            &mut buf,
            &mut pos,
            &mut error,
            0,
        );

        // Write to file
        let mut f = match File::create(filename) {
            Ok(f) => f,
            Err(_) => return MVPError::FileOpen,
        };
        if let Err(_) = f.write_all(&buf) {
            return MVPError::NoWrite;
        }
        error
    }

    pub fn print(&self, stream: &mut dyn Write) -> MVPError {
        _mvptree_print(stream, self.branch_factor, &self.node, 0)
    }

    pub fn clear(&mut self, _node: &mut Option<Box<Node>>) {
        self.node = None;
    }

    pub fn extend_mvpfile(&mut self) -> i32 {
        self.size += self.pgsize;
        0
    }
}

pub fn mvptree_read(filename: &str, distance_function: DistanceFunction) -> Result<MVPTree, MVPError> {
    use std::io::Read as _;
    let mut f = match File::open(filename) {
        Ok(f) => f,
        Err(_) => return Err(MVPError::FileNotFound),
    };
    let mut buf: Vec<u8> = Vec::new();
    if let Err(_) = f.read_to_end(&mut buf) {
        return Err(MVPError::FileOpen);
    }
    let mut pos = 0usize;
    let tag_bytes = TAG.as_bytes();
    pos += tag_bytes.len() + 1; // skip tag + null
    pos += 4; // skip version (i32)
    let bf = read_u8(&buf, &mut pos);
    let pl = read_u8(&buf, &mut pos);
    let lc = read_u8(&buf, &mut pos);
    let ht = read_u8(&buf, &mut pos);

    let datatype = MVPDataType::from_u8(ht);
    let mut tree = MVPTree::new(bf as usize, pl as usize, lc as usize, datatype, distance_function);
    pos = HEADER_SIZE;
    let mut error = MVPError::Success;
    let node = _mvptree_read_node(
        &buf,
        &mut pos,
        bf as usize,
        pl as usize,
        lc as usize,
        datatype,
        &mut error,
        0,
    );
    tree.node = node;
    if error != MVPError::Success {
        return Err(error);
    }
    Ok(tree)
}

pub fn error_to_string(error: MVPError) -> &'static str {
    let idx = match error {
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
        MVPError::FileNotFound => 23,
        MVPError::Unrecognized => 24,
    };
    ERROR_MSGS[idx]
}
