//! Process adapter for package inspection commands.

use clap::Parser;

fn main() {
    let args = qti_cli::PackageMakerArgs::parse();
    let is_failing_check = matches!(args.command, qti_cli::PackageMakerCommand::Check { ref path } if qti_cli::package_check_failed(path));
    match qti_cli::run_package_maker(args) {
        Ok(output) => {
            println!("{output}");
            if is_failing_check {
                std::process::exit(1);
            }
        }
        Err(error) => {
            eprintln!("ERROR: {error}");
            std::process::exit(error.exit_code());
        }
    }
}
