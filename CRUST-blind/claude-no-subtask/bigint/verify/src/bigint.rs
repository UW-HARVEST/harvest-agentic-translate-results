use std::cmp::Ordering;
use std::fmt;

#[derive(Clone, Debug)]
pub struct BigInt {
    negative: bool,
    digits: Vec<u8>,
}

impl BigInt {
    pub fn zero() -> Self {
        BigInt {
            negative: false,
            digits: vec![0],
        }
    }

    pub fn is_64_bit(&self) -> bool {
        // Match C: size > 10 -> false, size < 10 -> true, size == 10 -> false (fall-through).
        self.digits.len() < 10
    }

    pub fn remove_leading_zeros(&mut self) {
        let mut leading_zeros = 0usize;
        for &d in &self.digits {
            if d == 0 {
                leading_zeros += 1;
            } else {
                break;
            }
        }
        if leading_zeros == 0 {
            return;
        }
        if leading_zeros >= self.digits.len() {
            // All zeros: collapse to single 0 digit.
            self.digits.clear();
            self.digits.push(0);
            return;
        }
        self.digits.drain(..leading_zeros);
    }

    pub fn from_int(n: i64) -> Self {
        let negative = n < 0;
        // Use i128 to safely take absolute value, even for i64::MIN.
        let mut m: u128 = if n < 0 {
            (n as i128).unsigned_abs()
        } else {
            n as u128
        };
        let mut digits: Vec<u8> = Vec::new();
        if m == 0 {
            digits.push(0);
        } else {
            while m > 0 {
                digits.push((m % 10) as u8);
                m /= 10;
            }
            digits.reverse();
        }
        let mut result = BigInt { negative, digits };
        result.remove_leading_zeros();
        result
    }

    pub fn to_int(&self) -> i64 {
        let mut result: i64 = 0;
        for &d in &self.digits {
            result = result.wrapping_mul(10).wrapping_add(d as i64);
        }
        if self.negative {
            result.wrapping_neg()
        } else {
            result
        }
    }

    pub fn from_str(s: &str) -> Self {
        let (negative, rest) = if let Some(stripped) = s.strip_prefix('-') {
            (true, stripped)
        } else {
            (false, s)
        };
        let digits: Vec<u8> = rest
            .bytes()
            .map(|b| b.wrapping_sub(b'0'))
            .collect();
        BigInt { negative, digits }
    }

    pub fn copy(&self) -> BigInt {
        self.clone()
    }

    pub fn print(&self) {
        print!("{}", self);
    }

    pub fn is_zero(&self) -> bool {
        let mut c = self.clone();
        c.remove_leading_zeros();
        c.digits.len() == 1 && c.digits[0] == 0
    }

    pub fn lt_zero(&self) -> bool {
        if self.is_zero() {
            return false;
        }
        self.negative
    }

    pub fn gt_zero(&self) -> bool {
        if self.is_zero() {
            return false;
        }
        !self.negative
    }

    pub fn lezero(&self) -> bool {
        if self.is_zero() {
            return true;
        }
        self.negative
    }

    pub fn gezero(&self) -> bool {
        if self.is_zero() {
            return true;
        }
        !self.negative
    }

    pub fn abs(&self) -> Self {
        BigInt {
            negative: false,
            digits: self.digits.clone(),
        }
    }

    pub fn add(&self, other: &Self) -> Self {
        bigint_add(self.clone(), other.clone())
    }

    pub fn sub(&self, other: &Self) -> Self {
        bigint_sub(self.clone(), other.clone())
    }

    pub fn inc(&mut self) {
        let one = BigInt::from_str("1");
        let new_val = self.add(&one);
        *self = new_val;
    }

    pub fn dec(&mut self) {
        let one = BigInt::from_str("1");
        let new_val = self.sub(&one);
        *self = new_val;
    }

    pub fn mul(&self, other: &Self) -> Self {
        bigint_mul(self, other)
    }

    pub fn divmod(&self, other: &Self) -> (Self, Self) {
        bigint_divmod(self, other)
    }

    pub fn div(&self, other: &Self) -> Self {
        if self.is_64_bit() && other.is_64_bit() {
            return BigInt::from_int(self.to_int() / other.to_int());
        }
        let (q, _r) = bigint_divmod(self, other);
        q
    }

