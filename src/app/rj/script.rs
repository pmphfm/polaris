use lazy_static::lazy_static;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::HashMap;

use crate::app::{
	index::Song,
	rj::{
		error::ParseError as Error,
		parse::*,
		user_opts::{FieldsToAnnounce, Inclusion, UserAnnouncementOptions},
	},
};
use bitflags::bitflags;

#[allow(dead_code)]
#[derive(Hash, Clone, Copy, PartialEq, Eq, Debug)]
pub enum FieldId {
	Id,
	Path,
	Parent,
	TrackNumber,
	DiscNumber,
	Title,
	Artist,
	AlbumArtist,
	Year,
	Album,
	Artwork,
	Duration,
	Lyricist,
	Composer,
	Genre,
	Label,
}

lazy_static! {
	static ref DELIMITED_FIELD_TO_FIELDSET: HashMap<&'static str, FieldSet> = {
		let mut map = HashMap::new();
		map.insert(RESERVED_DELIMITED_FIELD_ID, FieldSet::ID);
		map.insert(RESERVED_DELIMITED_FIELD_PATH, FieldSet::PATH);
		map.insert(RESERVED_DELIMITED_FIELD_PARENT, FieldSet::PARENT);
		map.insert(
			RESERVED_DELIMITED_FIELD_TRACK_NUMBER,
			FieldSet::TRACK_NUMBER,
		);
		map.insert(RESERVED_DELIMITED_FIELD_DISC_NUMBER, FieldSet::DISC_NUMBER);
		map.insert(RESERVED_DELIMITED_FIELD_TITLE, FieldSet::TITLE);
		map.insert(RESERVED_DELIMITED_FIELD_ARTIST, FieldSet::ARTIST);
		map.insert(
			RESERVED_DELIMITED_FIELD_ALBUM_ARTIST,
			FieldSet::ALBUM_ARTIST,
		);
		map.insert(RESERVED_DELIMITED_FIELD_YEAR, FieldSet::YEAR);
		map.insert(RESERVED_DELIMITED_FIELD_ALBUM, FieldSet::ALBUM);
		map.insert(RESERVED_DELIMITED_FIELD_ARTWORK, FieldSet::ARTWORK);
		map.insert(RESERVED_DELIMITED_FIELD_DURATION, FieldSet::DURATION);
		map.insert(RESERVED_DELIMITED_FIELD_LYRICIST, FieldSet::LYRICIST);
		map.insert(RESERVED_DELIMITED_FIELD_COMPOSER, FieldSet::COMPOSER);
		map.insert(RESERVED_DELIMITED_FIELD_GENRE, FieldSet::GENRE);
		map.insert(RESERVED_DELIMITED_FIELD_LABEL, FieldSet::LABEL);
		map
	};
}

bitflags! {
	struct FieldSet: u32 {
	const ID            = 0b0000000000000001;
	const PATH          = 0b0000000000000010;
	const PARENT        = 0b0000000000000100;
	const TRACK_NUMBER  = 0b0000000000001000;
	const DISC_NUMBER   = 0b0000000000010000;
	const TITLE         = 0b0000000000100000;
	const ARTIST        = 0b0000000001000000;
	const ALBUM_ARTIST  = 0b0000000010000000;
	const YEAR          = 0b0000000100000000;
	const ALBUM         = 0b0000001000000000;
	const ARTWORK       = 0b0000010000000000;
	const DURATION      = 0b0000100000000000;
	const LYRICIST      = 0b0001000000000000;
	const COMPOSER      = 0b0010000000000000;
	const GENRE         = 0b0100000000000000;
	const LABEL         = 0b1000000000000000;
	}
}

