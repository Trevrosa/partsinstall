use std::{
    borrow::Cow,
    fs::{self, File},
    io,
    os::windows::fs::MetadataExt,
    path::{Path, PathBuf},
    process::{exit, Command},
    time::{Duration, Instant},
};

use glob::glob;
use humansize::{format_size, DECIMAL};
use partsinstall::{
    compare_numeric_extension, name_has_keywords, print_flush, prompt, prompt_user_for_path,
    prompt_user_for_usize, PathExt,
};

/// Parse the app name from `name`.
#[must_use]
pub fn parse_app_name(name: &Path) -> Option<Cow<'_, str>> {
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

pub fn find_final_name<'a>(
    app_name: &str,
    files: &'a mut [PathBuf],
    no_interaction: bool,
) -> (Cow<'a, str>, Duration) {
    if files.len() == 1 {
        if no_interaction {
            (files[0].to_string_lossy(), Duration::ZERO)
        } else {
            print_flush!("Only 1 file found, extract {:?}? (y/n): ", files[0]);

            // true
            if prompt().to_lowercase() != "y" {
                exit(1)
            }

            (files[0].to_string_lossy(), Duration::ZERO)
        }
    } else {
        let combine_start = Instant::now();

        let final_ext = files
            .iter()
            .find_map(|p| p.file_name())
            .expect("No file names could be found")
            .to_string_lossy();
        let final_ext = final_ext
            .split('.')
            // skip the file stem
            .skip(1)
            // find extension which is not a number
            // eg. from file.7z.001, we want 7z, ignoring 001.
            .find(|part| part.parse::<u32>().is_err())
            .expect("Could not determine output file extension");

        let final_name = format!("{app_name}.{final_ext}");

        println!("Combining to {final_name}");

        let combine_time = combine_files(files, &final_name, combine_start, no_interaction)
            .unwrap_or(Duration::ZERO);

        (Cow::Owned(final_name), combine_time)
    }
}

/// Combine `files` into one file named `final_name`, prompting user and exiting if needed.
#[allow(
    clippy::missing_panics_doc,
    reason = "We want to panic/exit if something fails here."
)]
pub fn combine_files(
    files: &mut [PathBuf],
    output_name: &str,
    start: Instant,
    no_interaction: bool,
) -> Option<Duration> {
    let final_file = File::create_new(output_name);

    if let Ok(mut final_file) = final_file {
        let files_len = files.len();

        // glob sorts alphanumerically, meaning it will sort correctly until a number is larger than 10.
        // eg. 01, 11, 02, 021, 03 will be how glob sorts numbers larger than 10.
        if files.len() > 10 {
            files.sort_by(|a, b| compare_numeric_extension(a, b));
        }

        for (n, file) in files.iter().enumerate() {
            if let Ok(metadata) = fs::metadata(file) {
                let size = format_size(metadata.file_size(), DECIMAL);
                println!("{}/{files_len}: combining {file:?} ({size})", n + 1);
            } else {
                println!("{}/{files_len}: combining {file:?}", n + 1);
            }

            // do not use BufReader here since we expect large files to be combined.
            // (benched and saw larger files took longer to combine with the use of BufReader than not.)
            let mut file = File::open(file).expect("File could not be opened");
            io::copy(&mut file, &mut final_file).expect("Failed to copy files");
        }

        Some(start.elapsed())
    } else {
        let err = final_file.expect_err("File must be Err here.");

        if matches!(err.kind(), io::ErrorKind::AlreadyExists) {
            // skip prompt
            if no_interaction {
                println!("File \"{output_name}\" already exists, extracting.");
                None
            } else {
                print_flush!("File \"{output_name}\" already exists, extract it? (y/n): ");

                if prompt().to_lowercase() != "y" {
                    exit(1);
                }

                None
            }
        } else {
            panic!("File {output_name} was unable to be created: {err:?}")
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
        .find(|d| d.path().is_dir() && name_has_keywords(name.split(' '), &d.path()));

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

/// Create shortcut from executable found in `destination`.
///
/// We want to fail silently, so this function returns `()`.
pub fn create_shortcut(app_name: &str, destination: &Path, no_interaction: bool) {
    let executables =
        glob(&destination.join("*exe").to_string_lossy()).expect("Invalid glob pattern used");
    let executables: Vec<PathBuf> = executables.filter_map(Result::ok).collect();

    let executable: PathBuf = if executables.is_empty() {
        // skip to end
        if no_interaction {
            println!("Could not find any installed executables.");
            return;
        }

        print_flush!("No installed executables could be found. (s)kip creating shortcut or (g)ive path manually? ");

        if prompt().to_lowercase() == "g" {
            prompt_user_for_path(destination)
        } else {
            return;
        }
    } else if let Some(found_executable) = executables
        .iter()
        .find(|p| name_has_keywords(app_name.split(' '), p))
    {
        // assume yes
        if no_interaction {
            println!("Found executable {:?}", &found_executable);
            dunce::canonicalize(found_executable.clone()).expect("Executable path should exist.")
        } else {
            print_flush!(
                "Found executable {:?}, is it correct? (y/n): ",
                &found_executable
            );

            if prompt().to_lowercase() == "y" {
                found_executable.clone()
            } else {
                if executables.len() == 1 {
                    println!("Found only 1 executable, cannot create shortcut.");
                    return;
                }

                println!("\nExecutables found:");
                for (n, executable) in executables.iter().enumerate() {
                    println!("{}: {executable:?}", n + 1);
                }

                let choice: usize = prompt_user_for_usize(executables.len());
                let choice = executables
                    .get(choice - 1)
                    .expect("should be less than # of executables");

                dunce::canonicalize(choice.clone()).expect("Chosen executable path should exist.")
            }
        }
    } else {
        println!("Found only 1 executable: {:?}", executables[0]);
        dunce::canonicalize(executables[0].clone()).expect("Executable path should exist.")
    };

    let appdata = std::env::var("APPDATA").expect("Could not find environment variable APPDATA");
    let start_menu = PathBuf::from(appdata).join(r"Microsoft\Windows\Start Menu\Programs");

    let shortcut = start_menu.join(format!("{app_name}.lnk"));
    let Ok(shortcut_dir) = dunce::canonicalize(destination) else {
        return;
    };

    // create a shortcut in powershell
    let script = format!(
        // do not need quotes around placeholder since PathBuf's Debug impl adds quotes
        r"$shortcut = (New-Object -COMObject WScript.Shell).CreateShortcut({shortcut:?});
            $shortcut.TargetPath = {executable:?};
            $shortcut.WorkingDirectory = {shortcut_dir:?};
            $shortcut.Save()",
    );

    let powershell = Command::new("powershell")
        .args(["-c", &script])
        .status()
        .expect("Failed to run powershell.");

    match powershell.code() {
        Some(0) => println!("Successfully created shortcut to {executable:?}."),
        Some(1) => {
            println!("Powershell encountered an uncaught error while creating the shortcut.");
        }
        code => println!("Powershell exit code: {code:?}"),
    }
}
