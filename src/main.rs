#![feature(str_split_remainder)]

use std::path::Path;
use std::process::{Command, ExitCode};

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "tbs")]
#[command(about = "Shortcut for commands.", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Get files from the device
    Pull {
        #[command(subcommand)]
        command: Pull,
    },
    /// Push files to the device
    Push {
        #[command(subcommand)]
        command: Push,
    },
    /// Remove files from the device
    Clear {
        #[command(subcommand)]
        command: Clear,
    },
    /// Run am (activity manager) commands
    AM {
        #[command(subcommand)]
        command: AM,
    },
}

#[derive(Debug, Subcommand)]
enum Push {
    /// Push file config.json from current directory to device
    Config,
}

#[derive(Debug, Subcommand)]
enum Pull {
    /// Get file config.json from the device
    Config,
    /// Get all logs from the device
    Log,
}

#[derive(Debug, Subcommand)]
enum Clear {
    /// Remove all logs from the device
    Log,
}

#[derive(Debug, Subcommand)]
enum AM {
    /// Register device
    Register {
        /// Phone number
        phone: String,
        /// One time token
        token: String,
    },
    /// Fake packet 433 sent
    RX {
        /// DUI (device unique identifier)
        #[arg(short, long, default_value = "30")]
        dui: String,
        /// Pendant address
        #[arg(short, long, default_value = "1")]
        address: String,
        /// Signal
        #[arg(short, long, default_value = "5")]
        signal: String,
    },
}

fn main() -> ExitCode {
    if run(Cli::parse()).is_ok() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn run(cli: Cli) -> Result<(), ()> {
    match cli.command {
        Commands::Pull { command } => match command {
            Pull::Config => {
                shell("adb pull /sdcard/TBS/config.json")?;
                open("config.json")?;
            }
            Pull::Log => {}
        }
        Commands::Push { command } => match command {
            Push::Config => {
                shell("adb push config.json /sdcard/TBS/")?;
                shell("adb shell am broadcast -a com.mobilehelp.action.config.updated")?;
            }
        }
        Commands::Clear { command } => match command {
            Clear::Log => {
                shell("adb shell rm -rf /sdcard/TBS/Log/*")?;
            }
        }
        Commands::AM { command } => match command {
            AM::Register { phone, token } => {
                shell(&format!("adb shell am start -n com.mobilehelp.alert/.ui.registration.WelcomeActivity -a com.mobilehelp.auto.register --es phone {phone} --es token {token}"))?;
            }
            AM::RX { dui, address, signal } => {
                shell(&format!("adb shell am start -n com.mobilehelp.stub.i2c/.services.I2CService -a fake --es dui {dui} --es address {address} --es signal {signal}"))?;
            }
        }
    }
    Ok(())
}

fn shell(program_line: &str) -> Result<(), ()> {
    let mut split = program_line.split(' ');
    let program = split.next().unwrap();
    let args: Vec<&str> = split.collect();
    let mut cmd = Command::new(program);
    if !args.is_empty() {
        cmd.args(args);
    }
    let ok = match cmd.spawn() {
        Ok(mut result) => {
            match result.wait() {
                Ok(status) => status.success(),
                Err(_) => false,
            }
        }
        Err(e) => {
            println!("{}: {}", program, e);
            false
        }
    };
    if ok { Ok(()) } else { Err(()) }
}

fn open(path: &str) -> Result<(), ()> {
    opener::open(Path::new(path)).map_err(|_| ())
}


