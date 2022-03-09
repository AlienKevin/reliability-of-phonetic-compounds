use headless_chrome::{Browser, LaunchOptionsBuilder};
use indexmap::IndexSet;
use itertools::Itertools;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::fs;
use std::fs::File;
use std::io::Write;
use std::time::Duration;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct CharInfo {
	pub variants: Vec<Variant>,
	pub radicals: Vec<char>,
	pub num_of_strokes: u8,
	pub stroke_order: String,
	pub construction_method: String,
	pub structure_kind: char,
	pub structure_mode: String,
	pub components: Vec<Component>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Variant {
	character: char,
	simplify_method: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum Component {
	Character(char),
	Combination(Vec<char>),
}

impl std::fmt::Display for Component {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Component::Character(c) => {
				write!(f, "{}", c)
			}
			Component::Combination(chars) => {
				write!(f, "[{}]", chars.iter().join(""))
			}
		}
	}
}

// Important Note: as soon as the website loads, you need to
// configure the dropdown "字符集" as "字符总集(81982)".
// Otherwise, some data (like stroke order) may be missing.
pub fn scrape_qxk_char_info() {
	let common_chars: IndexSet<char> = fs::read_to_string("data/merged_common_characters.txt")
		.unwrap()
		.chars()
		.filter(|c| !c.is_whitespace())
		.collect();
	let browser = Browser::new(
		LaunchOptionsBuilder::default()
			.headless(false)
			.sandbox(true)
			// .idle_browser_timeout(Duration::new(9999999, 0))
			.build()
			.unwrap(),
	)
	.unwrap();
	let tab = browser.wait_for_initial_tab().unwrap();
	tab.set_default_timeout(Duration::from_secs(10));
	let url = "https://qxk.bnu.edu.cn/#/danziDetail/49c12ccb-35cc-437b-af4a-3fe126df8fca/一/22d3af76-1ffe-46da-8c28-40e7dfe6b8d2/0";
	tab.navigate_to(&url).unwrap();
	// Take this time to configure the dropdown "字符集" as "字符总集(81982)"
	std::thread::sleep(Duration::from_secs(15));
	let mut output = File::create("data/qxk_char_info.json").unwrap();
	common_chars
		.iter()
		.for_each(|c| match scrape_char_info(*c, &tab) {
			Ok(char_info) => {
				writeln!(
					output,
					"\"{}\": {},",
					c,
					serde_json::to_string(&char_info).unwrap()
				);
			}
			Err(err) => {
				writeln!(output, "Error [{}]: {}", c, err);
			}
		});
}

pub fn scrape_phonetics_diff() {
	let browser = Browser::default().unwrap();
	let tab = browser.wait_for_initial_tab().unwrap();
	let url = "https://qxk.bnu.edu.cn/#/danziDetail/49c12ccb-35cc-437b-af4a-3fe126df8fca/友/22d3af76-1ffe-46da-8c28-40e7dfe6b8d2/0";
	tab.navigate_to(&url).unwrap();
	let mut output = File::create("data/phonetics_diff.tsv").unwrap();
	PHONETIC_DIFF
		.iter()
		.for_each(
			|(c, simp_phonetic, trad_phonetic)| match scrape_char_info(*c, &tab) {
				Ok(char_info) => {
					let is_phonetic_compound = char_info.construction_method.contains("形声");
					writeln!(
						output,
						"{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
						c,
						say_yes_no(is_phonetic_compound),
						simp_phonetic.unwrap_or('－'),
						say_yes_no(match simp_phonetic {
							Some(p) => char_info.components.contains(&Component::Character(*p)),
							None => false,
						}),
						trad_phonetic.unwrap_or('－'),
						say_yes_no(match trad_phonetic {
							Some(p) => char_info.components.contains(&Component::Character(*p)),
							None => false,
						}),
						char_info.construction_method,
						char_info.structure_kind,
						char_info.structure_mode,
						char_info.components.iter().join(","),
					);
				}
				Err(err) => {
					writeln!(output, "Error: {}", err);
				}
			},
		);
}

