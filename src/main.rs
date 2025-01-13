use std::{
    borrow::Cow, env, fs::{self, File}, io::{self, stderr, Write}, os::windows::fs::MetadataExt, panic::{self, PanicHookInfo}, path::{Path, PathBuf}, process::{exit, Command}, time::{Duration, Instant}
};

use clap::Parser;
use glob::{glob, Paths};
use humansize::{format_size, DECIMAL};
use partsinstall::{
    check_name, compare_numeric_extension, create_destination, flatten_dir, print_flush, prompt,
    prompt_user_for_path, prompt_user_for_usize,
};

#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    /// Name of application to install
    name: PathBuf,

    /// Destination of install
    #[arg(env = "pinst_destination")]
    destination: PathBuf,

    /// Do not create start menu shortcuts
    #[arg(short = 'S', long)]
    no_shortcut: bool,

    /// Do not flatten installed directories.
    #[arg(short = 'F', long)]
    no_flatten: bool,

    /// Assume answer that continues execution without interaction on all prompts
    #[arg(short = 'y', long)]
    no_interaction: bool,
}

/// Print only the `payload` on panic.
fn panic_hook(panic_info: &PanicHookInfo) {
    if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
        if writeln!(stderr(), "{s}").is_err() {
            println!("{s}");
        }
    } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
        if writeln!(stderr(), "{s}").is_err() {
            println!("{s}");
        }
    } else {
        let s = "Panic occurred";
        if writeln!(stderr(), "{s}").is_err() {
            println!("{s}");
        }
    }
}

/// Print summary and exit with exit code 0
fn success(
    combine_time: Duration,
    extract_time: Duration,
    flatten_time: Duration,
    start: Instant,
) -> ! {
    println!(
        "\nDone! (combining took {combine_time:?}, extracting took {extract_time:?}, flattening took {flatten_time:?}, total: {:?})",
        start.elapsed()
    );

    exit(0)
}

