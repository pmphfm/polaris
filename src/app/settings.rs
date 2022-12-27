use diesel::prelude::*;
use regex::Regex;
use serde::Deserialize;
use std::convert::TryInto;
use std::time::Duration;

use crate::app::rj::{AdminSettings, UserSettings};
use crate::db::{misc_settings, DB};

mod error;

pub use error::*;

#[derive(Clone, Default)]
pub struct AuthSecret {
	pub key: [u8; 32],
}

#[derive(Debug, Queryable)]
pub struct Settings {
	pub index_sleep_duration_seconds: i32,
	pub index_album_art_pattern: String,
}

#[derive(Debug, Default, Deserialize)]
pub struct NewSettings {
	pub reindex_every_n_seconds: Option<i32>,
	pub album_art_pattern: Option<String>,
}

#[derive(Clone)]
pub struct Manager {
	pub db: DB,
}

impl Manager {
	pub fn new(db: DB) -> Self {
		Self { db }
	}

	pub fn get_auth_secret(&self) -> Result<AuthSecret, Error> {
		use self::misc_settings::dsl::*;
		let mut connection = self.db.connect()?;
		let secret: Vec<u8> = misc_settings
			.select(auth_secret)
			.get_result(&mut connection)
			.map_err(|e| match e {
				diesel::result::Error::NotFound => Error::AuthSecretNotFound,
				_ => Error::Unspecified,
			})?;
		secret
			.try_into()
			.map_err(|_| Error::InvalidAuthSecret)
			.map(|key| AuthSecret { key })
	}

	pub fn get_index_sleep_duration(&self) -> Result<Duration, Error> {
		use self::misc_settings::dsl::*;
		let mut connection = self.db.connect()?;
		misc_settings
			.select(index_sleep_duration_seconds)
			.get_result(&mut connection)
			.map_err(|e| match e {
				diesel::result::Error::NotFound => Error::IndexSleepDurationNotFound,
				_ => Error::Unspecified,
			})
			.map(|s: i32| Duration::from_secs(s as u64))
	}

	pub fn get_index_album_art_pattern(&self) -> Result<Regex, Error> {
		use self::misc_settings::dsl::*;
		let mut connection = self.db.connect()?;
		misc_settings
			.select(index_album_art_pattern)
			.get_result(&mut connection)
			.map_err(|e| match e {
				diesel::result::Error::NotFound => Error::IndexAlbumArtPatternNotFound,
				_ => Error::Unspecified,
			})
			.and_then(|s: String| {
				Regex::new(&format!("(?i){}", &s)).map_err(|_| Error::IndexAlbumArtPatternInvalid)
			})
	}

	pub fn read(&self) -> Result<Settings, Error> {
		use self::misc_settings::dsl::*;
		let mut connection = self.db.connect()?;

		let settings: Settings = misc_settings
			.select((index_sleep_duration_seconds, index_album_art_pattern))
			.get_result(&mut connection)
			.map_err(|_| Error::Unspecified)?;

		Ok(settings)
	}

	pub fn amend(&self, new_settings: &NewSettings) -> Result<(), Error> {
		let mut connection = self.db.connect()?;

		if let Some(sleep_duration) = new_settings.reindex_every_n_seconds {
			diesel::update(misc_settings::table)
				.set(misc_settings::index_sleep_duration_seconds.eq(sleep_duration as i32))
				.execute(&mut connection)
				.map_err(|_| Error::Unspecified)?;
		}

		if let Some(ref album_art_pattern) = new_settings.album_art_pattern {
			diesel::update(misc_settings::table)
				.set(misc_settings::index_album_art_pattern.eq(album_art_pattern))
				.execute(&mut connection)
				.map_err(|_| Error::Unspecified)?;
		}

		Ok(())
	}

	pub fn get_rj_user_settings(&self) -> Result<UserSettings, Error> {
		use crate::db::rj_user_settings::dsl::*;
		let mut connection = self.db.connect()?;
		let (user_scripts, enable, person_names): (Option<String>, Option<i32>, String) =
			rj_user_settings
				.select((scripts, enable_by_default, tts_people))
				.get_result(&mut connection)
				.map_err(|e| match e {
					diesel::result::Error::NotFound => Error::IndexSleepDurationNotFound,
					_ => Error::Unspecified,
				})?;

		Ok(UserSettings {
			scripts: user_scripts,
			enable_by_default: enable.map(|f| f != 0),
			tts_people: serde_json::from_str(&person_names).unwrap(),
		})
	}

	pub fn get_rj_admin_settings(&self) -> Result<AdminSettings, Error> {
		use crate::db::rj_admin_settings::dsl::*;
		let mut connection = self.db.connect()?;
		let (url, key, enable_ssml): (Option<String>, Option<String>, i32) = rj_admin_settings
			.select((tts_service_url, tts_text_param_key, tts_enable_ssml))
			.get_result::<(Option<String>, Option<String>, i32)>(&mut connection)
			.map_err(|e| match e {
				diesel::result::Error::NotFound => Error::IndexSleepDurationNotFound,
				_ => Error::Unspecified,
			})?;
		Ok(AdminSettings {
			tts_url: url,
			tts_key: key,
			enable_ssml: enable_ssml != 0,
		})
	}

	pub fn put_rj_user_settings(&self, new_settings: &UserSettings) -> Result<(), Error> {
		use crate::db::rj_user_settings;
		let mut connection = self.db.connect()?;

		if let Some(user_script) = &new_settings.scripts {
			diesel::update(rj_user_settings::table)
				.set(rj_user_settings::scripts.eq(user_script))
				.execute(&mut connection)
				.map_err(|_| Error::Unspecified)?;
		}

		if let Some(enable) = new_settings.enable_by_default {
			diesel::update(rj_user_settings::table)
				.set(rj_user_settings::enable_by_default.eq(enable as i32))
				.execute(&mut connection)
				.map_err(|_| Error::Unspecified)?;
		}

		let person_names = serde_json::to_string(&new_settings.tts_people).unwrap();
		diesel::update(rj_user_settings::table)
			.set(rj_user_settings::tts_people.eq(person_names))
			.execute(&mut connection)
			.map_err(|_| Error::Unspecified)?;
		Ok(())
	}

	pub fn put_rj_admin_settings(&self, new_settings: &AdminSettings) -> Result<(), Error> {
		use crate::db::rj_admin_settings;
		let mut connection = self.db.connect()?;

		if let Some(url) = &new_settings.tts_url {
			diesel::update(rj_admin_settings::table)
				.set(rj_admin_settings::tts_service_url.eq(url))
				.execute(&mut connection)
				.map_err(|_| Error::Unspecified)?;
		}

		if let Some(key) = &new_settings.tts_key {
			diesel::update(rj_admin_settings::table)
				.set(rj_admin_settings::tts_text_param_key.eq(key))
				.execute(&mut connection)
				.map_err(|_| Error::Unspecified)?;
		}

		diesel::update(rj_admin_settings::table)
			.set(rj_admin_settings::tts_enable_ssml.eq(new_settings.enable_ssml as i32))
			.execute(&mut connection)
			.map_err(|_| Error::Unspecified)?;
		Ok(())
	}
}
