use crate::app::rj::error::ParseError as Error;
use crate::app::rj::user_opts::{FieldsToAnnounce, TensedUserField, UserAnnouncementOptions};
use std::collections::BTreeMap;
use std::mem;

/// We parse all possible sentences and keep that in memory.
/// There are several limitations of this approach but it gets job done and
/// is simple enough.
/// - We use BTreeMap for predictability in parsing.
/// - We build an acyclic graph of Fields.
/// - Each Field contains array of Fragments.
/// - At the end of parsing we get all possible fragments that contain only reserved fields.
/// - Fragments are divided into past and present/future tense for announcements.

pub static DEFAULT_DEPTH_LIMIT: usize = 5;

#[derive(Debug, Clone)]
pub struct Field {
	delimited_name: String,
	whole: bool,
	fragments: BTreeMap<String, bool>,
}

impl Field {
	pub fn has_self_dependency(&self) -> Option<String> {
		for fragment in self.fragments.keys() {
			if fragment.contains(&self.delimited_name) {
				return Some(fragment.to_owned());
			}
		}
		None
	}

	pub fn iter_fragments<'a>(&'a self) -> Box<dyn Iterator<Item = &String> + 'a> {
		Box::new(self.fragments.keys())
	}

	pub fn is_whole(&self) -> bool {
		self.whole
	}

	pub fn replace_and_keep(&mut self, from: &str, to: &str) -> Result<(), Error> {
		let mut new_fragments = self.fragments.clone();

		if let Some(fragment) = self.has_self_dependency() {
			return Err(Error::RecursiveDependency {
				name: self.delimited_name.to_owned(),
				fragment,
			});
		}

		for (fragment, used) in self.fragments.iter() {
			if fragment.contains(from) {
				new_fragments.insert(fragment.replace(from, to), *used);
				let new_used = new_fragments.get_mut(fragment).unwrap();
				*new_used = true;
			}
		}
		let _ = mem::replace(&mut self.fragments, new_fragments);
		Ok(())
	}

	pub fn each_fragment_is_resolved_once<T>(&self, map: &BTreeMap<String, T>) -> bool {
		let mut field_resolved = false;
		for fragment in self.fragments.keys() {
			let mut resolved = true;
			for word in fragment.split_whitespace() {
				let stripped_name = strip_delimiters(word);
				if is_field_name(word).unwrap()
					&& !(map.contains_key(&stripped_name) || is_reserved(&stripped_name))
				{
					resolved = false;
				}
			}
			if resolved {
				field_resolved = true;
			}
		}
		field_resolved
	}
}

static FIELD_DELIMITER: char = '^';

static RESERVED_FIELD_ID: &str = "id";
static RESERVED_FIELD_PATH: &str = "path";
static RESERVED_FIELD_PARENT: &str = "parent";
static RESERVED_FIELD_TRACK_NUMBER: &str = "track_number";
static RESERVED_FIELD_DISC_NUMBER: &str = "disc_number";
static RESERVED_FIELD_TITLE: &str = "title";
static RESERVED_FIELD_ARTIST: &str = "artist";
static RESERVED_FIELD_ALBUM_ARTIST: &str = "album_artist";
static RESERVED_FIELD_YEAR: &str = "year";
static RESERVED_FIELD_ALBUM: &str = "album";
static RESERVED_FIELD_ARTWORK: &str = "artwork";
static RESERVED_FIELD_DURATION: &str = "duration";
static RESERVED_FIELD_LYRICIST: &str = "lyricist";
static RESERVED_FIELD_COMPOSER: &str = "composer";
static RESERVED_FIELD_GENRE: &str = "genre";
static RESERVED_FIELD_LABEL: &str = "label";

