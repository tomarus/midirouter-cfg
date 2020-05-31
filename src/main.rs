// extern crate midir;

#[macro_use]
extern crate lazy_static;

use midir::{Ignore, MidiInput, MidiOutput};
use rustyline::error::ReadlineError;
use rustyline::Editor;
use std::error::Error;
use std::io::{stdin, stdout, Write};

mod app;
mod cmd;
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
		msg::handle_message,
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
				match cmd::parse_command(line.as_str(), &mut conn_out) {
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
