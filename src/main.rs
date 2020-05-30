// extern crate midir;

#[macro_use]
extern crate lazy_static;

use midir::{Ignore, MidiInput, MidiOutput, MidiOutputConnection};
use rustyline::error::ReadlineError;
use rustyline::Editor;
use std::error::Error;
use std::io::{stdin, stdout, Write};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::thread::sleep;
use std::time::Duration;
use thiserror::Error;

mod msg;
mod port;

fn main() -> Result<(), Box<dyn Error>> {
	let mut input = String::new();
	let mut midi_in = MidiInput::new("midir reading input")?;
	let midi_out = MidiOutput::new("midir output port")?;
	midi_in.ignore(Ignore::TimeAndActiveSense);
	// Get an input port (read from console if multiple are available)
	let in_ports = midi_in.ports();
	let in_port_num = match in_ports.len() {
		0 => return Err("no MIDI port found".into()),
		1 => {
			println!(
				"Choosing the only available MIDI port: {}",
				midi_in.port_name(&in_ports[0]).unwrap()
			);
			0
		}
		_ => {
			println!("Available MIDI ports:");
			for (i, p) in in_ports.iter().enumerate() {
				println!("{}: {}", i, midi_in.port_name(p).unwrap());
			}
			print!("Select port number to use for configuration: ");
			stdout().flush()?;
			let mut input = String::new();
			stdin().read_line(&mut input)?;
			input.trim().parse::<usize>()?
		}
	};
	let in_port = in_ports.get(in_port_num).ok_or("invalid port")?;
	let out_ports = midi_out.ports();
	let out_port = out_ports.get(in_port_num).ok_or("invalid port")?;
	let mut conn_out = midi_out.connect(&out_port, "router-read-output")?;
	// _conn_in needs to be a named parameter, because it needs to be kept alive until the end of the scope
	let _conn_in = midi_in.connect(
		in_port,
		"router-read-input",
		// move |stamp, message, _| handle_message(stamp, message),
		handle_message,
		(),
	)?;
	input.clear();

	let mut rl = Editor::<()>::new();
	println!("MIDI Router ready for configuration.");
	println!("Press \"h\" or \"help\" for a list of commands.");
	loop {
		let readline = rl.readline("\x1b[1mRouter\x1b[0m# ");
		match readline {
			Ok(line) => {
				rl.add_history_entry(line.as_str());
				let cmd = line.to_lowercase();
				if cmd == "quit" || cmd == "q" {
					break;
				}
				match parse_command(line.as_str(), &mut conn_out) {
					Ok(res) => {
						if res {
							break;
						};
					}
					Err(err) => {
						println!("Error: {}", err);
					}
				}
			}
			Err(ReadlineError::Interrupted) => {
				// println!("CTRL-C");
				break;
			}
			Err(ReadlineError::Eof) => {
				// println!("CTRL-D");
				break;
			}
			Err(err) => {
				println!("Error: {:?}", err);
				break;
			}
		}
	}

	println!("Bye.");
	Ok(())
}

//

lazy_static! {
	static ref MESSAGE: Mutex<msg::Message> = Mutex::new(msg::Message::new());
}

// handle_message is the midi input callback.
// only the last received msg is stored currently.
fn handle_message(tstamp: u64, message: &[u8], _: &mut ()) {
	if message[0] == 0xf0 {
		MESSAGE.lock().unwrap().tstamp = tstamp;
		MESSAGE.lock().unwrap().message = message.to_vec();
	}
}

#[derive(Debug, Error)]
enum AppError {
	#[error("source and/or dstport should be between 1 and 16")]
	PortNumber,

	#[error("timeout receiving MIDI data")]
	ReceiveTimeout,

	#[error("invalid message received: {0}")]
	InvalidMessage(String),

	#[error("error sending MIDI data")]
	SendError {
		#[from]
		source: midir::SendError,
	},

	#[error("error acquiring mutex {:?}", .source)]
	PoisonError {
		#[from]
		source: PoisonError<MutexGuard<'static, msg::Message>>,
	},
}

// block waits on a message.
// move this to a channel in the future.
fn block() -> Result<(), AppError> {
	let mut n: u16 = 0;
	loop {
		let mut msg = MESSAGE.lock()?;
		if msg.tstamp > msg.laststamp {
			msg.laststamp = msg.tstamp;
			return Ok(());
		}
		drop(msg);

		sleep(Duration::from_millis(1));
		n += 1;
		if n > 5000 {
			return Err(AppError::ReceiveTimeout);
		}
	}
}

