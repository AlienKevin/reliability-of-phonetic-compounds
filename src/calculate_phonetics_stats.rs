use csv::Error;
use indexmap::{IndexMap, IndexSet};
use itertools;
use serde::{Deserialize, Serialize};
use std::cmp;
use std::fmt;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;

#[derive(PartialEq, Eq, Hash, Clone)]
pub struct Pr {
	pub syllable: String,
	pub tone: char,
}

impl fmt::Display for Pr {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{}{}", self.syllable, self.tone)
	}
}

pub type Prs = IndexSet<Pr>;

pub type PrMatch = (SyllableMatch, PolyphoneLevel);

#[derive(PartialEq)]
pub enum SyllableMatch {
	SameTone,
	DiffTone,
	DiffSyllable,
}

#[derive(PartialEq)]
pub enum PolyphoneLevel {
	One,
	Two,
	Three,
}

pub struct CharInfo<'a> {
	pub character: char,
	pub char_prs: &'a Prs,
	pub phonetic: char,
	pub phonetic_prs: &'a Prs,
}

impl<'a> fmt::Display for CharInfo<'a> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(
			f,
			"{}\t{}\t{}\t{}",
			self.character,
			itertools::join(self.char_prs, ","),
			self.phonetic,
			itertools::join(self.phonetic_prs, ",")
		)
	}
}

// order of enum fields are meaningful
// used by cmp::max() where Consistent
// is the smaller compared to Semiconsistent
#[derive(PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum PhoneticConsistency {
	Consistent,
	Semiconsistent,
	Inconsistent,
}

pub struct ClassInfo<'a> {
	// consistent: phonetic consistently represent the same syllable and tone
	pub a1c: Vec<CharInfo<'a>>,
	// semi-consistent: phonetic consistently represent the same syllable but may have different tones
	pub a1s: Vec<CharInfo<'a>>,
	// inconsistent: phonetic sometimes represent different syllables
	pub a1i: Vec<CharInfo<'a>>,
	pub a2: Vec<CharInfo<'a>>,
	pub a3: Vec<CharInfo<'a>>,
	pub b1: Vec<CharInfo<'a>>,
	pub b2: Vec<CharInfo<'a>>,
	pub b3: Vec<CharInfo<'a>>,
}

impl<'a> ClassInfo<'a> {
	pub fn new() -> ClassInfo<'a> {
		let a1c = Vec::new();
		let a1s = Vec::new();
		let a1i = Vec::new();
		let a2 = Vec::new();
		let a3 = Vec::new();
		let b1 = Vec::new();
		let b2 = Vec::new();
		let b3 = Vec::new();
		ClassInfo {
			a1c,
			a1s,
			a1i,
			a2,
			a3,
			b1,
			b2,
			b3,
		}
	}
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Script {
	Simplified,
	Traditional,
}

impl Default for Script {
	fn default() -> Self {
		Script::Simplified
	}
}

pub fn get_script_name(script: &Script) -> &'static str {
	match script {
		Script::Simplified => "simplified",
		Script::Traditional => "traditional",
	}
}

pub fn calculate_phonetics_stats() {
	calculate_phonetics_stats_helper(Script::Simplified);
	calculate_phonetics_stats_helper(Script::Traditional);
}

pub fn get_common_chars(script: &Script) -> IndexSet<char> {
	let common_simplified_chars: IndexSet<char> =
		fs::read_to_string("data/3500_common_simplified_characters.txt")
			.unwrap()
			.chars()
			.filter(|c| !c.is_whitespace())
			.collect();
	match script {
		Script::Simplified => common_simplified_chars,
		Script::Traditional => {
			let simp_to_trad = get_script_conversion_from(Script::Simplified);
			let mut common_traditional_chars = IndexSet::new();
			common_simplified_chars.iter().for_each(|simp| {
				if simp_to_trad.contains_key(simp) {
					common_traditional_chars = common_traditional_chars
						.union(simp_to_trad.get(simp).unwrap())
						.map(|c| *c)
						.collect();
				} else {
					common_traditional_chars.insert(*simp);
				}
			});
			common_traditional_chars
		}
	}
}

