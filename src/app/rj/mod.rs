mod announce;
pub mod error;
mod parse;
mod script;

#[cfg(test)]
mod test;
mod user_opts;

pub use announce::*;

use crate::app::index::Song;
pub use error::ParseError;
use script::ScriptCache;
use serde::{Deserialize, Serialize};
use std::io::Read;
use ureq;

static SSML_HEADER_OPEN: &str = r#"<speak version='1.0' xmlns='http://www.w3.org/2001/10/synthesis' xmlns:mstts='http://www.w3.org/2001/mstts' xmlns:emo='http://www.w3.org/2009/10/emotionml' xml:lang="#;
static SSML_VOICE_ELEMENT_OPEN: &str = r#"<voice name="#;
static SSML_ELEMENT_CLOSE: &str = r#">"#;
static SSML_VOICE_ELEMENT_FOOTER: &str = r#"</voice>"#;
static SSML_FOOTER: &str = r#"</speak>"#;

/// The structure defines the profile of an RJ.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Person {
	name: String,
	voice_model: String,
	language: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct UserSettings {
	pub scripts: Option<String>,
	pub enable_by_default: Option<bool>,
	pub tts_people: Vec<Person>,
}

impl UserSettings {
	fn is_valid(&self) -> bool {
		self.scripts.is_some() && self.enable_by_default.is_some()
	}

	fn is_people_valid(&self) -> bool {
		if self.tts_people.is_empty() {
			return false;
		}

		for p in &self.tts_people {
			if p.name.is_empty() || p.voice_model.is_empty() || p.language.is_empty() {
				return false;
			}
		}
		true
	}
}

pub struct RestorableUserSettings {
	cache: Option<ScriptCache>,
	enable_by_default: bool,
	pub tts_people: Vec<Person>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AdminSettings {
	pub tts_url: Option<String>,
	pub tts_key: Option<String>,
	pub enable_ssml: bool,
}

impl AdminSettings {
	fn is_valid(&self) -> bool {
		self.tts_url.is_some() && self.tts_key.is_some()
	}
}

#[derive(Debug)]
pub struct Manager {
	enabled: bool,
	cache: Option<ScriptCache>,
	url: String,
	tts_key: String,
	enable_by_default: bool,
	enable_ssml: bool,
	tts_people: Vec<Person>,
}

static DEFAULT_URL: &str = "http://devel.lan:12345/api/tts";
static DEFAULT_TTS_KEY: &str = "text";

impl Default for Manager {
	fn default() -> Self {
		Self {
			enabled: false,
			cache: None,
			url: DEFAULT_URL.to_owned(),
			tts_key: DEFAULT_TTS_KEY.to_owned(),
			enable_by_default: false,
			enable_ssml: false,
			tts_people: vec![],
		}
	}
}

impl Manager {
	pub fn create(
		admin_settings: AdminSettings,
		user_settings: UserSettings,
	) -> Result<Manager, ParseError> {
		if admin_settings == AdminSettings::default() && user_settings == UserSettings::default() {
			return Ok(Manager::default());
		}

		if admin_settings.enable_ssml && !user_settings.is_people_valid() {
			return Ok(Manager::default());
		}

		if admin_settings.is_valid() && user_settings.is_valid() {
			return Ok(Manager {
				enabled: true,
				cache: Some(ScriptCache::create(
					user_settings.scripts.as_ref().unwrap(),
				)?),
				url: admin_settings.tts_url.to_owned().unwrap(),
				tts_key: admin_settings.tts_key.unwrap(),
				enable_by_default: user_settings.enable_by_default.unwrap(),
				enable_ssml: admin_settings.enable_ssml,
				tts_people: user_settings.tts_people,
			});
		}
		if admin_settings.is_valid() {
			return Ok(Manager {
				enabled: true,
				cache: Some(ScriptCache::default()),
				url: admin_settings.tts_url.to_owned().unwrap(),
				tts_key: admin_settings.tts_key.unwrap(),
				enable_by_default: false,
				enable_ssml: admin_settings.enable_ssml,
				tts_people: user_settings.tts_people,
			});
		}
		Ok(Manager::default())
	}

	fn get_current_host(&self) -> Option<&Person> {
		if !self.enable_ssml {
			return None;
		}
		Some(&self.tts_people[0])
	}