fn sysex(
	port: &mut MidiOutputConnection,
	command: &[u8],
	response: u8,
	length: usize,
) -> Result<(), AppError> {
	port.send(command)?;
	block()?;
	let msg = MESSAGE.lock()?;
	if !msg.verify(response, length) {
		return Err(AppError::InvalidMessage(format!("{:?}", msg)));
	}
	Ok(())
}

fn print_command(port: &mut MidiOutputConnection, srcport: u8) -> Result<(), AppError> {
	sysex(port, &[0xf0, 0x7d, 0x2a, 0x4d, 0x01, 0xf7], 0x41, 518)?;
	let msg = MESSAGE.lock()?;
	msg.handleprint(srcport);
	Ok(())
}

fn forward_command(
	port: &mut MidiOutputConnection,
	srcport: u8,
	dstport: u8,
	mode: u8,
) -> Result<(), AppError> {
	if (srcport == 0 || srcport > 16) || (dstport == 0 || dstport > 16) {
		return Err(AppError::PortNumber);
	}

	sysex(
		port,
		&[
			0xf0,
			0x7d,
			0x2a,
			0x4d,
			0x02,
			srcport - 1,
			mode,
			dstport - 1,
			0xf7,
		],
		0x42,
		6,
	)?;
	let msg = MESSAGE.lock()?;
	println!("{:?}", msg);
	println!("Configuration updated.");
	Ok(())
}

//

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
r or reload                    - Reload configuration from flash.
wr or write                    - Write current memory configuration to flash.
defaults                       - Write a default configuration to memory.
";
// id 1-16                - Identify port number by flashing leds for 10 seconds.
// dump                   - Write current configuration to file.
// restore                - Load configuration from dump file.

#[derive(Debug, Error)]
enum ParseError {
	#[error("Invalid mode, use \"all\", \"none\", \"to\" or \"rm\"")]
	InvalidMode,

	#[error("too few arguments, use \"h\" for help.")]
	Argument,

	#[error("name should be no more than 8 characters.")]
	NameLength,

	#[error("midi app: {source}")]
	ParseAppError {
		#[from]
		source: AppError,
	},

	#[error("acquire mutex: {source}")]
	PoisonError {
		#[from]
		source: PoisonError<MutexGuard<'static, msg::Message>>,
	},

	#[error("parse number: {source}")]
	IntError {
		#[from]
		source: std::num::ParseIntError,
	},
}

fn parse_command(command: &str, port: &mut MidiOutputConnection) -> Result<bool, ParseError> {
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
			sysex(port, &[0xf0, 0x7d, 0x2a, 0x4d, 0x00, 0xf7], 0x40, 9)?;
			let msg = MESSAGE.lock()?;
			msg.handleinfo();
		}
		"p" | "print" => {
			let srcport = input.get(1).unwrap_or(&"0").parse::<u8>()?;
			print_command(port, srcport)?;
			// let msg = MESSAGE.lock()?;
			// println!("{:?}", msg);
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
			forward_command(port, srcport, dstport, mode)?;
			print_command(port, srcport)?;
		}
		"wr" | "write" => {
			sysex(port, &[0xf0, 0x7d, 0x2a, 0x4d, 0x03, 0xf7], 0x43, 6)?;
			println!("Configuration saved.");
		}
		"r" | "reload" => {
			sysex(port, &[0xf0, 0x7d, 0x2a, 0x4d, 0x04, 0xf7], 0x44, 6)?;
			println!("Configuration reloaded from flash, use \"p\" to view.");
		}
		"defaults" => {
			sysex(port, &[0xf0, 0x7d, 0x2a, 0x4d, 0x05, 0xf7], 0x45, 6)?;
			println!(
				"Configuration reset to defaults, use \"p\" to view the new configuration, \"w\" to write to flash and \"r\" to reload from flash."
			);
		}
		"n" | "name" => {
			if input.len() < 2 {
				return Err(ParseError::Argument);
			}
			let srcport = input.get(1).unwrap_or(&"0").parse::<u8>()?;
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
			let msg = [&[0xf0, 0x7d, 0x2a, 0x4d, 0x06, srcport-1], newname.as_bytes(), &[0xf7]].concat();
			sysex(port, msg.as_slice(), 0x46, 6)?;
			println!("Port renamed, use \"p\" to view the new configuration.");
		}
		"t" | "test" => {
			sysex(port, &[0xf0, 0x7d, 0x2a, 0x4d, 0x06, 0xf7], 0x46, 6)?;
			let msg = MESSAGE.lock()?;
			println!("{:?}", msg);
		}
		"" => {}
		_ => println!("Invalid command: \"{}\", type \"h\" for help.", command),
	}
	Ok(false)
}
