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

fn is_nan_or_neg(x: f32) -> bool {
    x.is_nan() || x < 0.0
}

fn select_vantage_points(points: &[Arc<MVPDatapoint>], dist: DistanceFunction) -> Result<(usize, usize), ()> {
    if points.is_empty() { return Err(()); }
    let mut sv1_pos: usize = 0;
    let mut sv2_pos: Option<usize> = None;
    let mut max_dist = 0.0f32;
    for i in 0..points.len() {
        for j in (i + 1)..points.len() {
            let d = dist(&points[i], &points[j]);
            if is_nan_or_neg(d) { return Err(()); }
            if d > max_dist {
                max_dist = d;
                sv1_pos = i;
                sv2_pos = Some(j);
            }
        }
    }
    match sv2_pos {
        Some(s2) => Ok((sv1_pos, s2)),
        None => {
            if points.len() >= 1 { Ok((0, 0)) }
            else { Err(()) }
        }
    }
}

fn find_splits(points: &[Arc<MVPDatapoint>], vp: &MVPDatapoint, dist: DistanceFunction, length_m: usize) -> Result<Vec<f32>, ()> {
    if points.is_empty() || length_m == 0 { return Ok(vec![0.0; length_m]); }
    let nb = points.len();
    let mut dists: Vec<f32> = Vec::with_capacity(nb);
    for p in points {
        let d = dist(p, vp);
        if is_nan_or_neg(d) { return Err(()); }
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

fn sort_into_bins(
    points: &[Arc<MVPDatapoint>],
    skip1: Option<usize>,
    skip2: Option<usize>,
    vp: &MVPDatapoint,
    dist: DistanceFunction,
    bf: usize,
    pivots: &[f32],
) -> Result<(Vec<Vec<Arc<MVPDatapoint>>>, Vec<usize>), ()> {
    let length_m1 = bf - 1;
    let mut bins: Vec<Vec<Arc<MVPDatapoint>>> = (0..bf).map(|_| Vec::new()).collect();
    for (i, p) in points.iter().enumerate() {
        if Some(i) == skip1 || Some(i) == skip2 { continue; }
        let d = dist(vp, p);
        if is_nan_or_neg(d) { return Err(()); }
        let mut placed = false;
        for k in 0..length_m1 {
            if d <= pivots[k] {
                bins[k].push(p.clone());
                placed = true;
                break;
            }
        }
        if !placed {
            bins[length_m1].push(p.clone());
        }
    }
    let counts: Vec<usize> = bins.iter().map(|b| b.len()).collect();
    Ok((bins, counts))
}

fn find_distance_range_for_vp(points: &[Arc<MVPDatapoint>], vp: &MVPDatapoint, dist: DistanceFunction, lvl: usize, path_length: usize) -> Result<Vec<Arc<MVPDatapoint>>, ()> {
    let mut result = Vec::with_capacity(points.len());
    for p in points {
        let d = dist(vp, p);
        if is_nan_or_neg(d) { return Err(()); }
        let mut new_p = (**p).clone();
        if lvl < path_length {
            while new_p.path.len() <= lvl { new_p.path.push(0.0); }
            new_p.path[lvl] = d;
        }
        result.push(Arc::new(new_p));
    }
    Ok(result)
}

fn _mvptree_add(
    tree_bf: usize, tree_pl: usize, tree_lc: usize, dist: DistanceFunction,
    node: Option<Rc<RefCell<Node>>>, points: Vec<Arc<MVPDatapoint>>, lvl: usize,
) -> Result<Option<Rc<RefCell<Node>>>, MVPError> {
    let nbpoints = points.len();
    if nbpoints == 0 { return Ok(node); }
    let bf = tree_bf;
    let length_m1 = bf - 1;

    if node.is_none() {
        if nbpoints <= tree_lc + 2 {
            // create leaf
            let (sv1_pos, sv2_pos) = select_vantage_points(&points, dist).map_err(|_| MVPError::VpNoSelect)?;
            let sv1 = points[sv1_pos].clone();
            let sv2_opt = if sv2_pos != sv1_pos { Some(points[sv2_pos].clone()) } else { None };

            let points_updated = find_distance_range_for_vp(&points, &sv1, dist, lvl, tree_pl).map_err(|_| MVPError::NoSv1Range)?;

            let points_updated = if let Some(ref sv2) = sv2_opt {
                // need to use updated sv2 from points_updated
                let sv2_updated = points_updated[sv2_pos].clone();
                find_distance_range_for_vp(&points_updated, &sv2_updated, dist, lvl + 1, tree_pl).map_err(|_| MVPError::NoSv2Range)?
            } else {
                points_updated
            };

            let sv1_final = points_updated[sv1_pos].clone();
            let sv2_final = if sv2_pos != sv1_pos { Some(points_updated[sv2_pos].clone()) } else { None };

            let mut leaf = LeafNode::new(bf as u32);
            leaf.sv1 = Some(sv1_final.clone());
            leaf.sv2 = sv2_final.clone();

            for (i, p) in points_updated.iter().enumerate() {
                if i == sv1_pos || (sv2_pos != sv1_pos && i == sv2_pos) { continue; }
                let d1 = dist(p, &sv1_final);
                let d2 = if let Some(ref s2) = sv2_final { dist(p, s2) } else { 0.0 };
                leaf.d1.push(d1);
                leaf.d2.push(d2);
                leaf.points.push(p.clone());
            }
            leaf.nbpoints = leaf.points.len();
            Ok(Some(Rc::new(RefCell::new(Node::Leaf(leaf)))))
        } else {
            // create internal node
            let (sv1_pos, sv2_pos) = select_vantage_points(&points, dist).map_err(|_| MVPError::VpNoSelect)?;

            let points_updated = find_distance_range_for_vp(&points, &points[sv1_pos], dist, lvl, tree_pl).map_err(|_| MVPError::NoSv1Range)?;

            let sv1_final = points_updated[sv1_pos].clone();
            let sv2_final = points_updated[sv2_pos].clone();

            let m1 = find_splits(&points_updated, &sv1_final, dist, length_m1).map_err(|_| MVPError::NoSplits)?;

            let (bins, bin_lengths) = sort_into_bins(&points_updated, Some(sv1_pos), Some(sv2_pos), &sv1_final, dist, bf, &m1).map_err(|_| MVPError::NoSort)?;

            let mut all_m2 = vec![0.0f32; length_m1 * bf];
            let mut all_children: Vec<Option<Rc<RefCell<Node>>>> = Vec::new();

            for i in 0..bf {
                let bin_updated = find_distance_range_for_vp(&bins[i], &sv2_final, dist, lvl + 1, tree_pl).map_err(|_| MVPError::NoSv2Range)?;

                let m2_part = find_splits(&bin_updated, &sv2_final, dist, length_m1).map_err(|_| MVPError::NoSplits)?;
                for k in 0..length_m1 {
                    all_m2[i * length_m1 + k] = m2_part[k];
                }

                let (bins2, _bin2_lengths) = sort_into_bins(&bin_updated, None, None, &sv2_final, dist, bf, &m2_part).map_err(|_| MVPError::NoSort)?;

                for j in 0..bf {
                    let child = _mvptree_add(tree_bf, tree_pl, tree_lc, dist, None, bins2[j].clone(), lvl + 2)?;
                    all_children.push(child);
                }
            }

            let mut internal = InternalNode::new(bf as u32);
            internal.sv1 = Some(sv1_final);
            internal.sv2 = Some(sv2_final);
            internal.m1 = m1;
            internal.m2 = all_m2;
            for child_opt in all_children {
                if let Some(c) = child_opt {
                    internal.child_nodes.push(c);
                } else {
                    internal.child_nodes.push(Rc::new(RefCell::new(Node::Leaf(LeafNode::new(bf as u32)))));
                }
            }
            Ok(Some(Rc::new(RefCell::new(Node::Internal(internal)))))
        }
    } else {
        // node already exists
        let node_rc = node.unwrap();
        let is_leaf = matches!(&*node_rc.borrow(), Node::Leaf(_));

        if is_leaf {
            let (current_nb, has_room, has_sv2) = {
                let borrow = node_rc.borrow();
                if let Node::Leaf(ref leaf) = *borrow {
                    (leaf.nbpoints, leaf.nbpoints + nbpoints <= tree_lc, leaf.sv2.is_some())
                } else { unreachable!() }
            };

            if has_room {
                {
                    let mut borrow = node_rc.borrow_mut();
                    if let Node::Leaf(ref mut leaf) = *borrow {
                        let sv1 = leaf.sv1.clone().unwrap();
                        let points_updated = find_distance_range_for_vp(&points, &sv1, dist, lvl, tree_pl).map_err(|_| MVPError::NoSv1Range)?;

                        let mut pos = 0;
                        if !has_sv2 && !points_updated.is_empty() {
                            leaf.sv2 = Some(points_updated[0].clone());
                            pos = 1;
                        }

                        let sv2 = leaf.sv2.clone().unwrap();
                        let points_updated2 = find_distance_range_for_vp(&points_updated, &sv2, dist, lvl + 1, tree_pl).map_err(|_| MVPError::NoSv2Range)?;

                        if !has_sv2 && !points_updated2.is_empty() {
                            leaf.sv2 = Some(points_updated2[0].clone());
                        }

                        let sv1_ref = leaf.sv1.clone().unwrap();
                        let sv2_ref = leaf.sv2.clone().unwrap();
                        for i in pos..points_updated2.len() {
                            leaf.d1.push(dist(&points_updated2[i], &sv1_ref));
                            leaf.d2.push(dist(&points_updated2[i], &sv2_ref));
                            leaf.points.push(points_updated2[i].clone());
                        }
                        leaf.nbpoints = leaf.points.len();
                    }
                }
                Ok(Some(node_rc))
            } else {
                // not enough room - collect all points and rebuild
                let mut tmp_pts: Vec<Arc<MVPDatapoint>> = Vec::new();
                {
                    let borrow = node_rc.borrow();
                    if let Node::Leaf(ref leaf) = *borrow {
                        if let Some(ref s1) = leaf.sv1 { tmp_pts.push(s1.clone()); }
                        if let Some(ref s2) = leaf.sv2 { tmp_pts.push(s2.clone()); }
                        for p in &leaf.points { tmp_pts.push(p.clone()); }
                    }
                }
                tmp_pts.extend(points);
                drop(node_rc);
                _mvptree_add(tree_bf, tree_pl, tree_lc, dist, None, tmp_pts, lvl)
            }
        } else {
            // internal node - recurse
            let sv1 = {
                let borrow = node_rc.borrow();
                if let Node::Internal(ref int) = *borrow { int.sv1.clone().unwrap() } else { unreachable!() }
            };

            let points_updated = find_distance_range_for_vp(&points, &sv1, dist, lvl, tree_pl).map_err(|_| MVPError::NoSv1Range)?;

            let m1 = {
                let borrow = node_rc.borrow();
                if let Node::Internal(ref int) = *borrow { int.m1.clone() } else { unreachable!() }
            };

            let (bins, bin_lengths) = sort_into_bins(&points_updated, None, None, &sv1, dist, bf, &m1).map_err(|_| MVPError::NoSort)?;

            for i in 0..bf {
                if bin_lengths[i] == 0 { continue; }

                let sv2 = {
                    let borrow = node_rc.borrow();
                    if let Node::Internal(ref int) = *borrow { int.sv2.clone().unwrap() } else { unreachable!() }
                };

                let bin_updated = find_distance_range_for_vp(&bins[i], &sv2, dist, lvl + 1, tree_pl).map_err(|_| MVPError::NoSv2Range)?;

                let m2_slice = {
                    let borrow = node_rc.borrow();
                    if let Node::Internal(ref int) = *borrow {
                        int.m2[i * length_m1..(i + 1) * length_m1].to_vec()
                    } else { unreachable!() }
                };

                let (bins2, _) = sort_into_bins(&bin_updated, None, None, &sv2, dist, bf, &m2_slice).map_err(|_| MVPError::NoSort)?;

                for j in 0..bf {
                    let idx = i * bf + j;
                    let existing_child = {
                        let borrow = node_rc.borrow();
                        if let Node::Internal(ref int) = *borrow {
                            if idx < int.child_nodes.len() { Some(int.child_nodes[idx].clone()) } else { None }
                        } else { unreachable!() }
                    };
                    let child = _mvptree_add(tree_bf, tree_pl, tree_lc, dist, existing_child, bins2[j].clone(), lvl + 2)?;
                    if let Some(c) = child {
                        let mut borrow = node_rc.borrow_mut();
                        if let Node::Internal(ref mut int) = *borrow {
                            while int.child_nodes.len() <= idx {
                                int.child_nodes.push(Rc::new(RefCell::new(Node::Leaf(LeafNode::new(bf as u32)))));
                            }
                            int.child_nodes[idx] = c;
                        }
                    }
                }
            }
            Ok(Some(node_rc))
        }
    }
}

fn _mvptree_retrieve(
    tree_bf: usize, tree_pl: usize, k: usize, dist: DistanceFunction,
    root_node: &Rc<RefCell<Node>>,
    node: &Rc<RefCell<Node>>, target: &mut MVPDatapoint, radius: f32,
    results: &mut Vec<MVPDatapoint>, lvl: usize,
) -> MVPError {
    let bf = tree_bf;
    let length_m1 = bf - 1;
    let borrow = node.borrow();

    match &*borrow {
        Node::Leaf(leaf) => {
            let sv1 = match &leaf.sv1 { Some(s) => s, None => return MVPError::Success };
            let d1 = dist(target, sv1);
            if is_nan_or_neg(d1) { return MVPError::BadDistVal; }
            if lvl < tree_pl {
                while target.path.len() <= lvl { target.path.push(0.0); }
                target.path[lvl] = d1;
            }
            if d1 <= radius {
                results.push((**sv1).clone());
                if results.len() >= k { return MVPError::KNearestCap; }
            }

            // Check if root has sv2 (mirrors C: tree->node->leaf.sv2)
            let root_has_sv2 = {
                let rb = root_node.borrow();
                match &*rb {
                    Node::Leaf(rl) => rl.sv2.is_some(),
                    Node::Internal(_) => true,
                }
            };

            if root_has_sv2 {
                if let Some(ref sv2) = leaf.sv2 {
                    let d2 = dist(target, sv2);
                    if is_nan_or_neg(d2) { return MVPError::BadDistVal; }
                    if d2 <= radius {
                        results.push((**sv2).clone());
                        if results.len() >= k { return MVPError::KNearestCap; }
                    }
                    if lvl + 1 < tree_pl {
                        while target.path.len() <= lvl + 1 { target.path.push(0.0); }
                        target.path[lvl + 1] = d2;
                    }

                    for i in 0..leaf.nbpoints {
                        if d1 - radius <= leaf.d1[i] && d1 + radius >= leaf.d1[i] {
                            if d2 - radius <= leaf.d2[i] && d2 + radius >= leaf.d2[i] {
                                let endpath = if lvl + 1 < tree_pl { lvl + 1 } else { tree_pl };
                                let mut skip = false;
                                for j in 0..endpath {
                                    if j < target.path.len() && j < leaf.points[i].path.len() {
                                        if target.path[j] - radius <= leaf.points[i].path[j]
                                            && target.path[j] + radius >= leaf.points[i].path[j]
                                        { continue; } else { skip = true; break; }
                                    }
                                }
                                if !skip {
                                    let d = dist(target, &leaf.points[i]);
                                    if is_nan_or_neg(d) { return MVPError::BadDistVal; }
                                    if d <= radius {
                                        results.push((*leaf.points[i]).clone());
                                        if results.len() >= k { return MVPError::KNearestCap; }
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
            let sv1 = internal.sv1.as_ref().unwrap();
            let d1 = dist(target, sv1);
            if is_nan_or_neg(d1) { return MVPError::BadDistVal; }
            if d1 <= radius {
                results.push((**sv1).clone());
                if results.len() >= k { return MVPError::KNearestCap; }
            }
            if lvl < tree_pl {
                while target.path.len() <= lvl { target.path.push(0.0); }
                target.path[lvl] = d1;
            }

            let sv2 = internal.sv2.as_ref().unwrap();
            let d2 = dist(target, sv2);
            if is_nan_or_neg(d2) { return MVPError::BadDistVal; }
            if d2 <= radius {
                results.push((**sv2).clone());
                if results.len() >= k { return MVPError::KNearestCap; }
            }
            if lvl + 1 < tree_pl {
                while target.path.len() <= lvl + 1 { target.path.push(0.0); }
                target.path[lvl + 1] = d2;
            }

            // check <= each 1st level bin
            for i in 0..length_m1 {
                if d1 - radius <= internal.m1[i] {
                    for j in 0..length_m1 {
                        if d2 - radius <= internal.m2[i * length_m1 + j] {
                            if i * bf + j < internal.child_nodes.len() {
                                let err = _mvptree_retrieve(tree_bf, tree_pl, k, dist, root_node, &internal.child_nodes[i * bf + j], target, radius, results, lvl + 2);
                                if err != MVPError::Success { return err; }
                            }
                        }
                    }
                    // check >= last 2nd level bin
                    if d2 + radius >= internal.m2[i * length_m1 + length_m1 - 1] {
                        let idx = i * bf + length_m1;
                        if idx < internal.child_nodes.len() {
                            let err = _mvptree_retrieve(tree_bf, tree_pl, k, dist, root_node, &internal.child_nodes[idx], target, radius, results, lvl + 2);
                            if err != MVPError::Success { return err; }
                        }
                    }
                }
            }

            // check >= last 1st level bin
            if d1 + radius >= internal.m1[length_m1 - 1] {
                for j in 0..length_m1 {
                    if d2 - radius <= internal.m2[length_m1 * length_m1 + j] {
                        let idx = bf * length_m1 + j;
                        if idx < internal.child_nodes.len() {
                            let err = _mvptree_retrieve(tree_bf, tree_pl, k, dist, root_node, &internal.child_nodes[idx], target, radius, results, lvl + 2);
                            if err != MVPError::Success { return err; }
                        }
                    }
                }
                if d2 + radius >= internal.m2[length_m1 * length_m1 + length_m1 - 1] {
                    let idx = bf * length_m1 + length_m1;
                    if idx < internal.child_nodes.len() {
                        let err = _mvptree_retrieve(tree_bf, tree_pl, k, dist, root_node, &internal.child_nodes[idx], target, radius, results, lvl + 2);
                        if err != MVPError::Success { return err; }
                    }
                }
            }
            MVPError::Success
        }
    }
}

fn _mvptree_print(stream: &mut dyn Write, tree_bf: usize, node: &Option<Rc<RefCell<Node>>>, lvl: usize) -> MVPError {
    let bf = tree_bf;
    let length_m1 = bf - 1;
    let length_m2 = bf;
    let fanout = bf * bf;

    match node {
        None => {
            let _ = writeln!(stream, "NULL{}", lvl);
            MVPError::Success
        }
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
                            let err = _mvptree_print(stream, tree_bf, &Some(internal.child_nodes[i].clone()), lvl + 2);
                            if err != MVPError::Success { return err; }
                        } else {
                            let _ = writeln!(stream, "NULL{}", lvl + 2);
                        }
                    }
                    MVPError::Success
                }
            }
        }
    }
}

fn write_datapoint_to_buf(dp: Option<&Arc<MVPDatapoint>>, buf: &mut Vec<u8>, pos: &mut usize, path_length: usize) {
    match dp {
        None => {
            let active: u8 = 0;
            let bytelength: u32 = 0;
            buf_write_u8(buf, pos, active);
            buf_write_u32(buf, pos, bytelength);
        }
        Some(dp) => {
            let active: u8 = 1;
            let idlen = dp.id.len() as u8;
            let datalength = dp.datalen as u32;
            let type_size = dp.data_type as u32;
            let bytelength: u32 = 1 + idlen as u32 + 4 + datalength * type_size + (path_length as u32) * 4;
            buf_write_u8(buf, pos, active);
            buf_write_u32(buf, pos, bytelength);
            buf_write_u8(buf, pos, idlen);
            buf_write_bytes(buf, pos, dp.id.as_bytes());
            buf_write_u32(buf, pos, datalength);
            buf_write_bytes(buf, pos, &dp.data[..(datalength as usize * type_size as usize)]);
            for i in 0..path_length {
                let v = if i < dp.path.len() { dp.path[i] } else { 0.0 };
                buf_write_f32(buf, pos, v);
            }
        }
    }
}

fn buf_ensure(buf: &mut Vec<u8>, pos: usize, need: usize) {
    if pos + need > buf.len() {
        buf.resize(pos + need + 4096, 0);
    }
}

fn buf_write_u8(buf: &mut Vec<u8>, pos: &mut usize, v: u8) {
    buf_ensure(buf, *pos, 1);
    buf[*pos] = v;
    *pos += 1;
}

fn buf_write_u32(buf: &mut Vec<u8>, pos: &mut usize, v: u32) {
    buf_ensure(buf, *pos, 4);
    buf[*pos..*pos + 4].copy_from_slice(&v.to_ne_bytes());
    *pos += 4;
}

fn buf_write_i64(buf: &mut Vec<u8>, pos: &mut usize, v: i64) {
    buf_ensure(buf, *pos, 8);
    buf[*pos..*pos + 8].copy_from_slice(&v.to_ne_bytes());
    *pos += 8;
}

fn buf_write_f32(buf: &mut Vec<u8>, pos: &mut usize, v: f32) {
    buf_ensure(buf, *pos, 4);
    buf[*pos..*pos + 4].copy_from_slice(&v.to_ne_bytes());
    *pos += 4;
}

fn buf_write_bytes(buf: &mut Vec<u8>, pos: &mut usize, data: &[u8]) {
    buf_ensure(buf, *pos, data.len());
    buf[*pos..*pos + data.len()].copy_from_slice(data);
    *pos += data.len();
}

fn buf_read_u8(buf: &[u8], pos: &mut usize) -> u8 {
    let v = buf[*pos];
    *pos += 1;
    v
}

fn buf_read_u32(buf: &[u8], pos: &mut usize) -> u32 {
    let v = u32::from_ne_bytes(buf[*pos..*pos + 4].try_into().unwrap());
    *pos += 4;
    v
}

fn buf_read_i64(buf: &[u8], pos: &mut usize) -> i64 {
    let v = i64::from_ne_bytes(buf[*pos..*pos + 8].try_into().unwrap());
    *pos += 8;
    v
}

fn buf_read_f32(buf: &[u8], pos: &mut usize) -> f32 {
    let v = f32::from_ne_bytes(buf[*pos..*pos + 4].try_into().unwrap());
    *pos += 4;
    v
}

fn buf_read_bytes(buf: &[u8], pos: &mut usize, len: usize) -> Vec<u8> {
    let v = buf[*pos..*pos + len].to_vec();
    *pos += len;
    v
}

fn _mvptree_write_node(node: &Rc<RefCell<Node>>, buf: &mut Vec<u8>, pos: &mut usize, tree_bf: usize, tree_pl: usize, tree_lc: usize) -> MVPError {
    let borrow = node.borrow();
    match &*borrow {
        Node::Leaf(leaf) => {
            buf_write_u8(buf, pos, NodeType::LeafNode as u8);
            write_datapoint_to_buf(leaf.sv1.as_ref(), buf, pos, tree_pl);
            write_datapoint_to_buf(leaf.sv2.as_ref(), buf, pos, tree_pl);
            buf_write_u32(buf, pos, leaf.nbpoints as u32);

            let saved_pos = *pos;
            // reserve space for d1, d2, offset per point
            let entry_size = 4 + 4 + 8; // float + float + off_t
            *pos += tree_lc * entry_size;

            let mut sp = saved_pos;
            for i in 0..leaf.nbpoints {
                let d1v = if i < leaf.d1.len() { leaf.d1[i] } else { 0.0 };
                let d2v = if i < leaf.d2.len() { leaf.d2[i] } else { 0.0 };
                buf_write_f32(buf, &mut sp, d1v);
                buf_write_f32(buf, &mut sp, d2v);
                let offset = *pos as i64;
                buf_write_i64(buf, &mut sp, offset);
                write_datapoint_to_buf(Some(&leaf.points[i]), buf, pos, tree_pl);
            }
            MVPError::Success
        }
        Node::Internal(internal) => {
            let bf = tree_bf;
            let length_m1 = bf - 1;
            let length_m2 = (bf - 1) * bf;
            let fanout = bf * bf;

            buf_write_u8(buf, pos, NodeType::InternalNode as u8);
            write_datapoint_to_buf(internal.sv1.as_ref(), buf, pos, tree_pl);
            write_datapoint_to_buf(internal.sv2.as_ref(), buf, pos, tree_pl);

            for i in 0..length_m1 {
                buf_write_f32(buf, pos, internal.m1[i]);
            }
            for i in 0..length_m2 {
                buf_write_f32(buf, pos, internal.m2[i]);
            }

            let saved_pos = *pos;
            // reserve space: fileno(1) + offset(8) per child
            *pos += fanout * (1 + 8);

            let mut sp = saved_pos;
            for i in 0..fanout {
                let fileno: u8 = 0;
                if i < internal.child_nodes.len() {
                    let offset = *pos as i64;
                    buf_write_u8(buf, &mut sp, fileno);
                    buf_write_i64(buf, &mut sp, offset);
                    let err = _mvptree_write_node(&internal.child_nodes[i], buf, pos, tree_bf, tree_pl, tree_lc);
                    if err != MVPError::Success { return err; }
                } else {
                    buf_write_u8(buf, &mut sp, fileno);
                    buf_write_i64(buf, &mut sp, 0);
                }
            }
            MVPError::Success
        }
    }
}

fn read_datapoint_from_buf(buf: &[u8], pos: &mut usize, datatype: MVPDataType, path_length: usize) -> Option<Arc<MVPDatapoint>> {
    let active = buf_read_u8(buf, pos);
    let bytelength = buf_read_u32(buf, pos);
    if active == 0 && bytelength == 0 { return None; }

    let idlen = buf_read_u8(buf, pos) as usize;
    let id_bytes = buf_read_bytes(buf, pos, idlen);
    let id = String::from_utf8_lossy(&id_bytes).to_string();
    let datalength = buf_read_u32(buf, pos) as usize;
    let type_size = datatype as usize;
    let data = buf_read_bytes(buf, pos, datalength * type_size);
    let mut path = Vec::with_capacity(path_length);
    for _ in 0..path_length {
        path.push(buf_read_f32(buf, pos));
    }
    Some(Arc::new(MVPDatapoint {
        id,
        data,
        path,
        datalen: datalength,
        data_type: datatype,
    }))
}

fn _mvptree_read_node(buf: &[u8], pos: &mut usize, tree_bf: usize, tree_pl: usize, tree_lc: usize, datatype: MVPDataType) -> Result<Rc<RefCell<Node>>, MVPError> {
    let node_type = buf_read_u8(buf, pos);

    if node_type == NodeType::LeafNode as u8 {
        let sv1 = read_datapoint_from_buf(buf, pos, datatype, tree_pl);
        let sv2 = read_datapoint_from_buf(buf, pos, datatype, tree_pl);
        let nbpoints = buf_read_u32(buf, pos) as usize;

        let mut leaf = LeafNode::new(tree_bf as u32);
        leaf.sv1 = sv1;
        leaf.sv2 = sv2;
        leaf.nbpoints = nbpoints;

        let saved_pos = *pos;
        let mut sp = saved_pos;
        for i in 0..nbpoints {
            let d1 = buf_read_f32(buf, &mut sp);
            let d2 = buf_read_f32(buf, &mut sp);
            let offset = buf_read_i64(buf, &mut sp) as usize;
            *pos = offset;
            let dp = read_datapoint_from_buf(buf, pos, datatype, tree_pl);
            leaf.d1.push(d1);
            leaf.d2.push(d2);
            if let Some(p) = dp { leaf.points.push(p); }
        }
        Ok(Rc::new(RefCell::new(Node::Leaf(leaf))))
    } else if node_type == NodeType::InternalNode as u8 {
        let bf = tree_bf;
        let length_m1 = bf - 1;
        let length_m2 = (bf - 1) * bf;
        let fanout = bf * bf;

        let sv1 = read_datapoint_from_buf(buf, pos, datatype, tree_pl);
        let sv2 = read_datapoint_from_buf(buf, pos, datatype, tree_pl);

        let mut m1 = Vec::with_capacity(length_m1);
        for _ in 0..length_m1 { m1.push(buf_read_f32(buf, pos)); }
        let mut m2 = Vec::with_capacity(length_m2);
        for _ in 0..length_m2 { m2.push(buf_read_f32(buf, pos)); }

        let mut internal = InternalNode::new(bf as u32);
        internal.sv1 = sv1;
        internal.sv2 = sv2;
        internal.m1 = m1;
        internal.m2 = m2;

        let saved_pos = *pos;
        let mut sp = saved_pos;
        for _ in 0..fanout {
            let _fileno = buf_read_u8(buf, &mut sp);
            let offset = buf_read_i64(buf, &mut sp) as usize;
            *pos = offset;
            let child = _mvptree_read_node(buf, pos, tree_bf, tree_pl, tree_lc, datatype)?;
            internal.child_nodes.push(child);
        }
        Ok(Rc::new(RefCell::new(Node::Internal(internal))))
    } else {
        Err(MVPError::Unrecognized)
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

        if self.datatype as u32 == 0 {
            self.datatype = points[0].data_type;
        }
        if self.datatype != points[0].data_type {
            return MVPError::TypeMismatch;
        }

        let arc_points: Vec<Arc<MVPDatapoint>> = points.into_iter().map(|mut p| {
            p.path = vec![0.0; self.path_length];
            Arc::new(p)
        }).collect();

        match _mvptree_add(self.branch_factor, self.path_length, self.leaf_capacity, self.distance_function, self.node.take(), arc_points, 0) {
            Ok(new_node) => { self.node = new_node; MVPError::Success }
            Err(e) => e
        }
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
        let mut results = Vec::new();

        let err = _mvptree_retrieve(self.branch_factor, self.path_length, knearest, self.distance_function, node, node, &mut target_clone, radius, &mut results, 0);

        if err == MVPError::Success || err == MVPError::KNearestCap {
            Ok(results)
        } else {
            Err(err)
        }
    }
    pub fn write(&self, filename: &str, mode:i32) -> MVPError {
        if self.node.is_none() { return MVPError::ArgErr; }

        let node = self.node.as_ref().unwrap();

        // Determine hash type from root sv1
        let ht: u8 = {
            let borrow = node.borrow();
            match &*borrow {
                Node::Leaf(leaf) => leaf.sv1.as_ref().map(|s| s.data_type as u8).unwrap_or(1),
                Node::Internal(int) => int.sv1.as_ref().map(|s| s.data_type as u8).unwrap_or(1),
            }
        };

        let mut buf: Vec<u8> = vec![0u8; 4096];
        let mut pos: usize = 0;

        // write header
        let tag_bytes = TAG.as_bytes();
        buf_write_bytes(&mut buf, &mut pos, tag_bytes);
        buf_write_u8(&mut buf, &mut pos, 0); // null terminator
        buf_write_u32(&mut buf, &mut pos, VERSION);
        buf_write_u8(&mut buf, &mut pos, self.branch_factor as u8);
        buf_write_u8(&mut buf, &mut pos, self.path_length as u8);
        buf_write_u8(&mut buf, &mut pos, self.leaf_capacity as u8);
        buf_write_u8(&mut buf, &mut pos, ht);

        pos = HEADER_SIZE;

        let err = _mvptree_write_node(node, &mut buf, &mut pos, self.branch_factor, self.path_length, self.leaf_capacity);
        if err != MVPError::Success { return err; }

        // Write to file
        match File::create(filename) {
            Ok(mut f) => {
                if f.write_all(&buf[..pos]).is_err() { return MVPError::NoWrite; }
                MVPError::Success
            }
            Err(_) => MVPError::FileOpen,
        }
    }
    pub fn print(&self, stream: &mut dyn Write) -> MVPError {
        let err = _mvptree_print(stream, self.branch_factor, &self.node, 0);
        if err != MVPError::Success {
            let _ = writeln!(stream, "malformed tree: {}", error_to_string(err));
        }
        err
    }
    pub fn clear(&mut self, node: &mut Option<Box<Node>>) {
        self.node = None;
    }
    pub fn extend_mvpfile(&mut self)-> i32{
        self.buf.resize(self.buf.len() + self.pgsize as usize, 0);
        self.size += self.pgsize;
        0
    }
}

pub fn mvptree_read(filename: &str, distance_function: DistanceFunction) -> Result<MVPTree, MVPError> {
    let data = match std::fs::read(filename) {
        Ok(d) => d,
        Err(_) => {
            return Err(MVPError::FileNotFound);
        }
    };

    if data.len() < HEADER_SIZE {
        return Err(MVPError::FileOpen);
    }

    let mut pos: usize = 0;
    // read tag
    let tag_len = TAG.len();
    let _tag = &data[pos..pos + tag_len];
    pos += tag_len + 1; // +1 for null terminator
    let _version = buf_read_u32(&data, &mut pos);
    let bf = buf_read_u8(&data, &mut pos) as usize;
    let pl = buf_read_u8(&data, &mut pos) as usize;
    let lc = buf_read_u8(&data, &mut pos) as usize;
    let ht = buf_read_u8(&data, &mut pos);

    let datatype = match ht {
        1 => MVPDataType::ByteArray,
        2 => MVPDataType::UInt16Array,
        4 => MVPDataType::UInt32Array,
        8 => MVPDataType::UInt64Array,
        _ => MVPDataType::ByteArray,
    };

    pos = HEADER_SIZE;

    let node = _mvptree_read_node(&data, &mut pos, bf, pl, lc, datatype)?;

    Ok(MVPTree {
        branch_factor: bf,
        path_length: pl,
        leaf_capacity: lc,
        datatype,
        pos: 0,
        size: data.len() as i64,
        pgsize: 4096,
        buf: Vec::new(),
        node: Some(node),
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
        // This is not directly used in the tree - the free function select_vantage_points is used instead
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
        MVPError::FileNotFound => 24,
        MVPError::Unrecognized => 24,
    };
    ERROR_MSGS[idx]
}
