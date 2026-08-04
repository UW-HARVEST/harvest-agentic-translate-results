use std::fs::File;
use std::io::{self, Write, Read as _};
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
        // Mirrors create_internal() in c_src/mvptree.c:
        //   M1 has (bf - 1) floats (first-level split pivots)
        //   M2 has bf * (bf - 1) floats (second-level split pivots)
        //   child_nodes has bf * bf entries (initially empty)
        let bf = bf as usize;
        let length_m1 = if bf > 0 { bf - 1 } else { 0 };
        InternalNode {
            node_type: NodeType::InternalNode,
            sv1: None,
            sv2: None,
            m1: vec![0.0f32; length_m1],
            m2: vec![0.0f32; bf * length_m1],
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
        // Mirrors create_leaf() in c_src/mvptree.c. The parameter is the
        // leaf capacity in C; we use it to pre-allocate the vectors.
        let leafcap = bf as usize;
        LeafNode {
            node_type: NodeType::LeafNode,
            sv1: None,
            sv2: None,
            points: Vec::with_capacity(leafcap),
            d1: Vec::with_capacity(leafcap),
            d2: Vec::with_capacity(leafcap),
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

/* Helper: NaN/negative check, mirrors is_nan() check + d < 0 in mvptree.c */
fn is_bad_dist(d: f32) -> bool {
    d.is_nan() || d < 0.0f32
}

/* Helper: pick the two points (by index) that are at maximum distance from
 * each other under `dist`. Returns (sv1_pos, sv2_pos). On error returns
 * Err(()).  Mirrors select_vantage_points() in mvptree.c. */
fn select_vantage_points_impl(
    points: &[Arc<MVPDatapoint>],
    dist: DistanceFunction,
) -> Result<(i32, i32), ()> {
    let nb = points.len();
    if nb == 0 {
        return Err(());
    }
    let mut sv1_pos: i32 = if nb >= 1 { 0 } else { -1 };
    let mut sv2_pos: i32 = -1;
    let mut max_dist: f32 = 0.0;
    for i in 0..nb {
        for j in (i + 1)..nb {
            let d = dist(&points[i], &points[j]);
            if is_bad_dist(d) {
                return Err(());
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

/* Compute pivot values for splitting the points based on distance from vp.
 * Mirrors find_splits() in mvptree.c. Returns Vec of length `length_m` on
 * success. */
fn find_splits_impl(
    points: &[Arc<MVPDatapoint>],
    vp: &MVPDatapoint,
    dist: DistanceFunction,
    length_m: usize,
) -> Result<Vec<f32>, ()> {
    let nb = points.len();
    if nb == 0 || length_m == 0 {
        return Err(());
    }
    let mut dists: Vec<f32> = Vec::with_capacity(nb);
    for p in points.iter() {
        let d = dist(p, vp);
        if is_bad_dist(d) {
            return Err(());
        }
        dists.push(d);
    }
    // selection sort to mirror C, but the order doesn't matter much; use stable sort
    dists.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mut out = vec![0.0f32; length_m];
    for i in 0..length_m {
        let mut idx = (i + 1) * nb / (length_m + 1);
        if idx >= nb {
            idx = nb - 1;
        }
        out[i] = dists[idx];
    }
    Ok(out)
}

/* Sort points into branch_factor bins by distance to vp using `pivots` as
 * cut points. Mirrors sort_points() in mvptree.c. Skips indices sv1_pos and
 * sv2_pos (use -1 to skip nothing). */
fn sort_points_impl(
    points: &[Arc<MVPDatapoint>],
    sv1_pos: i32,
    sv2_pos: i32,
    vp: &MVPDatapoint,
    dist: DistanceFunction,
    branch_factor: usize,
    pivots: &[f32],
) -> Result<Vec<Vec<Arc<MVPDatapoint>>>, ()> {
    let length_m1 = branch_factor.saturating_sub(1);
    let mut bins: Vec<Vec<Arc<MVPDatapoint>>> = (0..branch_factor)
        .map(|_| Vec::new())
        .collect();
    for (i, p) in points.iter().enumerate() {
        if i as i32 == sv1_pos || i as i32 == sv2_pos {
            continue;
        }
        let d = dist(vp, p);
        if is_bad_dist(d) {
            return Err(());
        }
        let mut placed = false;
        for k in 0..length_m1 {
            if d <= pivots[k] {
                bins[k].push(p.clone());
                placed = true;
                break;
            }
        }
        if !placed && length_m1 > 0 && d > pivots[length_m1 - 1] {
            bins[length_m1].push(p.clone());
        } else if !placed && length_m1 == 0 {
            // No pivots - everything goes in bin 0
            bins[0].push(p.clone());
        }
    }
    Ok(bins)
}

/* Compute distances from each point to vp and assign into path[lvl] field.
 * Mirrors find_distance_range_for_vp() in mvptree.c. Because points are
 * stored as Arc<MVPDatapoint> (immutable shared), we can't update their
 * path here in-place; the public API returns the computed distances and
 * lets the caller handle them. */
fn find_distance_range_for_vp_impl(
    points: &[Arc<MVPDatapoint>],
    vp: &MVPDatapoint,
    dist: DistanceFunction,
) -> Result<Vec<f32>, ()> {
    let mut out = Vec::with_capacity(points.len());
    for p in points.iter() {
        let d = dist(vp, p);
        if is_bad_dist(d) {
            return Err(());
        }
        out.push(d);
    }
    Ok(out)
}

impl MVPTree {
    pub fn new(branch_factor: usize, path_length: usize, leaf_capacity: usize, datatype: MVPDataType, distance_function: DistanceFunction) -> Self {
        // Mirrors mvptree_alloc() in c_src/mvptree.c.
        MVPTree {
            branch_factor,
            path_length,
            leaf_capacity,
            datatype,
            pos: 0,
            size: 0,
            pgsize: 4096, // typical page size; sysconf(_SC_PAGESIZE) in C
            buf: Vec::new(),
            node: None,
            distance_function,
        }
    }
    pub fn add(&mut self, points: Vec<MVPDatapoint>) -> MVPError {
        // Mirrors mvptree_add() in c_src/mvptree.c.
        if points.is_empty() {
            return MVPError::Success;
        }
        if points[0].data_type != self.datatype {
            return MVPError::TypeMismatch;
        }
        // Allocate path arrays for each datapoint and wrap in Arc
        let arc_points: Vec<Arc<MVPDatapoint>> = points
            .into_iter()
            .map(|mut p| {
                p.path = vec![0.0f32; self.path_length];
                Arc::new(p)
            })
            .collect();

        let bf = self.branch_factor;
        let leafcap = self.leaf_capacity;
        let path_length = self.path_length;
        let dist = self.distance_function;

        match self._add_recursive(self.node.clone(), arc_points, 0, bf, leafcap, path_length, dist) {
            Ok(new_node) => {
                self.node = new_node;
                MVPError::Success
            }
            Err(e) => e,
        }
    }
    pub fn retrieve(&self, target: &MVPDatapoint, knearest: usize, radius: f32) -> Result<Vec<MVPDatapoint>, MVPError> {
        // Mirrors mvptree_retrieve() in c_src/mvptree.c.
        if knearest == 0 || radius < 0.0 {
            return Err(MVPError::ArgErr);
        }
        let node = match &self.node {
            Some(n) => n.clone(),
            None => return Err(MVPError::EmptyTree),
        };
        let mut target_path = vec![0.0f32; self.path_length];
        let mut results: Vec<Arc<MVPDatapoint>> = Vec::new();
        let dist = self.distance_function;
        let bf = self.branch_factor;
        let path_length = self.path_length;

        match Self::_retrieve_recursive(
            node,
            target,
            radius,
            knearest,
            &mut results,
            &mut target_path,
            0,
            bf,
            path_length,
            dist,
        ) {
            Ok(()) | Err(MVPError::KNearestCap) => Ok(results
                .into_iter()
                .map(|a| (*a).clone())
                .collect()),
            Err(e) => Err(e),
        }
    }
    pub fn write(&self, filename: &str, mode:i32) -> MVPError {
        // Mirrors mvptree_write() in c_src/mvptree.c. We use a simple
        // buffered write rather than mmap.
        if self.node.is_none() {
            return MVPError::ArgErr;
        }
        let node = self.node.as_ref().unwrap().clone();

        // Build buffer in memory
        let mut buf: Vec<u8> = Vec::new();

        // Header (HEADER_SIZE bytes)
        let tag_bytes = TAG.as_bytes();
        buf.extend_from_slice(tag_bytes);
        buf.push(0u8); // null terminator
        // version (4 bytes, little-endian)
        buf.extend_from_slice(&VERSION.to_le_bytes());
        // bf, pl, lc, ht (1 byte each)
        buf.push(self.branch_factor as u8);
        buf.push(self.path_length as u8);
        buf.push(self.leaf_capacity as u8);
        // datatype byte derived from top-level node
        let ht = self.datatype as u8;
        buf.push(ht);
        // pad to HEADER_SIZE
        while buf.len() < HEADER_SIZE {
            buf.push(0);
        }

        // Walk the tree and serialize nodes
        let mut error = MVPError::Success;
        Self::_write_node_recursive(node, &mut buf, &mut error, self.path_length, self.branch_factor, self.leaf_capacity, self.datatype);
        if error != MVPError::Success {
            return error;
        }

        // Write buffer to file
        let mut f = match File::create(filename) {
            Ok(f) => f,
            Err(_) => return MVPError::FileOpen,
        };
        if f.write_all(&buf).is_err() {
            return MVPError::NoWrite;
        }
        // The mode parameter is accepted but not applied here (would require
        // unix permissions which we ignore for portability).
        let _ = mode;
        MVPError::Success
    }
    pub fn print(&self, stream: &mut dyn Write) -> MVPError {
        // Mirrors mvptree_print() in c_src/mvptree.c.
        match &self.node {
            None => {
                let _ = writeln!(stream, "NULL0");
                MVPError::Success
            }
            Some(n) => {
                let err = Self::_print_recursive(stream, n.clone(), 0, self.branch_factor);
                if err != MVPError::Success {
                    let _ = writeln!(stream, "malformed tree: {}", error_to_string(err));
                }
                err
            }
        }
    }
    pub fn clear(&mut self, node: &mut Option<Box<Node>>) {
        // Mirrors mvptree_clear() in c_src/mvptree.c. Drop the entire tree.
        // The signature accepts an external Node ptr; we also clear our own.
        *node = None;
        self.node = None;
    }
    pub fn extend_mvpfile(&mut self)-> i32{
        // Mirrors extend_mvpfile() in c_src/mvptree.c. With our buffered
        // write strategy we just grow the in-memory buffer by one page.
        let pgsize = self.pgsize as usize;
        if pgsize == 0 {
            return -2;
        }
        let new_size = self.size as usize + pgsize;
        self.buf.resize(new_size, 0);
        self.size = new_size as i64;
        0
    }
}

impl MVPTree {
    /* Recursive add helper. Returns the (possibly new) node for the subtree
     * being modified or freshly built. Mirrors _mvptree_add() in mvptree.c. */
    fn _add_recursive(
        &self,
        existing: Option<Rc<RefCell<Node>>>,
        points: Vec<Arc<MVPDatapoint>>,
        lvl: usize,
        bf: usize,
        leafcap: usize,
        path_length: usize,
        dist: DistanceFunction,
    ) -> Result<Option<Rc<RefCell<Node>>>, MVPError> {
        if points.is_empty() {
            return Ok(existing);
        }
        let length_m1 = bf.saturating_sub(1);
        let nbpoints = points.len();

        match existing {
            None => {
                // Create new node
                if nbpoints <= leafcap + 2 {
                    // Create leaf
                    let mut leaf = LeafNode::new(leafcap as u32);
                    let (sv1_pos, sv2_pos) = match select_vantage_points_impl(&points, dist) {
                        Ok(pair) => pair,
                        Err(()) => return Err(MVPError::VpNoSelect),
                    };
                    if sv1_pos >= 0 {
                        leaf.sv1 = Some(points[sv1_pos as usize].clone());
                    }
                    if sv2_pos >= 0 {
                        leaf.sv2 = Some(points[sv2_pos as usize].clone());
                    }
                    // Compute distances and store remaining points
                    let sv1_ref = leaf.sv1.as_ref().map(|a| a.clone());
                    let sv2_ref = leaf.sv2.as_ref().map(|a| a.clone());
                    for (i, p) in points.iter().enumerate() {
                        if i as i32 == sv1_pos || i as i32 == sv2_pos {
                            continue;
                        }
                        let d1_val = match &sv1_ref {
                            Some(s) => dist(p, s),
                            None => 0.0,
                        };
                        let d2_val = match &sv2_ref {
                            Some(s) => dist(p, s),
                            None => 0.0,
                        };
                        if is_bad_dist(d1_val) || is_bad_dist(d2_val) {
                            return Err(MVPError::BadDistVal);
                        }
                        leaf.d1.push(d1_val);
                        leaf.d2.push(d2_val);
                        leaf.points.push(p.clone());
                    }
                    leaf.nbpoints = leaf.points.len();
                    let _ = (lvl, path_length, length_m1);
                    Ok(Some(Rc::new(RefCell::new(Node::Leaf(leaf)))))
                } else {
                    // Create internal node
                    let mut internal = InternalNode::new(bf as u32);
                    let (sv1_pos, sv2_pos) = match select_vantage_points_impl(&points, dist) {
                        Ok(pair) => pair,
                        Err(()) => return Err(MVPError::VpNoSelect),
                    };
                    if sv1_pos < 0 || sv2_pos < 0 {
                        return Err(MVPError::VpNoSelect);
                    }
                    internal.sv1 = Some(points[sv1_pos as usize].clone());
                    internal.sv2 = Some(points[sv2_pos as usize].clone());

                    let sv1_dp = internal.sv1.as_ref().unwrap().clone();
                    let sv2_dp = internal.sv2.as_ref().unwrap().clone();

                    // Find splits for sv1 (M1)
                    let m1 = match find_splits_impl(&points, &sv1_dp, dist, length_m1) {
                        Ok(m) => m,
                        Err(()) => return Err(MVPError::NoSplits),
                    };
                    internal.m1 = m1.clone();

                    // Sort points into bf bins using sv1
                    let bins = match sort_points_impl(
                        &points, sv1_pos, sv2_pos, &sv1_dp, dist, bf, &m1,
                    ) {
                        Ok(b) => b,
                        Err(()) => return Err(MVPError::NoSort),
                    };

                    // Initialize child_nodes capacity
                    internal.child_nodes = (0..bf * bf)
                        .map(|_| Rc::new(RefCell::new(Node::Leaf(LeafNode::new(leafcap as u32)))))
                        .collect();
                    // Replace placeholders below; we use Option-style by reassignment.
                    let mut child_nodes: Vec<Option<Rc<RefCell<Node>>>> =
                        (0..bf * bf).map(|_| None).collect();

                    // M2 (bf*(bf-1) entries)
                    internal.m2 = vec![0.0f32; bf * length_m1];

                    for i in 0..bf {
                        // Compute M2 for sv2 within this bin
                        if !bins[i].is_empty() {
                            let m2_part = match find_splits_impl(&bins[i], &sv2_dp, dist, length_m1) {
                                Ok(m) => m,
                                Err(()) => return Err(MVPError::NoSplits),
                            };
                            for k in 0..length_m1 {
                                internal.m2[i * length_m1 + k] = m2_part[k];
                            }
                            // Sort into bf 2nd-tier bins
                            let bins2 = match sort_points_impl(
                                &bins[i],
                                -1,
                                -1,
                                &sv2_dp,
                                dist,
                                bf,
                                &m2_part,
                            ) {
                                Ok(b) => b,
                                Err(()) => return Err(MVPError::NoSort),
                            };
                            for j in 0..bf {
                                let child = self._add_recursive(
                                    None,
                                    bins2[j].clone(),
                                    lvl + 2,
                                    bf,
                                    leafcap,
                                    path_length,
                                    dist,
                                )?;
                                child_nodes[i * bf + j] = child;
                            }
                        }
                    }

                    // Replace placeholders with real children where available
                    internal.child_nodes = child_nodes
                        .into_iter()
                        .map(|opt| {
                            opt.unwrap_or_else(|| {
                                Rc::new(RefCell::new(Node::Leaf(LeafNode::new(leafcap as u32))))
                            })
                        })
                        .collect();

                    Ok(Some(Rc::new(RefCell::new(Node::Internal(internal)))))
                }
            }
            Some(node_rc) => {
                // Existing node - decide leaf vs internal
                let is_leaf = matches!(&*node_rc.borrow(), Node::Leaf(_));
                if is_leaf {
                    // Try to add into existing leaf or rebuild
                    let leaf_full;
                    let existing_n;
                    {
                        let n = node_rc.borrow();
                        if let Node::Leaf(l) = &*n {
                            existing_n = l.nbpoints;
                            leaf_full = l.nbpoints + nbpoints > leafcap;
                        } else {
                            unreachable!();
                        }
                    }
                    if !leaf_full {
                        // Append into existing leaf
                        let mut n_mut = node_rc.borrow_mut();
                        if let Node::Leaf(leaf) = &mut *n_mut {
                            // If sv2 is missing, take first point as sv2
                            let mut start = 0usize;
                            if leaf.sv2.is_none() && !points.is_empty() {
                                leaf.sv2 = Some(points[0].clone());
                                start = 1;
                            }
                            let sv1_clone = leaf.sv1.clone();
                            let sv2_clone = leaf.sv2.clone();
                            for i in start..points.len() {
                                let p = &points[i];
                                let d1_val = match &sv1_clone {
                                    Some(s) => dist(p, s),
                                    None => 0.0,
                                };
                                let d2_val = match &sv2_clone {
                                    Some(s) => dist(p, s),
                                    None => 0.0,
                                };
                                if is_bad_dist(d1_val) || is_bad_dist(d2_val) {
                                    return Err(MVPError::BadDistVal);
                                }
                                leaf.d1.push(d1_val);
                                leaf.d2.push(d2_val);
                                leaf.points.push(p.clone());
                            }
                            leaf.nbpoints = leaf.points.len();
                        }
                        Ok(Some(node_rc.clone()))
                    } else {
                        // Rebuild from combined points
                        let mut combined: Vec<Arc<MVPDatapoint>> = Vec::new();
                        {
                            let n = node_rc.borrow();
                            if let Node::Leaf(leaf) = &*n {
                                if let Some(s) = &leaf.sv1 {
                                    combined.push(s.clone());
                                }
                                if let Some(s) = &leaf.sv2 {
                                    combined.push(s.clone());
                                }
                                for p in &leaf.points {
                                    combined.push(p.clone());
                                }
                            }
                        }
                        for p in points.into_iter() {
                            combined.push(p);
                        }
                        self._add_recursive(None, combined, lvl, bf, leafcap, path_length, dist)
                    }
                } else {
                    // Internal node - distribute points into children
                    let (sv1_clone, sv2_clone, m1_clone, m2_clone, child_count) = {
                        let n = node_rc.borrow();
                        if let Node::Internal(i) = &*n {
                            (
                                i.sv1.clone(),
                                i.sv2.clone(),
                                i.m1.clone(),
                                i.m2.clone(),
                                i.child_nodes.len(),
                            )
                        } else {
                            unreachable!();
                        }
                    };
                    let sv1_dp = match &sv1_clone {
                        Some(s) => s.clone(),
                        None => return Err(MVPError::Unrecognized),
                    };
                    let sv2_dp = match &sv2_clone {
                        Some(s) => s.clone(),
                        None => return Err(MVPError::Unrecognized),
                    };
                    let bins = match sort_points_impl(
                        &points, -1, -1, &sv1_dp, dist, bf, &m1_clone,
                    ) {
                        Ok(b) => b,
                        Err(()) => return Err(MVPError::NoSort),
                    };
                    for i in 0..bf {
                        if bins[i].is_empty() {
                            continue;
                        }
                        let pivot_slice: Vec<f32> = (0..length_m1)
                            .map(|k| m2_clone[i * length_m1 + k])
                            .collect();
                        let bins2 = match sort_points_impl(
                            &bins[i],
                            -1,
                            -1,
                            &sv2_dp,
                            dist,
                            bf,
                            &pivot_slice,
                        ) {
                            Ok(b) => b,
                            Err(()) => return Err(MVPError::NoSort),
                        };
                        for j in 0..bf {
                            let idx = i * bf + j;
                            if idx >= child_count {
                                continue;
                            }
                            let existing_child = {
                                let n = node_rc.borrow();
                                if let Node::Internal(internal) = &*n {
                                    Some(internal.child_nodes[idx].clone())
                                } else {
                                    None
                                }
                            };
                            let new_child = self._add_recursive(
                                existing_child,
                                bins2[j].clone(),
                                lvl + 2,
                                bf,
                                leafcap,
                                path_length,
                                dist,
                            )?;
                            if let Some(nc) = new_child {
                                let mut n_mut = node_rc.borrow_mut();
                                if let Node::Internal(internal) = &mut *n_mut {
                                    internal.child_nodes[idx] = nc;
                                }
                            }
                        }
                    }
                    Ok(Some(node_rc.clone()))
                }
            }
        }
    }

    /* Recursive retrieve helper. Mirrors _mvptree_retrieve() in mvptree.c. */
    fn _retrieve_recursive(
        node: Rc<RefCell<Node>>,
        target: &MVPDatapoint,
        radius: f32,
        knearest: usize,
        results: &mut Vec<Arc<MVPDatapoint>>,
        target_path: &mut Vec<f32>,
        lvl: usize,
        bf: usize,
        path_length: usize,
        dist: DistanceFunction,
    ) -> Result<(), MVPError> {
        let length_m1 = bf.saturating_sub(1);
        let n = node.borrow();
        match &*n {
            Node::Leaf(leaf) => {
                let sv1 = match &leaf.sv1 {
                    Some(s) => s.clone(),
                    None => return Ok(()),
                };
                let d1 = dist(target, &sv1);
                if is_bad_dist(d1) {
                    return Err(MVPError::BadDistVal);
                }
                if lvl < path_length {
                    target_path[lvl] = d1;
                }
                if d1 <= radius {
                    results.push(sv1.clone());
                    if results.len() >= knearest {
                        return Err(MVPError::KNearestCap);
                    }
                }
                if let Some(sv2) = &leaf.sv2 {
                    let d2 = dist(target, sv2);
                    if is_bad_dist(d2) {
                        return Err(MVPError::BadDistVal);
                    }
                    if d2 <= radius {
                        results.push(sv2.clone());
                        if results.len() >= knearest {
                            return Err(MVPError::KNearestCap);
                        }
                    }
                    if lvl + 1 < path_length {
                        target_path[lvl + 1] = d2;
                    }
                    for i in 0..leaf.nbpoints {
                        if d1 - radius <= leaf.d1[i] && d1 + radius >= leaf.d1[i] {
                            if d2 - radius <= leaf.d2[i] && d2 + radius >= leaf.d2[i] {
                                let p = &leaf.points[i];
                                let endpath = if lvl + 1 < path_length {
                                    lvl + 1
                                } else {
                                    path_length
                                };
                                let mut skip = false;
                                for j in 0..endpath {
                                    if j < p.path.len()
                                        && target_path[j] - radius <= p.path[j]
                                        && target_path[j] + radius >= p.path[j]
                                    {
                                        continue;
                                    } else {
                                        skip = true;
                                        break;
                                    }
                                }
                                if !skip {
                                    let d = dist(target, p);
                                    if is_bad_dist(d) {
                                        return Err(MVPError::BadDistVal);
                                    }
                                    if d <= radius {
                                        results.push(p.clone());
                                        if results.len() >= knearest {
                                            return Err(MVPError::KNearestCap);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Ok(())
            }
            Node::Internal(internal) => {
                let sv1 = match &internal.sv1 {
                    Some(s) => s.clone(),
                    None => return Err(MVPError::Unrecognized),
                };
                let sv2 = match &internal.sv2 {
                    Some(s) => s.clone(),
                    None => return Err(MVPError::Unrecognized),
                };
                let d1 = dist(target, &sv1);
                if is_bad_dist(d1) {
                    return Err(MVPError::BadDistVal);
                }
                if d1 <= radius {
                    results.push(sv1.clone());
                    if results.len() >= knearest {
                        return Err(MVPError::KNearestCap);
                    }
                }
                if lvl < path_length {
                    target_path[lvl] = d1;
                }
                let d2 = dist(target, &sv2);
                if is_bad_dist(d2) {
                    return Err(MVPError::BadDistVal);
                }
                if d2 <= radius {
                    results.push(sv2.clone());
                    if results.len() >= knearest {
                        return Err(MVPError::KNearestCap);
                    }
                }
                if lvl + 1 < path_length {
                    target_path[lvl + 1] = d2;
                }
                let child_nodes = internal.child_nodes.clone();
                let m1 = internal.m1.clone();
                let m2 = internal.m2.clone();
                drop(n);
                // Iterate first-level bins
                for i in 0..length_m1 {
                    if d1 - radius <= m1[i] {
                        for j in 0..length_m1 {
                            if d2 - radius <= m2[i * length_m1 + j] {
                                if let Some(child) = child_nodes.get(i * bf + j) {
                                    Self::_retrieve_recursive(
                                        child.clone(),
                                        target,
                                        radius,
                                        knearest,
                                        results,
                                        target_path,
                                        lvl + 2,
                                        bf,
                                        path_length,
                                        dist,
                                    )?;
                                }
                            }
                        }
                        if length_m1 > 0
                            && d2 + radius >= m2[i * length_m1 + length_m1 - 1]
                        {
                            if let Some(child) = child_nodes.get(i * bf + length_m1) {
                                Self::_retrieve_recursive(
                                    child.clone(),
                                    target,
                                    radius,
                                    knearest,
                                    results,
                                    target_path,
                                    lvl + 2,
                                    bf,
                                    path_length,
                                    dist,
                                )?;
                            }
                        }
                    }
                }
                if length_m1 > 0 && d1 + radius >= m1[length_m1 - 1] {
                    for j in 0..length_m1 {
                        if d2 - radius <= m2[length_m1 * length_m1 + j] {
                            if let Some(child) = child_nodes.get(bf * length_m1 + j) {
                                Self::_retrieve_recursive(
                                    child.clone(),
                                    target,
                                    radius,
                                    knearest,
                                    results,
                                    target_path,
                                    lvl + 2,
                                    bf,
                                    path_length,
                                    dist,
                                )?;
                            }
                        }
                    }
                    if length_m1 > 0
                        && d2 + radius >= m2[length_m1 * length_m1 + length_m1 - 1]
                    {
                        if let Some(child) = child_nodes.get(bf * length_m1 + length_m1) {
                            Self::_retrieve_recursive(
                                child.clone(),
                                target,
                                radius,
                                knearest,
                                results,
                                target_path,
                                lvl + 2,
                                bf,
                                path_length,
                                dist,
                            )?;
                        }
                    }
                }
                Ok(())
            }
        }
    }

    /* Serialize a single datapoint into buf.  Mirrors write_datapoint() in
     * mvptree.c. Returns the start position (offset). */
    fn _write_datapoint(
        buf: &mut Vec<u8>,
        dp: Option<&MVPDatapoint>,
        path_length: usize,
        datatype: MVPDataType,
    ) -> u64 {
        let start = buf.len() as u64;
        match dp {
            None => {
                buf.push(0u8); // active
                buf.extend_from_slice(&0u32.to_le_bytes()); // bytelength
            }
            Some(dp) => {
                let active: u8 = 1;
                let id_bytes = dp.id.as_bytes();
                let idlen: u8 = id_bytes.len().min(255) as u8;
                let datalength: u32 = dp.datalen as u32;
                let type_size = datatype as usize;
                let bytelength: u32 = (1
                    + idlen as usize
                    + 4
                    + (datalength as usize) * type_size
                    + path_length * 4) as u32;
                buf.push(active);
                buf.extend_from_slice(&bytelength.to_le_bytes());
                buf.push(idlen);
                buf.extend_from_slice(&id_bytes[..idlen as usize]);
                buf.extend_from_slice(&datalength.to_le_bytes());
                let data_byte_count = (datalength as usize) * type_size;
                if dp.data.len() >= data_byte_count {
                    buf.extend_from_slice(&dp.data[..data_byte_count]);
                } else {
                    buf.extend_from_slice(&dp.data);
                    for _ in dp.data.len()..data_byte_count {
                        buf.push(0);
                    }
                }
                for i in 0..path_length {
                    let f = if i < dp.path.len() { dp.path[i] } else { 0.0f32 };
                    buf.extend_from_slice(&f.to_le_bytes());
                }
            }
        }
        start
    }

    /* Recursively write a node to buf. Mirrors _mvptree_write() in mvptree.c. */
    fn _write_node_recursive(
        node: Rc<RefCell<Node>>,
        buf: &mut Vec<u8>,
        error: &mut MVPError,
        path_length: usize,
        bf: usize,
        leafcap: usize,
        datatype: MVPDataType,
    ) -> u64 {
        let start = buf.len() as u64;
        let n = node.borrow();
        match &*n {
            Node::Leaf(leaf) => {
                buf.push(NodeType::LeafNode as u8);
                Self::_write_datapoint(buf, leaf.sv1.as_deref(), path_length, datatype);
                Self::_write_datapoint(buf, leaf.sv2.as_deref(), path_length, datatype);
                let nbpoints: u32 = leaf.nbpoints as u32;
                buf.extend_from_slice(&nbpoints.to_le_bytes());
                // Reserve space for leafcap entries of (d1, d2, offset)
                let entry_size = 4 + 4 + 8; // f32 + f32 + u64 (off_t)
                let table_pos = buf.len();
                for _ in 0..leafcap {
                    for _ in 0..entry_size {
                        buf.push(0);
                    }
                }
                for i in 0..leaf.nbpoints {
                    let row_off = table_pos + i * entry_size;
                    let d1_bytes = leaf.d1[i].to_le_bytes();
                    let d2_bytes = leaf.d2[i].to_le_bytes();
                    buf[row_off..row_off + 4].copy_from_slice(&d1_bytes);
                    buf[row_off + 4..row_off + 8].copy_from_slice(&d2_bytes);
                    let offset = Self::_write_datapoint(
                        buf,
                        Some(&leaf.points[i]),
                        path_length,
                        datatype,
                    );
                    buf[row_off + 8..row_off + 16]
                        .copy_from_slice(&offset.to_le_bytes());
                }
            }
            Node::Internal(internal) => {
                buf.push(NodeType::InternalNode as u8);
                Self::_write_datapoint(buf, internal.sv1.as_deref(), path_length, datatype);
                Self::_write_datapoint(buf, internal.sv2.as_deref(), path_length, datatype);
                let length_m1 = bf.saturating_sub(1);
                let length_m2 = bf * length_m1;
                for i in 0..length_m1 {
                    let v = if i < internal.m1.len() { internal.m1[i] } else { 0.0 };
                    buf.extend_from_slice(&v.to_le_bytes());
                }
                for i in 0..length_m2 {
                    let v = if i < internal.m2.len() { internal.m2[i] } else { 0.0 };
                    buf.extend_from_slice(&v.to_le_bytes());
                }
                let fanout = bf * bf;
                let entry_size = 1 + 8;
                let table_pos = buf.len();
                for _ in 0..fanout {
                    for _ in 0..entry_size {
                        buf.push(0);
                    }
                }
                let children = internal.child_nodes.clone();
                drop(n);
                for i in 0..fanout {
                    let row_off = table_pos + i * entry_size;
                    let offset = if i < children.len() {
                        Self::_write_node_recursive(
                            children[i].clone(),
                            buf,
                            error,
                            path_length,
                            bf,
                            leafcap,
                            datatype,
                        )
                    } else {
                        0
                    };
                    buf[row_off] = 0; // fileno
                    buf[row_off + 1..row_off + 9]
                        .copy_from_slice(&offset.to_le_bytes());
                }
            }
        }
        let _ = error;
        start
    }

    /* Recursively print a node. Mirrors _mvptree_print() in mvptree.c. */
    fn _print_recursive(
        stream: &mut dyn Write,
        node: Rc<RefCell<Node>>,
        lvl: usize,
        bf: usize,
    ) -> MVPError {
        let length_m1 = bf.saturating_sub(1);
        let length_m2 = bf;
        let fanout = bf * bf;
        let n = node.borrow();
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
                MVPError::Success
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
                let children = internal.child_nodes.clone();
                drop(n);
                for i in 0..fanout {
                    if i < children.len() {
                        let err = Self::_print_recursive(stream, children[i].clone(), lvl + 2, bf);
                        if err != MVPError::Success {
                            return err;
                        }
                    }
                }
                MVPError::Success
            }
        }
    }
}

pub fn mvptree_read(filename: &str, distance_function: DistanceFunction) -> Result<MVPTree, MVPError> {
    // Mirrors mvptree_read() in c_src/mvptree.c. We read the whole file
    // into memory and parse it.
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
    // Parse header
    let tag_bytes = TAG.as_bytes();
    if buf.len() < tag_bytes.len() || &buf[..tag_bytes.len()] != tag_bytes {
        return Err(MVPError::FileOpen);
    }
    let mut pos = tag_bytes.len() + 1; // tag + null
    pos += 4; // version
    if buf.len() < pos + 4 {
        return Err(MVPError::FileOpen);
    }
    let bf = buf[pos] as usize;
    let pl = buf[pos + 1] as usize;
    let lc = buf[pos + 2] as usize;
    let ht = buf[pos + 3];
    let datatype = match ht {
        1 => MVPDataType::ByteArray,
        2 => MVPDataType::UInt16Array,
        4 => MVPDataType::UInt32Array,
        8 => MVPDataType::UInt64Array,
        _ => return Err(MVPError::Unrecognized),
    };

    let mut tree = MVPTree::new(bf, pl, lc, datatype, distance_function);

    // Read root node from HEADER_SIZE
    let mut cursor = HEADER_SIZE;
    let node = match read_node_recursive(&buf, &mut cursor, bf, pl, lc, datatype) {
        Ok(n) => n,
        Err(e) => return Err(e),
    };
    tree.node = node;
    Ok(tree)
}

/* Read a single datapoint at cursor; advances cursor. */
fn read_datapoint(
    buf: &[u8],
    cursor: &mut usize,
    path_length: usize,
    datatype: MVPDataType,
) -> Option<Arc<MVPDatapoint>> {
    if *cursor + 5 > buf.len() {
        return None;
    }
    let active = buf[*cursor];
    *cursor += 1;
    let mut bl_bytes = [0u8; 4];
    bl_bytes.copy_from_slice(&buf[*cursor..*cursor + 4]);
    let bytelength = u32::from_le_bytes(bl_bytes);
    *cursor += 4;
    if active == 0 && bytelength == 0 {
        return None;
    }
    if *cursor + 1 > buf.len() {
        return None;
    }
    let idlen = buf[*cursor] as usize;
    *cursor += 1;
    if *cursor + idlen > buf.len() {
        return None;
    }
    let id = String::from_utf8_lossy(&buf[*cursor..*cursor + idlen]).to_string();
    *cursor += idlen;
    if *cursor + 4 > buf.len() {
        return None;
    }
    let mut dl_bytes = [0u8; 4];
    dl_bytes.copy_from_slice(&buf[*cursor..*cursor + 4]);
    let datalength = u32::from_le_bytes(dl_bytes) as usize;
    *cursor += 4;
    let type_size = datatype as usize;
    let data_byte_count = datalength * type_size;
    if *cursor + data_byte_count > buf.len() {
        return None;
    }
    let data = buf[*cursor..*cursor + data_byte_count].to_vec();
    *cursor += data_byte_count;
    let mut path = Vec::with_capacity(path_length);
    for _ in 0..path_length {
        if *cursor + 4 > buf.len() {
            return None;
        }
        let mut fb = [0u8; 4];
        fb.copy_from_slice(&buf[*cursor..*cursor + 4]);
        path.push(f32::from_le_bytes(fb));
        *cursor += 4;
    }
    Some(Arc::new(MVPDatapoint {
        id,
        data,
        path,
        datalen: datalength,
        data_type: datatype,
    }))
}

/* Recursively read a node starting at cursor. Mirrors _mvptree_read_node()
 * in mvptree.c. */
fn read_node_recursive(
    buf: &[u8],
    cursor: &mut usize,
    bf: usize,
    path_length: usize,
    leafcap: usize,
    datatype: MVPDataType,
) -> Result<Option<Rc<RefCell<Node>>>, MVPError> {
    if *cursor >= buf.len() {
        return Ok(None);
    }
    let node_type_byte = buf[*cursor];
    *cursor += 1;
    if node_type_byte == NodeType::LeafNode as u8 {
        let mut leaf = LeafNode::new(leafcap as u32);
        leaf.sv1 = read_datapoint(buf, cursor, path_length, datatype);
        leaf.sv2 = read_datapoint(buf, cursor, path_length, datatype);
        if *cursor + 4 > buf.len() {
            return Err(MVPError::FileOpen);
        }
        let mut nb_bytes = [0u8; 4];
        nb_bytes.copy_from_slice(&buf[*cursor..*cursor + 4]);
        let nbpoints = u32::from_le_bytes(nb_bytes) as usize;
        *cursor += 4;
        leaf.nbpoints = nbpoints;
        let table_pos = *cursor;
        let entry_size = 4 + 4 + 8;
        for i in 0..nbpoints {
            let row_off = table_pos + i * entry_size;
            if row_off + entry_size > buf.len() {
                return Err(MVPError::FileOpen);
            }
            let mut d1b = [0u8; 4];
            d1b.copy_from_slice(&buf[row_off..row_off + 4]);
            let mut d2b = [0u8; 4];
            d2b.copy_from_slice(&buf[row_off + 4..row_off + 8]);
            let mut offb = [0u8; 8];
            offb.copy_from_slice(&buf[row_off + 8..row_off + 16]);
            leaf.d1.push(f32::from_le_bytes(d1b));
            leaf.d2.push(f32::from_le_bytes(d2b));
            let offset = u64::from_le_bytes(offb) as usize;
            let mut child_cursor = offset;
            if let Some(p) = read_datapoint(buf, &mut child_cursor, path_length, datatype) {
                leaf.points.push(p);
            }
        }
        Ok(Some(Rc::new(RefCell::new(Node::Leaf(leaf)))))
    } else if node_type_byte == NodeType::InternalNode as u8 {
        let mut internal = InternalNode::new(bf as u32);
        internal.sv1 = read_datapoint(buf, cursor, path_length, datatype);
        internal.sv2 = read_datapoint(buf, cursor, path_length, datatype);
        let length_m1 = bf.saturating_sub(1);
        let length_m2 = bf * length_m1;
        for i in 0..length_m1 {
            if *cursor + 4 > buf.len() {
                return Err(MVPError::FileOpen);
            }
            let mut fb = [0u8; 4];
            fb.copy_from_slice(&buf[*cursor..*cursor + 4]);
            if i < internal.m1.len() {
                internal.m1[i] = f32::from_le_bytes(fb);
            }
            *cursor += 4;
        }
        for i in 0..length_m2 {
            if *cursor + 4 > buf.len() {
                return Err(MVPError::FileOpen);
            }
            let mut fb = [0u8; 4];
            fb.copy_from_slice(&buf[*cursor..*cursor + 4]);
            if i < internal.m2.len() {
                internal.m2[i] = f32::from_le_bytes(fb);
            }
            *cursor += 4;
        }
        let fanout = bf * bf;
        let entry_size = 1 + 8;
        let table_pos = *cursor;
        let mut children: Vec<Rc<RefCell<Node>>> = Vec::with_capacity(fanout);
        for i in 0..fanout {
            let row_off = table_pos + i * entry_size;
            if row_off + entry_size > buf.len() {
                return Err(MVPError::FileOpen);
            }
            let mut offb = [0u8; 8];
            offb.copy_from_slice(&buf[row_off + 1..row_off + 9]);
            let offset = u64::from_le_bytes(offb) as usize;
            let mut child_cursor = offset;
            let child = if offset > 0 && offset < buf.len() {
                match read_node_recursive(buf, &mut child_cursor, bf, path_length, leafcap, datatype) {
                    Ok(Some(c)) => c,
                    _ => Rc::new(RefCell::new(Node::Leaf(LeafNode::new(leafcap as u32)))),
                }
            } else {
                Rc::new(RefCell::new(Node::Leaf(LeafNode::new(leafcap as u32))))
            };
            children.push(child);
        }
        internal.child_nodes = children;
        Ok(Some(Rc::new(RefCell::new(Node::Internal(internal)))))
    } else {
        Err(MVPError::Unrecognized)
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
    pub fn select_vantage_points(&mut self, nb:u32, sv1_pos: i32, sv2_pos: i32, dist: DistanceFunction) -> i32 {
        // Awkward signature; the C version operates on an array of points.
        // Without an array here there is nothing to compute; return success
        // as a placeholder. (See select_vantage_points_impl for the real
        // logic used by MVPTree::add.)
        let _ = (nb, sv1_pos, sv2_pos, dist);
        0
    }
    pub fn find_splits(&mut self, nb:u32, vp:&MVPDatapoint, tree: &MVPTree,  lengthM: u32) -> f32{
        // Awkward signature; the real logic lives in find_splits_impl.
        // Compute and return the distance from this point to the vp under
        // the tree's distance function as a representative value.
        let _ = (nb, lengthM);
        (tree.distance_function)(self, vp)
    }
    pub fn sort_points(&mut self, nb:u32, sv1_pos: i32, sv2_pos: i32, vp: &MVPDatapoint, tree: &MVPTree, counts: &mut Vec<Vec<i32>>, pivots: Vec<f32>) -> Vec<Vec<Vec<Arc<MVPDatapoint>>>> {
        // Awkward signature; the real logic lives in sort_points_impl.
        let _ = (nb, sv1_pos, sv2_pos, vp, tree, counts, pivots);
        Vec::new()
    }
    pub fn find_distance_range_for_vp(&mut self, nb:u32, vp: &MVPDatapoint, tree: &MVPTree, level: i32) -> i32 {
        // Compute distance and store into path[level] when applicable.
        // Mirrors find_distance_range_for_vp() for a single point.
        let d = (tree.distance_function)(self, vp);
        if d.is_nan() || d < 0.0 {
            return -2;
        }
        if level >= 0 && (level as usize) < tree.path_length {
            if (level as usize) >= self.path.len() {
                self.path.resize(tree.path_length, 0.0);
            }
            self.path[level as usize] = d;
        }
        let _ = nb;
        0
    }
    pub fn write(&self, tree: &MVPTree) -> i64 {
        // Returns a representative byte count (not a file offset) since this
        // method does not have access to the buffer used by MVPTree::write.
        let _ = tree;
        let id_bytes = self.id.as_bytes().len() as i64;
        let type_size = self.data_type as i64;
        1 + 4 + 1 + id_bytes + 4 + (self.datalen as i64) * type_size
            + (tree.path_length as i64) * 4
    }
}
pub fn error_to_string(error: MVPError) -> &'static str {
    // Mirrors mvp_errstr() in c_src/mvptree.c.
    let idx = error as usize;
    if idx < ERROR_MSGS.len() {
        ERROR_MSGS[idx]
    } else {
        "unknown error"
    }
}
