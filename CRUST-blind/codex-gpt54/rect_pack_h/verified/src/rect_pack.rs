use std::cmp::Ordering;
/// Output information for a packed rectangle
#[derive(Debug, Clone, Copy)]
pub struct RectOutInfo {
    /// X coordinate in the packed layout
    pub x: i32,
    /// Y coordinate in the packed layout
    pub y: i32,
    /// Whether the rectangle was successfully packed
    pub packed: bool,
    /// The page number where the rectangle was packed (for multi-page packing)
    pub page: i32,
}
impl Default for RectOutInfo {
    fn default() -> Self {
        Self {
            x: 0,
            y: 0,
            packed: false,
            page: 0,
        }
    }
}
/// A rectangle to be packed
#[derive(Debug, Clone)]
pub struct Rect {
    /// Unique identifier for the rectangle
    pub id: i32,
    /// Width of the rectangle
    pub w: i32,
    /// Height of the rectangle
    pub h: i32,
    /// Output information after packing
    pub info: RectOutInfo,
}
impl Rect {
    /// Create a new rectangle with the given dimensions
    pub fn new(id: i32, width: i32, height: i32) -> Self {
        Self {
            id,
            w: width,
            h: height,
            info: RectOutInfo::default(),
        }
    }
}
/// Rectangle packer using a binary tree algorithm
pub struct RectPacker;
impl RectPacker {
    /// Pack rectangles into a bin of the given maximum dimensions
    ///
    /// # Arguments
    ///
    /// * `max_w` - Maximum width of the packing area
    /// * `max_h` - Maximum height of the packing area
    /// * `paging` - Whether to allow multiple pages for packing
    /// * `rects` - Mutable slice of rectangles to pack
    ///
    /// # Returns
    ///
    /// `true` if all rectangles were successfully packed, `false` otherwise
    pub fn pack(max_w: i32, max_h: i32, paging: bool, rects: &mut [Rect]) -> bool {
        if rects.is_empty() {
            return true;
        }

        rects.sort_unstable_by(compare_rects);

        for rect in rects.iter_mut() {
            rect.info = RectOutInfo::default();
        }

        let mut ctx = PackCtx {
            max_w,
            max_h,
            page: 0,
            next: 0,
            last: rects.len() - 1,
        };

        let mut ok = false;
        let mut all_packed = false;

        while !ok {
            let res = pack_bin_tree(&mut ctx, rects);
            ok = res.all_fit;
            all_packed |= ok;

            if !paging || res.none_fit {
                break;
            }

            ctx.page += 1;
        }

        all_packed
    }
}
/// A node in the binary tree used for rectangle packing
struct BinNode {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    used: bool,
    right: Option<Box<BinNode>>,
    down: Option<Box<BinNode>>,
}
impl BinNode {
    /// Create a new bin node with the given dimensions
    fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self {
            x,
            y,
            w,
            h,
            used: false,
            right: None,
            down: None,
        }
    }
    /// Find a node in the tree that can fit a rectangle of the given dimensions
    fn find(&self, w: i32, h: i32) -> Option<&BinNode> {
        find_node(self, w, h)
    }
    /// Split this node after placing a rectangle of the given dimensions
    fn split(&mut self, w: i32, h: i32) -> &Self {
        self.used = true;
        self.down = Some(Box::new(BinNode::new(
            self.x,
            self.y + h,
            self.w,
            self.h - h,
        )));
        self.right = Some(Box::new(BinNode::new(
            self.x + w,
            self.y,
            self.w - w,
            h,
        )));
        self
    }
    /// Grow the bin to the right to accommodate a rectangle
    fn grow_right(&mut self, rect: &Rect, max_w: i32, max_h: i32) -> Option<&BinNode> {
        let _ = (max_w, max_h);
        grow_right_impl(self, rect.w, rect.h)
            .and_then(|(x, y)| find_node_by_position(self, x, y))
    }
    /// Grow the bin downward to accommodate a rectangle
    fn grow_down(&mut self, rect: &Rect, max_w: i32, max_h: i32) -> Option<&BinNode> {
        let _ = (max_w, max_h);
        grow_down_impl(self, rect.w, rect.h)
            .and_then(|(x, y)| find_node_by_position(self, x, y))
    }
    /// Grow the bin in the optimal direction to fit a rectangle
    fn grow(&mut self, rect: &Rect, max_w: i32, max_h: i32) -> Option<&BinNode> {
        let can_grow_down =
            rect.w <= self.w && i64::from(rect.h) + i64::from(self.h) <= i64::from(max_h);
        let can_grow_right =
            rect.h <= self.h && i64::from(rect.w) + i64::from(self.w) <= i64::from(max_w);

        let should_grow_right =
            can_grow_right && i64::from(self.h) >= i64::from(self.w) + i64::from(rect.w);
        let should_grow_down =
            can_grow_down && i64::from(self.w) >= i64::from(self.h) + i64::from(rect.h);

        if should_grow_right {
            return self.grow_right(rect, max_w, max_h);
        }
        if should_grow_down {
            return self.grow_down(rect, max_w, max_h);
        }
        if can_grow_right {
            return self.grow_right(rect, max_w, max_h);
        }
        if can_grow_down {
            return self.grow_down(rect, max_w, max_h);
        }

        None
    }
}

