use std::{env, ffi::OsString, process};

mod cli;
#[cfg(target_os = "linux")]
mod linux;

fn main() {
    let args: Vec<OsString> = env::args_os().skip(1).collect();
    match cli::parse(&args) {
        Ok(cli::Command::Help) => print!("{}", cli::HELP),
        Ok(cli::Command::Available) => available(),
        Ok(command) => run_platform(command),
        Err(error) => {
            eprintln!("{error}\nrun 'keyinject help'");
            process::exit(2);
        }
    }
}

#[cfg(target_os = "linux")]
fn available() {
    if let Err(error) = linux::available() {
        eprintln!("unavailable: {error}");
        process::exit(1);
    }
    println!("available");
}

#[cfg(not(target_os = "linux"))]
fn available() {
    eprintln!("unavailable: keyinject requires Linux /dev/uinput");
    process::exit(1);
}

#[cfg(target_os = "linux")]
fn run_platform(command: cli::Command) {
    linux::run(command);
}

#[cfg(not(target_os = "linux"))]
fn run_platform(_: cli::Command) {
    eprintln!("unavailable: keyinject requires Linux /dev/uinput");
    process::exit(1);
}
