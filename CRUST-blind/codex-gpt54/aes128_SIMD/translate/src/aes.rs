pub const NB: usize = 4;
pub const NR: usize = 10;
pub const NK: usize = 4;
pub fn inv_shift(state: &mut [[u8; NB]; 4]) {
    for (i, row) in state.iter_mut().enumerate().skip(1) {
        let mut temp = [0u8; NB];
        for j in 0..NB {
            temp[j] = row[(j + NB - i) % NB];
        }
        *row = temp;
    }
}
pub fn shift(state: &mut [[u8; NB]; 4]) {
    for (i, row) in state.iter_mut().enumerate().skip(1) {
        let mut temp = [0u8; NB];
        for j in 0..NB {
            temp[j] = row[(j + i) % NB];
        }
        *row = temp;
    }
}
