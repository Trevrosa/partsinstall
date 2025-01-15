#[cfg(test)]
mod tests;

use std::{
    borrow::Cow,
    cmp::Ordering,
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
        .to_string_lossy()
        .split('.')
        .find_map(|ext| ext.parse().ok())
        .expect("One or more paths did not contain a numeric extension.");
    let b: u32 = b
        .extension()
        .expect("One or more paths did not have a valid extension.")
        .to_string_lossy()
        .split('.')
        .find_map(|ext| ext.parse().ok())
        .expect("One or more paths did not contain a numeric extension.");

    a.cmp(&b)
}

/// Prompt user for a usize lower than `max`, retrying infinitely.
#[must_use]
pub fn prompt_user_for_usize(max: usize) -> usize {
    print_flush!("Choice: ");

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
pub fn prompt_user_for_path(start: &Path) -> PathBuf {
    print_flush!("Path: {}\\", start.to_string_lossy());

    let path = start.join(PathBuf::from(prompt()));

    let Ok(path) = dunce::canonicalize(path) else {
        return prompt_user_for_path(start);
    };

    path
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
