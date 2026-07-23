use soroban_sdk::contracterror;

/// Stable error codes shared by every contract in the workspace.
///
/// Codes from 900 to 999 are reserved for errors whose meaning is shared
/// consistently across multiple contract modules.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// The caller is not authorized to perform the requested operation.
    NotAuthorized = 900,

    /// The contract or component has already been initialized.
    AlreadyInitialized = 901,

    /// The contract or component has not been initialized.
    NotInitialized = 902,

    /// The supplied amount is invalid.
    InvalidAmount = 903,

    /// The requested operation or resource has expired.
    Expired = 904,

    /// The requested resource or entitlement has already been claimed.
    AlreadyClaimed = 905,

    /// The operation is unavailable while the contract is paused.
    Paused = 906,

    /// An arithmetic operation exceeded the supported numeric range.
    Overflow = 907,

    /// One or more supplied values are invalid.
    InvalidInput = 908,

    /// The requested resource could not be found.
    NotFound = 909,
}

#[cfg(test)]
mod tests {
    use super::Error;

    const ALL_ERRORS: [Error; 10] = [
        Error::NotAuthorized,
        Error::AlreadyInitialized,
        Error::NotInitialized,
        Error::InvalidAmount,
        Error::Expired,
        Error::AlreadyClaimed,
        Error::Paused,
        Error::Overflow,
        Error::InvalidInput,
        Error::NotFound,
    ];

    #[test]
    fn every_error_variant_has_a_unique_u32_code() {
        for (index, error) in ALL_ERRORS.iter().enumerate() {
            let code = *error as u32;

            for other in ALL_ERRORS.iter().skip(index + 1) {
                assert_ne!(
                    code, *other as u32,
                    "shared error variants must not reuse numeric code {code}"
                );
            }
        }
    }

    #[test]
    fn shared_error_codes_are_stable() {
        let expected = [
            (Error::NotAuthorized, 900),
            (Error::AlreadyInitialized, 901),
            (Error::NotInitialized, 902),
            (Error::InvalidAmount, 903),
            (Error::Expired, 904),
            (Error::AlreadyClaimed, 905),
            (Error::Paused, 906),
            (Error::Overflow, 907),
            (Error::InvalidInput, 908),
            (Error::NotFound, 909),
        ];

        for (error, expected_code) in expected {
            assert_eq!(error as u32, expected_code);
        }
    }
}
