use soroban_sdk::contracterror;

#[contracterror]
#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum AidError {
    NotFound = 1,
    AlreadyClaimed = 2,
    Expired = 3,
    Paused = 4,
    Unauthorized = 5,
    NotExpiredYet = 6,
    AlreadyRefunded = 7,
}