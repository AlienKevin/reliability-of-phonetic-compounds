use phonetics::calculate_phonetics_stats::calculate_phonetics_stats;
use phonetics::compare_phonetics::compare_phonetics;
use phonetics::extract_qxk_errors::extract_qxk_errors;
use phonetics::fix_phonetics_data::{detect_mismatched_categorization, fix_phonetics_data};
use phonetics::generate_merged_common_chars::generate_merged_common_chars;
use phonetics::scrape_phonetics_info::{scrape_phonetics_diff, scrape_qxk_char_info};

fn main() {
    calculate_phonetics_stats();
    // compare_phonetics();
    // match scrape_phonetics_diff() {
    //     Ok(_) => (),
    //     Err(error) => panic!("{:?}", error),
    // }
    // scrape_phonetics_diff();
    // fix_phonetics_data();
    // scrape_qxk_char_info();
    // extract_qxk_errors();
    // detect_mismatched_categorization();
}
