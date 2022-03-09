use super::calculate_phonetics_stats::{get_common_chars, Script};
use itertools::Itertools;
use std::fs::File;
use std::io::Write;

pub fn generate_merged_common_chars() {
	let output_str = get_common_chars(&Script::Traditional)
		.union(&get_common_chars(&Script::Simplified))
		.join("\n");
	let mut output = File::create(format!("data/merged_common_characters.txt")).unwrap();
	writeln!(output, "{}", output_str);
}