pub static RESERVED_DELIMITED_FIELD_ID: &str = "^id^";
pub static RESERVED_DELIMITED_FIELD_PATH: &str = "^path^";
pub static RESERVED_DELIMITED_FIELD_PARENT: &str = "^parent^";
pub static RESERVED_DELIMITED_FIELD_TRACK_NUMBER: &str = "^track_number^";
pub static RESERVED_DELIMITED_FIELD_DISC_NUMBER: &str = "^disc_number^";
pub static RESERVED_DELIMITED_FIELD_TITLE: &str = "^title^";
pub static RESERVED_DELIMITED_FIELD_ARTIST: &str = "^artist^";
pub static RESERVED_DELIMITED_FIELD_ALBUM_ARTIST: &str = "^album_artist^";
pub static RESERVED_DELIMITED_FIELD_YEAR: &str = "^year^";
pub static RESERVED_DELIMITED_FIELD_ALBUM: &str = "^album^";
pub static RESERVED_DELIMITED_FIELD_ARTWORK: &str = "^artwork^";
pub static RESERVED_DELIMITED_FIELD_DURATION: &str = "^duration^";
pub static RESERVED_DELIMITED_FIELD_LYRICIST: &str = "^lyricist^";
pub static RESERVED_DELIMITED_FIELD_COMPOSER: &str = "^composer^";
pub static RESERVED_DELIMITED_FIELD_GENRE: &str = "^genre^";
pub static RESERVED_DELIMITED_FIELD_LABEL: &str = "^label^";

static RESERVED_FIELD_NAMES: [&str; 16] = [
	RESERVED_FIELD_ID,
	RESERVED_FIELD_PATH,
	RESERVED_FIELD_PARENT,
	RESERVED_FIELD_TRACK_NUMBER,
	RESERVED_FIELD_DISC_NUMBER,
	RESERVED_FIELD_TITLE,
	RESERVED_FIELD_ARTIST,
	RESERVED_FIELD_ALBUM_ARTIST,
	RESERVED_FIELD_YEAR,
	RESERVED_FIELD_ALBUM,
	RESERVED_FIELD_ARTWORK,
	RESERVED_FIELD_DURATION,
	RESERVED_FIELD_LYRICIST,
	RESERVED_FIELD_COMPOSER,
	RESERVED_FIELD_GENRE,
	RESERVED_FIELD_LABEL,
];

fn get_delimited_name(name: &str) -> String {
	FIELD_DELIMITER.to_string() + name + &FIELD_DELIMITER.to_string()
}

fn strip_delimiters(name: &str) -> String {
	name.replace(FIELD_DELIMITER, "")
}

fn is_reserved(name: &str) -> bool {
	for reserved in RESERVED_FIELD_NAMES.iter() {
		if name.eq(*reserved) {
			return true;
		}
	}
	false
}

// Returns count and index of (first, last) occurrence.
fn count_delimiters(word: &str) -> (usize, (usize, usize)) {
	let mut count = 0;
	let mut first = 0;
	let mut last = 0;
	for (index, ch) in word.chars().enumerate() {
		if ch == FIELD_DELIMITER {
			if count == 0 {
				first = index;
			}
			count += 1;
			last = index;
		}
	}
	(count, (first, last))
}

// Returns true if the word is field name i.e. the word is delimited.
fn is_field_name(word: &str) -> Result<bool, usize> {
	let (count, (first, last)) = count_delimiters(word);

	if count == 0 {
		return Ok(false);
	}

	if count != 2 {
		return Err(count);
	}

	if first > 0 || last < (word.len() - 1) {
		return Err(count);
	}
	Ok(true)
}

#[derive(Debug)]
pub struct AnnouncementOptions {
	present: BTreeMap<String, Field>,
	past: BTreeMap<String, Field>,
	neutral: BTreeMap<String, Field>,
	tense: BTreeMap<String, TensedUserField>,
	pub tags_to_announce: FieldsToAnnounce,
}

impl AnnouncementOptions {
	fn build_map(&mut self, user_opts: &UserAnnouncementOptions) -> Result<(), Error> {
		for user_field in &user_opts.patterns {
			if self.neutral.contains_key(&user_field.name) {
				return Err(Error::DuplicateFragment(user_field.name.to_owned()));
			} else {
				let mut set = BTreeMap::new();
				for f in &user_field.fragments {
					set.insert(f.clone(), false);
				}
				self.neutral.insert(
					user_field.name.clone(),
					Field {
						delimited_name: get_delimited_name(&user_field.name),
						whole: user_field.whole,
						fragments: set,
					},
				);
			}
		}

		if user_opts.tense_patterns.is_none() {
			return Ok(());
		}

		for fragment in user_opts.tense_patterns.as_ref().unwrap() {
			if self.neutral.contains_key(&fragment.name) || self.tense.contains_key(&fragment.name)
			{
				return Err(Error::DuplicateFragment(fragment.name.to_owned()));
			} else {
				self.tense.insert(fragment.name.clone(), fragment.clone());
			}
		}
		Ok(())
	}