fn calculate_phonetics_stats_helper(script: Script) {
	let common_chars = get_common_chars(&script);
	let char_to_phonetic = get_char_to_phonetic(&script).unwrap();
	let result_dir = format!("results/{}", get_script_name(&script));
	fs::create_dir_all(&result_dir).unwrap();
	let mut output = File::create(format!("{result_dir}/overview_stats.txt")).unwrap();
	let num_of_all_chars = common_chars.len() as u64;
	let mut num_of_phonetic_compounds = 0;
	let mut num_of_uncommon_phonetics = 0;
	let mut common_phonetic_components: IndexSet<char> = IndexSet::new();
	let mut uncommon_phonetic_components: IndexSet<char> = IndexSet::new();

	char_to_phonetic.iter().for_each(|(char, phonetic)| {
		if common_chars.contains(char) {
			match phonetic {
				Some(phonetic) => {
					num_of_phonetic_compounds += 1;
					if common_chars.contains(phonetic) {
						common_phonetic_components.insert(*phonetic);
					} else {
						num_of_uncommon_phonetics += 1;
						uncommon_phonetic_components.insert(*phonetic);
					}
				}
				None => {}
			}
		}
	});

	writeln!(output, "# of all common characters: {}", num_of_all_chars);
	writeln!(
		output,
		"# of phonetic compounds: {}\
        \n\t- {}% of all characters",
		num_of_phonetic_compounds,
		percent(num_of_phonetic_compounds, num_of_all_chars)
	);
	let num_of_common_phonetic_components = common_phonetic_components.len() as u64;
	let num_of_uncommon_phonetic_components = uncommon_phonetic_components.len() as u64;
	let num_of_unique_phonetic_components =
		num_of_common_phonetic_components + num_of_uncommon_phonetic_components;
	writeln!(
		output,
		"# of unique phonetic components: {}\
		\n\t- {}% are common phonetics ({})\
		\n\t- {}% are uncommon phonetics ({})",
		num_of_unique_phonetic_components,
		percent(
			num_of_common_phonetic_components,
			num_of_unique_phonetic_components
		),
		num_of_common_phonetic_components,
		percent(
			num_of_uncommon_phonetic_components,
			num_of_unique_phonetic_components
		),
		num_of_uncommon_phonetic_components,
	);
	writeln!(
		File::create(format!("{result_dir}/phonetic_classes.txt")).unwrap(),
		"Common phonetic components ({} characters):\n{}\
		\nUncommon phonetic components ({} characters):\n{}",
		num_of_common_phonetic_components,
		itertools::join(common_phonetic_components.iter(), " "),
		num_of_uncommon_phonetic_components,
		itertools::join(uncommon_phonetic_components.iter(), " ")
	);
	writeln!(
		output,
		"# of phonetic compounds with uncommon phonetics: {}\
        \n\t- {}% all phonetic compounds\
        \n\t- {}% of all characters",
		num_of_uncommon_phonetics,
		percent(num_of_uncommon_phonetics, num_of_phonetic_compounds),
		percent(num_of_uncommon_phonetics, num_of_all_chars)
	);

	let char_to_pr = get_char_to_pinyin();
	let class_info = calculate_stats(
		format!("{result_dir}/pinyin_stats.txt"),
		&common_chars,
		&char_to_phonetic,
		&char_to_pr,
	);
	output_class_info(format!("{result_dir}/pinyin_classes.txt"), class_info);

	let char_to_pr = get_char_to_jyutping_lshk();
	let class_info = calculate_stats(
		format!("{result_dir}/jyutping_lshk_stats.txt"),
		&common_chars,
		&char_to_phonetic,
		&char_to_pr,
	);
	output_class_info(
		format!("{result_dir}/jyutping_lshk_classes.txt"),
		class_info,
	);

	let char_to_pr: IndexMap<char, Prs> = get_char_to_jyutping_rime(&char_to_pr);
	let class_info = calculate_stats(
		format!("{result_dir}/jyutping_rime_stats.txt"),
		&common_chars,
		&char_to_phonetic,
		&char_to_pr,
	);
	output_class_info(
		format!("{result_dir}/jyutping_rime_classes.txt"),
		class_info,
	);
}

