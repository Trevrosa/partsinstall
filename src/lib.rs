use std::{
    cmp::Ordering,
    io::{stdin, stdout, Write},
    path::{Path, PathBuf},
};

/// Check if a path contains any keywords from `keywords`
///
/// # Panics
///
/// Will panic if `path` does not have a file name or is not valid unicode.
pub fn check_name<'a>(keywords: impl IntoIterator<Item = &'a str>, path: &Path) -> bool {
    let name = path
        .file_name()
        .expect("Executable should have a name")
        .to_str()
        .expect("Executable name was not valid unicode");

    keywords.into_iter().any(|kw| name.contains(kw))
}

/// Compares numeric extensions of 2 paths (file.7z.001 < file.7z.002)
///
/// # Panics
///
/// Will panic if `a` or `b` do not have valid extensions,
/// do not contain valid unicode, or do not contain a numeric extension
#[must_use]
pub fn compare_numeric_extension(a: &Path, b: &Path) -> Ordering {
    let a: u32 = a
        .extension()
        .expect("One or more paths did not have a valid extension.")
        .to_str()
        .expect("One or more paths were not valid unicode.")
        .split('.')
        .find_map(|ext| ext.parse().ok())
        .expect("One or more paths did not contain a numeric extension.");
    let b: u32 = b
        .extension()
        .expect("One or more paths did not have a valid extension.")
        .to_str()
        .expect("One or more paths were not valid unicode.")
        .split('.')
        .find_map(|ext| ext.parse().ok())
        .expect("One or more paths did not contain a numeric extension.");

    a.cmp(&b)
}

/// Prompt user for a usize lower than `max`, retrying infinitely.
#[must_use]
pub fn prompt_user_for_usize(max: usize) -> usize {
    print!("Choice: ");
    flush_stdout();

    let result: Result<usize, _> = prompt().parse();

    let Ok(result) = result else {
        return prompt_user_for_usize(max);
    };

    if result > max {
        return prompt_user_for_usize(max);
    }

    result
}

/// Prompt user for a path, retrying infinitely.
#[must_use]
pub fn prompt_user_for_path() -> PathBuf {
    print!("path: ");
    flush_stdout();

    let path = PathBuf::from(prompt());

    let Ok(path) = dunce::canonicalize(path) else {
        return prompt_user_for_path();
    };

    path
}

/// Flush stdout.
///
/// # Panics
///
/// Will panic if stdout could not be flushed.
pub fn flush_stdout() {
    stdout().flush().expect("Failed to flush stdout");
}

/// Read a line from `stdin` and remove leading and trailling whitespace.
///
/// # Panics
///
/// Will panic if `stdin().read_line` fails.
#[must_use]
pub fn prompt() -> String {
    let mut result = String::new();
    stdin()
        .read_line(&mut result)
        .expect("Failed to read stdin");
    result.trim().to_string()
}