struct PackCtx {
    max_w: i32,
    max_h: i32,
    page: i32,
    next: usize,
    last: usize,
}

struct PackRes {
    all_fit: bool,
    none_fit: bool,
}

fn compare_rects(left: &Rect, right: &Rect) -> Ordering {
    let left_max = left.w.max(left.h);
    let right_max = right.w.max(right.h);
    right_max
        .cmp(&left_max)
        .then_with(|| right.w.min(right.h).cmp(&left.w.min(left.h)))
}

fn find_node(node: &BinNode, w: i32, h: i32) -> Option<&BinNode> {
    if node.used {
        node.right
            .as_deref()
            .and_then(|right| find_node(right, w, h))
            .or_else(|| node.down.as_deref().and_then(|down| find_node(down, w, h)))
    } else if w <= node.w && h <= node.h {
        Some(node)
    } else {
        None
    }
}

fn find_node_mut(node: &mut BinNode, w: i32, h: i32) -> Option<&mut BinNode> {
    if node.used {
        if let Some(found) = node
            .right
            .as_deref_mut()
            .and_then(|right| find_node_mut(right, w, h))
        {
            return Some(found);
        }

        node.down
            .as_deref_mut()
            .and_then(|down| find_node_mut(down, w, h))
    } else if w <= node.w && h <= node.h {
        Some(node)
    } else {
        None
    }
}

fn find_node_by_position(node: &BinNode, x: i32, y: i32) -> Option<&BinNode> {
    if node.x == x && node.y == y {
        return Some(node);
    }

    node.right
        .as_deref()
        .and_then(|right| find_node_by_position(right, x, y))
        .or_else(|| {
            node.down
                .as_deref()
                .and_then(|down| find_node_by_position(down, x, y))
        })
}

fn grow_right_impl(root: &mut BinNode, w: i32, h: i32) -> Option<(i32, i32)> {
    let old = std::mem::replace(root, BinNode::new(0, 0, 0, 0));
    let old_w = old.w;
    let old_h = old.h;

    *root = BinNode {
        x: 0,
        y: 0,
        w: old_w + w,
        h: old_h,
        used: true,
        right: Some(Box::new(BinNode::new(old_w, 0, w, old_h))),
        down: Some(Box::new(old)),
    };

    let node = find_node_mut(root, w, h)?;
    let x = node.x;
    let y = node.y;
    node.split(w, h);
    Some((x, y))
}

fn grow_down_impl(root: &mut BinNode, w: i32, h: i32) -> Option<(i32, i32)> {
    let old = std::mem::replace(root, BinNode::new(0, 0, 0, 0));
    let old_w = old.w;
    let old_h = old.h;

    *root = BinNode {
        x: 0,
        y: 0,
        w: old_w,
        h: old_h + h,
        used: true,
        right: Some(Box::new(old)),
        down: Some(Box::new(BinNode::new(0, old_h, old_w, h))),
    };

    let node = find_node_mut(root, w, h)?;
    let x = node.x;
    let y = node.y;
    node.split(w, h);
    Some((x, y))
}

fn pack_bin_tree(ctx: &mut PackCtx, rects: &mut [Rect]) -> PackRes {
    let mut res = PackRes {
        all_fit: true,
        none_fit: true,
    };

    let root_w = if rects[ctx.next].w <= ctx.max_w {
        rects[ctx.next].w
    } else {
        ctx.max_w
    };
    let root_h = if rects[ctx.next].h <= ctx.max_h {
        rects[ctx.next].h
    } else {
        ctx.max_h
    };

    let mut root = BinNode::new(0, 0, root_w, root_h);
    let mut contiguous = true;
    let mut last = ctx.last;

    for i in ctx.next..=ctx.last {
        if !rects[i].info.packed {
            let w = rects[i].w;
            let h = rects[i].h;

            if root.find(w, h).is_some() {
                let node = find_node_mut(&mut root, w, h).expect("validated by immutable search");
                let x = node.x;
                let y = node.y;
                node.split(w, h);

                rects[i].info.x = x;
                rects[i].info.y = y;
                rects[i].info.packed = true;
                rects[i].info.page = ctx.page;
                res.none_fit = false;
            } else if let Some(node) = root.grow(&rects[i], ctx.max_w, ctx.max_h) {
                rects[i].info.x = node.x;
                rects[i].info.y = node.y;
                rects[i].info.packed = true;
                rects[i].info.page = ctx.page;
                res.none_fit = false;
            } else {
                rects[i].info.packed = false;
                res.all_fit = false;
                contiguous = false;
                last = i;
            }
        }

        if contiguous {
            ctx.next = i + 1;
        }
    }

    ctx.last = last;
    res
}
