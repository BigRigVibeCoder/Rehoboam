//! Fixture: a matched `Field::Float` hands plane A an f32 unnamed.
pub fn bits(f: &parquet::record::Field) -> u32 {
    match f {
        parquet::record::Field::Float(x) => x.to_bits(),
        _ => 0,
    }
}
