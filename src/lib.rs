#[cfg(test)]
mod tests;

use std::{
    borrow::Cow,
    cmp::Ordering,
    fs,
    io::{self, stdin},
    path::{Path, PathBuf},
    process::exit,
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
/// I chose these values based on the most commonly used archive types on Windows specifically.
///
/// <https://documentation.help/7-Zip/formats.htm>
const ARCHIVE_EXTS: &[&str] = &["7z", "zip", "rar", "tgz"];

/// Provide convenience extension methods for [`Path`]
trait PathExt {
    fn is_archive(&self) -> bool;
    fn is_numeric(&self) -> bool;
    fn lossy_file_name(&self) -> Option<Cow<'_, str>>;
    fn lossy_file_stem(&self) -> Option<Cow<'_, str>>;
}

impl PathExt for Path {
    /// Returns true if the path's extension is in [`ARCHIVE_EXTS`]
    fn is_archive(&self) -> bool {
        let ext = self.extension();
        ext.map(|ext| ext.to_string_lossy())
            .is_some_and(|ext| ARCHIVE_EXTS.contains(&ext.as_ref()))
    }

    /// Returns true if the path's extension can be parsed as a `u32`.
    fn is_numeric(&self) -> bool {
        let ext = self.extension();
        ext.is_some_and(|ext| ext.to_string_lossy().parse::<u32>().is_ok())
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

/// Find the app name
#[must_use]
pub fn find_app_name(name: &Path) -> Option<Cow<'_, str>> {
    let name_str = name.lossy_file_name()?;

    // if `name` does not exist, it already probably is the app name,
    // so we can return it as the app name.
    if !name.exists() {
        return Some(name_str);
    }

    // if `name` exists and is a dir, it means any dots in the passed `name` are in the actual app name.
    // eg. in the app Test.App, .App is part of the name and is not a file extension.
    if name.is_dir() {
        return Some(name_str);
    }

    // we now know `name` exists and is a file (not a dir).

    // remove numeric extension. eg. app.7z.001 would become app.7z
    let name = if name.is_numeric() {
        name.lossy_file_stem()?
    } else {
        name.lossy_file_name()?
    };

    let file_name = Path::new(name.as_ref());

    // remove archive extension. eg. app.7z would become app
    let file_name = if file_name.is_archive() {
        file_name.lossy_file_stem()?
    } else {
        // here, file_name should already be a file name
        // since we only return file_stem or file_name above.
        // so we only need to use .to_string_lossy()
        file_name.to_string_lossy()
    };

    Some(Cow::Owned(file_name.into_owned()))
}

/// Create destination path, handling errors and giving prompts as needed.
///
/// # Panics
///
/// Will panic if destination folder already exists is not readable.
pub fn create_destination(destination: &Path, no_interaction: bool) {
    let Err(err) = fs::create_dir(destination) else {
        return;
    };

    match err.kind() {
        io::ErrorKind::AlreadyExists => {
            let Ok(files) = destination.read_dir() else {
                panic!("Destination folder already exists and could not be read.")
            };

            if no_interaction {
                println!("Destination folder already exists and is not empty, continuing because of -y flag.");
            } else if files.collect::<Vec<_>>().is_empty() {
                println!("Destination folder already exists but is empty, continuing.");
            } else {
                print_flush!(
                    "Destination folder already exists and is not empty. Continue anyway? (y/n): "
                );

                if prompt().to_lowercase() != "y" {
                    exit(1)
                }
            }
        }
        err => panic!("Could not create destination folder: {err}"),
    }
}

/// Move all contents of a directory called `name` in `dir` to `dir`.
/// eg. `App/App/files -> App/files`
#[allow(
    clippy::missing_panics_doc,
    reason = "The expect_err() used will never panic since it is in a let Ok() else block."
)]
pub fn flatten_dir(name: impl AsRef<str>, dir: &Path) {
    let Ok(dir_entries) = dir.read_dir() else {
        println!("Directory was not readable, not flattening.");
        return;
    };

    let name = name.as_ref();

    let inner_dir = dir_entries
        .filter_map(Result::ok)
        .find(|d| d.path().is_dir() && check_name(name.split(' '), &d.path()));

    let Some(inner_dir) = inner_dir else {
        println!("No inner directory to flatten.");
        return;
    };

    let Ok(inner_entries) = inner_dir.path().read_dir() else {
        println!("Could not read inner directory {:?}", inner_dir.path());
        return;
    };

    let mut flattened = 0;

    for inner_entry in inner_entries {
        let Ok(inner_entry) = inner_entry else {
            println!(
                "Skipped flattening inner file/folder, got error {}.",
                inner_entry
                    .expect_err(".err() must work in a let Ok() else block, how did we get here?")
            );
            continue;
        };

        let inner_entry_path = inner_entry.path();
        let moved_path = dir.join(inner_entry.file_name());

        if let Err(err) = fs::rename(&inner_entry_path, &moved_path) {
            println!(
                "Got error {} while trying to move {:?} to {:?}\n",
                err.kind(),
                inner_entry_path,
                moved_path
            );
            continue;
        }

        flattened += 1;
        print_flush!("Flattened {flattened} file(s)\r");
    }

    if let Err(err) = fs::remove_dir(inner_dir.path()) {
        println!(
            "Got error {:?} while removing inner folder {:?}",
            err.kind(),
            inner_dir.path()
        );
    } else {
        println!("Sucessfully flattened {flattened} file(s).\n");
    }
}

/// Check if a path contains any keywords from `keywords`
pub fn check_name<'a>(keywords: impl IntoIterator<Item = &'a str>, path: &Path) -> bool {
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