fn say_yes_no(b: bool) -> char {
	if b {
		'Y'
	} else {
		'N'
	}
}

fn scrape_char_info(
	char: char,
	tab: &std::sync::Arc<headless_chrome::Tab>,
) -> Result<CharInfo, Box<dyn std::error::Error>> {
	let search_bar_selector =
		"#searchOption > div.el-autocomplete > div.el-input.el-input--suffix > input";
	tab.wait_for_element(search_bar_selector)?.click()?;
	tab.press_key("Backspace")?;
	tab.type_str(&char.to_string())?.press_key("Enter")?;
	// wait for the search result to load
	std::thread::sleep(Duration::from_millis(600));
	let attributes_selector = "#app > section > main > div.danziDetail > div > div.danzi-attr > div.danzi-attr-cont > div > div.zi.clearfix > ul > li > label";
	let mut char_info: CharInfo = CharInfo::default();
	tab.wait_for_elements(attributes_selector)?
		.iter()
		.for_each(|element| {
			let element_text = element.get_inner_text().unwrap();
			let attribute = element_text.trim();
			if attribute.starts_with("简化字") || attribute.starts_with("繁体字") {
				char_info.variants = attribute
					.trim()
					.chars()
					.skip(4) // skip "简化字" or "繁体字" with a following space in the beginning
					.collect::<String>()
					.split_whitespace()
					.map(|variant_info| {
						let mut info = variant_info.chars();
						let character = info.next().unwrap();
						// skip left paren
						info.next();
						// skip right paren
						info.next_back();
						let simplify_method = info.collect();
						Variant {
							character,
							simplify_method,
						}
					})
					.collect();
			} else if attribute.starts_with("部首") {
				char_info.radicals = attribute
					.chars()
					.skip("部首".chars().count())
					.filter(|c| *c != ',')
					.collect();
			} else if attribute.starts_with("笔画数") {
				char_info.num_of_strokes = attribute
					.chars()
					.skip("笔画数".chars().count())
					.collect::<String>()
					.parse()
					.unwrap();
			} else if attribute.starts_with("笔顺") {
				char_info.stroke_order = attribute
					.chars()
					.skip("笔顺".chars().count())
					.collect::<String>();
			} else if attribute.starts_with("造字方法") {
				char_info.construction_method =
					attribute.chars().skip("造字方法".chars().count()).collect();
			} else if attribute.starts_with("结构类型") {
				char_info.structure_kind = attribute
					.chars()
					.skip("结构类型".chars().count())
					.next()
					.unwrap_or('�');
			} else if attribute.starts_with("构形模式") {
				char_info.structure_mode =
					attribute.chars().skip("构形模式".chars().count()).collect()
			}
		});

	let components_selector = "#app > section > main > div.danziDetail > div > div.danzi-attr > div.danzi-attr-cont > div > div.zi.clearfix > ul > li > div > div > div.el-scrollbar__wrap > div > div:nth-of-type(1) > ul > li > ul > li > span";
	let components_elements = tab.wait_for_elements(components_selector)?;
	let mut ignore_malformatted_expansion = false;
	char_info.components = components_elements
		.iter()
		.map(|element| {
			let str = element.get_inner_text().unwrap();
			if str.starts_with('（') {
				ignore_malformatted_expansion = true;
				None
			} else if str.starts_with('）') {
				ignore_malformatted_expansion = false;
				None
			} else if ignore_malformatted_expansion {
				None
			} else if str.starts_with('[') {
				let mut chars = str.chars();
				// remove '['
				chars.next();
				// remove ']'
				chars.next_back();
				Some(Component::Combination(chars.collect()))
			} else {
				let char = str.chars().next().unwrap();
				if char.is_whitespace() {
					None
				} else {
					Some(Component::Character(char))
				}
			}
		})
		.flatten()
		.collect();
	Ok(char_info)
}

