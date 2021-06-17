mod dictionary;

use dictionary::{DECODE_DICTIONARY, DICTIONARY};

use std::io::{self, BufReader, BufWriter, ErrorKind, Read, Write};
use std::mem;

use clap::Clap;

/// Encodes bytes passed to stdin into mnemonic sequence of dash-separated
/// words.
#[derive(Clap)]
#[clap(version = "0.1.0", author = "yunmikun <yunmikun2@protonmail.com>")]
struct Opts {
    /// Decode mnemonic sequence from stdin back into byte-sequence.
    #[clap(short, long)]
    decode: bool,
}

fn main() {
    let opts = Opts::parse();
    let mut stdin = BufReader::new(io::stdin());
    let mut stdout = BufWriter::new(io::stdout());

    if opts.decode {
        let mut input = String::new();
        stdin.read_to_string(&mut input).unwrap();
        trim_newline(&mut input);

        let buf = decode(input.split('-'));
        stdout.write(buf.as_slice())
            .expect("Failed to write to stdout");
    } else {
        encode(&mut stdin, &mut stdout);
    }
}

fn trim_newline(s: &mut String) {
    if s.ends_with('\n') {
        s.pop();
        if s.ends_with('\r') {
            s.pop();
        }
    }
}

// ==============================================================

fn decode<I, S>(input: I) -> Vec<u8>
    where
        S: AsRef<str>,
        I: Iterator<Item=S>,
{
    const EVEN_MARK: &str = "of";

    let mut even_mark = None;
    let mut word_indices = Vec::new();

    for (i, w) in input.enumerate() {
        let x = match w.as_ref() {
            evm if evm == EVEN_MARK => {
                even_mark = Some(i);
                continue;
            }
            word => match DECODE_DICTIONARY.get(word) {
                Some(index) => *index,
                None => panic!("Unknown word: {}", word),
            }
        };
        word_indices.push(x);
    }

    let words_count = word_indices.len();
    let bit_size = words_count * 11;
    let bit_rem = bit_size % 8;
    let buf_size = bit_size / 8 + if bit_rem > 0 { 1 } else { 0 };

    let mut data_size = buf_size;
    let mut buf = vec![0_u8; buf_size];

    for (n, x) in word_indices.iter().enumerate() {
        write_with_shift_11(&mut buf, *x, n);
    }

    match even_mark {
        Some(i) => {
            if i != words_count - 1 {
                panic!("Incorrect position of even mark");
            }
        }
        None => {
            if bit_rem != 0 {
                data_size -= 1;
            }
            if buf[buf_size - 1] & 0x80 > 0 {
                data_size -= 1;
            }
        }
    }

    buf.truncate(data_size);
    buf
}

fn write_with_shift_11(buf: &mut [u8], value: u16, index: usize) {
    let bit_length = index * 11;
    let byte_position = (bit_length / 8) as usize;
    let bit_shift = bit_length % 8;
    const VALUE_SHIFT: usize = 11 - 8;

    let applied_first_mask = (0xFF << (8 - bit_shift)) as u8;
    let applied_first = (value >> (bit_shift + VALUE_SHIFT)) as u8;
    buf[byte_position] &= applied_first_mask;
    buf[byte_position] |= applied_first;

    if bit_shift < 5 {
        let applied_second_mask = 0xFF >> (VALUE_SHIFT + bit_shift);
        let applied_second = (value << (8 - VALUE_SHIFT - bit_shift)) as u8;

        buf[byte_position + 1] &= applied_second_mask;
        buf[byte_position + 1] |= applied_second;
    } else if bit_shift == 5 {
        let applied_second = (0xFF & value) as u8;
        buf[byte_position + 1] = applied_second;
    } else {
        let applied_second = (value >> (bit_shift - 5)) as u8;
        let applied_third_mask = 0xFF >> (bit_shift - 5);
        let applied_third = (value << (16 - VALUE_SHIFT - bit_shift)) as u8;

        buf[byte_position + 1] = applied_second;
        buf[byte_position + 2] &= applied_third_mask;
        buf[byte_position + 2] |= applied_third;
    }
}