impl FieldSet {
	pub fn iter_flags() -> Vec<FieldSet> {
		vec![
			FieldSet::ID,
			FieldSet::PATH,
			FieldSet::PARENT,
			FieldSet::TRACK_NUMBER,
			FieldSet::DISC_NUMBER,
			FieldSet::TITLE,
			FieldSet::ARTIST,
			FieldSet::ALBUM_ARTIST,
			FieldSet::YEAR,
			FieldSet::ALBUM,
			FieldSet::ARTWORK,
			FieldSet::DURATION,
			FieldSet::LYRICIST,
			FieldSet::COMPOSER,
			FieldSet::GENRE,
			FieldSet::LABEL,
		]
	}

	pub fn from_word(value: &str) -> FieldSet {
		*DELIMITED_FIELD_TO_FIELDSET
			.get(value)
			.unwrap_or(&FieldSet::empty())
	}

	fn update_from_tags(
		include: &mut FieldSet,
		optional: &mut FieldSet,
		exclude: &mut FieldSet,
		inclusion: Inclusion,
		tag: FieldSet,
	) {
		match inclusion {
			Inclusion::Required => *include |= tag,
			Inclusion::Optional => *optional |= tag,
			Inclusion::Exclude => *exclude |= tag,
		};
	}

	fn from_tags_to_announce(tags: &FieldsToAnnounce) -> (FieldSet, FieldSet, FieldSet) {
		let mut include = FieldSet::empty();
		let mut optional = FieldSet::empty();
		let mut exclude = FieldSet::empty();

		Self::update_from_tags(
			&mut include,
			&mut optional,
			&mut exclude,
			tags.track_number,
			FieldSet::TRACK_NUMBER,
		);
		Self::update_from_tags(
			&mut include,
			&mut optional,
			&mut exclude,
			tags.disc_number,
			FieldSet::DISC_NUMBER,
		);
		Self::update_from_tags(
			&mut include,
			&mut optional,
			&mut exclude,
			tags.title,
			FieldSet::TITLE,
		);
		Self::update_from_tags(
			&mut include,
			&mut optional,
			&mut exclude,
			tags.artist,
			FieldSet::ARTIST,
		);
		Self::update_from_tags(
			&mut include,
			&mut optional,
			&mut exclude,
			tags.album_artist,
			FieldSet::ALBUM_ARTIST,
		);
		Self::update_from_tags(
			&mut include,
			&mut optional,
			&mut exclude,
			tags.year,
			FieldSet::YEAR,
		);
		Self::update_from_tags(
			&mut include,
			&mut optional,
			&mut exclude,
			tags.album,
			FieldSet::ALBUM,
		);
		Self::update_from_tags(
			&mut include,
			&mut optional,
			&mut exclude,
			tags.duration,
			FieldSet::DURATION,
		);
		Self::update_from_tags(
			&mut include,
			&mut optional,
			&mut exclude,
			tags.lyricist,
			FieldSet::LYRICIST,
		);
		Self::update_from_tags(
			&mut include,
			&mut optional,
			&mut exclude,
			tags.composer,
			FieldSet::COMPOSER,
		);
		Self::update_from_tags(
			&mut include,
			&mut optional,
			&mut exclude,
			tags.genre,
			FieldSet::GENRE,
		);
		Self::update_from_tags(
			&mut include,
			&mut optional,
			&mut exclude,
			tags.label,
			FieldSet::LABEL,
		);

		(include, optional, exclude)
	}
}

