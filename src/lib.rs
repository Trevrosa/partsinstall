#[cfg(test)]
mod tests;

pub mod tracing;

use std::{
    borrow::Cow,
    cmp::Ordering,
    fmt::Display,
    io::stdin,
    path::{Path, PathBuf},
};

/// print! then flush `stdout`. Will panic if stdout could not be written to or flushed.
#[macro_export]
macro_rules! print_flush {
    ( $($t:tt)* ) => {
        {
            use std::io::{stdout, Write};

            let mut stdout = stdout();
            write!(stdout, $($t)* ).unwrap();
            stdout.flush().unwrap();
        }
    }
}

/// List of archive extensions supported by the tool and 7z.
///
/// I chose these values based on the most commonly used archive types.
///
/// <https://documentation.help/7-Zip/formats.htm>
const ARCHIVE_EXTS: &[&str] = &["7z", "zip", "rar", "tgz"];

#[must_use]
pub fn cpu_cores() -> u16 {
    std::thread::available_parallelism()
        .unwrap()
        .get()
        .try_into()
        .unwrap()
}

/// Provide convenience extension methods for [`Path`]
pub trait PathExt {
    fn is_archive(&self) -> bool;
    fn is_numeric(&self) -> bool;
    fn lossy_extension(&self) -> Option<Cow<'_, str>>;
    fn lossy_file_name(&self) -> Option<Cow<'_, str>>;
    fn lossy_file_stem(&self) -> Option<Cow<'_, str>>;
}

impl PathExt for Path {
    /// Returns true if the path's extension is in [`ARCHIVE_EXTS`]
    fn is_archive(&self) -> bool {
        self.lossy_extension()
            .is_some_and(|ext| ARCHIVE_EXTS.contains(&ext.to_lowercase().as_ref()))
    }

    /// Returns true if the path's extension can be parsed as a `u32`.
    fn is_numeric(&self) -> bool {
        self.lossy_extension()
            .is_some_and(|ext| ext.parse::<u32>().is_ok())
    }

    // Conveneince function to get a `Path`'s lossy extension.
    fn lossy_extension(&self) -> Option<Cow<'_, str>> {
        self.extension().map(|ext| ext.to_string_lossy())
    }

    /// Convenience function to get a `Path`'s lossy file name.
    fn lossy_file_name(&self) -> Option<Cow<'_, str>> {
        self.file_name().map(|name| name.to_string_lossy())
    }

    /// Convenience function to get a `Path`'s lossy file stem.
    fn lossy_file_stem(&self) -> Option<Cow<'_, str>> {
        self.file_stem().map(|name| name.to_string_lossy())
    }
}

/// Check if a path contains any keywords from `keywords`
pub fn name_has_keywords<'a>(keywords: impl IntoIterator<Item = &'a str>, path: &Path) -> bool {
    let Some(name) = path.file_name() else {
        return false;
    };

    let name = name.to_string_lossy();
    keywords.into_iter().any(|kw| name.contains(kw))
}

#[inline]
fn get_numeric_extension(p: &Path) -> u32 {
    p.extension()
        .expect("One or more paths did not have a valid extension.")
        .to_string_lossy()
        .split('.')
        .find_map(|ext| ext.parse().ok())
        .expect("One or more paths did not contain a numeric extension.")
}

/// Compares numeric extensions of 2 paths (file.7z.001 < file.7z.002)
///
/// # Panics
///
/// Will panic if `a` or `b` do not have valid extensions,
/// do not contain valid unicode, or do not contain a numeric extension
#[must_use]
pub fn compare_numeric_extensions(a: &Path, b: &Path) -> Ordering {
    let a: u32 = get_numeric_extension(a);
    let b: u32 = get_numeric_extension(b);
    a.cmp(&b)
}

pub fn get_input() -> String {
    let mut resp = String::new();
    stdin().read_line(&mut resp).unwrap();
    resp.trim().to_string()
}

/// Infinitely prompt the user for some data `T`, parsing from a string response with `parse`, checking if it satisfies `condition`.
pub fn prompt<T, E, M: Display>(
    initial: M,
    parse: impl Fn(&str) -> Result<T, E>,
    condition: impl Fn(&T) -> bool,
) -> T {
    print_flush!("{initial}");

    let mut response = String::new();
    stdin().read_line(&mut response).unwrap();

    match parse(response.trim()) {
        Ok(parsed) if condition(&parsed) => parsed,
        _ => return prompt(initial, parse, condition),
    }
}

pub fn prompt_for_usize(max: usize) -> usize {
    prompt("Choice: ", str::parse, |n| *n > max)
}

pub fn prompt_for_path(parent: &Path) -> PathBuf {
    prompt(
        format_args!("Path: {}", parent.display()),
        |s| dunce::canonicalize(parent.join(s)),
        |_| true,
    )
}

// /// Prompt user for a path, retrying infinitely.
// #[must_use]
// pub fn prompt_user_for_path(start: &Path) -> PathBuf {
//     print_flush!("Path: {}\\", start.to_string_lossy());

//     let path = start.join(PathBuf::from(prompt()));

//     let Ok(path) = dunce::canonicalize(path) else {
//         return prompt_user_for_path(start);
//     };

//     path
// }
