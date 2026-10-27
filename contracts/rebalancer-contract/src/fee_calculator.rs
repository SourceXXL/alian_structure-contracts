use crate::Trade;
use soroban_sdk::Vec;

pub fn calculate_total_fees(trades: &Vec<Trade>) -> u128 {
    let mut total_fees = 0;
    for trade in trades.iter() {
        // Placeholder fee calculation (0.1% of trade amount)
        total_fees += trade.amount / 1000;
    }
    total_fees
}

/// Computes the net liquidity amount received when removing liquidity from a pool
/// with fee-on-transfer tokens, deducting transfer fees upfront to protect pool reserves.
pub fn calculate_fee_on_transfer_liquidity(amount_in: u128, fee_bps: u32) -> u128 {
    if fee_bps >= 10_000 {
        return 0;
    }
    let fee = (amount_in * fee_bps as u128) / 10_000;
    amount_in.saturating_sub(fee)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fee_on_transfer_liquidity_calculation() {
        let amount = 10_000u128;
        let fee_bps = 200u32; // 2% fee
        let net = calculate_fee_on_transfer_liquidity(amount, fee_bps);
        assert_eq!(net, 9_800);
    }

    #[test]
    fn test_fee_on_transfer_full_fee_returns_zero() {
        let amount = 10_000u128;
        let fee_bps = 10_000u32; // 100% fee
        let net = calculate_fee_on_transfer_liquidity(amount, fee_bps);
        assert_eq!(net, 0);
    }
}