fn wrap_name(name: &str, ssml: bool) -> String {
	if !ssml {
		return name.to_string();
	}
	format!(r#"<say-as interpret-as="name">{}</say-as>"#, name)
}

fn wrap_year(year: i32, ssml: bool) -> String {
	if !ssml {
		return format!("{}", year);
	}
	format!(r#"<say-as interpret-as="date">{}</say-as>"#, year)
}

fn wrap_number(number: i32, ssml: bool) -> String {
	if !ssml {
		return format!("{}", number);
	}
	format!(r#"<say-as interpret-as="cardinal">{}</say-as>"#, number)
}

fn extract_map_and_fieldset(song: &Song, ssml: bool) -> (HashMap<FieldSet, String>, FieldSet) {
	let mut map = HashMap::new();

	let mut set = FieldSet::empty();

	if let Some(track_number) = song.track_number {
		set |= FieldSet::TRACK_NUMBER;
		map.insert(FieldSet::TRACK_NUMBER, wrap_number(track_number, ssml));
	}

	if let Some(disc_number) = song.disc_number {
		set |= FieldSet::DISC_NUMBER;
		map.insert(FieldSet::DISC_NUMBER, wrap_number(disc_number, ssml));
	}

	if let Some(title) = &song.title {
		set |= FieldSet::TITLE;
		map.insert(FieldSet::TITLE, wrap_name(title, ssml));
	}

	if let Some(artist) = &song.artist {
		set |= FieldSet::ARTIST;
		map.insert(FieldSet::ARTIST, wrap_name(artist, ssml));
	}

	if let Some(album_artist) = &song.album_artist {
		set |= FieldSet::ALBUM_ARTIST;
		map.insert(FieldSet::ALBUM_ARTIST, wrap_name(album_artist, ssml));
	}

	if let Some(year) = song.year {
		set |= FieldSet::YEAR;
		map.insert(FieldSet::YEAR, wrap_year(year, ssml));
	}

	if let Some(album) = &song.album {
		set |= FieldSet::ALBUM;
		map.insert(FieldSet::ALBUM, wrap_name(album, ssml));
	}

	if let Some(duration) = song.duration {
		set |= FieldSet::DURATION;
		map.insert(FieldSet::DURATION, wrap_number(duration, ssml));
	}

	if let Some(lyricist) = &song.lyricist {
		set |= FieldSet::LYRICIST;
		map.insert(FieldSet::LYRICIST, wrap_name(lyricist, ssml));
	}

	if let Some(composer) = &song.composer {
		set |= FieldSet::COMPOSER;
		map.insert(FieldSet::COMPOSER, wrap_name(composer, ssml));
	}

	if let Some(genre) = &song.genre {
		set |= FieldSet::GENRE;
		map.insert(FieldSet::GENRE, wrap_name(genre, ssml));
	}

	if let Some(label) = &song.label {
		set |= FieldSet::LABEL;
		map.insert(FieldSet::LABEL, wrap_name(label, ssml));
	}

	(map, set)
}

impl From<&str> for FieldSet {
	fn from(value: &str) -> FieldSet {
		let mut set = Self::empty();
		for word in value.split_whitespace() {
			set |= Self::from_word(word);
		}
		set
	}
}

fn walk_map(self_map: &mut BTreeMap<FieldSet, BTreeSet<String>>, map: &BTreeMap<String, Field>) {
	for (_, field) in map.iter() {
		if !field.is_whole() {
			continue;
		}

		for fragment in field.iter_fragments() {
			let key = FieldSet::from(fragment.as_str());
			if let Some(v) = self_map.get_mut(&key) {
				v.insert(fragment.to_owned());
			} else {
				let mut v = BTreeSet::new();
				v.insert(fragment.to_owned());
				self_map.insert(key, v);
			}
		}
	}
}

#[derive(Debug, Clone)]
pub struct ScriptCache {
	past: BTreeMap<FieldSet, BTreeSet<String>>,
	present: BTreeMap<FieldSet, BTreeSet<String>>,
	conjunctions: Vec<String>,
	include: FieldSet,
	optional: FieldSet,
	exclude: FieldSet,
}

impl From<&AnnouncementOptions> for ScriptCache {
	fn from(opts: &AnnouncementOptions) -> ScriptCache {
		let (include, optional, exclude) = FieldSet::from_tags_to_announce(&opts.tags_to_announce);
		let mut cache = ScriptCache {
			past: BTreeMap::new(),
			present: BTreeMap::new(),
			conjunctions: opts.conjunctions.clone(),
			include,
			optional,
			exclude,
		};

		walk_map(&mut cache.past, opts.get_past());
		walk_map(&mut cache.past, opts.get_neutral());
		walk_map(&mut cache.present, opts.get_present());
		walk_map(&mut cache.present, opts.get_neutral());
		if cache.conjunctions.is_empty() {
			cache.conjunctions.push("".to_string());
		}

		cache
	}
}

impl ScriptCache {
	pub fn create(opts_str: &str) -> Result<ScriptCache, Error> {
		let mut user_opts: UserAnnouncementOptions =
			toml::from_str(opts_str).map_err(|e| Error::FailedToDeserialize(e.to_string()))?;
		let opts = AnnouncementOptions::from_user(&user_opts, DEFAULT_DEPTH_LIMIT)?;
		if user_opts.tags_to_announce.is_none() {
			user_opts.tags_to_announce = Some(FieldsToAnnounce::default());
		}

		let cache = ScriptCache::from(&opts);
		Ok(cache)
	}

	fn get_subset_tags(
		map: &BTreeMap<FieldSet, BTreeSet<String>>,
		set: FieldSet,
	) -> Option<(FieldSet, String)> {
		let start_point = rand::random::<usize>() % map.len();
		let mut found = None;
		for (index, (current_tag, current_set)) in map.iter().enumerate() {
			if !set.contains(*current_tag) {
				continue;
			}
			if index >= start_point {
				let rand_index = rand::random::<usize>() % (current_set.len());
				return Some((
					current_tag.to_owned(),
					current_set.iter().nth(rand_index).unwrap().to_owned(),
				));
			}
			if found.is_none() {
				let rand_index = rand::random::<usize>() % (current_set.len());
				found = Some((
					current_tag.to_owned(),
					current_set.iter().nth(rand_index).unwrap().to_owned(),
				));
			}
		}
		found
	}

	fn get_tag_announcement(map: &BTreeMap<FieldSet, BTreeSet<String>>, set: FieldSet) -> String {
		let mut need = set;
		let mut have = FieldSet::empty();
		let mut announcement = "".to_owned();
		while !need.is_empty() {
			if let Some((found_set, found_str)) = Self::get_subset_tags(map, need) {
				announcement = announcement + " " + &found_str;
				need = need.difference(found_set);
				have = have.union(found_set);
			} else {
				break;
			}
		}
		announcement
	}

	pub fn get_announcement(
		&self,
		song: &Song,
		present: bool,
		enable_ssml: bool,
	) -> Option<String> {
		let (field_song, mut have) = extract_map_and_fieldset(song, enable_ssml);
		have = have.difference(self.exclude);
		let filtered_include = have.intersection(self.include);
		let mut filtered_optional = have.intersection(self.optional);

		// Randomly select a subset of optional fields.
		for flag in FieldSet::iter_flags() {
			if filtered_optional & flag == flag && !rand::random::<bool>() {
				filtered_optional.toggle(flag);
			}
		}

		let mut announcement = Self::get_tag_announcement(
			match present {
				true => &self.present,
				false => &self.past,
			},
			filtered_include.union(filtered_optional),
		);
		announcement = announcement.trim().to_string();
		let tmp = announcement.clone();

		for word in tmp.split_whitespace() {
			let field = FieldSet::from_word(word);
			if field != FieldSet::empty() {
				announcement = announcement.replace(word, field_song.get(&field).unwrap())
			}
		}
		match announcement.is_empty() {
			true => None,
			false => Some(announcement),
		}
	}

	pub fn get_conjunction(&self) -> String {
		self.conjunctions[rand::random::<usize>() % self.conjunctions.len()].to_string()
	}
}

impl Default for ScriptCache {
	fn default() -> Self {
		ScriptCache::create(&UserAnnouncementOptions::en_default_script_json()).unwrap()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn default_scripts() {
		let hi = ScriptCache::create(&UserAnnouncementOptions::hi_default_script_toml()).unwrap();
		println!("hi_script: {:#?}", hi);
		let en = ScriptCache::create(&UserAnnouncementOptions::en_default_script_toml()).unwrap();
		println!("en_script: {:#?}", en);
		let ex = ScriptCache::create(&UserAnnouncementOptions::tutorial_script_toml()).unwrap();
		println!("ex_script: {:#?}", ex);
	}
}
