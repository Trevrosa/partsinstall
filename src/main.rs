/// Steps the cli tool takes.
mod steps;

use std::{
    env,
    io::{stderr, Write},
    panic::{self, PanicHookInfo},
    path::{Path, PathBuf},
    process::{exit, Command},
    time::{Duration, Instant},
};

use clap::Parser;
use glob::{glob, Paths};
use partsinstall::print_flush;
use steps::{create_destination, create_shortcut, find_final_name, flatten_dir, parse_app_name};

#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    /// Name of application in working directory to install
    name: PathBuf,

    /// Destination of install
    #[arg(env = "pinst_destination")]
    destination: PathBuf,

    /// Working directory the tool will use
    #[arg(short, long)]
    working_dir: Option<PathBuf>,

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

fn main() {
    let start = Instant::now();

    let args = Args::parse();

    panic::set_hook(Box::new(panic_hook));

    assert!(
        args.destination.exists(),
        "Destination {:?} does not exist.",
        args.destination
    );

    if let Some(working_dir) = args.working_dir {
        assert!(
            working_dir.exists(),
            "Working directory {working_dir:?} does not exist."
        );

        env::set_current_dir(&working_dir).expect("Could not set working directory.");
        println!("Using working directory: {working_dir:?}.\n");
    }

    let Some(app_name) = parse_app_name(&args.name) else {
        println!("Could not parse app name.");
        exit(1);
    };
    println!("Parsed name as: {app_name}\n");

    let glob_pattern = format!("{app_name}*");

    let files: Paths = if args.name.is_dir() {
        glob(
            &Path::new(app_name.as_ref())
                .join(&glob_pattern)
                .to_string_lossy(),
        )
        .expect("Glob pattern was not valid")
    } else {
        glob(&glob_pattern).expect("Glob pattern was not valid")
    };

    let mut files: Vec<PathBuf> = files.filter_map(Result::ok).collect();

    if files.is_empty() {
        println!("No files were found starting with the name {app_name}");
        exit(1);
    }

    let (final_name, combine_time) = find_final_name(&app_name, &mut files, args.no_interaction);

    let destination = args.destination.join(app_name.as_ref());
    println!("\nExtracting {app_name} to {destination:?}");

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
        flatten_dir(&app_name, &destination);
    }
    let flatten_time = flatten_start.elapsed();

    if args.no_shortcut {
        println!("Not creating start menu shortcut.");
    } else if env::consts::OS == "windows" {
        println!("Creating start menu shortcut:");
        create_shortcut(&app_name, &destination, args.no_interaction);
        success(combine_time, extract_time, flatten_time, start);
    } else {
        println!("Not creating start menu shortcuts, not on Windows.");
    }

    success(combine_time, extract_time, flatten_time, start);
}
