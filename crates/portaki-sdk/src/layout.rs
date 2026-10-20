//! The compile-time half of the module layout: `#[surface]`, `#[command]` and the other
//! declaring macros assert, in a `const`, that `file!()` is the file their declaration belongs in.
//! See `portaki-sdk-macros/src/layout.rs` for the table.

/// `file` — as `file!()` gives it — is `src/{expected}` of its crate, or is not in a `src/` at all
/// (a test, an example).
pub const fn in_place(file: &str, expected: &str) -> bool {
    let file = file.as_bytes();
    let expected = expected.as_bytes();
    // The last `src/` that starts a path component.
    let mut start = None;
    let mut at = 0;
    while at + 4 <= file.len() {
        if file[at] == b's'
            && file[at + 1] == b'r'
            && file[at + 2] == b'c'
            && (file[at + 3] == b'/' || file[at + 3] == b'\\')
            && (at == 0 || file[at - 1] == b'/' || file[at - 1] == b'\\')
        {
            start = Some(at + 4);
        }
        at += 1;
    }
    let Some(start) = start else {
        return true;
    };
    if file.len() - start != expected.len() {
        return false;
    }
    let mut i = 0;
    while i < expected.len() {
        let found = if file[start + i] == b'\\' {
            b'/'
        } else {
            file[start + i]
        };
        if found != expected[i] {
            return false;
        }
        i += 1;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::in_place;

    #[test]
    fn the_file_is_the_one_its_declaration_names() {
        assert!(in_place("src/host/settings.rs", "host/settings.rs"));
        assert!(in_place(
            "modules/rules/src/host/settings.rs",
            "host/settings.rs"
        ));
        assert!(!in_place(
            "modules/rules/src/host/stats.rs",
            "host/settings.rs"
        ));
        assert!(!in_place("src/lib.rs", "host/settings.rs"));
        assert!(in_place("tests/surfaces.rs", "host/settings.rs"));
        assert!(!in_place("resource/src/host.rs", "host/settings.rs"));
    }
}
