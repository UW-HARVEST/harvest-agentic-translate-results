use std::cell::RefCell;
use std::fs;
use std::io::Write;
use std::rc::Rc;
use std::sync::Arc;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

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

const DEFAULT_BRANCH_FACTOR: usize = 2;
const DEFAULT_PATH_LENGTH: usize = 5;
const DEFAULT_LEAF_CAPACITY: usize = 25;

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
        let fanout = bf.saturating_mul(bf);
        InternalNode {
            node_type: NodeType::InternalNode,
            sv1: None,
            sv2: None,
            m1: vec![0.0; bf.saturating_sub(1)],
            m2: vec![0.0; bf.saturating_mul(bf.saturating_sub(1))],
            child_nodes: (0..fanout).map(|_| null_child()).collect(),
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
        if points.is_empty() {
            return MVPError::Success;
        }

        if self.node.is_none() {
            self.datatype = points[0].data_type;
        } else if self.datatype != points[0].data_type {
            return MVPError::TypeMismatch;
        }

        let mut owned_points = points;
        for point in &mut owned_points {
            point.path = vec![0.0; self.path_length];
        }

        match add_recursive(self, self.node.clone(), owned_points, 0) {
            Ok(node) => {
                self.node = node;
                MVPError::Success
            }
            Err(error) => error,
        }
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
        if self.node.is_none() {
            return Err(MVPError::EmptyTree);
        }

        let mut query = target.clone();
        query.path = vec![0.0; self.path_length];

        let mut results = Vec::with_capacity(knearest);
        match retrieve_recursive(
            self,
            self.node.as_ref(),
            &mut query,
            radius,
            knearest,
            &mut results,
            0,
        ) {
            Ok(()) | Err(MVPError::KNearestCap) => Ok(results),
            Err(error) => Err(error),
        }
    }

    pub fn write(&self, filename: &str, mode: i32) -> MVPError {
        if filename.is_empty() || self.node.is_none() {
            return MVPError::ArgErr;
        }

        let mut buf = vec![0_u8; HEADER_SIZE];
        let tag_bytes = TAG.as_bytes();
        if tag_bytes.len() + 1 > HEADER_SIZE {
            return MVPError::NoWrite;
        }
        buf[..tag_bytes.len()].copy_from_slice(tag_bytes);
        buf[tag_bytes.len()] = 0;

        let mut header_pos = tag_bytes.len() + 1;
        buf[header_pos..header_pos + 4].copy_from_slice(&VERSION.to_le_bytes());
        header_pos += 4;
        if header_pos + 4 > HEADER_SIZE {
            return MVPError::NoWrite;
        }
        buf[header_pos] = self.branch_factor as u8;
        buf[header_pos + 1] = self.path_length as u8;
        buf[header_pos + 2] = self.leaf_capacity as u8;
        buf[header_pos + 3] = self.datatype as u8;

        let mut pos = HEADER_SIZE;
        if let Some(node) = self.node.as_ref() {
            if serialize_node(self, node, &mut buf, &mut pos).is_err() {
                return MVPError::NoWrite;
            }
        }

        if fs::write(filename, &buf).is_err() {
            return MVPError::NoWrite;
        }

        #[cfg(unix)]
        {
            let permissions = PermissionsExt::from_mode(mode as u32);
            if fs::set_permissions(filename, permissions).is_err() {
                return MVPError::NoWrite;
            }
        }

        MVPError::Success
    }

    pub fn print(&self, stream: &mut dyn Write) -> MVPError {
        match print_node(stream, self, self.node.as_ref(), 0) {
            Ok(()) => MVPError::Success,
            Err(error) => {
                let _ = writeln!(stream, "malformed tree: {}", error_to_string(error));
                error
            }
        }
    }

    pub fn clear(&mut self, node: &mut Option<Box<Node>>) {
        *node = None;
        self.node = None;
        self.buf.clear();
        self.pos = 0;
        self.size = 0;
    }

    pub fn extend_mvpfile(&mut self) -> i32 {
        if self.pgsize <= 0 {
            return -1;
        }
        self.size = self.size.saturating_add(self.pgsize);
        if self.buf.len() < self.size as usize {
            self.buf.resize(self.size as usize, 0);
        }
        0
    }
}

