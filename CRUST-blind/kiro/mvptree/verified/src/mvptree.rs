use std::fs::File;
use std::io::{self, Read, Write, Seek, SeekFrom};
use std::sync::Arc;
use std::rc::Rc;
use std::cell::RefCell;

pub const TAG: &str = "phashmvp2010";
pub const VERSION: u32 = 0x01000000;
pub const HEADER_SIZE: usize = 32;
pub const FILE_OFFSET_BITS: usize = 64;
pub const ERROR_MSGS: [&str; 25] = [
    "no error", "bad argument", "no distance function found", "mem alloc error",
    "no leaf node created", "no internal node created", "no path array alloc'd",
    "could not select vantage points", "could not calculate range from an sv1",
    "could not calculate range from an sv2", "points too compact", "could not sort points",
    "could not open file", "could not close file", "mmap error", "unmap eror", "no write",
    "could not extend file", "could not remap file", "datatypes in conflict",
    "no. retrieved exceeds k", "empty tree", "distance value either NaN or less than zero",
    "could not open file", "unrecognized node",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MVPDataType { ByteArray = 1, UInt16Array = 2, UInt32Array = 4, UInt64Array = 8 }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeType { InternalNode = 1, LeafNode }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MVPError {
    Success, ArgErr, NoDistanceFunc, MemAlloc, NoLeaf, NoInternal, PathAlloc,
    VpNoSelect, NoSv1Range, NoSv2Range, NoSpace, NoSort, FileOpen, FileClose,
    MemMap, Munmap, NoWrite, FileTruncate, MremapFail, TypeMismatch, KNearestCap,
    EmptyTree, NoSplits, BadDistVal, FileNotFound, Unrecognized,
}

#[derive(Debug, Clone)]
pub struct MVPDatapoint {
    pub id: String, pub data: Vec<u8>, pub path: Vec<f32>,
    pub datalen: usize, pub data_type: MVPDataType,
}

pub type DistanceFunction = fn(&MVPDatapoint, &MVPDatapoint) -> f32;

pub struct InternalNode {
    pub node_type: NodeType, pub sv1: Option<Arc<MVPDatapoint>>, pub sv2: Option<Arc<MVPDatapoint>>,
    pub m1: Vec<f32>, pub m2: Vec<f32>, pub child_nodes: Vec<Rc<RefCell<Node>>>,
}
impl InternalNode {
    pub fn new(bf: u32) -> Self {
        Self { node_type: NodeType::InternalNode, sv1: None, sv2: None,
               m1: vec![0.0; (bf-1) as usize], m2: vec![0.0; ((bf-1)*bf) as usize],
               child_nodes: Vec::new() }
    }
}

pub struct LeafNode {
    pub node_type: NodeType, pub sv1: Option<Arc<MVPDatapoint>>, pub sv2: Option<Arc<MVPDatapoint>>,
    pub points: Vec<Arc<MVPDatapoint>>, pub d1: Vec<f32>, pub d2: Vec<f32>, pub nbpoints: usize,
}
impl LeafNode {
    pub fn new(_cap: u32) -> Self {
        Self { node_type: NodeType::LeafNode, sv1: None, sv2: None,
               points: Vec::new(), d1: Vec::new(), d2: Vec::new(), nbpoints: 0 }
    }
}

pub enum Node { Leaf(LeafNode), Internal(InternalNode) }

pub struct MVPTree {
    pub branch_factor: usize, pub path_length: usize, pub leaf_capacity: usize,
    pub datatype: MVPDataType, pub pos: i64, pub size: i64, pub pgsize: i64,
    pub buf: Vec<u8>, pub node: Option<Rc<RefCell<Node>>>, pub distance_function: DistanceFunction,
}

// ---- helpers ----
fn select_vp(pts: &[Arc<MVPDatapoint>], d: DistanceFunction) -> Result<(i32,i32),()> {
    if pts.is_empty() { return Err(()); }
    let (mut s1, mut s2, mut mx) = (0i32, -1i32, 0.0f32);
    for i in 0..pts.len() { for j in i+1..pts.len() {
        let v = d(&pts[i],&pts[j]);
        if v.is_nan()||v<0.0 { return Err(()); }
        if v > mx { mx=v; s1=i as i32; s2=j as i32; }
    }}
    Ok((s1,s2))
}

fn calc_splits(pts: &[Arc<MVPDatapoint>], vp: &MVPDatapoint, d: DistanceFunction, lm: usize) -> Result<Vec<f32>,()> {
    let n = pts.len();
    if n==0||lm==0 { return Err(()); }
    let mut ds: Vec<f32> = pts.iter().map(|p| d(p,vp)).collect();
    if ds.iter().any(|v| v.is_nan()||*v<0.0) { return Err(()); }
    for i in 0..n.saturating_sub(1) { let mut m=i; for j in i+1..n { if ds[j]<ds[m]{m=j;} } if m!=i{ds.swap(i,m);} }
    Ok((0..lm).map(|i| ds[((i+1)*n/(lm+1)).min(n-1)]).collect())
}

fn sort_bins(pts: &[Arc<MVPDatapoint>], sk1: i32, sk2: i32, vp: &MVPDatapoint, d: DistanceFunction, bf: usize, piv: &[f32]) -> Result<Vec<Vec<Arc<MVPDatapoint>>>,()> {
    if pts.is_empty() { return Err(()); }
    let lm1 = bf-1;
    let mut bins: Vec<Vec<Arc<MVPDatapoint>>> = (0..bf).map(|_| Vec::new()).collect();
    for (i,p) in pts.iter().enumerate() {
        if i as i32==sk1||i as i32==sk2 { continue; }
        let v = d(vp,p); if v.is_nan()||v<0.0 { return Err(()); }
        let mut placed = false;
        for k in 0..lm1 { if v<=piv[k] { bins[k].push(Arc::clone(p)); placed=true; break; } }
        if !placed { bins[lm1].push(Arc::clone(p)); }
    }
    Ok(bins)
}

fn set_paths(pts: &[Arc<MVPDatapoint>], vp: &MVPDatapoint, d: DistanceFunction, lvl: usize, pl: usize) -> Result<Vec<Arc<MVPDatapoint>>,()> {
    let mut out = Vec::with_capacity(pts.len());
    for p in pts {
        let v = d(vp,p); if v.is_nan()||v<0.0 { return Err(()); }
        if lvl < pl {
            let mut np = (**p).clone();
            if np.path.len()<=lvl { np.path.resize(lvl+1,0.0); }
            np.path[lvl] = v;
            out.push(Arc::new(np));
        } else { out.push(Arc::clone(p)); }
    }
    Ok(out)
}

fn ebins(bf: usize) -> Vec<Vec<Arc<MVPDatapoint>>> { (0..bf).map(|_|Vec::new()).collect() }
fn enode() -> Rc<RefCell<Node>> { Rc::new(RefCell::new(Node::Leaf(LeafNode::new(0)))) }

fn _add(bf: usize, pl: usize, lc: usize, dist: DistanceFunction,
        node: Option<Rc<RefCell<Node>>>, points: &[Arc<MVPDatapoint>], lvl: usize,
) -> Result<Option<Rc<RefCell<Node>>>, MVPError> {
    let nb = points.len();
    if nb == 0 { return Ok(node); }
    let lm1 = bf - 1;

    if node.is_none() {
        if nb <= lc + 2 {
            let (s1,s2) = select_vp(points,dist).map_err(|_| MVPError::VpNoSelect)?;
            let sv1 = if s1>=0 { Some(Arc::clone(&points[s1 as usize])) } else { None };
            let sv2 = if s2>=0 { Some(Arc::clone(&points[s2 as usize])) } else { None };
            let p = if let Some(ref v)=sv1 { set_paths(points,v,dist,lvl,pl).map_err(|_|MVPError::NoSv1Range)? } else { points.to_vec() };
            let p = if let Some(ref v)=sv2 { set_paths(&p,v,dist,lvl+1,pl).map_err(|_|MVPError::NoSv2Range)? } else { p };
            let mut lf = LeafNode::new(lc as u32);
            lf.sv1=sv1; lf.sv2=sv2;
            for (i,pt) in p.iter().enumerate() {
                if i as i32==s1||i as i32==s2 { continue; }
                lf.d1.push(dist(pt, lf.sv1.as_ref().unwrap()));
                lf.d2.push(if let Some(ref v)=lf.sv2 { dist(pt,v) } else { 0.0 });
                lf.points.push(Arc::clone(pt));
            }
            lf.nbpoints = lf.points.len();
            return Ok(Some(Rc::new(RefCell::new(Node::Leaf(lf)))));
        }
        let (s1,s2) = select_vp(points,dist).map_err(|_| MVPError::VpNoSelect)?;
        let sv1 = Arc::clone(&points[s1 as usize]);
        let sv2 = Arc::clone(&points[s2 as usize]);
        let p = set_paths(points,&sv1,dist,lvl,pl).map_err(|_|MVPError::NoSv1Range)?;
        let m1 = calc_splits(&p,&sv1,dist,lm1).map_err(|_|MVPError::NoSplits)?;
        let bins = sort_bins(&p,s1,s2,&sv1,dist,bf,&m1).map_err(|_|MVPError::NoSort)?;
        let mut m2 = vec![0.0f32; lm1*bf];
        let mut ch: Vec<Rc<RefCell<Node>>> = Vec::with_capacity(bf*bf);
        for i in 0..bf {
            let bp = set_paths(&bins[i],&sv2,dist,lvl+1,pl).map_err(|_|MVPError::NoSv2Range)?;
            let sp = if !bp.is_empty() { calc_splits(&bp,&sv2,dist,lm1).map_err(|_|MVPError::NoSplits)? } else { vec![0.0;lm1] };
            for k in 0..lm1 { m2[i*lm1+k]=sp[k]; }
            let b2 = if !bp.is_empty() { sort_bins(&bp,-1,-1,&sv2,dist,bf,&sp).map_err(|_|MVPError::NoSort)? } else { ebins(bf) };
            for j in 0..bf {
                let c = _add(bf,pl,lc,dist,None,&b2[j],lvl+2)?;
                ch.push(c.unwrap_or_else(enode));
            }
        }
        let mut nd = InternalNode::new(bf as u32);
        nd.sv1=Some(sv1); nd.sv2=Some(sv2); nd.m1=m1; nd.m2=m2; nd.child_nodes=ch;
        return Ok(Some(Rc::new(RefCell::new(Node::Internal(nd)))));
    }

    let ex = node.unwrap();
    let is_leaf = matches!(&*ex.borrow(), Node::Leaf(_));
    if is_leaf {
        let cur = match &*ex.borrow() { Node::Leaf(l)=>l.nbpoints, _=>0 };
        if cur+nb <= lc {
            let sv1a = match &*ex.borrow() { Node::Leaf(l)=>l.sv1.clone(), _=>None };
            let p = if let Some(ref v)=sv1a { set_paths(points,v,dist,lvl,pl).map_err(|_|MVPError::NoSv1Range)? } else { points.to_vec() };
            let mut start = 0usize;
            let has2 = match &*ex.borrow() { Node::Leaf(l)=>l.sv2.is_some(), _=>false };
            if !has2 && !p.is_empty() {
                start = 1;
                match &mut *ex.borrow_mut() { Node::Leaf(l)=>l.sv2=Some(Arc::clone(&p[0])), _=>{} }
            }
            let sv2a = match &*ex.borrow() { Node::Leaf(l)=>l.sv2.clone(), _=>None };
            let p = if let Some(ref v)=sv2a { set_paths(&p,v,dist,lvl+1,pl).map_err(|_|MVPError::NoSv2Range)? } else { p };
            match &mut *ex.borrow_mut() {
                Node::Leaf(lf) => {
                    for i in start..p.len() {
                        lf.d1.push(dist(&p[i], lf.sv1.as_ref().unwrap()));
                        lf.d2.push(dist(&p[i], lf.sv2.as_ref().unwrap()));
                        lf.points.push(Arc::clone(&p[i]));
                    }
                    lf.nbpoints = lf.points.len();
                }
                _ => {}
            }
            Ok(Some(ex))
        } else {
            let mut tmp: Vec<Arc<MVPDatapoint>> = Vec::new();
            match &*ex.borrow() {
                Node::Leaf(l) => {
                    if let Some(ref v)=l.sv1 { tmp.push(Arc::clone(v)); }
                    if let Some(ref v)=l.sv2 { tmp.push(Arc::clone(v)); }
                    for p in &l.points { tmp.push(Arc::clone(p)); }
                } _ => {}
            }
            for p in points { tmp.push(Arc::clone(p)); }
            _add(bf,pl,lc,dist,None,&tmp,lvl)
        }
    } else {
        let sv1a = match &*ex.borrow() { Node::Internal(n)=>n.sv1.clone().unwrap(), _=>return Err(MVPError::Unrecognized) };
        let sv2a = match &*ex.borrow() { Node::Internal(n)=>n.sv2.clone().unwrap(), _=>return Err(MVPError::Unrecognized) };
        let m1c = match &*ex.borrow() { Node::Internal(n)=>n.m1.clone(), _=>return Err(MVPError::Unrecognized) };
        let p = set_paths(points,&sv1a,dist,lvl,pl).map_err(|_|MVPError::NoSv1Range)?;
        let bins = sort_bins(&p,-1,-1,&sv1a,dist,bf,&m1c).map_err(|_|MVPError::NoSort)?;
        for i in 0..bf {
            if bins[i].is_empty() { continue; }
            let bp = set_paths(&bins[i],&sv2a,dist,lvl+1,pl).map_err(|_|MVPError::NoSv2Range)?;
            let m2s: Vec<f32> = match &*ex.borrow() { Node::Internal(n)=>n.m2[i*lm1..i*lm1+lm1].to_vec(), _=>return Err(MVPError::Unrecognized) };
            let b2 = sort_bins(&bp,-1,-1,&sv2a,dist,bf,&m2s).map_err(|_|MVPError::NoSort)?;
            for j in 0..bf {
                let ci = i*bf+j;
                let ec = match &*ex.borrow() {
                    Node::Internal(n) => {
                        let child_rc = Rc::clone(&n.child_nodes[ci]);
                        let is_empty = match &*child_rc.borrow() {
                            Node::Leaf(l) => l.sv1.is_none() && l.nbpoints == 0,
                            _ => false,
                        };
                        if is_empty { None } else { Some(child_rc) }
                    }
                    _ => None,
                };
                let child = _add(bf,pl,lc,dist,ec,&b2[j],lvl+2)?;
                if let Some(c)=child { match &mut *ex.borrow_mut() { Node::Internal(n)=>n.child_nodes[ci]=c, _=>{} } }
            }
        }
        Ok(Some(ex))
    }
}

fn _retrieve(
    bf: usize, pl: usize, kn: usize, dist: DistanceFunction,
    node: &Rc<RefCell<Node>>, target: &mut MVPDatapoint, radius: f32,
    results: &mut Vec<MVPDatapoint>, lvl: usize,
) -> MVPError {
    let lm1 = bf - 1;
    // We need to collect data from the node without holding the borrow during recursion.
    // Clone what we need, then recurse.
    enum Info {
        Leaf {
            sv1: Option<Arc<MVPDatapoint>>, sv2: Option<Arc<MVPDatapoint>>,
            points: Vec<Arc<MVPDatapoint>>, d1: Vec<f32>, d2: Vec<f32>, nbpoints: usize,
        },
        Internal {
            sv1: Option<Arc<MVPDatapoint>>, sv2: Option<Arc<MVPDatapoint>>,
            m1: Vec<f32>, m2: Vec<f32>, children: Vec<Rc<RefCell<Node>>>,
        },
        Empty,
    }

    let info = {
        let b = node.borrow();
        match &*b {
            Node::Leaf(l) => Info::Leaf {
                sv1: l.sv1.clone(), sv2: l.sv2.clone(),
                points: l.points.clone(), d1: l.d1.clone(), d2: l.d2.clone(), nbpoints: l.nbpoints,
            },
            Node::Internal(n) => Info::Internal {
                sv1: n.sv1.clone(), sv2: n.sv2.clone(),
                m1: n.m1.clone(), m2: n.m2.clone(),
                children: n.child_nodes.iter().map(|c| Rc::clone(c)).collect(),
            },
        }
    };

    match info {
        Info::Leaf { sv1, sv2, points, d1, d2, nbpoints } => {
            let d1v = if let Some(ref sv) = sv1 {
                let d = dist(target, sv);
                if d.is_nan() || d < 0.0 { return MVPError::BadDistVal; }
                if lvl < pl {
                    if target.path.len() <= lvl { target.path.resize(lvl+1, 0.0); }
                    target.path[lvl] = d;
                }
                if d <= radius {
                    results.push((**sv).clone());
                    if results.len() >= kn { return MVPError::KNearestCap; }
                }
                d
            } else { return MVPError::Success; };

            if let Some(ref sv) = sv2 {
                let d2v = dist(target, sv);
                if d2v.is_nan() || d2v < 0.0 { return MVPError::BadDistVal; }
                if d2v <= radius {
                    results.push((**sv).clone());
                    if results.len() >= kn { return MVPError::KNearestCap; }
                }
                if lvl+1 < pl {
                    if target.path.len() <= lvl+1 { target.path.resize(lvl+2, 0.0); }
                    target.path[lvl+1] = d2v;
                }
                for i in 0..nbpoints {
                    if d1v - radius <= d1[i] && d1v + radius >= d1[i] {
                        if d2v - radius <= d2[i] && d2v + radius >= d2[i] {
                            let endpath = if lvl+1 < pl { lvl+1 } else { pl };
                            let mut skip = false;
                            for j in 0..endpath {
                                if j < target.path.len() && j < points[i].path.len() {
                                    if target.path[j]-radius <= points[i].path[j] && target.path[j]+radius >= points[i].path[j] {
                                        continue;
                                    } else { skip=true; break; }
                                }
                            }
                            if !skip {
                                let d = dist(target, &points[i]);
                                if d.is_nan() || d < 0.0 { return MVPError::BadDistVal; }
                                if d <= radius {
                                    results.push((*points[i]).clone());
                                    if results.len() >= kn { return MVPError::KNearestCap; }
                                }
                            }
                        }
                    }
                }
            }
            MVPError::Success
        }
        Info::Internal { sv1, sv2, m1, m2, children } => {
            let d1v = if let Some(ref sv) = sv1 {
                let d = dist(target, sv);
                if d.is_nan() || d < 0.0 { return MVPError::BadDistVal; }
                if d <= radius {
                    results.push((**sv).clone());
                    if results.len() >= kn { return MVPError::KNearestCap; }
                }
                if lvl < pl {
                    if target.path.len() <= lvl { target.path.resize(lvl+1, 0.0); }
                    target.path[lvl] = d;
                }
                d
            } else { return MVPError::Success; };

            let d2v = if let Some(ref sv) = sv2 {
                let d = dist(target, sv);
                if d.is_nan() || d < 0.0 { return MVPError::BadDistVal; }
                if d <= radius {
                    results.push((**sv).clone());
                    if results.len() >= kn { return MVPError::KNearestCap; }
                }
                if lvl+1 < pl {
                    if target.path.len() <= lvl+1 { target.path.resize(lvl+2, 0.0); }
                    target.path[lvl+1] = d;
                }
                d
            } else { return MVPError::Success; };

            // check <= each 1st level bin
            for i in 0..lm1 {
                if d1v - radius <= m1[i] {
                    for j in 0..lm1 {
                        if d2v - radius <= m2[i*lm1+j] {
                            let ci = i*bf+j;
                            if ci < children.len() {
                                let e = _retrieve(bf,pl,kn,dist,&children[ci],target,radius,results,lvl+2);
                                if e != MVPError::Success { return e; }
                            }
                        }
                    }
                    // check >= last 2nd level bin
                    if d2v + radius >= m2[i*lm1+lm1-1] {
                        let ci = i*bf+lm1;
                        if ci < children.len() {
                            let e = _retrieve(bf,pl,kn,dist,&children[ci],target,radius,results,lvl+2);
                            if e != MVPError::Success { return e; }
                        }
                    }
                }
            }
            // check >= last 1st level bin
            if d1v + radius >= m1[lm1-1] {
                for j in 0..lm1 {
                    if d2v - radius <= m2[lm1*lm1+j] {
                        let ci = bf*lm1+j;
                        if ci < children.len() {
                            let e = _retrieve(bf,pl,kn,dist,&children[ci],target,radius,results,lvl+2);
                            if e != MVPError::Success { return e; }
                        }
                    }
                }
                if d2v + radius >= m2[lm1*lm1+lm1-1] {
                    let ci = bf*lm1+lm1;
                    if ci < children.len() {
                        let e = _retrieve(bf,pl,kn,dist,&children[ci],target,radius,results,lvl+2);
                        if e != MVPError::Success { return e; }
                    }
                }
            }
            MVPError::Success
        }
        Info::Empty => MVPError::Success,
    }
}

fn _print(stream: &mut dyn Write, node: &Option<Rc<RefCell<Node>>>, bf: usize, lvl: usize) -> MVPError {
    let lm1 = bf-1;
    let fanout = bf*bf;
    match node {
        None => { let _ = writeln!(stream, "NULL{}", lvl); MVPError::Success }
        Some(rc) => {
            let b = rc.borrow();
            match &*b {
                Node::Leaf(l) => {
                    let _ = writeln!(stream, "LEAF{}  ({} points)", lvl, l.nbpoints);
                    if let Some(ref sv) = l.sv1 { let _ = writeln!(stream, "    sv1: {}", sv.id); }
                    if let Some(ref sv) = l.sv2 { let _ = writeln!(stream, "    sv2: {}", sv.id); }
                    for i in 0..l.nbpoints { let _ = writeln!(stream, "        point[{}]: {}", i, l.points[i].id); }
                    MVPError::Success
                }
                Node::Internal(n) => {
                    let _ = writeln!(stream, "INTERNAL{}", lvl);
                    if let Some(ref sv) = n.sv1 { let _ = writeln!(stream, "  sv1: {}", sv.id); }
                    if let Some(ref sv) = n.sv2 { let _ = writeln!(stream, "  sv2: {}", sv.id); }
                    for i in 0..lm1 { let _ = write!(stream, "  M1[{}] = {:.4};", i, n.m1[i]); }
                    for i in 0..bf { let _ = write!(stream, "  M2[{}] = {:.4};", i, n.m2[i]); }
                    let _ = writeln!(stream);
                    let children: Vec<Rc<RefCell<Node>>> = n.child_nodes.iter().map(|c| Rc::clone(c)).collect();
                    drop(b);
                    for i in 0..fanout {
                        let e = _print(stream, &Some(Rc::clone(&children[i])), bf, lvl+2);
                        if e != MVPError::Success { return e; }
                    }
                    MVPError::Success
                }
            }
        }
    }
}

fn write_dp(buf: &mut Vec<u8>, pos: &mut usize, dp: &Option<Arc<MVPDatapoint>>, pl: usize) -> usize {
    let start = *pos;
    match dp {
        None => {
            buf[*pos] = 0; *pos += 1; // active=0
            buf[*pos..*pos+4].copy_from_slice(&0u32.to_le_bytes()); *pos += 4;
        }
        Some(dp) => {
            buf[*pos] = 1; *pos += 1;
            let idlen = dp.id.len() as u8;
            let tp = dp.data_type as u32;
            let bytelength: u32 = 1 + idlen as u32 + 4 + dp.datalen as u32 * tp + pl as u32 * 4;
            buf[*pos..*pos+4].copy_from_slice(&bytelength.to_le_bytes()); *pos += 4;
            buf[*pos] = idlen; *pos += 1;
            buf[*pos..*pos+idlen as usize].copy_from_slice(dp.id.as_bytes()); *pos += idlen as usize;
            buf[*pos..*pos+4].copy_from_slice(&(dp.datalen as u32).to_le_bytes()); *pos += 4;
            let data_bytes = dp.datalen * tp as usize;
            buf[*pos..*pos+data_bytes].copy_from_slice(&dp.data[..data_bytes]); *pos += data_bytes;
            for i in 0..pl {
                let v = if i < dp.path.len() { dp.path[i] } else { 0.0 };
                buf[*pos..*pos+4].copy_from_slice(&v.to_le_bytes()); *pos += 4;
            }
        }
    }
    start
}

fn read_dp(buf: &[u8], pos: &mut usize, pl: usize, dt: MVPDataType) -> Option<Arc<MVPDatapoint>> {
    let active = buf[*pos]; *pos += 1;
    let bl = u32::from_le_bytes(buf[*pos..*pos+4].try_into().unwrap()); *pos += 4;
    if active == 0 && bl == 0 { return None; }
    let idlen = buf[*pos] as usize; *pos += 1;
    let id = String::from_utf8_lossy(&buf[*pos..*pos+idlen]).to_string(); *pos += idlen;
    let datalen = u32::from_le_bytes(buf[*pos..*pos+4].try_into().unwrap()) as usize; *pos += 4;
    let tp = dt as usize;
    let data = buf[*pos..*pos+datalen*tp].to_vec(); *pos += datalen*tp;
    let mut path = vec![0.0f32; pl];
    for i in 0..pl { path[i] = f32::from_le_bytes(buf[*pos..*pos+4].try_into().unwrap()); *pos += 4; }
    Some(Arc::new(MVPDatapoint { id, data, path, datalen, data_type: dt }))
}

fn _write_tree(buf: &mut Vec<u8>, pos: &mut usize, node: &Option<Rc<RefCell<Node>>>,
               bf: usize, lc: usize, pl: usize, pgsize: usize) -> MVPError {
    let node = match node { Some(n) => n, None => return MVPError::Success };
    let b = node.borrow();
    match &*b {
        Node::Leaf(l) => {
            // ensure capacity
            while *pos + pgsize > buf.len() { buf.resize(buf.len() + pgsize, 0); }
            buf[*pos] = NodeType::LeafNode as u8; *pos += 1;
            write_dp(buf, pos, &l.sv1, pl);
            write_dp(buf, pos, &l.sv2, pl);
            buf[*pos..*pos+4].copy_from_slice(&(l.nbpoints as u32).to_le_bytes()); *pos += 4;
            let saved = *pos;
            *pos += lc * (2*4 + 8); // reserve space for d1,d2,offset per point
            let mut sp = saved;
            for i in 0..l.nbpoints {
                while *pos + pgsize > buf.len() { buf.resize(buf.len() + pgsize, 0); }
                buf[sp..sp+4].copy_from_slice(&l.d1[i].to_le_bytes()); sp += 4;
                buf[sp..sp+4].copy_from_slice(&l.d2[i].to_le_bytes()); sp += 4;
                let offset = *pos as u64;
                write_dp(buf, pos, &Some(Arc::clone(&l.points[i])), pl);
                buf[sp..sp+8].copy_from_slice(&offset.to_le_bytes()); sp += 8;
            }
        }
        Node::Internal(n) => {
            while *pos + pgsize > buf.len() { buf.resize(buf.len() + pgsize, 0); }
            let lm1 = bf-1;
            let lm2 = lm1*bf;
            let fanout = bf*bf;
            buf[*pos] = NodeType::InternalNode as u8; *pos += 1;
            write_dp(buf, pos, &n.sv1, pl);
            write_dp(buf, pos, &n.sv2, pl);
            for i in 0..lm1 { buf[*pos..*pos+4].copy_from_slice(&n.m1[i].to_le_bytes()); *pos += 4; }
            for i in 0..lm2 { buf[*pos..*pos+4].copy_from_slice(&n.m2[i].to_le_bytes()); *pos += 4; }
            let saved = *pos;
            *pos += fanout * (1 + 8);
            let children: Vec<Rc<RefCell<Node>>> = n.child_nodes.iter().map(|c| Rc::clone(c)).collect();
            drop(b);
            let mut sp = saved;
            for i in 0..fanout {
                while *pos + pgsize > buf.len() { buf.resize(buf.len() + pgsize, 0); }
                let offset = *pos as u64;
                let child_has_points = {
                    let cb = children[i].borrow();
                    match &*cb {
                        Node::Leaf(l) => l.sv1.is_some() || l.nbpoints > 0,
                        Node::Internal(_) => true,
                    }
                };
                if child_has_points {
                    _write_tree(buf, pos, &Some(Rc::clone(&children[i])), bf, lc, pl, pgsize);
                }
                buf[sp] = 0; sp += 1; // fileno
                let off = if child_has_points { offset } else { 0u64 };
                buf[sp..sp+8].copy_from_slice(&off.to_le_bytes()); sp += 8;
            }
            return MVPError::Success;
        }
    }
    MVPError::Success
}

fn _read_node(buf: &[u8], pos: &mut usize, bf: usize, lc: usize, pl: usize, dt: MVPDataType) -> Result<Option<Rc<RefCell<Node>>>, MVPError> {
    if *pos >= buf.len() { return Ok(None); }
    let nt = buf[*pos]; *pos += 1;
    if nt == NodeType::LeafNode as u8 {
        let sv1 = read_dp(buf, pos, pl, dt);
        let sv2 = read_dp(buf, pos, pl, dt);
        let nbpoints = u32::from_le_bytes(buf[*pos..*pos+4].try_into().unwrap()) as usize; *pos += 4;
        let saved = *pos;
        let mut lf = LeafNode::new(lc as u32);
        lf.sv1 = sv1; lf.sv2 = sv2; lf.nbpoints = nbpoints;
        let mut sp = saved;
        for _ in 0..nbpoints {
            let d1 = f32::from_le_bytes(buf[sp..sp+4].try_into().unwrap()); sp += 4;
            let d2 = f32::from_le_bytes(buf[sp..sp+4].try_into().unwrap()); sp += 4;
            let offset = u64::from_le_bytes(buf[sp..sp+8].try_into().unwrap()) as usize; sp += 8;
            *pos = offset;
            let dp = read_dp(buf, pos, pl, dt);
            if let Some(p) = dp { lf.points.push(p); }
            lf.d1.push(d1); lf.d2.push(d2);
        }
        Ok(Some(Rc::new(RefCell::new(Node::Leaf(lf)))))
    } else if nt == NodeType::InternalNode as u8 {
        let lm1 = bf-1;
        let lm2 = lm1*bf;
        let fanout = bf*bf;
        let sv1 = read_dp(buf, pos, pl, dt);
        let sv2 = read_dp(buf, pos, pl, dt);
        let mut m1 = vec![0.0f32; lm1];
        for i in 0..lm1 { m1[i] = f32::from_le_bytes(buf[*pos..*pos+4].try_into().unwrap()); *pos += 4; }
        let mut m2 = vec![0.0f32; lm2];
        for i in 0..lm2 { m2[i] = f32::from_le_bytes(buf[*pos..*pos+4].try_into().unwrap()); *pos += 4; }
        let saved = *pos;
        let mut nd = InternalNode::new(bf as u32);
        nd.sv1 = sv1; nd.sv2 = sv2; nd.m1 = m1; nd.m2 = m2;
        let mut sp = saved;
        for _ in 0..fanout {
            let _fileno = buf[sp]; sp += 1;
            let offset = u64::from_le_bytes(buf[sp..sp+8].try_into().unwrap()) as usize; sp += 8;
            if offset == 0 {
                nd.child_nodes.push(enode());
            } else {
                *pos = offset;
                let child = _read_node(buf, pos, bf, lc, pl, dt)?;
                nd.child_nodes.push(child.unwrap_or_else(enode));
            }
        }
        Ok(Some(Rc::new(RefCell::new(Node::Internal(nd)))))
    } else {
        Err(MVPError::Unrecognized)
    }
}

impl MVPTree {
    pub fn new(branch_factor: usize, path_length: usize, leaf_capacity: usize, datatype: MVPDataType, distance_function: DistanceFunction) -> Self {
        Self { branch_factor, path_length, leaf_capacity, datatype,
               pos: 0, size: 0, pgsize: 4096, buf: Vec::new(), node: None, distance_function }
    }

    pub fn add(&mut self, points: Vec<MVPDatapoint>) -> MVPError {
        if points.is_empty() { return MVPError::Success; }
        if self.datatype as u32 == 0 {
            // This shouldn't happen with the enum, but match C behavior:
            // first add sets the type
        }
        if self.datatype != points[0].data_type {
            // If tree already has data of different type
            if self.node.is_some() { return MVPError::TypeMismatch; }
            self.datatype = points[0].data_type;
        }
        let arcs: Vec<Arc<MVPDatapoint>> = points.into_iter().map(|mut p| {
            p.path = vec![0.0; self.path_length];
            Arc::new(p)
        }).collect();
        match _add(self.branch_factor, self.path_length, self.leaf_capacity, self.distance_function, self.node.take(), &arcs, 0) {
            Ok(n) => { self.node = n; MVPError::Success }
            Err(e) => e
        }
    }

    pub fn retrieve(&self, target: &MVPDatapoint, knearest: usize, radius: f32) -> Result<Vec<MVPDatapoint>, MVPError> {
        if knearest == 0 || radius < 0.0 { return Err(MVPError::ArgErr); }
        let node = match &self.node { Some(n) => n, None => return Err(MVPError::EmptyTree) };
        let mut tgt = target.clone();
        tgt.path = vec![0.0; self.path_length];
        let mut results = Vec::new();
        let err = _retrieve(self.branch_factor, self.path_length, knearest, self.distance_function, node, &mut tgt, radius, &mut results, 0);
        if err != MVPError::Success && err != MVPError::KNearestCap { return Err(err); }
        Ok(results)
    }

    pub fn write(&self, filename: &str, _mode: i32) -> MVPError {
        if self.node.is_none() { return MVPError::ArgErr; }
        let pgsize = 4096usize;
        let mut buf = vec![0u8; pgsize];
        // write header
        let mut pos = 0usize;
        let tag_bytes = TAG.as_bytes();
        buf[pos..pos+tag_bytes.len()].copy_from_slice(tag_bytes);
        pos += tag_bytes.len();
        buf[pos] = 0; pos += 1; // null terminator
        buf[pos..pos+4].copy_from_slice(&VERSION.to_le_bytes()); pos += 4;
        buf[pos] = self.branch_factor as u8; pos += 1;
        buf[pos] = self.path_length as u8; pos += 1;
        buf[pos] = self.leaf_capacity as u8; pos += 1;
        // ht = type of sv1 of root
        let ht = {
            let b = self.node.as_ref().unwrap().borrow();
            match &*b {
                Node::Leaf(l) => l.sv1.as_ref().map(|s| s.data_type as u8).unwrap_or(self.datatype as u8),
                Node::Internal(n) => n.sv1.as_ref().map(|s| s.data_type as u8).unwrap_or(self.datatype as u8),
            }
        };
        buf[pos] = ht; pos += 1;
        pos = HEADER_SIZE;
        _write_tree(&mut buf, &mut pos, &self.node, self.branch_factor, self.leaf_capacity, self.path_length, pgsize);
        match File::create(filename) {
            Ok(mut f) => {
                if f.write_all(&buf[..pos]).is_err() { return MVPError::NoWrite; }
                MVPError::Success
            }
            Err(_) => MVPError::FileOpen,
        }
    }

    pub fn print(&self, stream: &mut dyn Write) -> MVPError {
        let e = _print(stream, &self.node, self.branch_factor, 0);
        if e != MVPError::Success {
            let _ = writeln!(stream, "malformed tree: {}", error_to_string(e));
        }
        e
    }

    pub fn clear(&mut self, _node: &mut Option<Box<Node>>) {
        self.node = None;
    }

    pub fn extend_mvpfile(&mut self) -> i32 {
        let new_size = self.buf.len() + self.pgsize as usize;
        self.buf.resize(new_size, 0);
        self.size = new_size as i64;
        0
    }
}

pub fn mvptree_read(filename: &str, distance_function: DistanceFunction) -> Result<MVPTree, MVPError> {
    let mut f = File::open(filename).map_err(|_| MVPError::FileNotFound)?;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf).map_err(|_| MVPError::FileOpen)?;
    if buf.len() < HEADER_SIZE { return Err(MVPError::FileOpen); }
    let mut pos = 0usize;
    let tag_len = TAG.len();
    pos += tag_len + 1; // skip tag + null
    let _version = u32::from_le_bytes(buf[pos..pos+4].try_into().unwrap()); pos += 4;
    let bf = buf[pos] as usize; pos += 1;
    let pl = buf[pos] as usize; pos += 1;
    let lc = buf[pos] as usize; pos += 1;
    let ht = buf[pos]; pos += 1;
    let dt = match ht { 1 => MVPDataType::ByteArray, 2 => MVPDataType::UInt16Array, 4 => MVPDataType::UInt32Array, 8 => MVPDataType::UInt64Array, _ => MVPDataType::ByteArray };
    pos = HEADER_SIZE;
    let node = _read_node(&buf, &mut pos, bf, lc, pl, dt)?;
    Ok(MVPTree {
        branch_factor: bf, path_length: pl, leaf_capacity: lc, datatype: dt,
        pos: 0, size: buf.len() as i64, pgsize: 4096, buf: Vec::new(),
        node, distance_function,
    })
}

impl MVPDatapoint {
    pub fn new(id: String, data: Vec<u8>, data_type: MVPDataType) -> Self {
        let datalen = data.len();
        MVPDatapoint { id, data, path: vec![], datalen, data_type }
    }

    pub fn select_vantage_points(&mut self, _nb: u32, _sv1_pos: i32, _sv2_pos: i32, _dist: DistanceFunction) -> i32 {
        // This is not used directly - the free function select_vp is used instead
        0
    }

    pub fn find_splits(&mut self, _nb: u32, _vp: &MVPDatapoint, _tree: &MVPTree, _lengthM: u32) -> f32 {
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
    ERROR_MSGS[error as usize]
}
