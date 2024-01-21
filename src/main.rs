use std::fs::{create_dir, remove_dir_all};
use std::path::Path;
use std::process::{Command, ExitCode};
use clap::{Parser, Subcommand};

type Result = std::result::Result<(), ()>;

trait Skipable {
    fn ignore(&self) -> Result;
}

impl<T, E> Skipable for std::result::Result<T, E> {
    fn ignore(&self) -> Result {
        match self {
            Ok(_) => Ok(()),
            Err(_) => Err(())
        }
    }
}

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
        #[arg(short, long, default_value = "32")]
        dui: String,
        /// Pendant address
        #[arg(short, long, default_value = "1")]
        address: String,
        /// Signal
        #[arg(short, long, default_value = "5")]
        signal: String,
    },
    /// Initiate a new video call
    Call {
        #[command(subcommand)]
        command: Call,
    },
}

#[derive(Debug, Subcommand)]
enum Call {
    /// Initiate a new Vidyo (LifeStream) call
    Vidyo {
        /// Room
        room: String,
        /// PIN (access code)
        #[arg(short, long)]
        pin: Option<String>,
        /// Patient's display name
        #[arg(short, long)]
        display_name: Option<String>,
        /// Host
        #[arg(short, long)]
        end_point: Option<String>,
    },
    /// Initiate a new Zoom call
    Zoom {
        /// Meeting number
        #[arg(short, long)]
        number: Option<String>,
        /// Password
        #[arg(short, long)]
        password: Option<String>,
        /// Meeting link,
        link: Option<String>,
    },
}

fn main() -> ExitCode {
    if run(Cli::parse()).is_ok() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn run(cli: Cli) -> Result {
    match cli.command {
        Commands::Pull { command } => match command {
            Pull::Config => {
                shell("adb pull /sdcard/TBS/config.json")?;
                open("config.json")?;
            }
            Pull::Log => {
                remove_dir_all("Log").ignore()?;
                create_dir("Log").ignore()?;
                shell("adb pull /sdcard/TBS/Log")?;
            }
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
                shell(&format!("adb shell am start \
                    -n com.mobilehelp.alert/.ui.registration.WelcomeActivity \
                    -a com.mobilehelp.auto.register \
                    --es phone {phone} \
                    --es token {token}"
                ))?;
            }
            AM::RX { dui, address, signal } => {
                shell(&format!("adb shell am startservice \
                    -n com.mobilehelp.stub.i2c/.services.I2CService \
                    -a fake \
                    --ei dui {dui} \
                    --ei address {address} \
                    --ei signal {signal}"
                ))?;
            }
            AM::Call { command } => match command {
                Call::Vidyo { room, pin, display_name, end_point } => {
                    let mut s = String::from(
                        "adb shell am start \
                        -n com.mobilehelp.vidyomeeting/.MainActivity \
                        -c android.intent.category.LAUNCHER \
                        -a android.intent.action.MAIN \
                        --activity-single-top \
                        --es room "
                    );
                    s += &room;
                    if let Some(pin) = pin {
                        s += " --es pin ";
                        s += &pin;
                    }
                    if let Some(display_name) = display_name {
                        s += " --es displayName \"";
                        s += &display_name;
                        s += "\"";
                    }
                    if let Some(end_point) = end_point {
                        s += " --es host ";
                        s += &end_point;
                    }
                    shell(&s)?;
                }
                Call::Zoom { number, password, link } => {
                    let mut s = String::from(
                        "adb shell am start \
                        -n com.mobilehelp.zoom/.activities.StartMeetingActivity \
                        -c android.intent.category.LAUNCHER \
                        -a android.intent.action.MAIN \
                        --activity-single-top"
                    );
                    if let Some(link) = link {
                        s += " --es meetingLink ";
                        s += &link;
                    } else {
                        s += " --es meetingNumber ";
                        s += &number.required("Meeting number");
                        s += " --es password ";
                        s += &password.required("Password");
                    }
                    shell(&s)?;
                }
            }
        }
    }
    Ok(())
}

fn shell(program_line: &str) -> Result {
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

fn open(path: &str) -> Result {
    opener::open(Path::new(path)).map_err(|_| ())
}

trait Required<T> {
    fn required(self, name: &str) -> T;
}

impl<T> Required<T> for Option<T> {
    fn required(self, name: &str) -> T {
        return match self {
            None => panic!("{name} is required."),
            Some(value) => value,
        };
    }
}

