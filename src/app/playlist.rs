use anyhow::Result;
use core::clone::Clone;
use diesel::prelude::*;
use diesel::BelongingToDsl;
use serde::{Deserialize, Serialize};
use std::ffi::{OsStr, OsString};
use std::fmt::Write;
use std::path::{Path, PathBuf, MAIN_SEPARATOR};

use crate::app::index::Song;
use crate::app::vfs;
use crate::db::{playlist_songs, playlists, songs, users, DB};

mod error;
mod m3u;
#[cfg(test)]
mod test;

pub use error::*;
pub use m3u::*;

#[allow(non_camel_case_types)]
#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub enum PlaylistType {
	m3u,
}

impl Default for PlaylistType {
	fn default() -> Self {
		Self::m3u
	}
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PlaylistExport {
	pub name: String,
	pub kind: Option<PlaylistType>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PlaylistImport {
	// Playlist name to save as.
	pub name: String,

	// Type of input playlist.
	pub kind: Option<PlaylistType>,

	// If true, partial playlist is imported.
	pub partial: Option<bool>,

	// If true, tries to match imperfect matches.
	pub fuzzy_match: Option<bool>,
}

#[derive(Clone)]
pub struct Manager {
	db: DB,
	vfs_manager: vfs::Manager,
}

impl Manager {
	pub fn new(db: DB, vfs_manager: vfs::Manager) -> Self {
		Self { db, vfs_manager }
	}

	pub fn list_playlists(&self, owner: &str) -> Result<Vec<String>, Error> {
		let mut connection = self.db.connect()?;

		let user: User = {
			use self::users::dsl::*;
			users
				.filter(name.eq(owner))
				.select((id,))
				.first(&mut connection)
				.optional()
				.map_err(anyhow::Error::new)?
				.ok_or(Error::UserNotFound)?
		};

		{
			use self::playlists::dsl::*;
			let found_playlists: Vec<String> = Playlist::belonging_to(&user)
				.select(name)
				.load(&mut connection)
				.map_err(anyhow::Error::new)?;
			Ok(found_playlists)
		}
	}

	pub fn save_playlist(
		&self,
		playlist_name: &str,
		owner: &str,
		content: &[String],
	) -> Result<(), Error> {
		let new_playlist: NewPlaylist;
		let playlist: Playlist;
		let vfs = self.vfs_manager.get_vfs()?;

		{
			let mut connection = self.db.connect()?;

			// Find owner
			let user: User = {
				use self::users::dsl::*;
				users
					.filter(name.eq(owner))
					.select((id,))
					.first(&mut connection)
					.optional()
					.map_err(anyhow::Error::new)?
					.ok_or(Error::UserNotFound)?
			};

			// Create playlist
			new_playlist = NewPlaylist {
				name: playlist_name.into(),
				owner: user.id,
			};

			diesel::insert_into(playlists::table)
				.values(&new_playlist)
				.execute(&mut connection)
				.map_err(anyhow::Error::new)?;

			playlist = {
				use self::playlists::dsl::*;
				playlists
					.select((id, owner))
					.filter(name.eq(playlist_name).and(owner.eq(user.id)))
					.get_result(&mut connection)
					.map_err(anyhow::Error::new)?
			}
		}

		let mut new_songs: Vec<NewPlaylistSong> = Vec::new();
		new_songs.reserve(content.len());

		for (i, path) in content.iter().enumerate() {
			let virtual_path = Path::new(&path);
			if let Some(real_path) = vfs
				.virtual_to_real(virtual_path)
				.ok()
				.and_then(|p| p.to_str().map(|s| s.to_owned()))
			{
				new_songs.push(NewPlaylistSong {
					playlist: playlist.id,
					path: real_path,
					ordering: i as i32,
				});
			}
		}

		{
			let mut connection = self.db.connect()?;
			connection
				.transaction::<_, diesel::result::Error, _>(|connection| {
					// Delete old content (if any)
					let old_songs = PlaylistSong::belonging_to(&playlist);
					diesel::delete(old_songs).execute(connection)?;

					// Insert content
					diesel::insert_into(playlist_songs::table)
						.values(&new_songs)
						.execute(&mut *connection)?; // TODO https://github.com/diesel-rs/diesel/issues/1822
					Ok(())
				})
				.map_err(anyhow::Error::new)?;
		}

		Ok(())
	}

	pub fn read_playlist_real(&self, playlist_name: &str, owner: &str) -> Result<Vec<Song>, Error> {
		let songs: Vec<Song>;
		let song_paths: Vec<String>;

		{
			let mut connection = self.db.connect()?;

			// Find owner
			let user: User = {
				use self::users::dsl::*;
				users
					.filter(name.eq(owner))
					.select((id,))
					.first(&mut connection)
					.optional()
					.map_err(anyhow::Error::new)?
					.ok_or(Error::UserNotFound)?
			};

			// Find playlist
			let playlist: Playlist = {
				use self::playlists::dsl::*;
				playlists
					.select((id, owner))
					.filter(name.eq(playlist_name).and(owner.eq(user.id)))
					.get_result(&mut connection)
					.optional()
					.map_err(|_| Error::PlaylistNotFound(playlist_name.to_string()))?
					.ok_or_else(|| Error::PlaylistNotFound(playlist_name.to_string()))?
			};
			let pid = playlist.id;

			song_paths = {
				use self::playlist_songs::dsl::*;
				playlist_songs
					.filter(playlist.eq(pid))
					.select(path)
					.order_by(ordering)
					.get_results(&mut connection)
					.map_err(anyhow::Error::new)?
			};

			songs = {
				use self::playlist_songs::dsl::{path as playlist_path, *};
				use self::songs::dsl::{id, path, *};
				playlist_songs
					.inner_join(songs.on(path.eq(playlist_path)))
					.select((
						id,
						path,
						parent,
						track_number,
						disc_number,
						title,
						artist,
						album_artist,
						year,
						album,
						artwork,
						duration,
						lyricist,
						composer,
						genre,
						label,
					))
					.get_results(&mut connection)
					.map_err(anyhow::Error::new)?
			};

			// Select songs. Not using Diesel because we need to LEFT JOIN using a custom column
			// 	let query = diesel::sql_query(
			// 		r#"
			// 	SELECT s.id, s.path, s.parent, s.track_number, s.disc_number, s.title, s.artist, s.album_artist, s.year, s.album, s.artwork, s.duration, s.lyricist, s.composer, s.genre, s.label
			// 	FROM playlist_songs ps
			// 	JOIN songs s ON ps.path = s.path
			// 	WHERE ps.playlist = ?
			// 	ORDER BY ps.ordering
			// "#,
			// 	);
			// 	let query = query.bind::<sql_types::Integer, _>(playlist.id);
			// 	songs = query.get_results(&connection).map_err(anyhow::Error::new)?;
		}

		let mut map = std::collections::HashMap::new();
		for (index, song) in songs.iter().enumerate() {
			map.insert(&song.path, index);
		}
		let mut missing_songs = Vec::new();
		for path in &song_paths {
			missing_songs.push(match map.get(path) {
				Some(index) => songs[*index].clone(),
				None => Song::error_song(path),
			});
		}

		log::error!("missing_songs {:?}", missing_songs);
		log::error!("songs {:?}", songs);
		log::error!("paths{:?}", song_paths);
		Ok(missing_songs)
	}

	pub fn read_playlist(&self, playlist_name: &str, owner: &str) -> Result<Vec<Song>, Error> {
		let vfs = self.vfs_manager.get_vfs()?;
		let songs = self.read_playlist_real(playlist_name, owner)?;

		// Map real path to virtual paths
		let virtual_songs = songs
			.into_iter()
			.filter_map(|s| s.virtualize(&vfs))
			.collect();

		Ok(virtual_songs)
	}

	pub fn delete_playlist(&self, playlist_name: &str, owner: &str) -> Result<(), Error> {
		let mut connection = self.db.connect()?;

		let user: User = {
			use self::users::dsl::*;
			users
				.filter(name.eq(owner))
				.select((id,))
				.first(&mut connection)
				.optional()
				.map_err(anyhow::Error::new)?
				.ok_or(Error::UserNotFound)?
		};

		{
			use self::playlists::dsl::*;
			let q = Playlist::belonging_to(&user).filter(name.eq(playlist_name));
			match diesel::delete(q)
				.execute(&mut connection)
				.map_err(anyhow::Error::new)?
			{
				0 => Err(Error::PlaylistNotFound(playlist_name.to_string())),
				_ => Ok(()),
			}
		}
	}

	pub fn export_playlist(&self, username: &str, export: PlaylistExport) -> Result<String, Error> {
		let songs = self.read_playlist_real(&export.name, username)?;
		create_m3u_playlist(&songs)
	}
}

#[derive(Identifiable, Queryable, Associations)]
#[diesel(belongs_to(User, foreign_key = owner))]
struct Playlist {
	id: i32,
	owner: i32,
}

#[derive(Identifiable, Queryable, Associations)]
#[diesel(belongs_to(Playlist, foreign_key = playlist))]
struct PlaylistSong {
	id: i32,
	playlist: i32,
}

#[derive(Insertable)]
#[diesel(table_name = playlists)]
struct NewPlaylist {
	name: String,
	owner: i32,
}

#[derive(Insertable)]
#[diesel(table_name = playlist_songs)]
struct NewPlaylistSong {
	playlist: i32,
	path: String,
	ordering: i32,
}

#[derive(Identifiable, Queryable)]
struct User {
	id: i32,
}

fn get_common_path(songs: &[Song]) -> Option<OsString> {
	if songs.len() < 2 {
		return None;
	}
	let mut common_path = PathBuf::from(&songs.get(0).unwrap().path);
	for song in &songs[1..] {
		let next_path = Path::new(&song.path);
		let iter = common_path.iter().zip(next_path.iter());
		let mut temp = PathBuf::new();
		for (c, n) in iter {
			if c == n {
				temp.push(c);
			} else {
				break;
			}
		}
		common_path = temp;
		if common_path.as_os_str().is_empty() {
			return None;
		}
	}
	let mut path = common_path.into_os_string();
	path.push(OsStr::new(&MAIN_SEPARATOR.to_string()));
	Some(path)
}

// Returns (common_path, buffer with with list of files).
pub(crate) fn strip_base_path(songs: &[Song]) -> (String, String) {
	let base_path = get_common_path(songs)
		.unwrap_or_else(|| OsString::from(""))
		.to_string_lossy()
		.to_string();
	let mut buffer = String::new();

	for song in songs {
		writeln!(
			&mut buffer,
			"{}",
			song.path.strip_prefix(&base_path).unwrap()
		)
		.unwrap();
	}
	(base_path, buffer)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_no_songs() {
		assert_eq!(strip_base_path(&[]), ("".to_string(), "".to_string()));
	}

	#[test]
	fn test_single_song() {
		assert_eq!(
			strip_base_path(&[Song::test_only_from_path("abc/def")]),
			("".to_string(), "abc/def\n".to_string())
		);
	}

	#[test]
	fn test_unique_paths() {
		assert_eq!(
			strip_base_path(&[
				Song::test_only_from_path("abc/def"),
				Song::test_only_from_path("def/ghi")
			]),
			("".to_string(), "abc/def\ndef/ghi\n".to_string())
		);
	}

	#[test]
	fn test_unique_paths_common_files() {
		assert_eq!(
			strip_base_path(&[
				Song::test_only_from_path("abc/def"),
				Song::test_only_from_path("def/def")
			]),
			("".to_string(), "abc/def\ndef/def\n".to_string())
		);
	}

	#[test]
	fn test_few_chars_common() {
		assert_eq!(
			strip_base_path(&[
				Song::test_only_from_path("abc/def"),
				Song::test_only_from_path("abf/ghi")
			]),
			("".to_string(), "abc/def\nabf/ghi\n".to_string())
		);
	}

	#[test]
	fn test_single_directory_common() {
		assert_eq!(
			strip_base_path(&[
				Song::test_only_from_path("abc/def"),
				Song::test_only_from_path("abc/ghi")
			]),
			("abc/".to_string(), "def\nghi\n".to_string())
		);
	}

	#[test]
	fn test_few_directories_common() {
		assert_eq!(
			strip_base_path(&[
				Song::test_only_from_path("a/bc/d/ef"),
				Song::test_only_from_path("a/bc/g/hi"),
				Song::test_only_from_path("a/bc/j/kl")
			]),
			("a/bc/".to_string(), "d/ef\ng/hi\nj/kl\n".to_string())
		);
	}
}
