use std::fmt;

// converts 2 7-bit bytes to 1 8-bit byte.
// msb can only be 1 or 0 as it represents bit 7.
fn midi8(msb: u8, lsb: u8) -> u8 {
	(msb << 7) + lsb
}

pub struct Port {
	forwards: u16,
}

impl From<&[u8]> for Port {
	fn from(data: &[u8]) -> Self {
		let m1 = midi8(data[0], data[1]);
		let m2 = midi8(data[2], data[3]);
		let mask = ((m1 as u16) << 8) + m2 as u16;
		Port { forwards: mask }
	}
}

impl fmt::Debug for Port {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "Forward: ")?;
		for i in 0..16 {
			let fwd = self.forwards & 1 << i;
			if fwd > 0 {
				write!(f, "{:0>2} ", i + 1)?;
			} else {
				write!(f, "   ")?;
			}
		}
		Ok(())
	}
}