pub fn get_char_to_phonetic(script: &Script) -> Result<IndexMap<char, Option<char>>, Error> {
	let file_name = &format!("data/phonetics_{}.tsv", get_script_name(script));
	let mut reader = csv::ReaderBuilder::new()
		.has_headers(false)
		.delimiter(b'\t')
		.from_path(file_name)?;
	let mut char_to_phonetic = IndexMap::new();
	for result in reader.records() {
		let record = result.expect("a CSV record");
		let character = record[0].chars().next().unwrap();
		let phonetic = if (&record[1]).is_empty() {
			None
		} else {
			Some(record[1].chars().next().unwrap())
		};
		char_to_phonetic.insert(character, phonetic);
	}
	Ok(char_to_phonetic)
}

fn output_class_info<'a, P: AsRef<Path>>(output_file: P, class_info: ClassInfo<'a>) {
	let mut output = File::create(output_file).unwrap();
	writeln!(
		output,
		"A1 Class ({} characters)\
        \n----------------------------------------------------\
        \nA1 Consistent Subclass ({} characters)\
        \n-----------------------------------------\
        \n{}\
        \nA1 Semi-Consistent Subclass ({} characters)\
        \n-----------------------------------------\
        \n{}\
        \nA1 Inconsistent Subclass ({} characters)\
        \n-----------------------------------------\
        \n{}\
        \n\nA2 Class ({} characters)\
        \n----------------------------------------------------\
        \n{}\
        \n\nA3 Class ({} characters)\
        \n----------------------------------------------------\
        \n{}\
        \n\nB1 Class ({} characters)\
        \n----------------------------------------------------\
        \n{}\
        \n\nB2 Class ({} characters)\
        \n----------------------------------------------------\
        \n{}\
        \n\nB3 Class ({} characters)\
        \n----------------------------------------------------\
        \n{}",
		class_info.a1c.len() + class_info.a1s.len() + class_info.a1i.len(),
		class_info.a1c.len(),
		itertools::join(class_info.a1c, "\n"),
		class_info.a1s.len(),
		itertools::join(class_info.a1s, "\n"),
		class_info.a1i.len(),
		itertools::join(class_info.a1i, "\n"),
		class_info.a2.len(),
		itertools::join(class_info.a2, "\n"),
		class_info.a3.len(),
		itertools::join(class_info.a3, "\n"),
		class_info.b1.len(),
		itertools::join(class_info.b1, "\n"),
		class_info.b2.len(),
		itertools::join(class_info.b2, "\n"),
		class_info.b3.len(),
		itertools::join(class_info.b3, "\n"),
	);
}

fn get_phonetic_consistencies(
	common_chars: &IndexSet<char>,
	char_to_phonetic: &IndexMap<char, Option<char>>,
	char_to_pr: &IndexMap<char, Prs>,
) -> IndexMap<char, PhoneticConsistency> {
	use PhoneticConsistency::*;
	let mut consistencies = IndexMap::new();
	get_phonetics(char_to_phonetic)
		.iter()
		.filter(|phonetic| common_chars.contains(*phonetic))
		.for_each(|phonetic| {
			if !char_to_pr.contains_key(phonetic) {
				println!("{phonetic}");
			}
			let phonetic_prs = char_to_pr.get(phonetic).unwrap();
			// Only account for A1 class
			if phonetic_prs.len() == 1 {
				consistencies.insert(
					*phonetic,
					char_to_pr
						.iter()
						.filter(|(character, _)| {
							char_to_phonetic.get(*character) == Some(&Some(*phonetic))
								&& common_chars.contains(*character)
						})
						.fold(Consistent, |consistency, (_, char_prs)| {
							use SyllableMatch::*;
							let (syllable_match, _) = get_best_pr_match(char_prs, phonetic_prs);
							cmp::max(
								match syllable_match {
									SameTone => Consistent,
									DiffTone => Semiconsistent,
									DiffSyllable => Inconsistent,
								},
								consistency,
							)
						}),
				);
			}
		});
	consistencies
}