pub fn mvptree_read(
    filename: &str,
    distance_function: DistanceFunction,
) -> Result<MVPTree, MVPError> {
    let data = match fs::read(filename) {
        Ok(data) => data,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok(MVPTree::new(
                DEFAULT_BRANCH_FACTOR,
                DEFAULT_PATH_LENGTH,
                DEFAULT_LEAF_CAPACITY,
                MVPDataType::ByteArray,
                distance_function,
            ));
        }
        Err(_) => return Err(MVPError::FileOpen),
    };

    if data.len() < HEADER_SIZE {
        return Err(MVPError::MemMap);
    }

    let mut pos = TAG.len() + 1;
    if pos + 8 > data.len() {
        return Err(MVPError::MemMap);
    }

    let _version = read_u32(&data, &mut pos).ok_or(MVPError::MemMap)?;
    let branch_factor = read_u8(&data, &mut pos).ok_or(MVPError::MemMap)? as usize;
    let path_length = read_u8(&data, &mut pos).ok_or(MVPError::MemMap)? as usize;
    let leaf_capacity = read_u8(&data, &mut pos).ok_or(MVPError::MemMap)? as usize;
    let data_type =
        decode_datatype(read_u8(&data, &mut pos).ok_or(MVPError::MemMap)?).ok_or(MVPError::Unrecognized)?;

    let mut tree = MVPTree::new(
        branch_factor,
        path_length,
        leaf_capacity,
        data_type,
        distance_function,
    );
    tree.size = data.len() as i64;
    tree.pgsize = 4096;

    let mut node_pos = HEADER_SIZE;
    if node_pos < data.len() {
        tree.node = Some(read_node(&data, &mut node_pos, &tree)?);
    }

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
        nb: u32,
        sv1_pos: i32,
        sv2_pos: i32,
        dist: DistanceFunction,
    ) -> i32 {
        if nb == 0 {
            return -1;
        }
        let _ = sv1_pos;
        let _ = sv2_pos;
        let value = dist(self, self);
        if is_bad_distance(value) {
            -2
        } else {
            0
        }
    }

    pub fn find_splits(
        &mut self,
        nb: u32,
        vp: &MVPDatapoint,
        tree: &MVPTree,
        length_m: u32,
    ) -> f32 {
        if nb == 0 || length_m == 0 {
            return -1.0;
        }
        let d = (tree.distance_function)(self, vp);
        if is_bad_distance(d) {
            -2.0
        } else {
            d
        }
    }

    pub fn sort_points(
        &mut self,
        nb: u32,
        sv1_pos: i32,
        sv2_pos: i32,
        vp: &MVPDatapoint,
        tree: &MVPTree,
        counts: &mut Vec<Vec<i32>>,
        pivots: Vec<f32>,
    ) -> Vec<Vec<Vec<Arc<MVPDatapoint>>>> {
        let _ = (nb, sv1_pos, sv2_pos, vp, tree);
        counts.clear();
        counts.resize(1, vec![0]);
        if pivots.is_empty() {
            vec![vec![vec![Arc::new(self.clone())]]]
        } else {
            vec![vec![Vec::new(); pivots.len() + 1]]
        }
    }

    pub fn find_distance_range_for_vp(
        &mut self,
        nb: u32,
        vp: &MVPDatapoint,
        tree: &MVPTree,
        level: i32,
    ) -> i32 {
        if nb == 0 || level < 0 {
            return -1;
        }
        let d = (tree.distance_function)(vp, self);
        if is_bad_distance(d) {
            return -2;
        }
        if (level as usize) < self.path.len() {
            self.path[level as usize] = d;
        }
        0
    }

    pub fn write(&self, tree: &MVPTree) -> i64 {
        serialized_datapoint_size(self, tree.path_length) as i64
    }
}

pub fn error_to_string(error: MVPError) -> &'static str {
    ERROR_MSGS[error as usize]
}

fn datatype_width(data_type: MVPDataType) -> usize {
    data_type as usize
}

fn decode_datatype(value: u8) -> Option<MVPDataType> {
    match value {
        1 => Some(MVPDataType::ByteArray),
        2 => Some(MVPDataType::UInt16Array),
        4 => Some(MVPDataType::UInt32Array),
        8 => Some(MVPDataType::UInt64Array),
        _ => None,
    }
}

fn effective_datalen(point: &MVPDatapoint) -> usize {
    let width = datatype_width(point.data_type);
    if point.datalen.saturating_mul(width) == point.data.len() {
        point.datalen
    } else if point.data.len() % width == 0 {
        point.data.len() / width
    } else {
        point.datalen
    }
}

fn is_bad_distance(value: f32) -> bool {
    value.is_nan() || value < 0.0
}

fn null_child() -> Rc<RefCell<Node>> {
    Rc::new(RefCell::new(Node::Leaf(LeafNode::new(0))))
}

fn is_null_node(node: &Node) -> bool {
    match node {
        Node::Leaf(leaf) => {
            leaf.sv1.is_none()
                && leaf.sv2.is_none()
                && leaf.nbpoints == 0
                && leaf.points.is_empty()
                && leaf.d1.is_empty()
                && leaf.d2.is_empty()
        }
        Node::Internal(internal) => {
            internal.sv1.is_none()
                && internal.sv2.is_none()
                && internal.m1.is_empty()
                && internal.m2.is_empty()
                && internal.child_nodes.is_empty()
        }
    }
}

fn clone_point(point: &Arc<MVPDatapoint>) -> MVPDatapoint {
    (**point).clone()
}

