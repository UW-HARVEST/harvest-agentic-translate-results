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
        // Match C behavior: returns true iff size < 10
        self.digits.len() < 10
    }
    pub fn remove_leading_zeros(&mut self) {
        if self.digits.is_empty() {
            self.digits.push(0);
            return;
        }
        let mut leading_zeros = 0usize;
        for i in 0..self.digits.len() {
            if self.digits[i] == 0 {
                leading_zeros += 1;
            } else {
                break;
            }
        }
        if leading_zeros == 0 {
            return;
        }
        if leading_zeros >= self.digits.len() {
            self.digits.clear();
            self.digits.push(0);
            return;
        }
        self.digits.drain(0..leading_zeros);
    }
    pub fn from_int(n: i64) -> Self {
        let negative = n < 0;
        let mut abs_n: u64 = n.unsigned_abs();
        if abs_n == 0 {
            return BigInt {
                negative: false,
                digits: vec![0],
            };
        }
        let mut digits = Vec::new();
        while abs_n > 0 {
            digits.push((abs_n % 10) as u8);
            abs_n /= 10;
        }
        digits.reverse();
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
            -result
        } else {
            result
        }
    }
    pub fn from_str(s: &str) -> Self {
        let bytes = s.as_bytes();
        let (negative, start) = if !bytes.is_empty() && bytes[0] == b'-' {
            (true, 1usize)
        } else {
            (false, 0usize)
        };
        let digits: Vec<u8> = bytes[start..].iter().map(|&b| b - b'0').collect();
        if digits.is_empty() {
            return BigInt {
                negative: false,
                digits: vec![0],
            };
        }
        BigInt { negative, digits }
    }
    pub fn copy(&self) -> BigInt {
        self.clone()
    }
    pub fn print(&self) {
        if self.negative {
            print!("-");
        }
        for &d in &self.digits {
            print!("{}", d);
        }
    }
    pub fn is_zero(&self) -> bool {
        self.digits.iter().all(|&d| d == 0)
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
        let mut a = self.clone();
        let mut b = other.clone();
        let result_negative;

        if a.negative && b.negative {
            result_negative = true;
        } else if a.negative {
            a.negative = false;
            return b.sub(&a);
        } else if b.negative {
            b.negative = false;
            return a.sub(&b);
        } else {
            result_negative = false;
        }

        let result_size = std::cmp::max(a.digits.len(), b.digits.len());
        let mut result_digits = vec![0u8; result_size];

        let mut carry: i64 = 0;
        for i in 0..result_size {
            let mut sum: i64 = carry;
            if i < a.digits.len() {
                sum += a.digits[a.digits.len() - i - 1] as i64;
            }
            if i < b.digits.len() {
                sum += b.digits[b.digits.len() - i - 1] as i64;
            }
            result_digits[result_size - i - 1] = (sum % 10) as u8;
            carry = sum / 10;
        }

        while carry > 0 {
            result_digits.insert(0, (carry % 10) as u8);
            carry /= 10;
        }

        let mut result = BigInt {
            negative: result_negative,
            digits: result_digits,
        };
        result.remove_leading_zeros();
        result
    }
    pub fn sub(&self, other: &Self) -> Self {
        let mut a = self.clone();
        let mut b = other.clone();

        if a.negative && b.negative {
            // Match C behavior exactly (note: the C implementation has a bug here,
            // computing -(|a| + |b|) instead of |b| - |a|, but tests don't exercise it)
            a.negative = false;
            b.negative = false;
            let mut result = b.add(&a);
            result.negative = true;
            return result;
        }
        if a.negative {
            // -a - b = -(|a| + b)
            b.negative = true;
            return a.add(&b);
        }
        if b.negative {
            // a - (-b) = a + b
            b.negative = false;
            return a.add(&b);
        }

        let result_size = std::cmp::max(a.digits.len(), b.digits.len());
        let mut result_digits = vec![0u8; result_size];

        let is_negative = lt(&a, &b);

        let (larger, smaller) = if gt(&a, &b) { (&a, &b) } else { (&b, &a) };

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
            result_digits[result_size - i - 1] = diff as u8;
        }

        let mut result = BigInt {
            negative: is_negative,
            digits: result_digits,
        };
        result.remove_leading_zeros();
        result
    }
    pub fn inc(&mut self) {
        let one = BigInt::from_int(1);
        *self = self.add(&one);
    }
    pub fn dec(&mut self) {
        let one = BigInt::from_int(1);
        *self = self.sub(&one);
    }
    pub fn mul(&self, other: &Self) -> Self {
        if self.is_64_bit() && other.is_64_bit() {
            return BigInt::from_int(self.to_int() * other.to_int());
        }

        let a_len = self.digits.len();
        let b_len = other.digits.len();
        let n = a_len + b_len;
        let mut acc: Vec<i64> = vec![0; n];

        for i in 0..a_len {
            for j in 0..b_len {
                let pos = n - i - j - 1;
                acc[pos] += (self.digits[a_len - i - 1] as i64)
                    * (other.digits[b_len - j - 1] as i64);
            }
        }

        // Normalize: propagate carries from LSB to MSB
        let mut carry: i64 = 0;
        let mut result_digits = vec![0u8; n];
        for i in 0..n {
            let pos = n - i - 1;
            let sum = acc[pos] + carry;
            result_digits[pos] = (sum % 10) as u8;
            carry = sum / 10;
        }

        // Any remaining carry becomes leading digits
        while carry > 0 {
            result_digits.insert(0, (carry % 10) as u8);
            carry /= 10;
        }

        let mut result = BigInt {
            negative: self.negative != other.negative,
            digits: result_digits,
        };
        result.remove_leading_zeros();
        result
    }
    pub fn divmod(&self, other: &Self) -> (Self, Self) {
        if self.is_64_bit() && other.is_64_bit() {
            let n = self.to_int();
            let d = other.to_int();
            let q = n / d;
            let r = n % d;
            return (BigInt::from_int(q), BigInt::from_int(r));
        }

        let negative = self.negative != other.negative;
        let mut numerator = self.clone();
        numerator.negative = false;
        let mut denominator = other.clone();
        denominator.negative = false;
        numerator.remove_leading_zeros();
        denominator.remove_leading_zeros();

        // Schoolbook long division on absolute values
        let mut remainder = BigInt::zero();
        let mut quotient_digits: Vec<u8> = Vec::new();

        // Pre-compute denominator * k for k in 1..=9 to speed up trial
        // (but for simplicity, compute on demand)
        for &dig in &numerator.digits {
            // remainder = remainder * 10 + dig
            remainder.digits.push(dig);
            remainder.remove_leading_zeros();

            // Find largest k in 0..=9 such that denominator * k <= remainder
            let mut k: u8 = 0;
            for trial in (1u8..=9).rev() {
                let trial_bi = BigInt::from_int(trial as i64);
                let prod = denominator.mul(&trial_bi);
                if !gt(&prod, &remainder) {
                    k = trial;
                    break;
                }
            }

            if k > 0 {
                let k_bi = BigInt::from_int(k as i64);
                let prod = denominator.mul(&k_bi);
                remainder = remainder.sub(&prod);
            }

            quotient_digits.push(k);
        }

        let mut quotient = BigInt {
            negative: false,
            digits: if quotient_digits.is_empty() {
                vec![0]
            } else {
                quotient_digits
            },
        };
        quotient.remove_leading_zeros();

        if negative {
            quotient.negative = true;
            remainder.negative = true;
        }

        (quotient, remainder)
    }
    pub fn div(&self, other: &Self) -> Self {
        if self.is_64_bit() && other.is_64_bit() {
            return BigInt::from_int(self.to_int() / other.to_int());
        }
        self.divmod(other).0
    }
    pub fn r#mod(&self, other: &Self) -> Self {
        if self.is_64_bit() && other.is_64_bit() {
            return BigInt::from_int(self.to_int() % other.to_int());
        }
        self.divmod(other).1
    }
    pub fn pow(&self, exponent: &Self) -> Self {
        if exponent.negative {
            return BigInt::zero();
        }
        let mut result = BigInt::from_int(1);
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
            let mut base = self.to_int();
            let mut exp = exponent.to_int();
            let m = modulo.to_int();
            base %= m;
            while exp > 0 {
                if exp % 2 == 1 {
                    result = (result * base) % m;
                }
                exp >>= 1;
                base = (base * base) % m;
            }
            return BigInt::from_int(result);
        }

        if exponent.negative {
            return BigInt::zero();
        }
        if exponent.is_zero() {
            return BigInt::from_int(1);
        }

        // Standard binary exponentiation for the bigint case
        let mut result = BigInt::from_int(1);
        let mut base = self.r#mod(modulo);
        let mut exp = exponent.clone();
        let two = BigInt::from_int(2);

        while !exp.is_zero() {
            if exp.is_odd() {
                let prod = result.mul(&base);
                result = prod.r#mod(modulo);
            }
            exp = exp.div(&two);
            let sq = base.mul(&base);
            base = sq.r#mod(modulo);
        }

        result
    }
    pub fn modinv(&self, modulo: &Self) -> Self {
        let m0 = modulo.clone();
        let mut y = BigInt::from_int(0);
        let mut x = BigInt::from_int(1);
        let one = BigInt::from_int(1);
        let mut a = self.clone();
        let mut m = modulo.clone();

        while !a.is_zero() {
            let q = m.div(&a);
            let qa = q.mul(&a);
            let t = m.sub(&qa);
            m = a.clone();
            a = t;

            let qx = q.mul(&x);
            let t2 = y.sub(&qx);
            y = x.clone();
            x = t2;
        }

        if m.lt_zero() {
            m = m.add(&m0);
        }

        if m == one {
            let mut result = y.r#mod(&m0);
            if result.lt_zero() {
                result = result.add(&m0);
            }
            return result;
        }

        BigInt::from_int(0)
    }
    pub fn sqrt(&self) -> Self {
        let one = BigInt::from_int(1);
        let two = BigInt::from_int(2);
        let mut low = BigInt::zero();
        let mut high = self.clone();

        while lt(&low, &high) {
            let sum = low.add(&high);
            let mid = sum.div(&two);
            let mid_squared = mid.mul(&mid);
            if lt(&mid_squared, self) {
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
        if self.is_even() {
            return false;
        }
        if self.digits.len() > 1 {
            let last = self.digits[self.digits.len() - 1];
            if last == 5 || last == 0 {
                return false;
            }
        }
        // Sum of digits divisible by 3
        let mut sum = BigInt::from_int(0);
        for &d in &self.digits {
            let dbi = BigInt::from_int(d as i64);
            sum = sum.add(&dbi);
        }
        let three = BigInt::from_int(3);
        let mod_result = sum.r#mod(&three);
        if mod_result.is_zero() {
            return false;
        }

        let sqrt_n = self.sqrt();
        let mut i = BigInt::from_int(2);
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
    gt(a, b) || a == b
}
pub fn le(a: &BigInt, b: &BigInt) -> bool {
    !gt(a, b)
}
pub fn delete(a: &mut BigInt) {
    a.digits.clear();
    a.negative = false;
}
impl std::ops::Neg for BigInt {
    type Output = BigInt;
    fn neg(mut self) -> BigInt {
        self.negative = !self.negative;
        self
    }
}
impl PartialEq for BigInt {
    fn eq(&self, other: &Self) -> bool {
        let mut a = self.clone();
        let mut b = other.clone();
        a.remove_leading_zeros();
        b.remove_leading_zeros();
        if a.digits.len() != b.digits.len() {
            return false;
        }
        if a.negative != b.negative {
            return false;
        }
        a.digits == b.digits
    }
}
impl Eq for BigInt {}
impl PartialOrd for BigInt {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self == other {
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