	fn iterate_all_fragments<T>(
		&self,
		fun: &mut dyn FnMut(&str, &Field, &str) -> (bool, T),
		default_return: T,
	) -> T {
		for (name, field) in self.neutral.iter() {
			for fragment in field.fragments.keys() {
				let (should_break, ret) = fun(name, field, fragment);
				if should_break {
					return ret;
				}
			}
		}
		default_return
	}

	fn iterate_all_words<T>(
		&self,
		fun: &mut dyn FnMut(&str, &Field, &str, &str) -> (bool, T),
		default_return: T,
	) -> T {
		for (name, field) in self.neutral.iter() {
			for fragment in field.fragments.keys() {
				let words = fragment.split_whitespace();
				for word in words {
					let (should_break, ret) = fun(name, field, fragment, word);
					if should_break {
						return ret;
					}
				}
			}
		}
		default_return
	}

	fn uses_reserved_name_internal<T>(map: &BTreeMap<String, T>) -> Result<(), Error> {
		for name in map.keys() {
			if is_reserved(name) {
				return Err(Error::FragmentUsesReservedName {
					name: name.to_owned(),
					reserved: format!("{:?}", RESERVED_FIELD_NAMES),
				});
			}
		}
		Ok(())
	}

	fn uses_reserved_name(&self) -> Result<(), Error> {
		let _ = Self::uses_reserved_name_internal(&self.neutral)?;
		Self::uses_reserved_name_internal(&self.tense)
	}

	fn has_self_dependency(&self) -> Result<(), Error> {
		for (name, field) in self.neutral.iter() {
			if let Some(fragment) = field.has_self_dependency() {
				return Err(Error::SelfRecursion {
					name: name.to_owned(),
					fragment,
				});
			}
		}
		Ok(())
	}

	fn has_delimiter_only_at_start_end(&self) -> Result<(), Error> {
		self.iterate_all_words(
			&mut |name, _: &Field, fragment: &str, word: &str| -> (bool, Result<(), Error>) {
				let x = is_field_name(word);
				if let Err(count) = x {
					if count != 2 {
						return (
							true,
							Err(Error::OddNumberOfDelimiters {
								count,
								delimiter: FIELD_DELIMITER,
								name: name.to_owned(),
								fragment: fragment.to_owned(),
								word: word.to_owned(),
							}),
						);
					}
					return (
						true,
						Err(Error::InterleavedDelimiter {
							delimiter: FIELD_DELIMITER,
							name: name.to_owned(),
							fragment: fragment.to_owned(),
							word: word.to_owned(),
						}),
					);
				}
				(false, Ok(()))
			},
			Ok(()),
		)
	}

	fn verify_missing_name(&self) -> Result<(), Error> {
		self.iterate_all_words(
			&mut |name, _: &Field, fragment: &str, word: &str| {
				if is_field_name(word).unwrap() {
					let field_name = strip_delimiters(word);
					if !self.neutral.contains_key(&field_name)
						&& !self.tense.contains_key(&field_name)
						&& !is_reserved(&field_name)
					{
						return (
							true,
							Err(Error::ExpansionFailed {
								name: name.to_owned(),
								field: field_name,
								fragment: fragment.to_owned(),
							}),
						);
					}
				}
				(false, Ok(()))
			},
			Ok(()),
		)
	}

	fn has_unresolved(&self) -> bool {
		self.iterate_all_words(
			&mut |_name, _field: &Field, _fragment: &str, word: &str| {
				if is_field_name(word).unwrap() && !is_reserved(&strip_delimiters(word)) {
					return (true, true);
				}
				(false, false)
			},
			false,
		)
	}

	pub fn deflate(&mut self, depth_limit: usize) -> Result<(), Error> {
		for _i in 0..depth_limit {
			if !self.has_unresolved() {
				break;
			}

			let temp = self.neutral.clone();
			for (tmp_name, tmp_field) in &temp {
				for tmp_fragment in tmp_field.fragments.keys() {
					for (map_name, map_field) in &mut self.neutral {
						if map_name != tmp_name {
							let _ = map_field
								.replace_and_keep(&tmp_field.delimited_name, tmp_fragment)?;
						}
					}
				}
			}
		}
		Ok(())
	}

