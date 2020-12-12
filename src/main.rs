mod dictionary;

use dictionary::DICTIONARY;

use std::io;
use std::io::{BufReader, ErrorKind, Read};

fn main() {
    let mut stdin = BufReader::new(io::stdin());
    let mut words: Vec<&str> = vec![];

    loop {
        let mut buf: [u8; 11] = [0; 11];

        match stdin.read(&mut buf) {
            Err(ref e) if e.kind() == ErrorKind::Interrupted => continue,
            Err(_) => panic!("Failed to read stdio!"),
            Ok(0) => break,
            Ok(read_size) => {
                let m = read_size * 8;
                let words_count = m / 11 + if m % 11 > 0 { 1 } else { 0 };

                for _ in 0..words_count {
                    let i = (u16::from(buf[0]) << 3) | ((u16::from(buf[1]) & 0b11100000) >> 5);
                    words.push(DICTIONARY[i as usize]);
                    shift_11(&mut buf);
                }
            }
        }
    }

    print!("{}", words.join("-"));
}

/// Shift the buffer left for 11 bits.
fn shift_11(buf: &mut [u8]) {
    let len = buf.len();

    for i in 0..(len - 2) {
        buf[i] = ((buf[i + 1] & 0b11111) << 3) | ((buf[i + 2] & 0b11100000) >> 5)
    }

    buf[len - 2] = (buf[len - 1] & 0b11111) << 3;
    buf[len - 1] = 0;
}

#[cfg(test)]
mod tests {
    use crate::shift_11;

    #[test]
    fn shift_11_works_correctly() {
        let mut source = [
            0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x30, 0x31,
        ];

        let expected = [
            0x91, 0x99, 0xa1, 0xa9, 0xb1, 0xb9, 0xc1, 0xc9, 0x81, 0x88, 0x0,
        ];

        shift_11(&mut source);
        assert_eq!(source, expected);
    }
}
