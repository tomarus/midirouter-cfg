use std::fmt;
use pretty_hex::*;

use super::port;

pub struct Message {
	pub tstamp: u64,
	pub laststamp: u64,
	pub message: Vec<u8>,
}

impl Message {
	pub fn new() -> Message {
		Message {
			tstamp: 0,
			laststamp: 0,
			message: vec![],
		}
	}

	pub fn verify(&self, response: u8, length: usize) -> bool {
		self.message.len() == length
			&& self.message.len() > 4
			&& self.message[4] == response
			&& self.message[..4] == [0xf0, 0x7d, 0x2a, 0x4d]
	}

	pub fn handleinfo(&self) {
		println!("Device Version: {:#?}", self.message[5]);
		println!("Input Ports: {}", self.message[6]);
		println!("Output Ports: {}", self.message[7]);
	}

	pub fn handleprint(&self, port: u8) {
		if port != 0 {
			self.handleprint_portn(port as usize - 1);
			return;
		}
		for n in 0..16 {
			self.handleprint_portn(n);
		}
	}

	fn handleprint_portn(&self, port: usize) {
		let m = &self.message[(port * 32) + 5..(port * 32) + 37];
		let portst = port::Port::from(m);
		println!("Port {:0>2}: {:?}", port + 1, portst);
	}
}

impl fmt::Debug for Message {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		writeln!(f, "{}: {:?}", self.tstamp, self.message.hex_dump())?;
		Ok(())
	}
}
