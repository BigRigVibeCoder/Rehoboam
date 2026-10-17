//! Fixture: rh-stream takes a float from parquet's record API without naming a float type
//! (SPR-013; the Architect's parquet review): the gate must report `get_double` and `Field::Float`.
mod field_float;

pub fn reading(row: &parquet::record::Row) -> u64 {
    row.get_double(0).map(|x| x.to_bits()).unwrap_or(0)
}
