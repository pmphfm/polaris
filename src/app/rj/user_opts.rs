use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum Inclusion {
	Required,
	Optional,
	Exclude,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FieldsToAnnounce {
	pub track_number: Inclusion,
	pub disc_number: Inclusion,
	pub title: Inclusion,
	pub artist: Inclusion,
	pub album_artist: Inclusion,
	pub year: Inclusion,
	pub album: Inclusion,
	pub duration: Inclusion,
	pub lyricist: Inclusion,
	pub composer: Inclusion,
	pub genre: Inclusion,
	pub label: Inclusion,
}

impl Default for FieldsToAnnounce {
	fn default() -> Self {
		FieldsToAnnounce {
			track_number: Inclusion::Exclude,
			disc_number: Inclusion::Exclude,
			title: Inclusion::Required,
			artist: Inclusion::Required,
			album_artist: Inclusion::Optional,
			year: Inclusion::Optional,
			album: Inclusion::Required,
			duration: Inclusion::Exclude,
			lyricist: Inclusion::Required,
			composer: Inclusion::Required,
			genre: Inclusion::Optional,
			label: Inclusion::Exclude,
		}
	}
}

// This is user input field. Keep it simple.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserField {
	pub name: String,
	pub whole: bool,
	pub fragments: Vec<String>,
}

// This is user input field. Keep it simple.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TensedUserField {
	pub name: String,
	pub past: String,
	pub present: String,
}

// This is user input field. Keep it simple.
#[derive(Debug, Serialize, Deserialize)]
pub struct UserAnnouncementOptions {
	#[serde(rename = "pattern")]
	pub patterns: Vec<UserField>,
	#[serde(rename = "tense_pattern")]
	pub tense_patterns: Option<Vec<TensedUserField>>,
	pub conjunctions: Option<Vec<String>>,
	pub tags_to_announce: Option<FieldsToAnnounce>,
}

impl UserAnnouncementOptions {
	pub fn en_default_script_json() -> String {
		let opts: Self = toml::from_str(&Self::en_default_script_toml()).unwrap();
		serde_json::to_string(&opts).unwrap()
	}

	#[cfg(test)]
	pub fn tutorial_script_toml() -> String {
		include_str!("tutorial_script.toml").to_owned()
	}

	pub fn en_default_script_toml() -> String {
		include_str!("en_default_script.toml").to_owned()
	}

	#[cfg(test)]
	pub fn hi_default_script_toml() -> String {
		include_str!("hi_default_script.toml").to_owned()
	}

	#[cfg(test)]
	pub fn en_default() -> Self {
		toml::from_str(&UserAnnouncementOptions::en_default_script_toml()).unwrap()
	}

	#[cfg(test)]
	pub fn hi_default() -> Self {
		toml::from_str(&UserAnnouncementOptions::hi_default_script_toml()).unwrap()
	}

	#[cfg(test)]
	pub fn tutorial_default() -> Self {
		toml::from_str(&UserAnnouncementOptions::tutorial_script_toml()).unwrap()
	}
}

impl Default for UserAnnouncementOptions {
	fn default() -> Self {
		Self {
			patterns: vec![],
			tense_patterns: None,
			conjunctions: None,
			tags_to_announce: Some(FieldsToAnnounce::default()),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_default_scripts() {
		let hi = UserAnnouncementOptions::hi_default_script_toml();
		let en = UserAnnouncementOptions::en_default_script_toml();
		let ex = UserAnnouncementOptions::tutorial_script_toml();

		assert_ne!(hi, en);
		assert_ne!(hi, ex);
		assert_ne!(en, ex);

		let _hi_uo: UserAnnouncementOptions = toml::from_str(&hi).unwrap();
		let _en_uo: UserAnnouncementOptions = toml::from_str(&hi).unwrap();
		let _ex_uo: UserAnnouncementOptions = toml::from_str(&hi).unwrap();

		let _hi = UserAnnouncementOptions::hi_default();
		let _en = UserAnnouncementOptions::en_default();
		let _ex = UserAnnouncementOptions::tutorial_default();
	}
}
