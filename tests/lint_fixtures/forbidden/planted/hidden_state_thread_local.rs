//! Planted control: a thread_local! in a plane-A helper (SPR-027 T-02; W31).
thread_local! {
    static LOCAL_COUNTER: u64 = 0;
}
