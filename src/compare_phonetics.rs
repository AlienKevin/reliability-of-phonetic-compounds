use super::calculate_phonetics_stats::{get_char_to_phonetic, Script};

pub fn compare_phonetics() {
	let simp_phonetics = get_char_to_phonetic(&Script::Simplified).unwrap();
	let trad_phonetics = get_char_to_phonetic(&Script::Traditional).unwrap();
	simp_phonetics.iter().for_each(|(char, simp_phonetic)| {
		if trad_phonetics.contains_key(char) {
			let trad_phonetic = trad_phonetics.get(char).unwrap();
			if trad_phonetic != simp_phonetic {
				println!(
					"{char}\tS={}\tT={}",
					simp_phonetic.unwrap_or('x'),
					trad_phonetic.unwrap_or('x')
				)
			}
		}
	});
}
