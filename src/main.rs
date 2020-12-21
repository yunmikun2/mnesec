mod dictionary;

use dictionary::{DECODE_DICTIONARY, DICTIONARY};

use std::io;
use std::io::{BufReader, BufWriter, ErrorKind, Read, Write};

use clap::Clap;
use regex::Regex;

#[macro_use]
extern crate lazy_static;

lazy_static! {
    static ref IS_PADDED_RE: Regex = Regex::new(r".*\-of\-.*").unwrap();
    static ref ENCODED_RE: Regex = Regex::new(r"\-of\-.*").unwrap();
    static ref PADDING_RE: Regex = Regex::new(r".*\-of\-").unwrap();
}

/// Encodes bytes passed to stdin into mnemonic sequence of dash-separated
/// words.
#[derive(Clap)]
#[clap(version = "0.1.0", author = "yunmikun <yunmikun2@protonmail.com>")]
struct Opts {
    /// Decode mnemonic sequence from stdin back into byte-sequence.
    #[clap(short, long)]
    decode: bool,

    /// Show all padding words that are used.
    #[clap(long)]
    show_padding_words: bool,
}

fn main() {
    let opts = Opts::parse();
    let mut stdin = BufReader::new(io::stdin());
    let mut stdout = BufWriter::new(io::stdout());

    if opts.show_padding_words {
        show_padding_words();
        return;
    }

    if opts.decode {
        decode(&mut stdin, &mut stdout);
    } else {
        encode(&mut stdin, &mut stdout);
    }
}

fn show_padding_words() {
    let padding_words: Vec<String> = (0..11)
        .map(|i| String::from(DICTIONARY[i * 186 + 1]))
        .collect();

    let output = padding_words.join("\n");
    println!("{}", output);
}

fn decode<R, W>(mut stdin: &mut BufReader<R>, stdout: &mut BufWriter<W>)
where
    R: Read,
    W: Write,
{
    let input = read_string_from_reader(&mut stdin);
    let is_padded = IS_PADDED_RE.is_match(&input);
    let encoded_string = ENCODED_RE.replace_all(&input, "");

    let word_indices: Vec<u16> = words_to_indices(&encoded_string);

    let mut buf: Vec<u8> = {
        let buf_size = bytes_encoded_for_words(word_indices.len());
        vec![0; buf_size]
    };

    for (n, x) in word_indices.iter().enumerate() {
        write_with_shift_11(&mut buf, x, &n);
    }

    let final_buf = if is_padded {
        buf.split_last().unwrap().1
    } else {
        buf.as_slice()
    };

    stdout.write(&final_buf).expect("Failed to write to stdout");
}

fn read_string_from_reader<R>(reader: &mut R) -> String
where
    R: Read,
{
    let mut buf: String = String::new();

    loop {
        match reader.read_to_string(&mut buf) {
            Err(ref e) if e.kind() == ErrorKind::Interrupted => continue,
            Err(e) => panic!("Failed to read stdio: {}", e),
            Ok(_) => break,
        }
    }

    trim_newline(&mut buf);
    buf
}

fn trim_newline(s: &mut String) {
    if s.ends_with('\n') {
        s.pop();
        if s.ends_with('\r') {
            s.pop();
        }
    }
}

fn words_to_indices(string: &str) -> Vec<u16> {
    string
        .split("-")
        .map(|word| match DECODE_DICTIONARY.get(word) {
            Some(index) => index.clone(),
            None => panic!("Unknown word: {}", word),
        })
        .collect()
}

// TODO!: Remove. Left it here 'coz don't know if we are gonna need
// it.
#[allow(dead_code)]
fn bit_shift_for_word(word: &str, is_padded: bool) -> usize {
    if is_padded {
        match DECODE_DICTIONARY.get(word) {
            Some(index) => ((index - 1) / 186) as usize,
            None => panic!("Unknown padding word: {}", word),
        }
    } else {
        0
    }
}

fn bytes_encoded_for_words(n: usize) -> usize {
    let bits_used = n * 11;
    let bits_left = bits_used % 8;
    bits_used / 8 + if bits_left > 0 { 1 } else { 0 }
}

