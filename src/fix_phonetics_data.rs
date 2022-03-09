use super::calculate_phonetics_stats::{get_char_to_phonetic, get_script_name, Script};
use super::scrape_phonetics_info::{CharInfo, Component};
use csv::{Error, WriterBuilder};
use indexmap::IndexMap;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;

pub fn fix_phonetics_data() {
	let mut simp_phonetics = get_char_to_phonetic(&Script::Simplified).unwrap();
	let mut trad_phonetics = get_char_to_phonetic(&Script::Traditional).unwrap();
	let diff_phonetics = get_phonetics_diff().unwrap();
	simp_phonetics.extend(diff_phonetics.iter());
	trad_phonetics.extend(diff_phonetics.iter());
	write_phonetics_data(simp_phonetics, &Script::Simplified).unwrap();
	write_phonetics_data(trad_phonetics, &Script::Traditional).unwrap();
}

pub fn detect_mismatched_categorization() {
	let char_info_file = File::open("data/qxk_char_info.json").unwrap();
	let char_info: HashMap<char, CharInfo> = serde_json::from_reader(char_info_file).unwrap();
	let mut simp_phonetics = get_char_to_phonetic(&Script::Simplified).unwrap();
	let mut trad_phonetics = get_char_to_phonetic(&Script::Traditional).unwrap();
	let mut total_simp_wrong = 0;
	let mut simp_false_positive = 0;
	simp_phonetics.iter().for_each(|(c, phonetic)| {
		// pc stands for phonetic compound
		let simp_is_pc = match phonetic {
			Some(_) => true,
			None => false,
		};
		let qxk_is_pc = char_info.get(c).unwrap().structure_mode == "义音合成";
		if simp_is_pc != qxk_is_pc {
			total_simp_wrong += 1;
			print!("{},", c);
			if simp_is_pc && !qxk_is_pc {
				simp_false_positive += 1;
			}
		}
	});
	println!("\nTotal simplified wrong = {total_simp_wrong}");
	println!("Simplified false positive = {simp_false_positive}");
	println!(
		"Simplified false negative = {}",
		total_simp_wrong - simp_false_positive
	);

	let mut total_trad_wrong = 0;
	let mut trad_false_positive = 0;
	let mut trad_false_positive_file =
		File::create("data/phonetics_trad_false_positive.txt").unwrap();
	let mut trad_false_negative_file =
		File::create("data/phonetics_trad_false_negative.txt").unwrap();
	trad_phonetics.iter().for_each(|(c, phonetic)| {
		// pc stands for phonetic compound
		let trad_is_pc = match phonetic {
			Some(_) => true,
			None => false,
		};
		if let Some(char_info) = char_info.get(c) {
			let qxk_is_pc = char_info.structure_mode == "义音合成"
				&& !char_info.components.iter().any(|component| {
					if let Component::Combination(_) = component {
						true
					} else {
						false
					}
				});
			if trad_is_pc != qxk_is_pc {
				total_trad_wrong += 1;
				if trad_is_pc && !qxk_is_pc {
					trad_false_positive += 1;
					write!(trad_false_positive_file, "{}\u{20}", c);
				} else {
					write!(trad_false_negative_file, "{}\u{20}", c);
				}
			}
		}
	});
	println!("\n# of traditional wrong = {total_trad_wrong}");
	println!("Traditional false positive = {trad_false_positive}");
	println!(
		"Traditional false negative = {}",
		total_trad_wrong - trad_false_positive
	);
}

fn write_phonetics_data(data: IndexMap<char, Option<char>>, script: &Script) -> Result<(), Error> {
	let file_name = &format!("data/phonetics_{}.tsv", get_script_name(script));
	let mut writer = WriterBuilder::new().delimiter(b'\t').from_path(file_name)?;
	data.iter().for_each(|(char, phonetic)| {
		writer
			.write_record(&[
				char.to_string(),
				phonetic.map(|p| p.to_string()).unwrap_or(String::new()),
			])
			.unwrap();
	});
	Ok(())
}

fn get_phonetics_diff() -> Result<IndexMap<char, Option<char>>, Error> {
	let file_name = &format!("data/phonetics_diff.tsv");
	let mut reader = csv::ReaderBuilder::new()
		.flexible(true)
		.delimiter(b'\t')
		.from_path(file_name)?;
	let mut char_to_phonetic = IndexMap::new();
	for result in reader.records() {
		let record = result.expect("a CSV record");
		let character = record[0].chars().next().unwrap();
		let phonetic = if &record[1] == "Y" {
			// separately defined phonetic component
			if record.len() == 11 {
				Some(record[10].chars().next().unwrap())
			}
			// defined in 北京师范声旁
			else if &record[3] == "Y" {
				Some(record[2].chars().next().unwrap())
			}
			// defined in 中央大学声旁
			else {
				Some(record[4].chars().next().unwrap())
			}
		} else {
			None
		};
		char_to_phonetic.insert(character, phonetic);
	}
	Ok(char_to_phonetic)
}