    pub fn r#mod(&self, other: &Self) -> Self {
        if self.is_64_bit() && other.is_64_bit() {
            return BigInt::from_int(self.to_int() % other.to_int());
        }
        let (_q, r) = bigint_divmod(self, other);
        r
    }

    pub fn pow(&self, exponent: &Self) -> Self {
        if exponent.negative {
            return BigInt::from_str("0");
        }
        let mut result = BigInt::from_str("1");
        if exponent.is_zero() {
            return result;
        }
        let mut b = exponent.clone();
        while !b.is_zero() {
            result = result.mul(self);
            b.dec();
        }
        result
    }

    pub fn fast_pow(&self, exponent: &Self, modulo: &Self) -> Self {
        if self.is_64_bit() && exponent.is_64_bit() && modulo.is_64_bit() {
            let mut result: i64 = 1;
            let mut base: i64 = self.to_int();
            let mut exp: i64 = exponent.to_int();
            let m: i64 = modulo.to_int();
            base = base % m;
            while exp > 0 {
                if exp % 2 == 1 {
                    result = result.wrapping_mul(base) % m;
                }
                exp >>= 1;
                base = base.wrapping_mul(base) % m;
            }
            return BigInt::from_int(result);
        }

        if exponent.negative {
            return BigInt::from_str("0");
        }
        if exponent.is_zero() {
            return BigInt::from_str("1");
        }

        let a_red = self.r#mod(modulo);
        let mut result = a_red.clone();
        let mut b_save = exponent.clone();
        let mut b = exponent.clone();
        b.dec();

        let mut pow_acc = BigInt::from_str("1");
        loop {
            let two = BigInt::from_str("2");
            let halved = b.div(&two);
            if !halved.gt_zero() {
                break;
            }
            // result = result^2 mod m
            let squared = result.mul(&result);
            result = squared.r#mod(modulo);
            // b = b / 2
            b = b.div(&two);
            // pow_acc = pow_acc * 2
            pow_acc = pow_acc.add(&pow_acc);
        }

        b_save = b_save.sub(&pow_acc);

        if b_save.is_odd() {
            let prod = result.mul(&a_red);
            result = prod.r#mod(modulo);
            b_save.dec();
            pow_acc.inc();
        }

        let five = BigInt::from_int(5);
        if lt(&b_save, &five) {
            while !b_save.is_zero() {
                let prod = result.mul(&a_red);
                result = prod.r#mod(modulo);
                b_save.dec();
                pow_acc.inc();
            }
            return result;
        }

        let subpart = a_red.fast_pow(&b_save, modulo);
        let combined = result.mul(&subpart);
        combined.r#mod(modulo)
    }

    pub fn modinv(&self, modulo: &Self) -> Self {
        let m0 = modulo.clone();
        let mut y = BigInt::from_str("0");
        let mut x = BigInt::from_str("1");
        let one = BigInt::from_str("1");
        let mut a = self.clone();
        let mut m = modulo.clone();

        while !a.is_zero() {
            let q = m.div(&a);
            let qa = q.mul(&a);
            let new_a = m.sub(&qa);
            m = a;
            a = new_a;

            let qx = q.mul(&x);
            let new_x = y.sub(&qx);
            y = x;
            x = new_x;
        }

        if m.lt_zero() {
            m = m.add(&m0);
        }

        if eq(&m, &one) {
            let mut result = y.r#mod(&m0);
            if result.lt_zero() {
                result = result.add(&m0);
            }
            return result;
        }

        BigInt::from_int(0)
    }

    pub fn sqrt(&self) -> Self {
        let one = BigInt::from_str("1");
        let two = BigInt::from_str("2");
        let mut low = BigInt::from_str("0");
        let mut high = self.clone();
        while lt(&low, &high) {
            let sum = low.add(&high);
            let mid = sum.div(&two);
            let sq = mid.mul(&mid);
            if lt(&sq, self) {
                low = mid.add(&one);
            } else {
                high = mid;
            }
        }
        low
    }

    pub fn is_even(&self) -> bool {
        if self.digits.is_empty() {
            return true;
        }
        self.digits[self.digits.len() - 1] % 2 == 0
    }

    pub fn is_odd(&self) -> bool {
        if self.digits.is_empty() {
            return false;
        }
        self.digits[self.digits.len() - 1] % 2 == 1
    }

    pub fn is_prime(&self) -> bool {
        // Even -> not prime (note: matches C, treats 2 as not prime).
        if self.is_even() {
            return false;
        }
        // If size > 1 and last digit is 5 or 0, not prime.
        if self.digits.len() > 1 {
            let last = self.digits[self.digits.len() - 1];
            if last == 5 || last == 0 {
                return false;
            }
        }
        // Sum of digits divisible by 3 -> not prime.
        let mut sum = BigInt::from_str("0");
        for &d in &self.digits {
            let dig = BigInt::from_int(d as i64);
            sum = sum.add(&dig);
        }
        let three = BigInt::from_str("3");
        let r = sum.r#mod(&three);
        if r.is_zero() {
            return false;
        }

        // Trial division up to sqrt(n).
        let sqrt_n = self.sqrt();
        let mut i = BigInt::from_str("2");
        while le(&i, &sqrt_n) {
            let r = self.r#mod(&i);
            if r.is_zero() {
                return false;
            }
            i.inc();
        }
        true
    }
}