fn take_child(child: &Rc<RefCell<Node>>) -> Option<Rc<RefCell<Node>>> {
    if is_null_node(&child.borrow()) {
        None
    } else {
        Some(Rc::clone(child))
    }
}

fn select_vantage_points(
    points: &[MVPDatapoint],
    distance_function: DistanceFunction,
) -> Result<(usize, Option<usize>), MVPError> {
    if points.is_empty() {
        return Err(MVPError::VpNoSelect);
    }

    let mut sv1_pos = 0usize;
    let mut sv2_pos = None;
    let mut max_distance = 0.0_f32;

    for i in 0..points.len() {
        for j in (i + 1)..points.len() {
            let d = distance_function(&points[i], &points[j]);
            if is_bad_distance(d) {
                return Err(MVPError::VpNoSelect);
            }
            if d > max_distance {
                max_distance = d;
                sv1_pos = i;
                sv2_pos = Some(j);
            }
        }
    }

    Ok((sv1_pos, sv2_pos))
}

fn assign_distance_range(
    points: &mut [MVPDatapoint],
    vp: &MVPDatapoint,
    tree: &MVPTree,
    level: usize,
) -> Result<(), MVPError> {
    for point in points {
        let d = (tree.distance_function)(vp, point);
        if is_bad_distance(d) {
            return Err(MVPError::BadDistVal);
        }
        if level < tree.path_length {
            point.path[level] = d;
        }
    }
    Ok(())
}

fn find_splits_for_points(
    points: &[MVPDatapoint],
    vp: &MVPDatapoint,
    tree: &MVPTree,
    length_m: usize,
) -> Result<Vec<f32>, MVPError> {
    if points.is_empty() || length_m == 0 {
        return Err(MVPError::NoSplits);
    }

    let mut distances = Vec::with_capacity(points.len());
    for point in points {
        let d = (tree.distance_function)(point, vp);
        if is_bad_distance(d) {
            return Err(MVPError::NoSplits);
        }
        distances.push(d);
    }
    distances.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let mut pivots = vec![0.0; length_m];
    for (i, pivot) in pivots.iter_mut().enumerate() {
        let mut index = ((i + 1) * points.len()) / (length_m + 1);
        if index >= points.len() {
            index = points.len() - 1;
        }
        *pivot = distances[index];
    }
    Ok(pivots)
}

fn sort_points_into_bins(
    points: &[MVPDatapoint],
    sv1_pos: Option<usize>,
    sv2_pos: Option<usize>,
    vp: &MVPDatapoint,
    tree: &MVPTree,
    pivots: &[f32],
) -> Result<Vec<Vec<MVPDatapoint>>, MVPError> {
    let bf = tree.branch_factor.max(1);
    let mut bins = vec![Vec::new(); bf];

    for (index, point) in points.iter().enumerate() {
        if Some(index) == sv1_pos || Some(index) == sv2_pos {
            continue;
        }

        let d = (tree.distance_function)(vp, point);
        if is_bad_distance(d) {
            return Err(MVPError::NoSort);
        }

        if pivots.is_empty() {
            bins[0].push(point.clone());
            continue;
        }

        let mut placed = false;
        for (pivot_index, pivot) in pivots.iter().enumerate() {
            if d <= *pivot {
                bins[pivot_index].push(point.clone());
                placed = true;
                break;
            }
        }
        if !placed {
            bins[pivots.len()].push(point.clone());
        }
    }

    Ok(bins)
}

