//! A trait impl converts a public type into a float: public without `pub`.

/// A drawn value.
pub struct Draw(i32);

impl From<Draw> for f64 {
    fn from(d: Draw) -> Self {
        f64::from(d.0)
    }
}