// --- Module-private helpers ---

fn bigint_add(mut a: BigInt, mut b: BigInt) -> BigInt {
    let result_negative;
    if a.negative && b.negative {
        result_negative = true;
    } else if a.negative {
        a.negative = false;
        return bigint_sub(b, a);
    } else if b.negative {
        b.negative = false;
        return bigint_sub(a, b);
    } else {
        result_negative = false;
    }

    let result_size = a.digits.len().max(b.digits.len());
    let mut digits = vec![0u8; result_size];
    let mut carry: i64 = 0;
    for i in 0..result_size {
        let mut sum: i64 = carry;
        if i < a.digits.len() {
            sum += a.digits[a.digits.len() - i - 1] as i64;
        }
        if i < b.digits.len() {
            sum += b.digits[b.digits.len() - i - 1] as i64;
        }
        digits[result_size - i - 1] = (sum % 10) as u8;
        carry = sum / 10;
    }
    while carry > 0 {
        digits.insert(0, (carry % 10) as u8);
        carry /= 10;
    }
    let mut result = BigInt {
        negative: result_negative,
        digits,
    };
    result.remove_leading_zeros();
    result
}

fn bigint_sub(mut a: BigInt, mut b: BigInt) -> BigInt {
    if a.negative && b.negative {
        // NOTE: Match C behavior, which has a known bug here:
        // it computes -(|a| + |b|) instead of (b - a) as it should.
        a.negative = false;
        b.negative = false;
        let mut result = bigint_add(b, a);
        result.negative = true;
        return result;
    }

    if a.negative {
        // -a - b = -(a + b)  (with b currently positive, treat it as if negative)
        b.negative = true;
        return bigint_add(a, b);
    }

    if b.negative {
        // a - (-b) = a + b
        b.negative = false;
        return bigint_add(a, b);
    }

    let result_size = a.digits.len().max(b.digits.len());
    let mut digits = vec![0u8; result_size];

    let is_negative = lt(&a, &b);

    let (larger, smaller) = if gt(&a, &b) {
        (a.clone(), b.clone())
    } else {
        (b.clone(), a.clone())
    };

    let mut carry: i64 = 0;
    for i in 0..result_size {
        let mut diff: i64 = carry;
        if i < larger.digits.len() {
            diff += larger.digits[larger.digits.len() - i - 1] as i64;
        }
        if i < smaller.digits.len() {
            diff -= smaller.digits[smaller.digits.len() - i - 1] as i64;
        }
        if diff < 0 {
            diff += 10;
            carry = -1;
        } else {
            carry = 0;
        }
        digits[result_size - i - 1] = diff as u8;
    }

    let mut result = BigInt {
        negative: is_negative,
        digits,
    };
    result.remove_leading_zeros();
    result
}