fn add_recursive(
    tree: &MVPTree,
    node: Option<Rc<RefCell<Node>>>,
    mut points: Vec<MVPDatapoint>,
    level: usize,
) -> Result<Option<Rc<RefCell<Node>>>, MVPError> {
    if points.is_empty() {
        return Ok(node);
    }

    if node.is_none() {
        if points.len() <= tree.leaf_capacity + 2 {
            let (sv1_pos, sv2_pos) = select_vantage_points(&points, tree.distance_function)?;
            let mut leaf = LeafNode::new(tree.leaf_capacity as u32);

            let sv1_owned = points[sv1_pos].clone();
            assign_distance_range(&mut points, &sv1_owned, tree, level).map_err(|_| MVPError::NoSv1Range)?;
            leaf.sv1 = Some(Arc::new(sv1_owned));

            if let Some(sv2_index) = sv2_pos {
                let sv2_owned = points[sv2_index].clone();
                assign_distance_range(&mut points, &sv2_owned, tree, level + 1)
                    .map_err(|_| MVPError::NoSv2Range)?;
                leaf.sv2 = Some(Arc::new(sv2_owned));
            }

            for (index, point) in points.into_iter().enumerate() {
                if index == sv1_pos || Some(index) == sv2_pos {
                    continue;
                }
                let d1 = if let Some(sv1) = leaf.sv1.as_ref() {
                    (tree.distance_function)(&point, sv1)
                } else {
                    0.0
                };
                let d2 = if let Some(sv2) = leaf.sv2.as_ref() {
                    (tree.distance_function)(&point, sv2)
                } else {
                    0.0
                };
                if is_bad_distance(d1) || is_bad_distance(d2) {
                    return Err(MVPError::BadDistVal);
                }
                leaf.d1[leaf.nbpoints] = d1;
                leaf.d2[leaf.nbpoints] = d2;
                leaf.points.push(Arc::new(point));
                leaf.nbpoints += 1;
            }

            return Ok(Some(Rc::new(RefCell::new(Node::Leaf(leaf)))));
        }

        let (sv1_pos, sv2_pos) = select_vantage_points(&points, tree.distance_function)?;
        let mut internal = InternalNode::new(tree.branch_factor as u32);

        let sv1_owned = points[sv1_pos].clone();
        assign_distance_range(&mut points, &sv1_owned, tree, level).map_err(|_| MVPError::NoSv1Range)?;
        internal.sv1 = Some(Arc::new(sv1_owned.clone()));

        let m1 = find_splits_for_points(&points, &sv1_owned, tree, tree.branch_factor.saturating_sub(1))?;
        internal.m1 = m1.clone();

        let bins = sort_points_into_bins(
            &points,
            Some(sv1_pos),
            sv2_pos,
            &sv1_owned,
            tree,
            &m1,
        )?;

        let sv2_index = sv2_pos.ok_or(MVPError::VpNoSelect)?;
        let sv2_owned = points[sv2_index].clone();
        internal.sv2 = Some(Arc::new(sv2_owned.clone()));

        let length_m1 = tree.branch_factor.saturating_sub(1);
        for (i, bin) in bins.into_iter().enumerate() {
            if bin.is_empty() {
                continue;
            }

            let mut bin_for_ranges = bin.clone();
            assign_distance_range(&mut bin_for_ranges, &sv2_owned, tree, level + 1)
                .map_err(|_| MVPError::NoSv2Range)?;

            let m2 = find_splits_for_points(&bin_for_ranges, &sv2_owned, tree, length_m1)?;
            let offset = i * length_m1;
            for (j, pivot) in m2.iter().enumerate() {
                internal.m2[offset + j] = *pivot;
            }

            let second_bins = sort_points_into_bins(&bin_for_ranges, None, None, &sv2_owned, tree, &m2)?;
            for (j, second_bin) in second_bins.into_iter().enumerate() {
                let child = add_recursive(tree, None, second_bin, level + 2)?;
                internal.child_nodes[i * tree.branch_factor + j] =
                    child.unwrap_or_else(null_child);
            }
        }

        return Ok(Some(Rc::new(RefCell::new(Node::Internal(internal)))));
    }

    let node_rc = node.unwrap();
    let replacement = {
        let borrowed = node_rc.borrow();
        match &*borrowed {
            Node::Leaf(leaf) => {
                if leaf.nbpoints + points.len() <= tree.leaf_capacity {
                    None
                } else {
                    let mut merged = Vec::new();
                    if let Some(sv1) = leaf.sv1.as_ref() {
                        merged.push(clone_point(sv1));
                    }
                    if let Some(sv2) = leaf.sv2.as_ref() {
                        merged.push(clone_point(sv2));
                    }
                    for point in &leaf.points {
                        merged.push(clone_point(point));
                    }
                    merged.extend(points.clone());
                    Some(merged)
                }
            }
            Node::Internal(_) => None,
        }
    };

    if let Some(merged) = replacement {
        return add_recursive(tree, None, merged, level);
    }

    {
        let mut borrowed = node_rc.borrow_mut();
        match &mut *borrowed {
            Node::Leaf(leaf) => {
                let sv1_owned = leaf.sv1.as_ref().map(clone_point).ok_or(MVPError::NoSv1Range)?;
                assign_distance_range(&mut points, &sv1_owned, tree, level).map_err(|_| MVPError::NoSv1Range)?;

                let mut start = 0usize;
                if leaf.sv2.is_none() {
                    let sv2_owned = points[0].clone();
                    assign_distance_range(&mut points, &sv2_owned, tree, level + 1)
                        .map_err(|_| MVPError::NoSv2Range)?;
                    leaf.sv2 = Some(Arc::new(sv2_owned));
                    start = 1;
                } else {
                    let sv2_owned = leaf.sv2.as_ref().map(clone_point).ok_or(MVPError::NoSv2Range)?;
                    assign_distance_range(&mut points, &sv2_owned, tree, level + 1)
                        .map_err(|_| MVPError::NoSv2Range)?;
                }

                for point in points.into_iter().skip(start) {
                    let d1 = if let Some(sv1) = leaf.sv1.as_ref() {
                        (tree.distance_function)(&point, sv1)
                    } else {
                        0.0
                    };
                    let d2 = if let Some(sv2) = leaf.sv2.as_ref() {
                        (tree.distance_function)(&point, sv2)
                    } else {
                        0.0
                    };
                    if is_bad_distance(d1) || is_bad_distance(d2) {
                        return Err(MVPError::BadDistVal);
                    }
                    leaf.d1[leaf.nbpoints] = d1;
                    leaf.d2[leaf.nbpoints] = d2;
                    leaf.points.push(Arc::new(point));
                    leaf.nbpoints += 1;
                }
            }
            Node::Internal(internal) => {
                let sv1_owned = internal.sv1.as_ref().map(clone_point).ok_or(MVPError::NoSv1Range)?;
                assign_distance_range(&mut points, &sv1_owned, tree, level).map_err(|_| MVPError::NoSv1Range)?;
                let bins = sort_points_into_bins(&points, None, None, &sv1_owned, tree, &internal.m1)?;

                let sv2_owned = internal.sv2.as_ref().map(clone_point).ok_or(MVPError::NoSv2Range)?;
                let length_m1 = tree.branch_factor.saturating_sub(1);
                for (i, bin) in bins.into_iter().enumerate() {
                    if bin.is_empty() {
                        continue;
                    }

                    let mut bin_for_ranges = bin.clone();
                    assign_distance_range(&mut bin_for_ranges, &sv2_owned, tree, level + 1)
                        .map_err(|_| MVPError::NoSv2Range)?;
                    let pivots = internal.m2[i * length_m1..i * length_m1 + length_m1].to_vec();
                    let second_bins =
                        sort_points_into_bins(&bin_for_ranges, None, None, &sv2_owned, tree, &pivots)?;

                    for (j, second_bin) in second_bins.into_iter().enumerate() {
                        let child_index = i * tree.branch_factor + j;
                        let existing_child = take_child(&internal.child_nodes[child_index]);
                        let new_child = add_recursive(tree, existing_child, second_bin, level + 2)?;
                        internal.child_nodes[child_index] = new_child.unwrap_or_else(null_child);
                    }
                }
            }
        }
    }

    Ok(Some(node_rc))
}