	pub fn get_announcement(
		&self,
		song: &Song,
		present: bool,
	) -> Result<String, error::ParseError> {
		if !self.enabled {
			return Err(ParseError::RjServiceDisabled);
		}
		Ok(self
			.cache
			.as_ref()
			.unwrap()
			.get_announcement(song, present, self.enable_ssml)
			.unwrap_or_else(|| "".to_owned()))
	}

	fn build_ssml_header(&self) -> String {
		assert!(self.enable_ssml);
		format!(
			r#"{}'{}'{}"#,
			SSML_HEADER_OPEN,
			self.get_current_host().unwrap().language,
			SSML_ELEMENT_CLOSE,
		)
	}

	fn build_ssml_voice(&self) -> String {
		assert!(self.enable_ssml);
		format!(
			r#"{}'{}'{}"#,
			SSML_VOICE_ELEMENT_OPEN,
			self.get_current_host().unwrap().voice_model,
			SSML_ELEMENT_CLOSE,
		)
	}

	pub fn build_packet(&self, script: String) -> String {
		if !self.enable_ssml {
			return script;
		}
		format!(
			r#"{}{}{}{}{}"#,
			&self.build_ssml_header(),
			&self.build_ssml_voice(),
			&script,
			SSML_VOICE_ELEMENT_FOOTER,
			SSML_FOOTER
		)
	}

	/// Gets announcement speech for a song.
	/// This is a blocking call and it may take really long to synthesize voice.
	/// Make sure that you call this on a thread that is not running async tasks.
	pub fn get_speech(&self, script: &str) -> Result<(String, Vec<u8>), ParseError> {
		if !self.enabled {
			return Err(ParseError::RjServiceDisabled);
		}
		let body = ureq::get(&self.url).query(&self.tts_key, script).call();
		let mut buf = vec![];
		let content_type = body.content_type().to_owned();
		body.into_reader()
			.read_to_end(&mut buf)
			.map_err(|op| ParseError::FailedToTTS(op.to_string()))?;
		Ok((content_type, buf))
	}

	/// Returns a randomly selected conjunction that can be used to join announcements of next song
	/// and the song after that.
	pub fn get_conjunction(&self) -> String {
		if let Some(cache) = &self.cache {
			return cache.get_conjunction();
		}
		"".to_string()
	}

	/// Updates script cache and enable_by_default.
	/// Nothing is changed on failure to parse/verify script.
	/// Returns old settings in a restore-able format.
	pub fn update_user_settings(
		&mut self,
		user_settings: UserSettings,
	) -> Result<RestorableUserSettings, ParseError> {
		if !user_settings.is_valid() {
			return Err(ParseError::InvalidInput(
				"arguments cannot be null".to_string(),
			));
		}
		let cache = ScriptCache::create(user_settings.scripts.as_ref().unwrap())?;
		let ret = RestorableUserSettings {
			cache: self.cache.take(),
			enable_by_default: self.enable_by_default,
			tts_people: self.tts_people.clone(),
		};
		self.cache = Some(cache);
		self.enable_by_default = user_settings.enable_by_default.unwrap();
		self.tts_people = user_settings.tts_people;
		Ok(ret)
	}

	pub fn restore_user_settings(&mut self, mut to_restore: RestorableUserSettings) {
		self.cache = to_restore.cache.take();
		self.enable_by_default = to_restore.enable_by_default;
		self.tts_people = to_restore.tts_people;
	}

	/// Updates TTS server details.
	/// Nothing is changed on failure to parse/verify script.
	/// Returns old settings.
	pub fn update_admin_settings(
		&mut self,
		admin_settings: AdminSettings,
	) -> Result<AdminSettings, ParseError> {
		if !admin_settings.is_valid() {
			return Err(ParseError::InvalidInput(
				"arguments cannot be null".to_string(),
			));
		}

		let old = AdminSettings {
			tts_url: Some(self.url.clone()),
			tts_key: Some(self.tts_key.clone()),
			enable_ssml: self.enable_ssml,
		};
		self.url = admin_settings.tts_url.unwrap();
		self.tts_key = admin_settings.tts_key.unwrap();
		self.enable_ssml = admin_settings.enable_ssml;
		Ok(old)
	}
}