#[allow(clippy::too_many_lines)]
fn main() {
    let start = Instant::now();

    let args = Args::parse();

    panic::set_hook(Box::new(panic_hook));

    assert!(
        args.destination.exists(),
        "Destination {:?} does not exist.",
        args.destination
    );
    assert_eq!(
        env::consts::OS,
        "windows",
        "partsinstall is meant for Windows."
    );

    let name_stem = args
        .name
        .file_stem()
        .expect("NAME was not valid.")
        .to_string_lossy();

    let glob_pattern = format!("{name_stem}*");

    let files: Paths = if args.name.is_dir() {
        glob(
            Path::new(name_stem.as_ref())
                .join(&glob_pattern)
                .to_string_lossy()
                .as_ref(),
        )
        .expect("Glob pattern was not valid")
    } else {
        glob(&glob_pattern).expect("Glob pattern was not valid")
    };

    let mut files: Vec<PathBuf> = files.filter_map(Result::ok).collect();

    if files.is_empty() {
        println!("No files were found starting with the name {name_stem}");
        exit(1);
    }

    let mut combine_time = Duration::ZERO;

    let final_name = if files.len() == 1 {
        if args.no_interaction {
            files[0].to_string_lossy()
        } else {
            print_flush!("Only 1 file found, extract {:?}? (y/n): ", files[0]);

            // true
            if prompt().to_lowercase() != "y" {
                exit(1)
            }

            files[0].to_string_lossy()
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
            .find(|part| !part.chars().all(char::is_numeric))
            .expect("Could not determine output file extension");

        let final_name = format!("{name_stem}.{final_ext}");
        println!("Combining to {final_name}");
        let final_file = File::create_new(&final_name);

        match final_file {
            Ok(mut final_file) => {
                let files_len = files.len();

                // glob sorts alphanumerically, meaning it will sort correctly until a number is larger than 10.
                // eg. 01, 11, 02, 021, 03 will be how glob sorts numbers larger than 10.
                if files.len() > 10 {
                    files.sort_by(|a, b| compare_numeric_extension(a, b));
                }

                for (n, file) in files.iter().enumerate() {
                    if let Ok(metadata) = fs::metadata(&file) { 
                        let size = format_size(metadata.file_size(), DECIMAL);
                        println!("{}/{files_len}: combining {file:?} ({size})", n + 1);
                    } else {
                        println!("{}/{files_len}: combining {file:?}", n + 1);
                    }
                    let mut file = File::open(file).expect("File could not be opened");
                    io::copy(&mut file, &mut final_file).expect("Failed to copy files");
                }

                combine_time = combine_start.elapsed();
            }
            Err(err) => match err.kind() {
                io::ErrorKind::AlreadyExists => {
                    // skip prompt
                    if args.no_interaction {
                        println!("File \"{final_name}\" already exists, extracting.");
                    } else {
                        print_flush!("File \"{final_name}\" already exists, extract it? (y/n): ");

                        if prompt().to_lowercase() != "y" {
                            exit(1);
                        }
                    }
                }
                err => panic!("File {final_name} was unable to be created: {err:?}"),
            },
        };

        Cow::Owned(final_name)
    };

    let destination = args.destination.join(name_stem.as_ref());
    println!("\nExtracting {name_stem} to {destination:?}");

    create_destination(&destination, args.no_interaction);

    let destination_str = destination.to_string_lossy();
    let destination_arg = format!("-o{destination_str}");

    let sevenzip_args: &[&str] = if args.no_interaction {
        print_flush!("\n7z using -y");
        // x - extract with full paths (https://documentation.help/7-Zip/extract_full.htm)
        &["x", &destination_arg, "-y", &final_name]
    } else {
        &["x", &destination_arg, &final_name]
    };

    let extract_start = Instant::now();
    let sevenzip = Command::new("7z")
        .args(sevenzip_args)
        .status()
        .expect("Could not run 7z");

    println!();

    // found here: https://documentation.help/7-Zip/exit_codes.htm
    match sevenzip.code().expect("Could not determine 7z's exit code") {
        // ok (no error or warning)
        0 | 1 => {}
        2 => panic!("7z encounted a fatal error"),
        7 => panic!("7z: command line error"),
        8 => panic!("7z: not enough memory for operation"),
        255 => panic!("7z: user stopped the process"),
        code => panic!("Unknown 7z exit code {code} encountered"),
    }

    let extract_time = extract_start.elapsed();

    let flatten_start = Instant::now();
    if args.no_flatten {
        println!("Not flattening install directory.");
    } else {
        flatten_dir(&name_stem, &destination);
    }
    let flatten_time = flatten_start.elapsed();

    if args.no_shortcut {
        println!("Not creating start menu shortcut.");
    } else {
        println!("Creating start menu shortcut:");

        let executables =
            glob(&destination.join("*exe").to_string_lossy()).expect("Invalid glob pattern used");
        let executables: Vec<PathBuf> = executables.filter_map(Result::ok).collect();

        let executable: PathBuf = if executables.is_empty() {
            // skip to end
            if args.no_interaction {
                println!("Could not find any installed executables.");
                success(combine_time, extract_time, flatten_time, start);
            }

            print_flush!("No installed executables could be found. (s)kip creating shortcut or (g)ive path manually? ");

            if prompt().to_lowercase() == "g" {
                prompt_user_for_path(&destination)
            } else {
                success(combine_time, extract_time, flatten_time, start);
            }
        } else if let Some(found_executable) = executables
            .iter()
            .find(|p| check_name(name_stem.split(' '), p))
        {
            // assume yes
            if args.no_interaction {
                println!("Found executable {:?}", &found_executable);
                dunce::canonicalize(found_executable.clone())
                    .expect("Executable path should exist.")
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
                        success(combine_time, extract_time, flatten_time, start);
                    }

                    println!("\nExecutables found:");
                    for (n, executable) in executables.iter().enumerate() {
                        println!("{}: {executable:?}", n + 1);
                    }

                    let choice: usize = prompt_user_for_usize(executables.len());
                    let choice = executables
                        .get(choice - 1)
                        .expect("should be less than # of executables");

                    dunce::canonicalize(choice.clone())
                        .expect("Chosen executable path should exist.")
                }
            }
        } else {
            println!("Found only 1 executable: {:?}", executables[0]);
            dunce::canonicalize(executables[0].clone()).expect("Executable path should exist.")
        };

        let appdata =
            std::env::var("APPDATA").expect("Could not find environment variable APPDATA");
        let start_menu = PathBuf::from(appdata).join(r"Microsoft\Windows\Start Menu\Programs");
        let shortcut = start_menu.join(format!("{name_stem}.lnk"));

        // create a shortcut in powershell
        let script = format!(
            // do not need quotes around placeholder since PathBuf's Debug impl adds quotes
            r"$shortcut = (New-Object -COMObject WScript.Shell).CreateShortcut({:?}); $shortcut.TargetPath = {:?}; $shortcut.Save()",
            &shortcut, &executable
        );

        let pwsh = Command::new("powershell")
            .args(["-c", &script])
            .status()
            .expect("Failed to run powershell.");

        match pwsh.code() {
            Some(0) => println!("Successfully created shortcut to {executable:?}."),
            Some(1) => {
                println!("Powershell encountered an uncaught error while creating the shortcut.");
            }
            code => println!("Powershell exit code: {code:?}"),
        }
    }

    success(combine_time, extract_time, flatten_time, start);
}