fn push_result(
    results: &mut Vec<MVPDatapoint>,
    point: &Arc<MVPDatapoint>,
    knearest: usize,
) -> Result<(), MVPError> {
    results.push(clone_point(point));
    if results.len() >= knearest {
        Err(MVPError::KNearestCap)
    } else {
        Ok(())
    }
}

fn retrieve_recursive(
    tree: &MVPTree,
    node: Option<&Rc<RefCell<Node>>>,
    target: &mut MVPDatapoint,
    radius: f32,
    knearest: usize,
    results: &mut Vec<MVPDatapoint>,
    level: usize,
) -> Result<(), MVPError> {
    let Some(node_rc) = node else {
        return Ok(());
    };
    let borrowed = node_rc.borrow();
    if is_null_node(&borrowed) {
        return Ok(());
    }

    match &*borrowed {
        Node::Leaf(leaf) => {
            let Some(sv1) = leaf.sv1.as_ref() else {
                return Ok(());
            };

            let d1 = (tree.distance_function)(target, sv1);
            if is_bad_distance(d1) {
                return Err(MVPError::BadDistVal);
            }
            if level < tree.path_length {
                target.path[level] = d1;
            }
            if d1 <= radius {
                push_result(results, sv1, knearest)?;
            }

            let Some(sv2) = leaf.sv2.as_ref() else {
                return Ok(());
            };

            let d2 = (tree.distance_function)(target, sv2);
            if is_bad_distance(d2) {
                return Err(MVPError::BadDistVal);
            }
            if d2 <= radius {
                push_result(results, sv2, knearest)?;
            }
            if level + 1 < tree.path_length {
                target.path[level + 1] = d2;
            }

            let end_path = usize::min(level + 1, tree.path_length);
            for i in 0..leaf.nbpoints {
                if d1 - radius <= leaf.d1[i] && d1 + radius >= leaf.d1[i] {
                    if d2 - radius <= leaf.d2[i] && d2 + radius >= leaf.d2[i] {
                        let point = &leaf.points[i];
                        let mut skip = false;
                        for j in 0..end_path {
                            if !(target.path[j] - radius <= point.path[j]
                                && target.path[j] + radius >= point.path[j])
                            {
                                skip = true;
                                break;
                            }
                        }
                        if !skip {
                            let d = (tree.distance_function)(target, point);
                            if is_bad_distance(d) {
                                return Err(MVPError::BadDistVal);
                            }
                            if d <= radius {
                                push_result(results, point, knearest)?;
                            }
                        }
                    }
                }
            }
        }
        Node::Internal(internal) => {
            let Some(sv1) = internal.sv1.as_ref() else {
                return Ok(());
            };
            let Some(sv2) = internal.sv2.as_ref() else {
                return Ok(());
            };

            let d1 = (tree.distance_function)(target, sv1);
            if is_bad_distance(d1) {
                return Err(MVPError::BadDistVal);
            }
            if d1 <= radius {
                push_result(results, sv1, knearest)?;
            }
            if level < tree.path_length {
                target.path[level] = d1;
            }

            let d2 = (tree.distance_function)(target, sv2);
            if is_bad_distance(d2) {
                return Err(MVPError::BadDistVal);
            }
            if d2 <= radius {
                push_result(results, sv2, knearest)?;
            }
            if level + 1 < tree.path_length {
                target.path[level + 1] = d2;
            }

            let bf = tree.branch_factor;
            let length_m1 = bf.saturating_sub(1);
            for i in 0..length_m1 {
                if d1 - radius <= internal.m1[i] {
                    for j in 0..length_m1 {
                        if d2 - radius <= internal.m2[i * length_m1 + j] {
                            retrieve_recursive(
                                tree,
                                Some(&internal.child_nodes[i * bf + j]),
                                target,
                                radius,
                                knearest,
                                results,
                                level + 2,
                            )?;
                        }
                    }
                    if d2 + radius >= internal.m2[i * length_m1 + length_m1 - 1] {
                        retrieve_recursive(
                            tree,
                            Some(&internal.child_nodes[i * bf + length_m1]),
                            target,
                            radius,
                            knearest,
                            results,
                            level + 2,
                        )?;
                    }
                }
            }

            if !internal.m1.is_empty() && d1 + radius >= internal.m1[length_m1 - 1] {
                for j in 0..length_m1 {
                    if d2 - radius <= internal.m2[length_m1 * length_m1 + j] {
                        retrieve_recursive(
                            tree,
                            Some(&internal.child_nodes[bf * length_m1 + j]),
                            target,
                            radius,
                            knearest,
                            results,
                            level + 2,
                        )?;
                    }
                }
                if d2 + radius >= internal.m2[length_m1 * length_m1 + length_m1 - 1] {
                    retrieve_recursive(
                        tree,
                        Some(&internal.child_nodes[bf * length_m1 + length_m1]),
                        target,
                        radius,
                        knearest,
                        results,
                        level + 2,
                    )?;
                }
            }
        }
    }

    Ok(())
}

