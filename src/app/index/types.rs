use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::app::vfs::VFS;
use crate::db::songs;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CollectionFile {
	Directory(Directory),
	Song(Song),
}

#[derive(Debug, PartialEq, Eq, Queryable, QueryableByName, Serialize, Deserialize, Clone)]
#[table_name = "songs"]
pub struct Song {
	#[serde(skip_serializing, skip_deserializing)]
	id: i32,
	pub path: String,
	#[serde(skip_serializing, skip_deserializing)]
	pub parent: String,
	pub track_number: Option<i32>,
	pub disc_number: Option<i32>,
	pub title: Option<String>,
	pub artist: Option<String>,
	pub album_artist: Option<String>,
	pub year: Option<i32>,
	pub album: Option<String>,
	pub artwork: Option<String>,
	pub duration: Option<i32>,
	pub lyricist: Option<String>,
	pub composer: Option<String>,
	pub genre: Option<String>,
	pub label: Option<String>,
}

impl Song {
	pub fn virtualize(mut self, vfs: &VFS) -> Option<Song> {
		self.path = match vfs.real_to_virtual(Path::new(&self.path)) {
			Ok(p) => p.to_string_lossy().into_owned(),
			_ => return None,
		};
		if let Some(artwork_path) = self.artwork {
			self.artwork = match vfs.real_to_virtual(Path::new(&artwork_path)) {
				Ok(p) => Some(p.to_string_lossy().into_owned()),
				_ => None,
			};
		}
		Some(self)
	}

	pub fn error_song(path: &str) -> Self {
		Song {
			id: 0,
			path: path.to_string(),
			parent: path.to_string(),
			track_number: None,
			disc_number: None,
			title: Some(format!("error {}", path)),
			artist: Some("error artist".to_string()),
			album_artist: None,
			year: None,
			album: Some("error album".to_string()),
			artwork: None,
			duration: None,
			lyricist: None,
			composer: None,
			genre: None,
			label: None,
		}
	}

	#[cfg(test)]
	pub fn test_only_from_path(path: &str) -> Self {
		Song {
			id: 0,
			path: path.to_string(),
			parent: "".to_string(),
			track_number: None,
			disc_number: None,
			title: None,
			artist: None,
			album_artist: None,
			year: None,
			album: None,
			artwork: None,
			duration: None,
			lyricist: None,
			composer: None,
			genre: None,
			label: None,
		}
	}
}

#[derive(Debug, PartialEq, Eq, Queryable, Serialize, Deserialize)]
pub struct Directory {
	#[serde(skip_serializing, skip_deserializing)]
	id: i32,
	pub path: String,
	#[serde(skip_serializing, skip_deserializing)]
	pub parent: Option<String>,
	pub artist: Option<String>,
	pub year: Option<i32>,
	pub album: Option<String>,
	pub artwork: Option<String>,
	pub date_added: i32,
}

impl Directory {
	pub fn virtualize(mut self, vfs: &VFS) -> Option<Directory> {
		self.path = match vfs.real_to_virtual(Path::new(&self.path)) {
			Ok(p) => p.to_string_lossy().into_owned(),
			_ => return None,
		};
		if let Some(artwork_path) = self.artwork {
			self.artwork = match vfs.real_to_virtual(Path::new(&artwork_path)) {
				Ok(p) => Some(p.to_string_lossy().into_owned()),
				_ => None,
			};
		}
		Some(self)
	}
}

#[derive(Deserialize, Debug, Queryable, Serialize)]
pub struct RjRequest {
	pub prev: Option<String>,
	pub next: Option<String>,
	pub next_next: Option<String>,
}