	// If field doesn't exists then creates it before adding fragment.
	fn add_and_insert(
		map: &mut BTreeMap<String, Field>,
		name: &str,
		fragment: String,
		field: &Field,
	) {
		if !map.contains_key(name) {
			map.insert(
				name.to_string(),
				Field {
					delimited_name: field.delimited_name.clone(),
					whole: field.whole,
					fragments: BTreeMap::new(),
				},
			);
		}
		map.get_mut(name).unwrap().fragments.insert(fragment, false);
	}

	pub fn deflate_tense(&mut self) -> Result<(), Error> {
		let mut tmp_past = vec![];
		let mut tmp_present = vec![];
		let mut used = vec![];
		let _ = self.iterate_all_fragments(
			&mut |name: &str, _field: &Field, fragment: &str| -> (bool, Result<(), Error>) {
				let mut past_fragment = fragment.to_string();
				let mut present_fragment = fragment.to_string();
				for (field_name, field) in &self.tense {
					let delimited_name = get_delimited_name(field_name);
					if !fragment.contains(&delimited_name) {
						continue;
					}
					past_fragment = past_fragment.replace(&delimited_name, &field.past);
					present_fragment = present_fragment.replace(&delimited_name, &field.present);
					used.push((name.to_string(), fragment.to_string()));
					// *_field.fragments.get_mut(fragment).unwrap() = true;
				}
				tmp_past.push((name.to_string(), past_fragment));
				tmp_present.push((name.to_string(), present_fragment));
				(false, Ok(()))
			},
			Ok(()),
		)?;
		for (name, fragment) in used {
			*self
				.neutral
				.get_mut(&name)
				.unwrap()
				.fragments
				.get_mut(&fragment)
				.unwrap() = true;
		}
		for (name, fragment) in tmp_past {
			Self::add_and_insert(
				&mut self.past,
				&name,
				fragment,
				self.neutral.get(&name).unwrap(),
			);
		}
		for (name, fragment) in tmp_present {
			Self::add_and_insert(
				&mut self.present,
				&name,
				fragment,
				self.neutral.get(&name).unwrap(),
			);
		}
		Ok(())
	}

	fn get_unresolved(map: &BTreeMap<String, Field>) -> Vec<(String, String)> {
		let mut ret = vec![];
		for (name, field) in map {
			for fragment in field.fragments.keys() {
				for word in fragment.split_whitespace() {
					if is_field_name(word).unwrap() && !is_reserved(&strip_delimiters(word)) {
						ret.push((name.to_string(), fragment.to_string()));
						break;
					}
				}
			}
		}
		ret
	}

	fn each_field_is_resolved_once(&self, depth: usize) -> Result<(), Error> {
		for (name, field) in self.neutral.iter() {
			if !field.each_fragment_is_resolved_once(&self.tense) {
				return Err(Error::TooDeep {
					depth,
					name: name.to_owned(),
					fragment: "".to_string(),
				});
			}
		}
		Ok(())
	}

	fn remove_unresolved_internal(
		map: &mut BTreeMap<String, Field>,
		to_remove: &[(String, String)],
	) {
		for (name, fragment) in to_remove {
			let f = map.get_mut(name).unwrap();
			f.fragments.remove(fragment);
		}
	}

	fn remove_unresolved(&mut self, depth: usize) -> Result<(), Error> {
		let mut to_remove = vec![];
		let _ = self.iterate_all_words(
			&mut |name, field: &Field, fragment: &str, word: &str| {
				if is_field_name(word).unwrap() && !is_reserved(&strip_delimiters(word)) {
					if !field.fragments.get(fragment).unwrap() {
						return (
							true,
							Err(Error::TooDeep {
								depth,
								name: name.to_owned(),
								fragment: fragment.to_owned(),
							}),
						);
					} else {
						to_remove.push((name.to_string(), fragment.to_string()));
					}
				}
				(false, Ok(()))
			},
			Ok(()),
		)?;
		for (name, field) in &mut self.neutral {
			for (to_remove_name, to_remove_fragment) in &to_remove {
				if name == to_remove_name {
					field.fragments.remove(to_remove_fragment);
				}
			}
		}
		let past_remove = Self::get_unresolved(&self.past);
		Self::remove_unresolved_internal(&mut self.past, &past_remove);
		let present_remove = Self::get_unresolved(&self.present);
		Self::remove_unresolved_internal(&mut self.present, &present_remove);
		Ok(())
	}

