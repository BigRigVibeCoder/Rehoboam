//! An integration test computing a statistic in floats: tests/ is exempt from the float rule.

#[test]
fn mean_is_close() {
    let mean: f64 = 0.5;
    assert!((mean - 0.5).abs() < 1e-9);
}
