use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use magnetite_syntax::{SourceFile, discover};

/// The Magnetite compiler.
#[derive(Parser)]
#[command(name = "mgtc", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// List the modules (.mgt.md files) in a package.
    Files {
        /// The package directory.
        #[arg(default_value = ".")]
        package: PathBuf,
    },
    /// Type-check a package without generating code.
    Check {
        #[arg(default_value = ".")]
        package: PathBuf,
    },
    /// Generate a Rust crate from a package.
    Build {
        #[arg(default_value = ".")]
        package: PathBuf,
    },
    /// Run every example in a package as a test.
    Test {
        #[arg(default_value = ".")]
        package: PathBuf,
    },
}

fn main() -> ExitCode {
    match Cli::parse().command {
        Command::Files { package } => files(&package),
        Command::Check { .. } => not_yet("check", "the parser (step 2) and checker (step 3)"),
        Command::Build { .. } => not_yet("build", "code generation (steps 4–5)"),
        Command::Test { .. } => not_yet("test", "code generation (step 5)"),
    }
}

fn files(package: &std::path::Path) -> ExitCode {
    let paths = match discover(package) {
        Ok(paths) => paths,
        Err(error) => {
            eprintln!("error: can't read {}: {error}", package.display());
            return ExitCode::FAILURE;
        }
    };
    if paths.is_empty() {
        eprintln!("error: no .mgt.md files in {}", package.display());
        return ExitCode::FAILURE;
    }
    for path in paths {
        match SourceFile::load(&path) {
            Ok(file) => println!(
                "{:<16} {} lines  {}",
                file.module_name(),
                file.text.lines().count(),
                path.display()
            ),
            Err(error) => {
                eprintln!("error: can't read {}: {error}", path.display());
                return ExitCode::FAILURE;
            }
        }
    }
    ExitCode::SUCCESS
}

fn not_yet(command: &str, needs: &str) -> ExitCode {
    eprintln!("mgtc {command} isn't implemented yet: it needs {needs}.");
    ExitCode::FAILURE
}
