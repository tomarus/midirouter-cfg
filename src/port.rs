use std::fmt;

// converts 2 7-bit bytes to 1 8-bit byte.
// msb can only be 1 or 0 as it represents bit 7.
fn midi8(msb: u8, lsb: u8) -> u8 {
	(msb << 7) + lsb
}

pub struct Port {
	forwards: u16,
	name: String,
}

impl From<&[u8]> for Port {
	fn from(data: &[u8]) -> Self {
		// construct forward mask
		let m1 = midi8(data[0], data[1]);
		let m2 = midi8(data[2], data[3]);
		let mask = ((m1 as u16) << 8) + m2 as u16;

		// construct port name
		let s: &[u8] = &[data[17], data[19], data[21], data[23], data[25], data[27], data[29], data[31]];
		let st = match String::from_utf8(s.to_vec()) {
			Ok(s) => s,
			Err(_) => "".to_string(),
		};

		Port {
			forwards: mask,
			name: st.trim_matches(char::from(0)).to_string(),
		}
	}
}

impl fmt::Debug for Port {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{:<8} Forward: ", self.name)?;
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