pub fn get_phonetics<'a>(char_to_phonetic: &'a IndexMap<char, Option<char>>) -> IndexSet<char> {
	let mut phonetics = IndexSet::new();
	char_to_phonetic
		.values()
		.for_each(|maybe_phonetic| match maybe_phonetic {
			Some(phonetic) => {
				phonetics.insert(*phonetic);
			}
			None => {}
		});
	phonetics
}

pub fn calculate_stats<'a, P: AsRef<Path>>(
	output_file: P,
	common_chars: &'a IndexSet<char>,
	char_to_phonetic: &'a IndexMap<char, Option<char>>,
	char_to_pr: &'a IndexMap<char, Prs>,
) -> ClassInfo<'a> {
	let trad_to_simp = get_script_conversion_from(Script::Traditional);
	common_chars.iter().for_each(|char| {
		if !char_to_pr.contains_key(char) {
			let simps = trad_to_simp.get(char).unwrap();
			if simps.len() > 1 {
				println!("{char} maps to {} simplified variants", simps.len());
			}
			println!(
				"{char},{}",
				itertools::join(char_to_pr.get(&simps[0]).unwrap(), ",")
			);
		}
	});
	let consistencies = get_phonetic_consistencies(common_chars, char_to_phonetic, char_to_pr);
	let mut num_of_phonetic_compounds = 0;
	let mut num_of_prs_for_all_chars = 0;
	let mut num_of_prs_for_phonetics = 0;
	let mut num_of_polyphone_phonetics = 0;
	let mut num_of_same_tone = 0;
	let mut num_of_a1 = 0;
	let mut num_of_a2 = 0;
	let mut num_of_a3 = 0;
	let mut num_of_diff_tone = 0;
	let mut num_of_b1 = 0;
	let mut num_of_b2 = 0;
	let mut num_of_b3 = 0;
	let mut class_info = ClassInfo::new();
	char_to_phonetic.iter().for_each(|(character, phonetic)| {
		if !common_chars.contains(character) {
			return;
		}
		if !char_to_pr.contains_key(character) {
			println!("{} is not found", character);
		}
		let char_prs = char_to_pr.get(character).unwrap();
		num_of_prs_for_all_chars += char_prs.len();
		match phonetic {
			Some(phonetic) => {
				num_of_phonetic_compounds += 1;
				num_of_prs_for_phonetics += char_prs.len();
				// the phonetic is in the list of commonly used chars
				if common_chars.contains(phonetic) {
					let phonetic_prs = char_to_pr.get(phonetic).unwrap();
					let char_info = CharInfo {
						character: *character,
						char_prs,
						phonetic: *phonetic,
						phonetic_prs,
					};
					match get_best_pr_match(char_prs, phonetic_prs) {
						(SyllableMatch::SameTone, polyphone_level) => {
							num_of_same_tone += 1;
							if polyphone_level != PolyphoneLevel::One {
								num_of_polyphone_phonetics += 1;
							}
							match polyphone_level {
								PolyphoneLevel::One => {
									num_of_a1 += 1;
									use PhoneticConsistency::*;
									match consistencies.get(phonetic).unwrap() {
										Consistent => {
											class_info.a1c.push(char_info);
										}
										Semiconsistent => {
											class_info.a1s.push(char_info);
										}
										Inconsistent => {
											class_info.a1i.push(char_info);
										}
									}
								}
								PolyphoneLevel::Two => {
									num_of_a2 += 1;
									class_info.a2.push(char_info);
								}
								PolyphoneLevel::Three => {
									num_of_a3 += 1;
									class_info.a3.push(char_info);
								}
							}
						}
						(SyllableMatch::DiffTone, polyphone_level) => {
							num_of_diff_tone += 1;
							if polyphone_level != PolyphoneLevel::One {
								num_of_polyphone_phonetics += 1;
							}
							match polyphone_level {
								PolyphoneLevel::One => {
									num_of_b1 += 1;
									class_info.b1.push(char_info);
								}
								PolyphoneLevel::Two => {
									num_of_b2 += 1;
									class_info.b2.push(char_info);
								}
								PolyphoneLevel::Three => {
									num_of_b3 += 1;
									class_info.b3.push(char_info);
								}
							}
						}
						(SyllableMatch::DiffSyllable, polyphone_level) => {
							if polyphone_level != PolyphoneLevel::One {
								num_of_polyphone_phonetics += 1;
							}
						}
					}
				}
			}
			None => {}
		}
	});

	let mut output = File::create(output_file).unwrap();
	let num_of_all_chars = common_chars.len() as u64;
	let num_of_eg_chars = 5;
	writeln!(
		output,
		"Average # of pronunciations per character: {:.2}",
		(num_of_prs_for_all_chars as f64 / num_of_all_chars as f64)
	);
	writeln!(
		output,
		"Average # of pronunciations per phonetic compound: {:.2}",
		(num_of_prs_for_phonetics as f64 / num_of_phonetic_compounds as f64)
	);
	writeln!(
		output,
		"# of polyphonic phonetic compounds (compound is polyphonic\
        \nor their phonetic component is polyphonic or both): {}\
        \n\t- {}% all phonetic compounds\
        \n\t- {}% of all characters",
		num_of_polyphone_phonetics,
		percent(num_of_polyphone_phonetics, num_of_phonetic_compounds),
		percent(num_of_polyphone_phonetics, num_of_all_chars)
	);
	let num_of_a_and_b = num_of_same_tone + num_of_diff_tone;
	writeln!(
		output,
		"# of A and B class phonetics: {}\
		\n\t- {}% of all phonetic compounds\
        \n\t- {}% of all characters",
		num_of_a_and_b,
		percent(num_of_a_and_b, num_of_phonetic_compounds),
		percent(num_of_a_and_b, num_of_all_chars)
	);
	writeln!(
		output,
		"# of A class phonetics matching both syllable and tone: {}\
        \n\t- {}% of all phonetic compounds\
        \n\t- {}% of all characters",
		num_of_same_tone,
		percent(num_of_same_tone, num_of_phonetic_compounds),
		percent(num_of_same_tone, num_of_all_chars)
	);
	writeln!(
		output,
		"# of A1 phonetics: {}\
        \n\t- {}% of all A class same-tone phonetics\
        \n\t- {}% of all phonetic compounds\
        \n\t- # of A1C consistent phonetics: {}\
        \n\t\t- {}% of all A1 class\
        \n\t\t- eg: {}\
        \n\t- # of A1S semiconsistent phonetics: {}\
        \n\t\t- {}% of all A1 class\
        \n\t\t- eg: {}\
        \n\t- # of A1I inconsistent phonetics: {}\
        \n\t\t- {}% of all A1 class\
        \n\t\t- eg: {}",
		num_of_a1,
		percent(num_of_a1, num_of_same_tone),
		percent(num_of_a1, num_of_phonetic_compounds),
		class_info.a1c.len(),
		percent(class_info.a1c.len().try_into().unwrap(), num_of_a1),
		example_chars_to_string(&class_info.a1c[..num_of_eg_chars]),
		class_info.a1s.len(),
		percent(class_info.a1s.len().try_into().unwrap(), num_of_a1),
		example_chars_to_string(&class_info.a1s[..num_of_eg_chars]),
		class_info.a1i.len(),
		percent(class_info.a1i.len().try_into().unwrap(), num_of_a1),
		example_chars_to_string(&class_info.a1i[..num_of_eg_chars])
	);
	writeln!(
		output,
		"# of A2 phonetics: {}\
        \n\t- {}% of all A class same-tone phonetics\
        \n\t- {}% of all phonetic compounds\
        \n\t- eg: {}",
		num_of_a2,
		percent(num_of_a2, num_of_same_tone),
		percent(num_of_a2, num_of_phonetic_compounds),
		example_chars_to_string(&class_info.a2[..num_of_eg_chars])
	);
	writeln!(
		output,
		"# of A3 phonetics: {}\
        \n\t- {}% of all A class same-tone phonetics\
        \n\t- {}% of all phonetic compounds\
        \n\t- eg: {}",
		num_of_a3,
		percent(num_of_a3, num_of_same_tone),
		percent(num_of_a3, num_of_phonetic_compounds),
		example_chars_to_string(&class_info.a3[..num_of_eg_chars])
	);
	writeln!(
		output,
		"# of B class phonetics matching syllable but not tone: {}\
        \n\t- {}% of all phonetic compounds\
        \n\t- {}% of all characters",
		num_of_diff_tone,
		percent(num_of_diff_tone, num_of_phonetic_compounds),
		percent(num_of_diff_tone, num_of_all_chars)
	);
	writeln!(
		output,
		"# of B1 phonetics: {}\
        \n\t- {}% of all B class different-tone phonetics\
        \n\t- {}% of all phonetic compounds\
        \n\t- eg: {}",
		num_of_b1,
		percent(num_of_b1, num_of_diff_tone),
		percent(num_of_b1, num_of_phonetic_compounds),
		example_chars_to_string(&class_info.b1[..num_of_eg_chars])
	);
	writeln!(
		output,
		"# of B2 phonetics: {}\
        \n\t- {}% of all B class different-tone phonetics\
        \n\t- {}% of all phonetic compounds\
        \n\t- eg: {}",
		num_of_b2,
		percent(num_of_b2, num_of_diff_tone),
		percent(num_of_b2, num_of_phonetic_compounds),
		example_chars_to_string(&class_info.b2[..num_of_eg_chars])
	);
	writeln!(
		output,
		"# of B3 phonetics: {}\
        \n\t- {}% of all B class different-tone phonetics\
        \n\t- {}% of all phonetic compounds\
        \n\t- eg: {}",
		num_of_b3,
		percent(num_of_b3, num_of_diff_tone),
		percent(num_of_b3, num_of_phonetic_compounds),
		example_chars_to_string(&class_info.b3[..num_of_eg_chars])
	);
	class_info
}

