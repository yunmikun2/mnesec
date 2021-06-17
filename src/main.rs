use mnesec::{decode, encode};
use std::io::{self, BufReader, BufWriter, Write, BufRead};
use clap::Clap;

/// Encodes bytes passed to stdin into mnemonic sequence of dash-separated
/// words.
#[derive(Clap)]
#[clap(version = "0.1.0", author = "yunmikun <yunmikun2@protonmail.com>")]
struct Opts {
    /// Decode mnemonic sequence from stdin back into byte-sequence.
    #[clap(short, long)]
    decode: bool,
    /// Don't print trailing newline
    #[clap(short, long)]
    no_newline: bool,
}

fn main() {
    let opts: Opts = Opts::parse();
    let mut stdin = BufReader::new(io::stdin());
    let mut stdout = BufWriter::new(io::stdout());

    if opts.decode {
        let input = stdin
            .split(b'-')
            .map(|b| {
                String::from_utf8(b.expect("Error reading stdin"))
                    .expect("Error decoding UTF-8")
            });
        let buf = decode(input);
        stdout.write(buf.as_slice())
            .expect("Failed to write to stdout");
    } else {
        let words = encode(&mut stdin);

        for (i, word) in words.iter().enumerate() {
            match i {
                0 => stdout.write_all(word.as_bytes()),
                _ => stdout.write_all(&[b'-'])
                        .and_then(|_| stdout.write_all(word.as_bytes())),
            }
                .expect("Failed to write to stdout");
        }

        if !opts.no_newline {
            println!();
        }
    }
}
