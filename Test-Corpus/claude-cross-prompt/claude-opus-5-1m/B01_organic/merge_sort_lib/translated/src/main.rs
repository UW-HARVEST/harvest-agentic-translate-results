// Translated from c_src/src/lib.c
// The original C source is a library (built as SHARED in CMakeLists.txt)
// implementing a merge_sort function. The library performs no I/O of its own,
// so the executable produces no output (byte-identical empty output).

#[derive(Copy, Clone, Default)]
pub struct SpritebatchSprite {
    pub texture_id: u64,
    pub sort_bits: i32,
}

fn spritebatch_internal_sprite_less_than_or_equal(
    a: &SpritebatchSprite,
    b: &SpritebatchSprite,
) -> bool {
    if a.sort_bits <= b.sort_bits {
        return true;
    }
    if a.sort_bits == b.sort_bits && a.texture_id <= b.texture_id {
        return true;
    }
    false
}

fn spritebatch_internal_merge_sort_iteration(
    a: &[SpritebatchSprite],
    lo: usize,
    split: usize,
    hi: usize,
    b: &mut [SpritebatchSprite],
) {
    let mut i = lo;
    let mut j = split;
    for k in lo..hi {
        if i < split
            && (j >= hi || spritebatch_internal_sprite_less_than_or_equal(&a[i], &a[j]))
        {
            b[k] = a[i];
            i += 1;
        } else {
            b[k] = a[j];
            j += 1;
        }
    }
}

fn spritebatch_internal_merge_sort_recurse(
    b: &mut [SpritebatchSprite],
    lo: usize,
    hi: usize,
    a: &mut [SpritebatchSprite],
) {
    if hi - lo <= 1 {
        return;
    }
    let split = (lo + hi) / 2;
    spritebatch_internal_merge_sort_recurse(a, lo, split, b);
    spritebatch_internal_merge_sort_recurse(a, split, hi, b);
    // The recurse call above takes (a, lo, split, b) where the names reflect
    // that on each recursion the roles of source/dest buffers swap.
    // Here we need to read from `b` (source) and write merged result to `a` (dest).
    // But the iteration helper signature is (source, lo, split, hi, dest).
    // In C: spritebatch_internal_merge_sort_iteration(b, lo, split, hi, a);
    spritebatch_internal_merge_sort_iteration(b, lo, split, hi, a);
}

pub fn merge_sort(a: &mut [SpritebatchSprite], b: &mut [SpritebatchSprite], size: usize) {
    // memcpy(b, a, sizeof(spritebatch_sprite_t) * size);
    b[..size].copy_from_slice(&a[..size]);
    spritebatch_internal_merge_sort_recurse(b, 0, size, a);
}

fn main() {
    // Original C source is a library with no I/O; produce no output.
}