	pub fn from_user(
		user_opts: &UserAnnouncementOptions,
		depth_limit: usize,
	) -> Result<Self, Error> {
		let mut opts = Self {
			present: BTreeMap::new(),
			past: BTreeMap::new(),
			neutral: BTreeMap::new(),
			tense: BTreeMap::new(),
			tags_to_announce: user_opts
				.tags_to_announce
				.as_ref()
				.unwrap_or(&FieldsToAnnounce::default())
				.clone(),
		};
		let _ = opts.build_map(user_opts)?;
		let _ = opts.has_self_dependency()?;
		let _ = opts.uses_reserved_name()?;
		let _ = opts.has_delimiter_only_at_start_end()?;
		let _ = opts.verify_missing_name()?;
		let _ = opts.deflate(depth_limit)?;
		let _ = opts.deflate_tense()?;
		let _ = opts.each_field_is_resolved_once(depth_limit)?;
		let _ = opts.remove_unresolved(depth_limit)?;
		Ok(opts)
	}

	pub fn get_past(&self) -> &BTreeMap<String, Field> {
		&self.past
	}

	pub fn get_present(&self) -> &BTreeMap<String, Field> {
		&self.present
	}

	pub fn get_neutral(&self) -> &BTreeMap<String, Field> {
		&self.neutral
	}

	#[cfg(test)]
	pub fn hi_default() -> Self {
		Self::from_user(&UserAnnouncementOptions::hi_default(), 5).unwrap()
	}

	#[cfg(test)]
	pub fn en_default() -> Self {
		Self::from_user(&UserAnnouncementOptions::en_default(), 5).unwrap()
	}

	#[cfg(test)]
	pub fn tutorial_default() -> Self {
		Self::from_user(&UserAnnouncementOptions::tutorial_default(), 5).unwrap()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::app::rj::user_opts::UserField;

	fn sample_input() -> &'static str {
		r#"
        {
          "pattern": [
            {
              "name": "announce_title",
              "whole": true,
              "fragments": [
                "title1",
                "title2"
              ]
            },
            {
              "name": "announce_album",
              "whole": true,
              "fragments": [
                "album1",
                "album2"
              ]
            },
            {
              "name": "announce_artist",
              "whole": true,
              "fragments": [
                "artists1",
                "artists2"
              ]
            }
          ]
        }
        "#
	}

	#[test]
	fn parse_test() {
		let _opts: UserAnnouncementOptions = serde_json::from_str(sample_input()).unwrap();
	}

	#[test]
	fn from_user_valid() {
		let user_opts: UserAnnouncementOptions = serde_json::from_str(sample_input()).unwrap();

		let r = AnnouncementOptions::from_user(&user_opts, DEFAULT_DEPTH_LIMIT);
		assert!(r.is_ok());
	}

	#[test]
	fn from_user_duplicate() {
		let mut user_opts: UserAnnouncementOptions = serde_json::from_str(sample_input()).unwrap();

		user_opts.patterns.push(UserField {
			name: "announce_title".to_string(),
			whole: true,
			fragments: vec![],
		});
		let r = AnnouncementOptions::from_user(&user_opts, DEFAULT_DEPTH_LIMIT);
		assert!(matches!(r.unwrap_err(), Error::DuplicateFragment(..)));
	}

	#[test]
	fn from_user_cyclic_dependency_in_self() {
		let mut user_opts: UserAnnouncementOptions = serde_json::from_str(sample_input()).unwrap();

		user_opts.patterns.push(UserField {
			name: "user1".to_string(),
			whole: true,
			fragments: vec![get_delimited_name("user2"), get_delimited_name("user1")],
		});
		let r = AnnouncementOptions::from_user(&user_opts, DEFAULT_DEPTH_LIMIT);
		assert!(matches!(r.unwrap_err(), Error::SelfRecursion { .. }));
	}

