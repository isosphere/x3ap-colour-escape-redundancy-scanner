use std::io::prelude::*;
use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct CLIOptions {
    #[arg(short, long, value_name = "FILE")]
    save_file: PathBuf,

    #[arg(short, long, help = "Number of escape pairs to scan for", default_value = "3")]
    escape_pairs: usize,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Scan the specified save file for redundant colour escapes")]
    Scan,
}

#[derive(Debug)]
struct RedundantlyEscapedText {
    position: usize, // i.e.: starts here
    escapes: usize, // i.e.: this many escape pairs total
    text: String, // i.e.: this text was escaped
}

fn main() {
    let opts = CLIOptions::parse();
    let save_file = opts.save_file.as_path();
    let escape_pairs = opts.escape_pairs;
    
    let compressed_data = match std::fs::read(save_file) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Error reading file: {}", e);
            std::process::exit(1);
        }
    };

    println!("File size: {} bytes", compressed_data.len());

    match opts.command {
        Some(Commands::Scan) => {
            let mut gz = flate2::read::GzDecoder::new(&compressed_data[..]);
            let mut uncompressed_data = Vec::new();

            let mut records = Vec::new();

            match gz.read_to_end(&mut uncompressed_data) {
                Ok(_bytes) => {
                    let mut start_position = 0;
                    let mut escape_counter = 0;
                    let mut last_was_escape = false;

                    // if true, we've passed the opening escapes and are either within the escaped string or the closing escapes after it.
                    let mut beyond_opening_escape = false;
                    let mut within_ending_escape = false;
                    let mut valid_ascii = false;

                    let mut this_text = String::new();

                    let mut byte_counter = 0;
                    for c in uncompressed_data {
                        match beyond_opening_escape {
                            // either before or within the opening escape sequence
                            false => {
                                if c == 0x1b { // this is the start of a colour escape
                                    escape_counter += 1;
                                    if escape_counter == 1 {
                                        start_position = byte_counter;
                                    }
                                    last_was_escape = true;
                                } else if last_was_escape {
                                    last_was_escape = false; // this is the end of a colour escape
                                }
                                else if escape_counter >= 1 {
                                    beyond_opening_escape = true;
                                    last_was_escape = false;

                                    // redundant code, but don't want to miss the first character
                                    if c >= 0x20 && c <= 0x7E {
                                        valid_ascii = true;
                                        this_text.push(c as char);
                                    } else {
                                        valid_ascii = false;
                                    }                                     
                                }                            
                            },
                            // beyond the opening escape sequence
                            true => {
                                if c == 0x1b { // closing escape, just ignore it
                                    last_was_escape = true;
                                    within_ending_escape = true;
                                } else if last_was_escape { // closing escaped character, reset last_was_escape and ignore
                                    last_was_escape = false; 
                                } else if !within_ending_escape {
                                    if c >= 0x20 && c <= 0x7E {
                                        valid_ascii = true;
                                        this_text.push(c as char);
                                    } else {
                                        valid_ascii = false;
                                    }                                    
                                } else { // we've passed everything, so reset everything and add the record.
                                    if escape_counter >= escape_pairs && valid_ascii {
                                        records.push(RedundantlyEscapedText {
                                            position: start_position,
                                            escapes: escape_counter,
                                            text: this_text.clone(),
                                        });
                                    }
                                    within_ending_escape = false;
                                    beyond_opening_escape = false;
                                    escape_counter = 0;                                    
                                    this_text.clear();
                                    valid_ascii = false;
                                }
                            }
                        }
                        byte_counter += 1;
                    }

                    println!("Redundant colour escapes: {:?}", records);
                }
                Err(e) => {
                    eprintln!("Error decompressing save file: {}", e);
                    std::process::exit(1);
                }
            }
        }
        None => {
            eprintln!("No command specified");
            std::process::exit(1);
        }
    }
}
