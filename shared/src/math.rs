/// Safely multiply `a` by `b`, returning `None` on overflow.
pub fn safe_mul(a: i128, b: i128) -> Option<i128> {
    a.checked_mul(b)
}

/// Safely add `a` and `b`, returning `None` on overflow.
pub fn safe_add(a: i128, b: i128) -> Option<i128> {
    a.checked_add(b)
}

/// Safely subtract `b` from `a`, returning `None` on underflow.
pub fn safe_sub(a: i128, b: i128) -> Option<i128> {
    a.checked_sub(b)
}

/// Calculate `(amount * bps) / 10_000` where `bps` is basis points.
///
/// Returns `None` if any intermediate step overflows.
pub fn bps_of(amount: i128, bps: i128) -> Option<i128> {
    safe_mul(amount, bps)?.checked_div(10_000)
}