fn encode<R, W>(stdin: &mut BufReader<R>, stdout: &mut BufWriter<W>)
    where
        R: Read,
        W: Write,
{
    const BUF_SIZE: usize = 11;

    let mut words: Vec<&str> = vec![];

    let (mut buf_a, mut buf_b) = ([0_u8; BUF_SIZE], [0_u8; BUF_SIZE]);
    let mut read_buf = &mut buf_a;
    let mut work_buf = &mut buf_b;

    let mut work_rs: usize = 0;
    let mut bytes_read: usize = 0;

    loop {
        let read_size = match stdin.read(read_buf) {
            Err(ref e) if e.kind() == ErrorKind::Interrupted => continue,
            Err(e) => panic!("Failed to read stdio: {}", e),
            Ok(sz) => sz,
        };

        if work_rs > 0 {
            let bit_size = work_rs * 8;
            let bit_shift = bit_size % 11;
            let words_count = bit_size / 11 + if bit_shift > 0 { 1 } else { 0 };

            if read_size == 0 && bit_shift != 0 &&
                (words.len() + words_count) * 11 / 8 > bytes_read {
                // /\ words count + pending words
                // \/ set subtract bit

                work_buf[work_rs + if words_count % 8 == 0 {0} else {1}] |= 0x80;
            }

            for _ in 0..words_count {
                let word_index =
                    (u16::from(work_buf[0]) << 3) | ((u16::from(work_buf[1]) & 0b11100000) >> 5);

                words.push(DICTIONARY[word_index as usize]);
                shift_11(work_buf);
            }
        }

        if read_size == 0 {
            break;
        }

        bytes_read += read_size;
        work_rs = read_size;
        mem::swap(&mut read_buf, &mut work_buf);
    }

    if bytes_read % 11 == 0 {
        words.insert(words.len() - 1, "of"); // push even word
    }

    stdout
        .write(words.join("-").as_bytes())
        .expect("Failed to write to stdout");
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
    use crate::{decode, encode, shift_11, write_with_shift_11};
    use rand::{Rng, RngCore};
    use std::io::{BufReader, BufWriter};

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

    #[test]
    fn write_with_shift_11_works_correctly() {
        let mut source = [0b11111111, 0b10100000, 0b00000000];
        let value = 0b11111111111;
        let expected = [0b11111111, 0b10111111, 0b11111100];

        write_with_shift_11(&mut source, value, 1);
        assert_eq!(source, expected);
    }

    #[test]
    fn write_with_shift_11_works_when_bit_shift_is_greater_than_5() {
        // On 5th word the bit shift is 7; on 2th it's 6.

        let mut source: [u8; 9] = [0; 9];

        let value = 0b11111111111;
        let expected = [0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x1, 0xFF, 0b11000000];

        write_with_shift_11(&mut source, value, 5);
        assert_eq!(source, expected);
    }

    fn impl_encode_decode_soft(original: &[u8]) -> bool {
        let mut mediator: Vec<u8> = Vec::new();
        let result;
        {
            let mut stdin1 = BufReader::new(&original[..]);
            let mut stdout1 = BufWriter::new(&mut mediator);
            encode(&mut stdin1, &mut stdout1);
        }
        {
            let input = String::from_utf8(mediator).unwrap();
            result = decode(input.split('-'));
        }
        if original == result.as_slice() {
            true
        } else {
            println!("   {:x?} ({})\n!= {:x?}\n",
                     original, original.len(), result.as_slice());
            false
        }
    }

    fn impl_encode_decode(original: &[u8]) {
        assert!(impl_encode_decode_soft(original), "len {}", original.len());
    }

    #[test]
    fn naive_encoding_with_decoding_produces_original_sequence_of_bytes() {
        let original = rand::thread_rng().gen::<[u8; 11]>();
        impl_encode_decode(&original[..]);
    }

    #[test]
    fn decoding_produces_original_sequence_for_small_inputs() {
        let data =
            [0xaa, 0xab, 0xac, 0xad, 0xaf,
                0xba, 0xbb, 0xbc, 0xbd, 0xbf];

        let mut failures = 0;
        for i in 1..=10 {
            if !impl_encode_decode_soft(&data[..i]) {
                failures += 1;
            }
        }
        assert_eq!(failures, 0);
    }

    #[test]
    fn decoding_produces_original_sequence_for_block_even_inputs() {
        const MAX_MUL: usize = 10;
        let mut data = vec![0; 11 * MAX_MUL];
        rand::thread_rng().fill_bytes(&mut data);

        let mut failures = 0;
        for m in 1..=MAX_MUL {
            if !impl_encode_decode_soft(&data[..11 * m]) {
                failures += 1;
            }
        }
        assert_eq!(failures, 0);
    }

    #[test]
    fn decoding_produces_original_sequence_for_block_odd_inputs() {
        const START: usize = 12;
        const END: usize = START * 3;
        let mut data = vec![0; END];
        rand::thread_rng().fill_bytes(&mut data);

        let mut failures = 0;
        for i in START..=END {
            if !impl_encode_decode_soft(&data[..i]) {
                failures += 1;
            }
        }
        assert_eq!(failures, 0);
    }

    #[test]
    fn decoding_produces_original_sequence_for_random_input() {
        const MAX_DATA_LEN: usize = 128;
        const TEST_PASSES: u32 = 25;

        for pass in 0..TEST_PASSES {
            let data_len = rand::thread_rng().gen_range(1, MAX_DATA_LEN);
            let mut original = vec![0; data_len];
            rand::thread_rng().fill_bytes(original.as_mut_slice());

            println!("pass {}: {:x?}", pass, original);
            impl_encode_decode(original.as_slice());
        }
    }
}
