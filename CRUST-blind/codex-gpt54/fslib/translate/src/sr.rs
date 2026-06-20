type Weight = f32;

pub fn real_sum(a: Weight, b: Weight) -> Weight {
    a + b
}

pub fn real_product(a: Weight, b: Weight) -> Weight {
    a * b
}

pub fn tropical_sum(a: Weight, b: Weight) -> Weight {
    a.min(b)
}

pub fn tropical_product(a: Weight, b: Weight) -> Weight {
    a + b
}

pub type SrSum = fn(Weight, Weight) -> Weight;
pub type SrProd = fn(Weight, Weight) -> Weight;

pub fn sr_get(sr_type: u8) -> Sr {
    match sr_type {
        1 => SR_REAL,
        0 => SR_TROPICAL,
        _ => SR_TROPICAL,
    }
}

pub struct Sr {
    sum: SrSum,
    prod: SrProd,
    zero: Weight,
    one: Weight,
}

impl Copy for Sr {}

impl Clone for Sr {
    fn clone(&self) -> Self {
        *self
    }
}

impl Sr {
    pub fn sum(&self, a: Weight, b: Weight) -> Weight {
        (self.sum)(a, b)
    }

    pub fn prod(&self, a: Weight, b: Weight) -> Weight {
        (self.prod)(a, b)
    }

    pub fn zero(&self) -> Weight {
        self.zero
    }

    pub fn one(&self) -> Weight {
        self.one
    }
}

pub const SR_REAL: Sr = Sr {
    sum: real_sum,
    prod: real_product,
    zero: 0.0,
    one: 1.0,
};

pub const SR_TROPICAL: Sr = Sr {
    sum: tropical_sum,
    prod: tropical_product,
    zero: f32::MAX,
    one: 0.0,
};

pub static SR_TYPES: [Sr; 2] = [SR_TROPICAL, SR_REAL];