fn push_u8(buf: &mut Vec<u8>, value: u8, pos: &mut usize) {
    buf.push(value);
    *pos += 1;
}

fn push_u32(buf: &mut Vec<u8>, value: u32, pos: &mut usize) {
    buf.extend_from_slice(&value.to_le_bytes());
    *pos += 4;
}

fn push_f32(buf: &mut Vec<u8>, value: f32, pos: &mut usize) {
    buf.extend_from_slice(&value.to_le_bytes());
    *pos += 4;
}

fn serialized_datapoint_size(point: &MVPDatapoint, path_length: usize) -> usize {
    let idlen = point.id.len() as u8 as usize;
    let datalen = effective_datalen(point);
    1 + 4 + 1 + idlen + 4 + datalen * datatype_width(point.data_type) + path_length * 4
}

fn serialize_datapoint(
    point: Option<&Arc<MVPDatapoint>>,
    tree: &MVPTree,
    buf: &mut Vec<u8>,
    pos: &mut usize,
) -> Result<i64, MVPError> {
    let start = *pos as i64;
    let Some(point) = point else {
        push_u8(buf, 0, pos);
        push_u32(buf, 0, pos);
        return Ok(start);
    };

    let datalen = effective_datalen(point) as u32;
    let type_width = datatype_width(point.data_type);
    let id_bytes = point.id.as_bytes();
    let idlen = id_bytes.len() as u8;
    let id_slice = &id_bytes[..idlen as usize];
    let body_len =
        1_u32 + idlen as u32 + 4 + datalen.saturating_mul(type_width as u32) + (tree.path_length * 4) as u32;

    push_u8(buf, 1, pos);
    push_u32(buf, body_len, pos);
    push_u8(buf, idlen, pos);
    buf.extend_from_slice(id_slice);
    *pos += id_slice.len();
    push_u32(buf, datalen, pos);

    let data_bytes = datalen as usize * type_width;
    if point.data.len() >= data_bytes {
        buf.extend_from_slice(&point.data[..data_bytes]);
    } else {
        buf.extend_from_slice(&point.data);
        buf.resize(buf.len() + (data_bytes - point.data.len()), 0);
    }
    *pos += data_bytes;

    let mut path_bytes = vec![0.0_f32; tree.path_length];
    for (dst, src) in path_bytes.iter_mut().zip(point.path.iter()) {
        *dst = *src;
    }
    for value in path_bytes {
        push_f32(buf, value, pos);
    }

    Ok(start)
}

