//! This script generates `dictionary.rs` file that provides
//! DICTIONARY and DECODE_DICTIONARY static variables with the
//! dictionary used to encode byte sequences and decode
//! word-sequences. DICTIONARY is a slice of static strings (index -> word)
//! and DECODE_DICTIONARY is a `Map<&'static str, u16>` (word -> index).

use std::error::Error;
use std::fmt::Write;
use std::path::Path;
use std::{env, fs};

fn rerun(files: &[&str]) {
    for f in files {
        println!("cargo:rerun-if-changed={}", f);
    }
}

#[rustfmt::skip]
fn main() -> Result<(), Box<dyn Error>> {
    const DELIMITERS: &'static [char] = &['\n', ' '];

    let dict = fs::read_to_string("dict.txt")?;
    let dict: Vec<&str> = dict.split(DELIMITERS).filter(|a| !a.is_empty()).collect();
    let mut dict_data = String::new();

    dict_data.push_str(r#"
    use phf::{phf_map, Map};
    pub const DICTIONARY: &'static [&str] = &[
    "#);

    for s in dict.iter() {
        write!(dict_data, r#""{}","#, s)?;
    }

    dict_data.push_str(r#"
    ];
    pub const DECODE_DICTIONARY: Map<&'static str, u16> = phf_map! {
    "#);

    for (i, s) in dict.iter().enumerate() {
        write!(dict_data, r#""{}" => {},"#, s, i)?;
    }

    dict_data.push_str(r#"};"#);

    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("dict.rs");
    fs::write(dest_path, dict_data)?;

    rerun(&["build.rs", "dict.txt"]);

    Ok(())
}