fn example_chars_to_string(exmaple_chars: &[CharInfo]) -> String {
	itertools::join(
		exmaple_chars.iter().map(|c| {
			format!(
				"{} {} ({} {})",
				c.character,
				itertools::join(c.char_prs, ", "),
				c.phonetic,
				itertools::join(c.phonetic_prs, ", ")
			)
		}),
		"; ",
	)
}

fn get_best_pr_match(prs1: &Prs, prs2: &Prs) -> PrMatch {
	let pr1_is_polyphone = prs1.len() > 1;
	let pr2_is_polyphone = prs2.len() > 1;
	let polyphone_level = if pr1_is_polyphone && pr2_is_polyphone {
		PolyphoneLevel::Three
	} else if pr1_is_polyphone || pr2_is_polyphone {
		PolyphoneLevel::Two
	} else {
		PolyphoneLevel::One
	};
	let syllable_match = itertools::iproduct!(prs1, prs2).fold(
		SyllableMatch::DiffSyllable,
		|best_pr_match, (pr1, pr2)| {
			if pr1.syllable == pr2.syllable {
				if pr1.tone == pr2.tone {
					SyllableMatch::SameTone
				} else if best_pr_match == SyllableMatch::DiffSyllable {
					SyllableMatch::DiffTone
				} else {
					best_pr_match
				}
			} else {
				best_pr_match
			}
		},
	);
	(syllable_match, polyphone_level)
}