fn bigint_mul(a: &BigInt, b: &BigInt) -> BigInt {
    if a.is_64_bit() && b.is_64_bit() {
        return BigInt::from_int(a.to_int().wrapping_mul(b.to_int()));
    }

    let result_size = a.digits.len() + b.digits.len();
    let mut acc: Vec<u64> = vec![0u64; result_size];
    for i in 0..a.digits.len() {
        for j in 0..b.digits.len() {
            let idx = result_size - i - j - 1;
            let prod =
                (a.digits[a.digits.len() - i - 1] as u64) * (b.digits[b.digits.len() - j - 1] as u64);
            acc[idx] += prod;
        }
    }

    let mut digits = vec![0u8; result_size];
    let mut carry: u64 = 0;
    for i in 0..result_size {
        let sum = acc[result_size - i - 1] + carry;
        digits[result_size - i - 1] = (sum % 10) as u8;
        carry = sum / 10;
    }

    while carry > 0 {
        digits.insert(0, (carry % 10) as u8);
        carry /= 10;
    }

    let mut result = BigInt {
        negative: a.negative != b.negative,
        digits,
    };
    result.remove_leading_zeros();
    result
}

fn bigint_divmod(numerator: &BigInt, denominator: &BigInt) -> (BigInt, BigInt) {
    if numerator.is_64_bit() && denominator.is_64_bit() {
        let n = numerator.to_int();
        let d = denominator.to_int();
        let q = n / d;
        let r = n % d;
        return (BigInt::from_int(q), BigInt::from_int(r));
    }

    let mut quotient = BigInt::from_str("0");
    let negative = numerator.negative != denominator.negative;
    let mut numerator = numerator.abs();
    let denominator = denominator.abs();

    // Slow long division by repeated subtraction (matches the C reference).
    while numerator.gt_zero() {
        numerator = numerator.sub(&denominator);
        if numerator.lt_zero() {
            break;
        }
        quotient.inc();
    }
    if numerator.lt_zero() {
        numerator = numerator.add(&denominator);
    }

    let mut remainder = numerator;
    quotient.remove_leading_zeros();

    if negative {
        quotient.negative = true;
        remainder.negative = true;
    }

    (quotient, remainder)
}

// --- Comparison free functions (match C semantics) ---

fn eq(a: &BigInt, b: &BigInt) -> bool {
    let mut a = a.clone();
    let mut b = b.clone();
    a.remove_leading_zeros();
    b.remove_leading_zeros();
    if a.digits.len() != b.digits.len() {
        return false;
    }
    if a.negative != b.negative {
        return false;
    }
    for i in 0..a.digits.len() {
        if a.digits[i] != b.digits[i] {
            return false;
        }
    }
    true
}

pub fn gt(a: &BigInt, b: &BigInt) -> bool {
    let mut a = a.clone();
    let mut b = b.clone();
    a.remove_leading_zeros();
    b.remove_leading_zeros();
    if a.digits.len() > b.digits.len() {
        return !a.negative;
    }
    if a.digits.len() < b.digits.len() {
        return a.negative;
    }
    for i in 0..a.digits.len() {
        if a.digits[i] > b.digits[i] {
            return !a.negative;
        }
        if a.digits[i] < b.digits[i] {
            return a.negative;
        }
    }
    false
}

pub fn lt(a: &BigInt, b: &BigInt) -> bool {
    !ge(a, b)
}

pub fn ge(a: &BigInt, b: &BigInt) -> bool {
    gt(a, b) || eq(a, b)
}

pub fn le(a: &BigInt, b: &BigInt) -> bool {
    !gt(a, b)
}

pub fn delete(a: &mut BigInt) {
    // Rust manages memory automatically, so this just clears state to mirror C semantics.
    a.digits.clear();
    a.negative = false;
}

impl std::ops::Neg for BigInt {
    type Output = BigInt;
    fn neg(self) -> BigInt {
        BigInt {
            negative: !self.negative,
            digits: self.digits,
        }
    }
}

impl PartialEq for BigInt {
    fn eq(&self, other: &Self) -> bool {
        eq(self, other)
    }
}

impl Eq for BigInt {}

impl PartialOrd for BigInt {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if eq(self, other) {
            Some(Ordering::Equal)
        } else if gt(self, other) {
            Some(Ordering::Greater)
        } else {
            Some(Ordering::Less)
        }
    }
}

impl fmt::Display for BigInt {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.negative {
            write!(f, "-")?;
        }
        for &d in &self.digits {
            write!(f, "{}", d)?;
        }
        Ok(())
    }
}
