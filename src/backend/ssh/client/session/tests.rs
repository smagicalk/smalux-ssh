use super::*;

#[test]
fn pty_dimensions_are_never_zero() {
    let size = TerminalSize::new(0, 0);

    assert_eq!(columns(size), 1);
    assert_eq!(rows(size), 1);
}