pub fn percent(numerator: u64, denominator: u64) -> u64 {
	((numerator as f64 / denominator as f64) * 100.0).round() as u64
}

fn get_char_to_pinyin() -> IndexMap<char, Prs> {
	let mut reader = csv::ReaderBuilder::new()
		.has_headers(false)
		.flexible(true)
		.from_path("data/pinyin.csv")
		.unwrap();
	let mut char_to_pinyin: IndexMap<char, Prs> = IndexMap::new();
	for result in reader.records() {
		let record = result.expect("a CSV record");
		let mut iter = record.into_iter();
		let character_cell = iter.next().unwrap();
		let character = character_cell.chars().next().unwrap();
		let pinyins: Prs = iter
			.map(|pinyin| {
				let mut chars = pinyin.chars();
				// remove the tone which is the last char
				let tone = chars.next_back().unwrap();
				let syllable = chars.as_str().to_string();
				Pr { syllable, tone }
			})
			.collect();
		char_to_pinyin.insert(character, pinyins);
	}
	char_to_pinyin
}

// Shortcomings that make some monophonic characters polyphonic
// * Contains both "正音" and "懶音"
//   eg: 我 ngo5 (正音) o5 (懶音), 你 nei5 (正音) lei5 (懶音)
// * Contains some tonal sandhis
//   eg: 紅 gung1 hung4 hung2 紅紅哋(hung4 hung2 dei2)
fn get_char_to_jyutping_lshk() -> IndexMap<char, Prs> {
	let mut reader = csv::ReaderBuilder::new()
		.delimiter(b'\t')
		.from_path("data/jyutping_lshk.tsv")
		.unwrap();
	let mut char_to_jyutping: IndexMap<char, Prs> = IndexMap::new();
	for result in reader.records() {
		let record = result.expect("a TSV record");
		let character = record[0].chars().next().unwrap();
		// check if jyutping entry is filled and contains a single jyutping syllable
		// * radicals (𧘇 "衣字底") and unit characters (𠺖) may have a separate entry
		//   for explanation without a jyutping entry
		// * characters like 𠯢 (saa1 aa6) have two syllables
		if !record[2].is_empty() && !record[2].contains(char::is_whitespace) {
			let mut jyutping_chars = record[2].chars();
			// remove tone which is the last character
			let tone = jyutping_chars.next_back().unwrap();
			let syllable = jyutping_chars.as_str().to_string();
			let jyutping = Pr { syllable, tone };
			char_to_jyutping
				.entry(character)
				.or_insert(IndexSet::new())
				.insert(jyutping);
		}
	}
	char_to_jyutping
}