fn write_with_shift_11(buf: &mut [u8], value: &u16, index: &usize) {
    let bit_length = index * 11;
    let byte_position = bit_length / 8;
    let bit_shift = bit_length % 8;
    const VALUE_SHIFT: usize = 11 - 8;

    let applied_first_mask = (0xFF << (8 - bit_shift)) as u8;
    let applied_first = (value >> (bit_shift + VALUE_SHIFT)) as u8;
    buf[byte_position as usize] &= applied_first_mask;
    buf[byte_position as usize] |= applied_first;

    if bit_shift < 5 {
        let applied_second_mask = 0xFF >> (VALUE_SHIFT + bit_shift);
        let applied_second = (value << (8 - VALUE_SHIFT - bit_shift)) as u8;

        buf[byte_position + 1 as usize] &= applied_second_mask;
        buf[byte_position + 1 as usize] |= applied_second;
    } else if bit_shift == 5 {
        let applied_second = (0xFF & value) as u8;
        buf[byte_position + 1 as usize] = applied_second;
    } else {
        let applied_second = (value >> (bit_shift - 5)) as u8;
        let applied_third_mask = 0xFF >> (bit_shift - 5);
        let applied_third = (value << (16 - VALUE_SHIFT - bit_shift)) as u8;

        buf[byte_position + 1 as usize] = applied_second;
        buf[byte_position + 2 as usize] &= applied_third_mask;
        buf[byte_position + 2 as usize] |= applied_third;
    }
}

fn encode<R, W>(stdin: &mut BufReader<R>, stdout: &mut BufWriter<W>)
where
    R: Read,
    W: Write,
{
    let mut words: Vec<&str> = vec![];
    let mut buf: [u8; 11] = [0; 11];
    let mut final_shift: usize = 0;

    loop {
        let read_size = match stdin.read(&mut buf) {
            Err(ref e) if e.kind() == ErrorKind::Interrupted => continue,
            Err(e) => panic!("Failed to read stdio: {}", e),
            Ok(0) => break,
            Ok(sz) => sz,
        };

        let words_count = {
            let bit_size = read_size * 8;
            let bit_shift = bit_size % 11;

            if bit_shift > 0 {
                final_shift = bit_shift;
            }

            bit_size / 11 + if bit_shift > 0 { 1 } else { 0 }
        };

        for _ in 0..words_count {
            let i = (u16::from(buf[0]) << 3) | ((u16::from(buf[1]) & 0b11100000) >> 5);
            words.push(DICTIONARY[i as usize]);
            shift_11(&mut buf);
        }
    }

    if final_shift > 0 {
        let word_index = final_shift * 186 + 1;
        words.push("of");
        words.push(DICTIONARY[word_index as usize]);
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

        write_with_shift_11(&mut source, &value, &1);
        assert_eq!(source, expected);
    }

    #[test]
    fn write_with_shift_11_works_when_bit_shift_is_greater_than_5() {
        // On 5th word the bit shift is 7; on 2th it's 6.

        let mut source: [u8; 9] = [0; 9];

        let value = 0b11111111111;
        let expected = [0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x1, 0xFF, 0b11000000];

        write_with_shift_11(&mut source, &value, &5);
        assert_eq!(source, expected);
    }

    #[test]
    fn naive_encoding_with_decoding_produces_original_sequence_of_bytes() {
        let original = rand::thread_rng().gen::<[u8; 11]>();
        let mut mediator: Vec<u8> = Vec::new();
        let mut result: Vec<u8> = Vec::new();
        {
            let mut stdin1 = BufReader::new(&original[..]);
            let mut stdout1 = BufWriter::new(&mut mediator);
            encode(&mut stdin1, &mut stdout1);
        }
        {
            let mut stdin2 = BufReader::new(&mediator[..]);
            let mut stdout2 = BufWriter::new(&mut result);
            decode(&mut stdin2, &mut stdout2);
        }
        assert_eq!(&original, &result[..]);
    }

    #[test]
    fn any_encoding_with_decoding_produces_original_sequence_of_bytes() {
        const MAX_DATA_LEN: usize = 128;
        const TEST_PASSES: u32 = 25;

        for pass in 0..TEST_PASSES {
            let data_len = rand::thread_rng().gen_range(0, MAX_DATA_LEN);
            let mut original = vec![0; data_len];

            rand::thread_rng().fill_bytes(original.as_mut_slice());
            let mut mediator: Vec<u8> = Vec::new();
            let mut result: Vec<u8> = Vec::new();
            {
                let mut stdin1 = BufReader::new(&original[..]);
                let mut stdout1 = BufWriter::new(&mut mediator);
                encode(&mut stdin1, &mut stdout1);
            }
            {
                let mut stdin2 = BufReader::new(&mediator[..]);
                let mut stdout2 = BufWriter::new(&mut result);
                decode(&mut stdin2, &mut stdout2);
            }
            assert_eq!(&original, &result[..],
                       "pass {}, len {}", pass, data_len);
        }
    }
}