lazy_static! {
	static ref PHONETIC_DIFF: [(char, Option<char>, Option<char>); 323] = [
		('友', Some('又'), None),
		('匹', Some('八'), None),
		('仁', Some('人'), None),
		('化', Some('𠤎'), None),
		('分', Some('八'), None),
		('打', None, Some('丁')),
		('巧', Some('丂'), None),
		('叮', Some('丁'), None),
		('叫', Some('丩'), None),
		('代', Some('弋'), None),
		('他', Some('也'), None),
		('汁', Some('十'), None),
		('宁', Some('丁'), None),
		('尼', None, Some('匕')),
		('邦', None, Some('丰')),
		('式', Some('弋'), None),
		('刑', Some('开'), None),
		('考', Some('丂'), None),
		('托', None, Some('乇')),
		('朽', Some('丂'), None),
		('吏', Some('史'), None),
		('在', Some('才'), None),
		('有', Some('又'), None),
		('存', Some('才'), None),
		('夸', Some('于'), None),
		('列', Some('歹'), None),
		('成', Some('丁'), None),
		('劣', Some('少'), None),
		('吆', Some('幺'), None),
		('廷', Some('𡈼'), None),
		('舌', Some('干'), None),
		('企', Some('止'), None),
		('旨', Some('匕'), None),
		('州', None, Some('州')),
		('污', Some('亏'), None),
		('汛', Some('卂'), None),
		('迅', Some('卂'), None),
		('异', Some('巳'), Some('己')),
		('收', Some('丩'), None),
		('如', Some('女'), None),
		('妃', Some('己'), None),
		('她', Some('也'), None),
		('巡', Some('巛'), None),
		('形', Some('开'), None),
		('坏', None, Some('不')),
		('扯', Some('止'), None),
		('坎', Some('欠'), None),
		('均', Some('匀'), Some('勻')),
		('投', None, Some('殳')),
		('坑', Some('亢'), None),
		('志', Some('士'), None),
		('劫', Some('去'), None),
		('杜', Some('土'), None),
		('杉', Some('彡'), None),
		('李', Some('子'), None),
		('否', Some('不'), None),
		('肖', Some('小'), None),
		('盯', Some('丁'), None),
		('呈', Some('𡈼'), None),
		('助', Some('且'), None),
		('吭', Some('亢'), None),
		('牡', Some('土'), None),
		('私', Some('厶'), None),
		('你', Some('尔'), None),
		('余', Some('余'), None),
		('肚', Some('土'), None),
		('灼', Some('勺'), None),
		('沙', Some('少'), None),
		('沃', Some('夭'), None),
		('泛', Some('乏'), None),
		('宏', Some('厷'), None),
		('即', Some('卩'), None),
		('姊', Some('𠂔'), None),
		('妒', Some('户'), Some('戶')),
		('拓', Some('石'), None),
		('坤', None, Some('申')),
		('拖', Some('也'), None),
		('茂', Some('戊'), None),
		('析', None, Some('斤')),
		('杭', Some('亢'), None),
		('述', Some('术'), Some('朮')),
		('枕', Some('冘'), None),
		('郁', None, Some('有')),
		('叔', Some('尗'), None),
		('歧', Some('支'), None),
		('昂', Some('卬'), None),
		('迪', None, Some('由')),
		('岸', Some('干'), None),
		('佳', None, Some('圭')),
		('往', Some('王'), None),
		('所', None, Some('戶')),
		('肴', Some('爻'), None),
		('乳', None, Some('孚')),
		('股', None, Some('殳')),
		('疙', None, Some('乞')),
		('炕', Some('亢'), None),
		('郎', Some('良'), None),
		('居', None, Some('古')),
		('降', Some('夅'), None),
		('珍', Some('㐱'), None),
		('挖', None, Some('穵')),
		('荐', None, Some('存')),
		('茫', Some('亡'), None),
		('柳', Some('卯'), None),
		('勃', Some('孛'), None),
		('研', Some('开'), None),
		('砂', Some('少'), None),
		('砍', None, Some('欠')),
		('眨', None, Some('乏')),
		('幽', Some('幺'), None),
		('追', Some('𠂤'), None),
		('俊', Some('夋'), None),
		('胚', Some('不'), Some('丕')),
		('怨', Some('夗'), None),
		('哀', Some('衣'), None),
		('疫', None, Some('殳')),
		('施', Some('也'), None),
		('姜', Some('羊'), None),
		('洼', None, Some('圭')),
		('洒', None, Some('西')),
		('派', Some('𠂢'), None),
		('津', Some('聿'), None),
		('恒', Some('亘'), None),
		('宣', Some('亘'), None),
		('室', Some('至'), None),
		('冠', Some('元'), None),
		('退', Some('艮'), None),
		('既', Some('旡'), None),
		('屎', Some('尸'), None),
		('柔', Some('矛'), None),
		('蚕', None, Some('天')),
		('起', Some('己'), None),
		('捌', Some('别'), Some('別')),
		('恐', Some('巩'), None),
		('耽', Some('冘'), None),
		('莽', Some('茻'), None),
		('哥', None, Some('可')),
		('酌', Some('勺'), None),
		('配', Some('己'), None),
		('晒', None, Some('西')),
		('蚌', Some('丰'), None),
		('峰', Some('夆'), None),
		('秤', None, Some('平')),
		('候', Some('侯'), None),
		('倍', Some('咅'), None),
		('徒', Some('土'), None),
		('航', Some('亢'), None),
		('爹', None, Some('多')),
		('胸', Some('凶'), Some('匈')),
		('留', Some('卯'), None),
		('郭', None, Some('享')),
		('准', Some('隹'), None),
		('疾', Some('矢'), None),
		('唐', Some('庚'), None),
		('剖', Some('咅'), None),
		('部', Some('咅'), None),
		('涂', None, Some('余')),
		('浸', Some('𠬶'), None),
		('悖', None, Some('孛')),
		('宴', Some('妟'), None),
		('朗', Some('良'), None),
		('陷', Some('臽'), None),
		('陪', Some('咅'), None),
		('堆', Some('隹'), None),
		('捻', None, Some('念')),
		('掐', Some('臽'), None),
		('掠', None, Some('京')),
		('培', Some('咅'), None),
		('菩', Some('咅'), None),
		('萍', Some('苹'), None),
		('梳', Some('㐬'), None),
		('梭', Some('夋'), None),
		('副', Some('畐'), None),
		('戚', Some('尗'), None),
		('野', Some('予'), None),
		('曼', Some('冒'), None),
		('啄', Some('豖'), None),
		('蛇', Some('它'), None),
		('患', Some('吅'), Some('串')),
		('唯', Some('隹'), None),
		('崖', Some('圭'), None),
		('崔', Some('隹'), None),
		('帷', Some('隹'), None),
		('甜', None, Some('甘')),
		('笛', None, Some('由')),
		('悠', Some('攸'), None),
		('假', Some('叚'), None),
		('徙', Some('止'), None),
		('豚', Some('豕'), None),
		('毫', None, Some('毛')),
		('粘', None, Some('占')),
		('涯', Some('厓'), None),
		('渠', Some('巨'), None),
		('淫', Some('㸒'), None),
		('淳', None, Some('享')),
		('寂', Some('尗'), None),
		('密', Some('宓'), None),
		('逮', Some('隶'), None),
		('隆', Some('降'), None),
		('琢', Some('豖'), None),
		('塔', Some('荅'), None),
		('越', Some('戉'), None),
		('趁', Some('㐱'), None),
		('博', Some('甫'), None),
		('插', None, Some('臿')),
		('壹', Some('吉'), None),
		('葡', Some('甫'), Some('匍')),
		('棱', Some('夌'), None),
		('焚', None, Some('林')),
		('棵', None, Some('果')),
		('椎', Some('隹'), None),
		('棉', Some('帛'), None),
		('雁', Some('厂'), None),
		('雄', Some('厷'), None),
		('晴', Some('生'), Some('青')),
		('喘', Some('耑'), None),
		('喻', Some('俞'), None),
		('幅', Some('畐'), None),
		('氯', Some('录'), Some('彔')),
		('等', Some('寺'), None),
		('筑', Some('竹'), None),
		('答', None, Some('合')),
		('御', Some('御'), None),
		('舒', Some('予'), None),
		('逾', Some('俞'), None),
		('禽', Some('今'), None),
		('腊', None, Some('昔')),
		('就', None, Some('尤')),
		('敦', None, Some('享')),
		('童', Some('重'), None),
		('竣', Some('夋'), None),
		('道', Some('首'), None),
		('遂', Some('㒸'), None),
		('焰', Some('臽'), None),
		('渝', Some('俞'), None),
		('滋', Some('兹'), None),
		('惰', Some('左'), None),
		('愉', Some('俞'), None),
		('富', Some('畐'), None),
		('窗', Some('囱'), Some('囪')),
		('雇', Some('户'), Some('戶')),
		('粥', Some('米'), None),
		('疏', Some('疋'), None),
		('隙', Some('𡭴'), None),
		('瑟', Some('必'), None),
		('肆', Some('聿'), None),
		('聘', Some('甹'), None),
		('蒜', Some('祘'), None),
		('勤', Some('堇'), None),
		('靴', None, Some('化')),
		('蒙', Some('冡'), None),
		('楚', Some('疋'), None),
		('碌', Some('录'), None),
		('睦', Some('坴'), None),
		('睫', Some('疌'), None),
		('鄙', Some('啚'), None),
		('愚', Some('禺'), None),
		('暇', Some('叚'), None),
		('蜈', Some('吴'), Some('吳')),
		('蜂', Some('逢'), None),
		('稚', Some('隹'), None),
		('触', None, Some('虫')),
		('痹', Some('畀'), None),
		('新', Some('亲'), Some('斤')),
		('慈', Some('兹'), None),
		('福', Some('畐'), None),
		('碧', Some('白'), None),
		('璃', Some('离'), None),
		('墟', Some('虚'), Some('虛')),
		('酸', Some('夋'), None),
		('碟', Some('枼'), None),
		('磁', Some('兹'), None),
		('需', Some('而'), None),
		('墅', Some('野'), None),
		('僚', Some('尞'), None),
		('鼻', Some('畀'), None),
		('貌', Some('皃'), None),
		('疑', Some('矢'), None),
		('孵', None, Some('孚')),
		('豪', Some('高'), None),
		('瘟', Some('昷'), None),
		('端', Some('耑'), None),
		('漾', None, Some('羕')),
		('察', Some('祭'), None),
		('蜜', Some('必'), None),
		('寥', None, Some('翏')),
		('熊', Some('能'), None),
		('撩', Some('尞'), None),
		('蕊', Some('惢'), None),
		('敷', Some('甫'), None),
		('蝠', Some('畐'), None),
		('僵', Some('畺'), None),
		('膝', None, Some('桼')),
		('澳', Some('奥'), Some('奧')),
		('懊', Some('奥'), Some('奧')),
		('履', Some('复'), None),
		('撼', Some('咸'), Some('感')),
		('橘', Some('矞'), None),
		('融', Some('虫'), None),
		('嘴', Some('觜'), None),
		('篡', Some('算'), None),
		('邀', Some('敫'), None),
		('衡', Some('行'), None),
		('激', Some('敫'), None),
		('鞠', Some('匊'), None),
		('藐', None, Some('貌')),
		('檀', Some('亶'), None),
		('蹋', Some('𦐇'), None),
		('魏', Some('鬼'), None),
		('簧', Some('黄'), Some('黃')),
		('繁', None, Some('敏')),
		('徽', Some('微'), None),
		('臊', Some('喿'), None),
		('癌', Some('嵒'), None),
		('燥', Some('喿'), None),
		('臀', Some('殿'), None),
		('覆', Some('复'), Some('復')),
		('戳', None, Some('翟')),
		('馨', Some('声'), None),
		('躁', None, Some('喿')),
		('籍', Some('耤'), None),
		('囊', Some('襄'), None),
		('罐', Some('雚'), None),
	];
}