fn get_char_to_jyutping_rime(char_to_jyutping_lshk: &IndexMap<char, Prs>) -> IndexMap<char, Prs> {
	let mut reader = csv::ReaderBuilder::new()
		.delimiter(b'\t')
		.has_headers(false)
		.flexible(true)
		.from_path("data/jyutping_rime.tsv")
		.unwrap();
	let mut char_to_jyutping: IndexMap<char, Prs> = IndexMap::new();
	let trad_to_simp = get_script_conversion_from(Script::Traditional);
	for result in reader.records() {
		let record = result.expect("a TSV record");
		let mut jyutping_chars = record[1].chars();
		// remove tone which is the last character
		let tone = jyutping_chars.next_back().unwrap();
		let syllable = jyutping_chars.as_str().to_string();
		let jyutping = Pr { syllable, tone };
		let trad_char = record[0].chars().next().unwrap();
		// add the traditional character prs first
		char_to_jyutping
			.entry(trad_char)
			.or_insert(IndexSet::new())
			.insert(jyutping.clone());
		// convert traditional to simplified variants and add their prs
		if trad_to_simp.contains_key(&trad_char) {
			let simp_chars = trad_to_simp.get(&trad_char).unwrap();
			// One traditional mapped to one simplified
			if simp_chars.len() == 1 {
				char_to_jyutping
					.entry(simp_chars[0])
					.or_insert(IndexSet::new())
					.insert(jyutping);
			}
			// One traditional mapped to more than 1 simplified
			// Use lshk data instead because we don't know
			// which sounds map to which simplified characters
			// This happens very rarely (less than 20 times)
			// And even fewer cases are common characters
			else {
				simp_chars.iter().for_each(|simp_char| {
					char_to_jyutping
						.entry(*simp_char)
						.or_insert(IndexSet::new())
						.union(char_to_jyutping_lshk.get(simp_char).unwrap());
				});
			}
		}
	}
	char_to_jyutping
}

fn get_script_conversion_from(script: Script) -> IndexMap<char, IndexSet<char>> {
	let file_path = match script {
		Script::Simplified => "data/simp_to_trad.tsv",
		Script::Traditional => "data/trad_to_simp.tsv",
	};
	let mut reader = csv::ReaderBuilder::new()
		.delimiter(b'\t')
		.has_headers(false)
		.from_path(file_path)
		.unwrap();
	let mut trad_to_simp = IndexMap::new();
	for result in reader.records() {
		let record = result.expect("a TSV record");
		let trad_char = record[0].chars().next().unwrap();
		let simp_chars = record[1].chars().filter(|c| !c.is_whitespace()).collect();
		trad_to_simp.insert(trad_char, simp_chars);
	}
	trad_to_simp
}
