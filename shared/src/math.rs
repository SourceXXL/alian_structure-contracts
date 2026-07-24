use crate::errors::Error;

const BPS_DENOMINATOR: i128 = 10_000;

/// Adds two values, returning [`Error::Overflow`] if the result is out of range.
pub fn checked_add(a: i128, b: i128) -> Result<i128, Error> {
    a.checked_add(b).ok_or(Error::Overflow)
}

/// Subtracts `b` from `a`, returning [`Error::Overflow`] if the result is out of
/// range.
pub fn checked_sub(a: i128, b: i128) -> Result<i128, Error> {
    a.checked_sub(b).ok_or(Error::Overflow)
}

/// Multiplies two values, returning [`Error::Overflow`] if the result is out of
/// range.
pub fn checked_mul(a: i128, b: i128) -> Result<i128, Error> {
    a.checked_mul(b).ok_or(Error::Overflow)
}

/// Divides `a` by `b`.
///
/// Division by zero returns [`Error::InvalidAmount`]. The only overflowing
/// signed division (`i128::MIN / -1`) returns [`Error::Overflow`].
pub fn checked_div(a: i128, b: i128) -> Result<i128, Error> {
    if b == 0 {
        return Err(Error::InvalidAmount);
    }

    a.checked_div(b).ok_or(Error::Overflow)
}

/// Applies a basis-point rate to a positive monetary `amount`.
///
/// `bps` must be in the inclusive range `0..=10_000`. Results are rounded down
/// (towards zero, which is equivalent to floor because valid amounts and basis
/// points are non-negative). The calculation splits the amount into quotient
/// and remainder components so valid values such as
/// `apply_bps(i128::MAX, 10_000)` do not overflow in an intermediate product.
pub fn apply_bps(amount: i128, bps: i128) -> Result<i128, Error> {
    if amount <= 0 || !(0..=BPS_DENOMINATOR).contains(&bps) {
        return Err(Error::InvalidAmount);
    }

    let whole = checked_mul(checked_div(amount, BPS_DENOMINATOR)?, bps)?;
    let remainder = checked_div(checked_mul(amount % BPS_DENOMINATOR, bps)?, BPS_DENOMINATOR)?;

    checked_add(whole, remainder)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checked_add_handles_i128_edges() {
        assert_eq!(checked_add(i128::MAX, 0), Ok(i128::MAX));
        assert_eq!(checked_add(i128::MIN, 0), Ok(i128::MIN));
        assert_eq!(checked_add(i128::MAX, 1), Err(Error::Overflow));
        assert_eq!(checked_add(i128::MIN, -1), Err(Error::Overflow));
    }

    #[test]
    fn checked_sub_handles_i128_edges() {
        assert_eq!(checked_sub(i128::MAX, 0), Ok(i128::MAX));
        assert_eq!(checked_sub(i128::MIN, 0), Ok(i128::MIN));
        assert_eq!(checked_sub(i128::MAX, -1), Err(Error::Overflow));
        assert_eq!(checked_sub(i128::MIN, 1), Err(Error::Overflow));
    }

    #[test]
    fn checked_mul_handles_i128_edges() {
        assert_eq!(checked_mul(i128::MAX, 1), Ok(i128::MAX));
        assert_eq!(checked_mul(i128::MIN, 1), Ok(i128::MIN));
        assert_eq!(checked_mul(i128::MAX, 2), Err(Error::Overflow));
        assert_eq!(checked_mul(i128::MIN, -1), Err(Error::Overflow));
    }

    #[test]
    fn checked_div_handles_i128_edges_and_zero() {
        assert_eq!(checked_div(i128::MAX, 1), Ok(i128::MAX));
        assert_eq!(checked_div(i128::MIN, 1), Ok(i128::MIN));
        assert_eq!(checked_div(i128::MIN, -1), Err(Error::Overflow));
        assert_eq!(checked_div(1, 0), Err(Error::InvalidAmount));
    }

    #[test]
    fn checked_operations_match_i128_for_boundary_matrix() {
        let values = [
            i128::MIN,
            i128::MIN + 1,
            -BPS_DENOMINATOR,
            -1,
            0,
            1,
            BPS_DENOMINATOR,
            i128::MAX - 1,
            i128::MAX,
        ];

        for a in values {
            for b in values {
                assert_eq!(checked_add(a, b), a.checked_add(b).ok_or(Error::Overflow));
                assert_eq!(checked_sub(a, b), a.checked_sub(b).ok_or(Error::Overflow));
                assert_eq!(checked_mul(a, b), a.checked_mul(b).ok_or(Error::Overflow));

                let expected_division = if b == 0 {
                    Err(Error::InvalidAmount)
                } else {
                    a.checked_div(b).ok_or(Error::Overflow)
                };
                assert_eq!(checked_div(a, b), expected_division);
            }
        }
    }

    #[test]
    fn apply_bps_supports_zero_and_full_rate_boundaries() {
        assert_eq!(apply_bps(123_456, 0), Ok(0));
        assert_eq!(apply_bps(123_456, 10_000), Ok(123_456));
        assert_eq!(apply_bps(i128::MAX, 10_000), Ok(i128::MAX));
    }

    #[test]
    fn apply_bps_rounds_down() {
        assert_eq!(apply_bps(101, 100), Ok(1));
        assert_eq!(apply_bps(9_999, 1), Ok(0));
        assert_eq!(apply_bps(10_001, 1), Ok(1));
    }

    #[test]
    fn apply_bps_rejects_invalid_amounts_and_rates() {
        assert_eq!(apply_bps(0, 100), Err(Error::InvalidAmount));
        assert_eq!(apply_bps(-1, 100), Err(Error::InvalidAmount));
        assert_eq!(apply_bps(1, -1), Err(Error::InvalidAmount));
        assert_eq!(apply_bps(1, 10_001), Err(Error::InvalidAmount));
    }
}
