pub const NB: usize = 4;
pub const NR: usize = 10;
pub const NK: usize = 4;
pub fn inv_shift(state: &mut [[u8; NB]; 4]) {
    for (i, row) in state.iter_mut().enumerate().skip(1) {
        row.rotate_right(i % NB);
    }
}
pub fn shift(state: &mut [[u8; NB]; 4]) {
    for (i, row) in state.iter_mut().enumerate().skip(1) {
        row.rotate_left(i % NB);
    }
}