fn serialize_node(
    tree: &MVPTree,
    node: &Rc<RefCell<Node>>,
    buf: &mut Vec<u8>,
    pos: &mut usize,
) -> Result<i64, MVPError> {
    let borrowed = node.borrow();
    if is_null_node(&borrowed) {
        return Ok(0);
    }

    let start = *pos as i64;
    match &*borrowed {
        Node::Leaf(leaf) => {
            push_u8(buf, NodeType::LeafNode as u8, pos);
            let _ = serialize_datapoint(leaf.sv1.as_ref(), tree, buf, pos)?;
            let _ = serialize_datapoint(leaf.sv2.as_ref(), tree, buf, pos)?;
            push_u32(buf, leaf.nbpoints as u32, pos);

            let entry_start = *pos;
            let reserved = tree.leaf_capacity * (4 + 4 + 8);
            buf.resize(buf.len() + reserved, 0);
            *pos += reserved;

            for i in 0..leaf.nbpoints {
                let offset = serialize_datapoint(Some(&leaf.points[i]), tree, buf, pos)?;
                let base = entry_start + i * 16;
                buf[base..base + 4].copy_from_slice(&leaf.d1[i].to_le_bytes());
                buf[base + 4..base + 8].copy_from_slice(&leaf.d2[i].to_le_bytes());
                buf[base + 8..base + 16].copy_from_slice(&offset.to_le_bytes());
            }
        }
        Node::Internal(internal) => {
            push_u8(buf, NodeType::InternalNode as u8, pos);
            let _ = serialize_datapoint(internal.sv1.as_ref(), tree, buf, pos)?;
            let _ = serialize_datapoint(internal.sv2.as_ref(), tree, buf, pos)?;
            for pivot in &internal.m1 {
                push_f32(buf, *pivot, pos);
            }
            for pivot in &internal.m2 {
                push_f32(buf, *pivot, pos);
            }

            let fanout = tree.branch_factor * tree.branch_factor;
            let entry_start = *pos;
            let reserved = fanout * (1 + 8);
            buf.resize(buf.len() + reserved, 0);
            *pos += reserved;

            for i in 0..fanout {
                let offset = serialize_node(tree, &internal.child_nodes[i], buf, pos)?;
                let base = entry_start + i * 9;
                buf[base] = 0;
                buf[base + 1..base + 9].copy_from_slice(&offset.to_le_bytes());
            }
        }
    }

    Ok(start)
}

fn read_u8(buf: &[u8], pos: &mut usize) -> Option<u8> {
    let value = *buf.get(*pos)?;
    *pos += 1;
    Some(value)
}

fn read_u32(buf: &[u8], pos: &mut usize) -> Option<u32> {
    let bytes = buf.get(*pos..*pos + 4)?;
    *pos += 4;
    Some(u32::from_le_bytes(bytes.try_into().ok()?))
}

fn read_i64(buf: &[u8], pos: &mut usize) -> Option<i64> {
    let bytes = buf.get(*pos..*pos + 8)?;
    *pos += 8;
    Some(i64::from_le_bytes(bytes.try_into().ok()?))
}

fn read_f32(buf: &[u8], pos: &mut usize) -> Option<f32> {
    let bytes = buf.get(*pos..*pos + 4)?;
    *pos += 4;
    Some(f32::from_le_bytes(bytes.try_into().ok()?))
}

fn read_datapoint(
    buf: &[u8],
    pos: &mut usize,
    path_length: usize,
    data_type: MVPDataType,
) -> Result<Option<Arc<MVPDatapoint>>, MVPError> {
    let active = read_u8(buf, pos).ok_or(MVPError::MemMap)?;
    let bytelength = read_u32(buf, pos).ok_or(MVPError::MemMap)?;
    if active == 0 && bytelength == 0 {
        return Ok(None);
    }

    let idlen = read_u8(buf, pos).ok_or(MVPError::MemMap)? as usize;
    let id_bytes = buf.get(*pos..*pos + idlen).ok_or(MVPError::MemMap)?;
    *pos += idlen;
    let id = String::from_utf8_lossy(id_bytes).into_owned();

    let datalen = read_u32(buf, pos).ok_or(MVPError::MemMap)? as usize;
    let data_len_bytes = datalen * datatype_width(data_type);
    let data = buf
        .get(*pos..*pos + data_len_bytes)
        .ok_or(MVPError::MemMap)?
        .to_vec();
    *pos += data_len_bytes;

    let mut path = Vec::with_capacity(path_length);
    for _ in 0..path_length {
        path.push(read_f32(buf, pos).ok_or(MVPError::MemMap)?);
    }

    Ok(Some(Arc::new(MVPDatapoint {
        id,
        data,
        path,
        datalen,
        data_type,
    })))
}

