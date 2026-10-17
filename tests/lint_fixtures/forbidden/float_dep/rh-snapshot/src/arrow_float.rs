//! A keyframe column read as Arrow floats, its bits laundered into an integer (SPR-007 T-02).

use arrow_array::Float64Array;

pub fn bits(column: &Float64Array) -> u64 {
    column.value(0).to_bits()
}
