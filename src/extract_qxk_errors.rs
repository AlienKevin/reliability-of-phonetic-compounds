use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

pub fn extract_qxk_errors() {
	let error_message_start = "Error [";
	// File hosts must exist in current path before this produces output
	if let Ok(lines) = read_lines("data/qxk_char_info.json") {
		for line in lines {
			if let Ok(line) = line {
				if line.starts_with(error_message_start) {
					print!(
						"{}, ",
						line.chars().skip(error_message_start.len()).next().unwrap()
					);
				}
			}
		}
	} else {
		println!("Error reading file");
	}
}

// The output is wrapped in a Result to allow matching on errors
// Returns an Iterator to the Reader of the lines of the file.
fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
	P: AsRef<Path>,
{
	let file = File::open(filename)?;
	Ok(io::BufReader::new(file).lines())
}
