use midir::MidiOutputConnection;
use thiserror::Error;

use super::app;

const HELP: &str = "Available commands:
h or help                      - Help and information.
q or quit or CTRL-C or CTRL-D  - Exit configuration.
i or info                      - Shows general info about the device and test MIDI connection.
p or print                     - Display configuration dump.
p or print 1-16                - Display configuration dump for this port only.
f or forward 1-16 none         - Clears forwarding for this port marking the port inactive.
f or forward 1-16 all          - Forwards this port to all ports except itself (default).
f or forward 1-16 to 1-16      - Forwards this port to another port.
f or forward 1-16 add 1-16     - The same as \"to\".
f or forward 1-16 rm 1-16      - Remove forward from this port to another port.
n or name 1-16 new_name        - Rename port, up to 8 chars. Use \"clear\" to clear the name.
id 1-16                        - Identify port number by turning on LEDs for approx 10 seconds.
r or reload                    - Reload configuration from flash.
wr or write                    - Write current memory configuration to flash.
defaults                       - Write a default configuration to memory.
";
// dump                   - Write current configuration to file.
// restore                - Load configuration from dump file.

#[derive(Debug, Error)]
pub enum ParseError {
	#[error("Invalid mode, use \"all\", \"none\", \"to\" or \"rm\"")]
	InvalidMode,

	#[error("invalid number of arguments, use \"h\" for help.")]
	Argument,

	#[error("name should be no more than 8 characters.")]
	NameLength,

	#[error("midi app: {source}")]
	ParseAppError {
		#[from]
		source: app::AppError,
	},

	#[error("parse number: {source}")]
	IntError {
		#[from]
		source: std::num::ParseIntError,
	},
}

pub fn parse_command(command: &str, port: &mut MidiOutputConnection) -> Result<bool, ParseError> {
	let input: Vec<&str> = command.trim_end().split(' ').collect();
	let command = input
		.first()
		.expect("expected to find a command") // XXX:
		.to_lowercase();
	let mc = command.as_str();

	match mc {
		"h" | "help" => {
			println!("{}", HELP);
		}
		"i" | "info" => {
			app::info_command(port)?;
		}
		"p" | "print" => {
			let srcport = input.get(1).unwrap_or(&"0").parse::<u8>()?;
			app::print_command(port, srcport)?;
		}
		"f" | "forward" => {
			if input.len() < 3 {
				return Err(ParseError::Argument);
			}
			let srcport = input[1].parse::<u8>()?;
			let mode = match input[2] {
				"all" => 1,
				"none" => 2,
				"to" => 3,
				"add" => 3,
				"rm" => 4,
				_ => return Err(ParseError::InvalidMode),
			};
			let dstport = input.get(3).unwrap_or(&"1").parse::<u8>()?;
			app::forward_command(port, srcport, dstport, mode)?;
			app::print_command(port, srcport)?;
		}
		"wr" | "write" => {
			app::sysex(port, &[0xf0, 0x7d, 0x2a, 0x4d, 0x03, 0xf7], 0x43, 6)?;
			println!("Configuration saved.");
		}
		"r" | "reload" => {
			app::sysex(port, &[0xf0, 0x7d, 0x2a, 0x4d, 0x04, 0xf7], 0x44, 6)?;
			println!("Configuration reloaded from flash, use \"p\" to view.");
		}
		"defaults" => {
			app::sysex(port, &[0xf0, 0x7d, 0x2a, 0x4d, 0x05, 0xf7], 0x45, 6)?;
			println!(
				"Configuration reset to defaults, use \"p\" to view the new configuration, \"w\" to write to flash and \"r\" to reload from flash."
			);
		}
		"n" | "name" => {
			if input.len() < 2 {
				return Err(ParseError::Argument);
			}
			let srcport = input[1].parse::<u8>()?;
			let mut newname = match input.get(2) {
				None => return Err(ParseError::Argument),
				Some(n) => n,
			};
			if newname.len() > 8 {
				return Err(ParseError::NameLength);
			}
			if newname == &"clear" {
				newname = &""
			}
			let msg = [
				&[0xf0, 0x7d, 0x2a, 0x4d, 0x06, srcport - 1],
				newname.as_bytes(),
				&[0xf7],
			]
			.concat();
			app::sysex(port, msg.as_slice(), 0x46, 6)?;
			println!("Port renamed, use \"p\" to view the new configuration.");
		}
		"id" => {
			if input.len() != 2 {
				return Err(ParseError::Argument);
			}
			let srcport = input[1].parse::<u8>()?;
			app::sysex(
				port,
				&[0xf0, 0x7d, 0x2a, 0x4d, 0x07, srcport - 1, 0xf7],
				0x47,
				6,
			)?;
			println!("Port {} LEDs turned on for approx 10 seconds.", srcport);
		}
		"" => {}
		_ => println!("Invalid command: \"{}\", type \"h\" for help.", command),
	}
	Ok(false)
}
