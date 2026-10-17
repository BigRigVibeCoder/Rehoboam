//! Fixture: a test reading its golden file (exempt from the file-io rule).

#[test]
fn golden() {
    let _ = std::fs::read_to_string("golden.txt");
}