fn read_node(
    buf: &[u8],
    pos: &mut usize,
    tree: &MVPTree,
) -> Result<Rc<RefCell<Node>>, MVPError> {
    let node_type = read_u8(buf, pos).ok_or(MVPError::MemMap)?;
    match node_type {
        2 => {
            let mut leaf = LeafNode::new(tree.leaf_capacity as u32);
            leaf.sv1 = read_datapoint(buf, pos, tree.path_length, tree.datatype)?;
            leaf.sv2 = read_datapoint(buf, pos, tree.path_length, tree.datatype)?;
            leaf.nbpoints = read_u32(buf, pos).ok_or(MVPError::MemMap)? as usize;

            let mut saved_pos = *pos;

            for i in 0..leaf.nbpoints {
                let d1 = read_f32(buf, &mut saved_pos).ok_or(MVPError::MemMap)?;
                let d2 = read_f32(buf, &mut saved_pos).ok_or(MVPError::MemMap)?;
                let offset = read_i64(buf, &mut saved_pos).ok_or(MVPError::MemMap)?;

                leaf.d1[i] = d1;
                leaf.d2[i] = d2;
                if offset > 0 {
                    let mut point_pos = offset as usize;
                    if let Some(point) =
                        read_datapoint(buf, &mut point_pos, tree.path_length, tree.datatype)?
                    {
                        leaf.points.push(point);
                    }
                }
            }

            Ok(Rc::new(RefCell::new(Node::Leaf(leaf))))
        }
        1 => {
            let mut internal = InternalNode::new(tree.branch_factor as u32);
            internal.sv1 = read_datapoint(buf, pos, tree.path_length, tree.datatype)?;
            internal.sv2 = read_datapoint(buf, pos, tree.path_length, tree.datatype)?;

            for pivot in &mut internal.m1 {
                *pivot = read_f32(buf, pos).ok_or(MVPError::MemMap)?;
            }
            for pivot in &mut internal.m2 {
                *pivot = read_f32(buf, pos).ok_or(MVPError::MemMap)?;
            }

            let mut saved_pos = *pos;
            for child in &mut internal.child_nodes {
                let _fileno = read_u8(buf, &mut saved_pos).ok_or(MVPError::MemMap)?;
                let offset = read_i64(buf, &mut saved_pos).ok_or(MVPError::MemMap)?;
                if offset > 0 {
                    let mut child_pos = offset as usize;
                    *child = read_node(buf, &mut child_pos, tree)?;
                } else {
                    *child = null_child();
                }
            }

            Ok(Rc::new(RefCell::new(Node::Internal(internal))))
        }
        _ => Err(MVPError::Unrecognized),
    }
}

fn print_node(
    stream: &mut dyn Write,
    tree: &MVPTree,
    node: Option<&Rc<RefCell<Node>>>,
    level: usize,
) -> Result<(), MVPError> {
    let Some(node_rc) = node else {
        writeln!(stream, "NULL{}", level).map_err(|_| MVPError::NoWrite)?;
        return Ok(());
    };
    let borrowed = node_rc.borrow();
    if is_null_node(&borrowed) {
        writeln!(stream, "NULL{}", level).map_err(|_| MVPError::NoWrite)?;
        return Ok(());
    }

    match &*borrowed {
        Node::Leaf(leaf) => {
            writeln!(stream, "LEAF{}  ({} points)", level, leaf.nbpoints)
                .map_err(|_| MVPError::NoWrite)?;
            if let Some(sv1) = leaf.sv1.as_ref() {
                writeln!(stream, "    sv1: {}", sv1.id).map_err(|_| MVPError::NoWrite)?;
            }
            if let Some(sv2) = leaf.sv2.as_ref() {
                writeln!(stream, "    sv2: {}", sv2.id).map_err(|_| MVPError::NoWrite)?;
            }
            for (i, point) in leaf.points.iter().take(leaf.nbpoints).enumerate() {
                writeln!(stream, "        point[{}]: {}", i, point.id)
                    .map_err(|_| MVPError::NoWrite)?;
            }
        }
        Node::Internal(internal) => {
            writeln!(stream, "INTERNAL{}", level).map_err(|_| MVPError::NoWrite)?;
            if let Some(sv1) = internal.sv1.as_ref() {
                writeln!(stream, "  sv1: {}", sv1.id).map_err(|_| MVPError::NoWrite)?;
            }
            if let Some(sv2) = internal.sv2.as_ref() {
                writeln!(stream, "  sv2: {}", sv2.id).map_err(|_| MVPError::NoWrite)?;
            }

            for (i, pivot) in internal.m1.iter().enumerate() {
                write!(stream, "  M1[{}] = {:.4};", i, pivot).map_err(|_| MVPError::NoWrite)?;
            }
            for i in 0..tree.branch_factor {
                if let Some(pivot) = internal.m2.get(i) {
                    write!(stream, "  M2[{}] = {:.4};", i, pivot).map_err(|_| MVPError::NoWrite)?;
                }
            }
            writeln!(stream).map_err(|_| MVPError::NoWrite)?;

            for child in &internal.child_nodes {
                print_node(stream, tree, Some(child), level + 2)?;
            }
        }
    }

    Ok(())
}