	#[test]
	fn from_user_uses_reserved_name() {
		let mut user_opts: UserAnnouncementOptions = serde_json::from_str(sample_input()).unwrap();

		user_opts.patterns.push(UserField {
			name: "title".to_string(),
			whole: true,
			fragments: vec![get_delimited_name("user2"), get_delimited_name("user1")],
		});
		let r = AnnouncementOptions::from_user(&user_opts, DEFAULT_DEPTH_LIMIT);
		assert!(matches!(
			r.unwrap_err(),
			Error::FragmentUsesReservedName { .. }
		));
	}

	#[test]
	fn from_user_uses_odd_delimiter() {
		let mut user_opts: UserAnnouncementOptions = serde_json::from_str(sample_input()).unwrap();

		user_opts.patterns.push(UserField {
			name: "user1".to_string(),
			whole: true,
			fragments: vec!["adf ^user2".to_string()],
		});
		let r = AnnouncementOptions::from_user(&user_opts, DEFAULT_DEPTH_LIMIT);
		assert!(matches!(
			r.unwrap_err(),
			Error::OddNumberOfDelimiters { .. }
		));
	}

	#[test]
	fn from_user_uses_even_interleaved_delimiter() {
		let mut user_opts: UserAnnouncementOptions = serde_json::from_str(sample_input()).unwrap();

		user_opts.patterns.push(UserField {
			name: "user1".to_string(),
			whole: true,
			fragments: vec!["adf ^us^er2".to_string()],
		});
		let r = AnnouncementOptions::from_user(&user_opts, DEFAULT_DEPTH_LIMIT);
		assert!(matches!(r.unwrap_err(), Error::InterleavedDelimiter { .. }));
	}

	#[test]
	fn from_user_uses_missing_name() {
		let mut user_opts: UserAnnouncementOptions = serde_json::from_str(sample_input()).unwrap();

		user_opts.patterns.push(UserField {
			name: "user1".to_string(),
			whole: true,
			fragments: vec!["Next one is a ^cat^ song.".to_string()],
		});
		let r = AnnouncementOptions::from_user(&user_opts, DEFAULT_DEPTH_LIMIT);
		assert!(matches!(r.unwrap_err(), Error::ExpansionFailed { .. }));
	}

	#[test]
	fn from_user_recursive_dependency() {
		let user_opts: UserAnnouncementOptions = serde_json::from_str(
			r#"
        {
          "pattern": [
            {
              "name": "announce_title",
              "whole": true,
              "fragments": [
                "title1",
                "title2",
                "abc ^announce_album^ def"
              ]
            },
            {
              "name": "announce_album",
              "whole": true,
              "fragments": [
                "album1",
                "album2",
                "ghi jkl ^announce_artist^"
              ]
            },
            {
              "name": "announce_artist",
              "whole": true,
              "fragments": [
                "artists1",
                "artists2",
                "^announce_title^ mno pqr"
              ]
            }
          ]
        }
        "#,
		)
		.unwrap();
		let r = AnnouncementOptions::from_user(&user_opts, DEFAULT_DEPTH_LIMIT);
		assert!(matches!(r.unwrap_err(), Error::RecursiveDependency { .. }));
	}

	#[test]
	fn from_user_too_deep() {
		let user_opts: UserAnnouncementOptions = serde_json::from_str(
			r#"
        {
          "pattern": [
            {
              "name": "a",
              "whole": true,
              "fragments": [
                "^b^"
              ]
            },
            {
              "name": "b",
              "whole": true,
              "fragments": [
                "^c^"
              ]
            },
            {
              "name": "c",
              "whole": true,
              "fragments": [
                "^d^"
              ]
            },
            {
              "name": "d",
              "whole": true,
              "fragments": [
                "^e^"
              ]
            },
            {
              "name": "e",
              "whole": true,
              "fragments": [
                "^f^"
              ]
            },
            {
              "name": "f",
              "whole": true,
              "fragments": [
                "^g^"
              ]
            },
            {
              "name": "g",
              "whole": true,
              "fragments": [
                " ^h^ "
              ]
            },
            {
              "name": "h",
              "whole": true,
              "fragments": [
                "  done  "
              ]
            }
          ]
        }
        "#,
		)
		.unwrap();
		let r = AnnouncementOptions::from_user(&user_opts, 5);
		assert!(r.is_ok());
	}

	#[test]
	fn parse_default_scripts() {
		let _hi = AnnouncementOptions::hi_default();
		let _en = AnnouncementOptions::en_default();
		let _ex = AnnouncementOptions::tutorial_default();
	}
}
