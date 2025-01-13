use std::{
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

            if files.collect::<Vec<_>>().is_empty() {
                println!("Destination folder already exists but is empty, continuing.");
            } else if no_interaction {
                println!("Destination folder already exists and is not empty, continuing because of -y flag.");
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
pub fn flatten_dir(name: impl AsRef<str>, dir: &Path) {
    let Ok(dir_entries) = dir.read_dir() else {
        println!("Directory was not readable, not flattening.");
        return;
    };

    let name = name.as_ref();

    for entry in dir_entries {
        let Ok(entry) = entry else {
            println!("Skipping {entry:?} for flattening, could not be read.");
            continue;
        };
        let Ok(entry_metadata) = entry.metadata() else {
            println!(
                "Skipping {:?} for flattening, could not read file metadata.",
                entry.path()
            );
            continue;
        };

        // we only want directories that matches `name`'s keywords
        if !entry_metadata.is_dir() || !name.split(' ').any(|kw| name.contains(kw)) {
            continue;
        }

        let Ok(inner_entries) = entry.path().read_dir() else {
            println!("Inner directory {entry:?} was not readable, not flattening.");
            continue;
        };

        let mut moved_entries = 0;
        for inner_entry in inner_entries {
            let Ok(inner_entry) = inner_entry else {
                println!("Skipping {inner_entry:?} for flattening, could not be read.");
                continue;
            };

            let inner_entry_path = inner_entry.path();
            let Some(inner_entry_name) = inner_entry_path.file_name() else {
                println!(
                    "Skipping {inner_entry_path:?} for flatenning, could not determine file name."
                );
                continue;
            };

            let moved_name = dir.join(inner_entry_name);

            if let Err(err) = fs::rename(&inner_entry_path, moved_name) {
                println!(
                    "Got error {:?} while moving {inner_entry_path:?}",
                    err.kind()
                );
                continue;
            }

            moved_entries += 1;
        }

        if let Err(err) = fs::remove_dir(entry.path()) {
            println!("Got error {} trying to remove inner folder.", err.kind());
        }

        println!("Successfully flattened {moved_entries} files/folders to {dir:?}",);
    }

    println!();
}

/// Check if a path contains any keywords from `keywords`
///
/// # Panics
///
/// Will panic if `path` does not have a file name or is not valid unicode.
pub fn check_name<'a>(keywords: impl IntoIterator<Item = &'a str>, path: &Path) -> bool {
    let name = path
        .file_name()
        .expect("Executable should have a name")
        .to_string_lossy();

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
