//! Planted control: a Mutex<u64> field in a stub system (SPR-027 T-02; W31).
pub struct StubSystem {
    counter: std::sync::Mutex<u64>,
}
