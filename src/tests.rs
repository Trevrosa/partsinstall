use std::path::Path;

use crate::PathExt;

#[test]
fn test_archive_ext() {
    let archive = Path::new("test.7z");
    assert!(archive.is_archive());

    let cased_archive = Path::new("test.7Z");
    assert!(cased_archive.is_archive());

    let cased_archive2 = Path::new("test.zIP");
    assert!(cased_archive2.is_archive());

    let multi_ext_archive = Path::new("test.app.7z");
    assert!(multi_ext_archive.is_archive());

    let multi_ext_not_archive = Path::new("test.app.exe");
    assert!(!multi_ext_not_archive.is_archive());

    let empty = Path::new("");
    assert!(!empty.is_archive());

    let no_extension = Path::new("test");
    assert!(!no_extension.is_archive());

    let not_archive = Path::new("test.txt");
    assert!(!not_archive.is_archive());
}

#[test]
fn test_numeric_ext() {
    let numeric = Path::new("a.003");
    assert!(numeric.is_numeric());

    let non_numeric = Path::new("a.abc");
    assert!(!non_numeric.is_numeric());

    let empty = Path::new("");
    assert!(!empty.is_numeric());
}
