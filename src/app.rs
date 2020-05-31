use midir::MidiOutputConnection;
use std::sync::{MutexGuard, PoisonError};
use std::thread::sleep;
use std::time::Duration;
use thiserror::Error;

use super::msg;

#[derive(Debug, Error)]
pub enum AppError {
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
		let mut msg = msg::MESSAGE.lock()?;
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

pub fn sysex(
	port: &mut MidiOutputConnection,
	command: &[u8],
	response: u8,
	length: usize,
) -> Result<(), AppError> {
	port.send(command)?;
	block()?;
	let msg = msg::MESSAGE.lock()?;
	if !msg.verify(response, length) {
		return Err(AppError::InvalidMessage(format!("{:?}", msg)));
	}
	Ok(())
}

pub fn info_command(port: &mut MidiOutputConnection) -> Result<(), AppError> {
	sysex(port, &[0xf0, 0x7d, 0x2a, 0x4d, 0x00, 0xf7], 0x40, 9)?;
	let msg = msg::MESSAGE.lock()?;
	msg.handleinfo();
	Ok(())
}

pub fn print_command(port: &mut MidiOutputConnection, srcport: u8) -> Result<(), AppError> {
	sysex(port, &[0xf0, 0x7d, 0x2a, 0x4d, 0x01, 0xf7], 0x41, 518)?;
	let msg = msg::MESSAGE.lock()?;
	msg.handleprint(srcport);
	Ok(())
}

pub fn forward_command(
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
	let msg = msg::MESSAGE.lock()?;
	println!("{:?}", msg);
	println!("Configuration updated.");
	Ok(())
}
