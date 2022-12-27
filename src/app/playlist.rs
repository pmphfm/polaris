use core::clone::Clone;
use diesel::prelude::*;
use diesel::BelongingToDsl;
use serde::{Deserialize, Serialize};
use std::ffi::{OsStr, OsString};
use std::fmt::Write;
use std::path::{Path, PathBuf, MAIN_SEPARATOR};

use crate::app::index::Song;
use crate::app::vfs;
use crate::db::{self, playlist_songs, playlists, songs, users, DB};

mod m3u;

pub use m3u::*;

#[derive(thiserror::Error, Debug)]
pub enum Error {
	#[error(transparent)]
	Database(#[from] diesel::result::Error),
	#[error(transparent)]
	DatabaseConnection(#[from] db::Error),
	#[error("User not found")]
	UserNotFound,
	#[error("Playlist not found: {0}")]
	PlaylistNotFound(String),
	#[error(transparent)]
	Vfs(#[from] vfs::Error),
}

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
				.optional()?
				.ok_or(Error::UserNotFound)?
		};

		{
			use self::playlists::dsl::*;
			let found_playlists: Vec<String> = Playlist::belonging_to(&user)
				.select(name)
				.load(&mut connection)?;
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
					.optional()?
					.ok_or(Error::UserNotFound)?
			};

			// Create playlist
			new_playlist = NewPlaylist {
				name: playlist_name.into(),
				owner: user.id,
			};

			diesel::insert_into(playlists::table)
				.values(&new_playlist)
				.execute(&mut connection)?;

			playlist = {
				use self::playlists::dsl::*;
				playlists
					.select((id, owner))
					.filter(name.eq(playlist_name).and(owner.eq(user.id)))
					.get_result(&mut connection)?
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
			connection.transaction::<_, diesel::result::Error, _>(|connection| {
				// Delete old content (if any)
				let old_songs = PlaylistSong::belonging_to(&playlist);
				diesel::delete(old_songs).execute(connection)?;

				// Insert content
				diesel::insert_into(playlist_songs::table)
					.values(&new_songs)
					.execute(&mut *connection)?; // TODO https://github.com/diesel-rs/diesel/issues/1822
				Ok(())
			})?;
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
					.optional()?
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
					.get_results(&mut connection)?
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
					.get_results(&mut connection)?
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
				.optional()?
				.ok_or(Error::UserNotFound)?
		};

		{
			use self::playlists::dsl::*;
			let q = Playlist::belonging_to(&user).filter(name.eq(playlist_name));
			match diesel::delete(q).execute(&mut connection)? {
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

#[cfg(test)]
mod test {

	use std::path::{Path, PathBuf};
	use std::str::FromStr;

	use crate::app::playlist::{
		strip_base_path, PlaylistExport, PlaylistType, M3U_COMMON_PATH, M3U_HEADER, M3U_RMIM_FIELDS,
	};
	use crate::app::test;
	use crate::test_name;

	const TEST_USER: &str = "test_user";
	const TEST_PASSWORD: &str = "password";
	const TEST_PLAYLIST_NAME: &str = "Chill & Grill";
	const TEST_MOUNT_NAME: &str = "root";
	const TEST_ALL_SONGS_COUNT: usize = 13;

	fn test_songs_path() -> String {
		let songs_path: PathBuf = ["test-data", "small-collection"].iter().collect();
		songs_path.to_string_lossy().into_owned()
	}

	#[test]
	fn save_playlist_golden_path() {
		let ctx = test::ContextBuilder::new(test_name!())
			.user(TEST_USER, TEST_PASSWORD, false)
			.build();

		ctx.playlist_manager
			.save_playlist(TEST_PLAYLIST_NAME, TEST_USER, &Vec::new())
			.unwrap();

		let found_playlists = ctx.playlist_manager.list_playlists(TEST_USER).unwrap();
		assert_eq!(found_playlists.len(), 1);
		assert_eq!(found_playlists[0], TEST_PLAYLIST_NAME);
	}

	#[test]
	fn save_playlist_is_idempotent() {
		let ctx = test::ContextBuilder::new(test_name!())
			.user(TEST_USER, TEST_PASSWORD, false)
			.mount(TEST_MOUNT_NAME, &test_songs_path())
			.build();

		ctx.index.update().unwrap();

		let playlist_content: Vec<String> = ctx
			.index
			.flatten(Path::new(TEST_MOUNT_NAME))
			.unwrap()
			.into_iter()
			.map(|s| s.path)
			.collect();
		assert_eq!(playlist_content.len(), TEST_ALL_SONGS_COUNT);

		ctx.playlist_manager
			.save_playlist(TEST_PLAYLIST_NAME, TEST_USER, &playlist_content)
			.unwrap();

		ctx.playlist_manager
			.save_playlist(TEST_PLAYLIST_NAME, TEST_USER, &playlist_content)
			.unwrap();

		let songs = ctx
			.playlist_manager
			.read_playlist(TEST_PLAYLIST_NAME, TEST_USER)
			.unwrap();
		assert_eq!(songs.len(), TEST_ALL_SONGS_COUNT);
	}

	#[test]
	fn delete_playlist_golden_path() {
		let ctx = test::ContextBuilder::new(test_name!())
			.user(TEST_USER, TEST_PASSWORD, false)
			.build();

		let playlist_content = Vec::new();

		ctx.playlist_manager
			.save_playlist(TEST_PLAYLIST_NAME, TEST_USER, &playlist_content)
			.unwrap();

		ctx.playlist_manager
			.delete_playlist(TEST_PLAYLIST_NAME, TEST_USER)
			.unwrap();

		let found_playlists = ctx.playlist_manager.list_playlists(TEST_USER).unwrap();
		assert_eq!(found_playlists.len(), 0);
	}

	#[test]
	fn read_playlist_golden_path() {
		let ctx = test::ContextBuilder::new(test_name!())
			.user(TEST_USER, TEST_PASSWORD, false)
			.mount(TEST_MOUNT_NAME, &test_songs_path())
			.build();

		ctx.index.update().unwrap();

		let playlist_content: Vec<String> = ctx
			.index
			.flatten(Path::new(TEST_MOUNT_NAME))
			.unwrap()
			.into_iter()
			.map(|s| s.path)
			.collect();
		assert_eq!(playlist_content.len(), TEST_ALL_SONGS_COUNT);

		ctx.playlist_manager
			.save_playlist(TEST_PLAYLIST_NAME, TEST_USER, &playlist_content)
			.unwrap();

		let songs = ctx
			.playlist_manager
			.read_playlist(TEST_PLAYLIST_NAME, TEST_USER)
			.unwrap();

		assert_eq!(songs.len(), TEST_ALL_SONGS_COUNT);
		assert_eq!(songs[0].title, Some("Above The Water".to_owned()));

		let first_song_path: PathBuf = [
			TEST_MOUNT_NAME,
			"Khemmis",
			"Hunted",
			"01 - Above The Water.mp3",
		]
		.iter()
		.collect();
		assert_eq!(songs[0].path, first_song_path.to_str().unwrap());
	}

	#[test]
	fn read_playlist_with_broken_path() {
		let ctx = test::ContextBuilder::new(test_name!())
			.user(TEST_USER, TEST_PASSWORD, false)
			.mount(TEST_MOUNT_NAME, &test_songs_path())
			.build();

		ctx.index.update().unwrap();
		let mut playlist_content: Vec<String> = ctx
			.index
			.flatten(Path::new(TEST_MOUNT_NAME))
			.unwrap()
			.into_iter()
			.map(|s| s.path)
			.collect();
		assert_eq!(playlist_content.len(), TEST_ALL_SONGS_COUNT);
		let error_song_path = format!("{}-not-found.mp3", playlist_content[0]);
		playlist_content.push(error_song_path.clone());
		ctx.playlist_manager
			.save_playlist(TEST_PLAYLIST_NAME, TEST_USER, &playlist_content)
			.unwrap();

		let songs = ctx
			.playlist_manager
			.read_playlist(TEST_PLAYLIST_NAME, TEST_USER)
			.unwrap();

		assert_eq!(songs.len(), TEST_ALL_SONGS_COUNT + 1);

		let first_song_path: PathBuf = [
			TEST_MOUNT_NAME,
			"Khemmis",
			"Hunted",
			"01 - Above The Water.mp3",
		]
		.iter()
		.collect();
		assert_eq!(songs[0].path, first_song_path.to_str().unwrap());
		let error_song = &songs[songs.len() - 1];
		let mut error_song_real_path = PathBuf::from_str(&test_songs_path()).unwrap();
		error_song_real_path.push("Khemmis");
		error_song_real_path.push("Hunted");
		error_song_real_path.push("01 - Above The Water.mp3-not-found.mp3");

		assert_eq!(
			error_song.title,
			Some(format!("error {}", error_song_real_path.to_str().unwrap()))
		);
		assert_eq!(error_song.artist, Some(format!("error artist")));
		assert_eq!(error_song.album, Some(format!("error album")));
		assert_eq!(error_song.path, error_song_path);
	}

	#[test]
	fn test_export_playlist() {
		let ctx = test::ContextBuilder::new(test_name!())
			.user(TEST_USER, TEST_PASSWORD, false)
			.mount(TEST_MOUNT_NAME, "test-data/small-collection")
			.build();

		ctx.index.update().unwrap();

		let all_songs = ctx.index.flatten(Path::new(TEST_MOUNT_NAME)).unwrap();
		let playlist_content: Vec<String> = all_songs.iter().map(|s| s.path.clone()).collect();
		assert_eq!(playlist_content.len(), TEST_ALL_SONGS_COUNT);
		ctx.playlist_manager
			.save_playlist(TEST_PLAYLIST_NAME, TEST_USER, &playlist_content)
			.unwrap();

		ctx.playlist_manager
			.save_playlist(TEST_PLAYLIST_NAME, TEST_USER, &playlist_content)
			.unwrap();

		let all_songs = ctx
			.playlist_manager
			.read_playlist_real(TEST_PLAYLIST_NAME, TEST_USER)
			.unwrap();

		let found = ctx
			.playlist_manager
			.export_playlist(
				TEST_USER,
				PlaylistExport {
					name: TEST_PLAYLIST_NAME.to_string(),
					kind: Some(PlaylistType::m3u),
				},
			)
			.unwrap();
		let (common_path, buffer) = strip_base_path(&all_songs);
		let expected = format!(
			"{}\n{} {}={}\n{}",
			M3U_HEADER, M3U_RMIM_FIELDS, M3U_COMMON_PATH, common_path, buffer
		);
		assert_eq!(expected, found);
	}
}
