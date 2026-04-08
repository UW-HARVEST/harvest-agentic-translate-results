pub const NB: usize = 4;
pub const NR: usize = 10;
pub const NK: usize = 4;

pub fn shift(state: &mut [[u8; NB]; 4]) {
    for i in 1..4 {
        let mut temp = [0u8; NB];
        for j in 0..NB {
            temp[j] = state[i][(j + i) % NB];
        }
        state[i] = temp;
    }
}

pub fn inv_shift(state: &mut [[u8; NB]; 4]) {
    for i in 1..4 {
        let mut temp = [0u8; NB];
        for j in 0..NB {
            temp[j] = state[i][((j as isize - i as isize + NB as isize) as usize) % NB];
        }
        state[i] = temp;
    }
}
