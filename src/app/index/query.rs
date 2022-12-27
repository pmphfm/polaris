use diesel::dsl::sql;
use diesel::prelude::*;
use diesel::sql_types;
use regex::Regex;
use std::ops::Range;
use std::path::{Path, PathBuf};

use super::*;
use crate::db::{self, directories, songs};

// A token is one of the field of song structure followed by ':' and a word or words within a
// single or double quotes.
// query should contain only one occurrence of token.
// Ex. composer:"Some Composer" and lyricist:lyricist_name
fn parse_token(query: &str, token: &str) -> (Option<String>, String) {
	let mut substr = token.to_string();
	substr.push(':');
	let count = query.matches(&substr).count();

	if count == 0 || count > 1 {
		return (None, query.to_string());
	}

	// The query can be in the form
	// 1 'artist:artist_name generic_query'
	// 2 'generic_query artist:artist_name'
	// 3 'generic_query artist:artist_name generic_query'
	// In case of 2 and 3 we will have more than one string after split.
	let mut splits: Vec<&str> = query.split(&substr).collect();
	let mut query: String = "".to_string();
	if splits.len() > 1 {
		query.push_str(splits.remove(0).trim());
	}
	let re = Regex::new(r#""([^"]+)"|'([^']+)'|^([\w\-]+)"#).unwrap();
	let t = match re.find(splits[0]) {
		Some(x) => x,
		None => {
			return (None, query);
		}
	};
	let artist = "%".to_string()
		+ splits[0][t.start()..t.end()]
			.replace(['\'', '"'], "")
			.trim() + "%";
	let rest = splits[0][t.end()..].trim();

	if !rest.is_empty() {
		if query.is_empty() {
			query = rest.to_string();
		} else {
			query.push(' ');
			query.push_str(rest);
		}
	}

	if !artist.is_empty() {
		return (Some(artist), query.to_string());
	}

	(None, query)
}

fn parse_year(query: &str, token: &str) -> (Option<Range<i32>>, String) {
	let (raw_years, ret) = parse_token(query, token);

	println!("{:?}", raw_years);

	let raw_years = match raw_years {
		Some(x) => x.replace('%', ""),
		None => {
			return (None, ret);
		}
	};
	let hyphen_count = raw_years.matches('-').count();

	if hyphen_count > 1 {
		return (None, ret);
	}

	let string_years: Vec<&str> = raw_years.split('-').collect();
	println!("{:?}", string_years);
	let start = string_years[0].parse::<i32>();
	let mut end = Ok(0);
	if hyphen_count == 0 {
		if start.is_ok() {
			end = Ok(*start.as_ref().unwrap());
		}
	} else {
		end = string_years[1].parse::<i32>();
	}
	if start.is_err() || end.is_err() {
		return (None, ret);
	}

	(Some(start.unwrap()..end.unwrap() + 1_i32), ret)
}

#[derive(Default, Debug, PartialEq)]
pub struct QueryFields {
	pub title: Option<String>,
	pub artist: Option<String>,
	pub album_artist: Option<String>,
	pub album: Option<String>,
	pub lyricist: Option<String>,
	pub composer: Option<String>,
	pub genre: Option<String>,
	pub general_query: Option<String>,
	pub years: Option<Range<i32>>,
}

pub fn parse_query(query: &str) -> QueryFields {
	// Replace multiple spaces and trim leading and trailing spaces.
	let re = Regex::new(r"\s+").unwrap();
	let query = re.replace_all(&query.to_ascii_lowercase(), " ").to_string();
	let query = query.trim().to_string();
	let (title, query) = parse_token(&query, "title");
	let (album_artist, query) = parse_token(&query, "album_artist");
	let (artist, query) = parse_token(&query, "artist");
	let (album, query) = parse_token(&query, "album");
	let (lyricist, query) = parse_token(&query, "lyricist");
	let (composer, query) = parse_token(&query, "composer");
	let (genre, query) = parse_token(&query, "genre");
	let (years, query) = parse_year(&query, "year");
	QueryFields {
		title,
		artist,
		album_artist,
		album,
		lyricist,
		composer,
		genre,
		general_query: Some(query),
		years,
	}
}

#[derive(thiserror::Error, Debug)]
pub enum QueryError {
	#[error(transparent)]
	Database(#[from] diesel::result::Error),
	#[error(transparent)]
	DatabaseConnection(#[from] db::Error),
	#[error("Song was not found: `{0}`")]
	SongNotFound(PathBuf),
	#[error(transparent)]
	Vfs(#[from] vfs::Error),
}

sql_function!(
	#[aggregate]
	fn random() -> Integer;
);

impl Index {
	pub fn browse<P>(&self, virtual_path: P) -> Result<Vec<CollectionFile>, QueryError>
	where
		P: AsRef<Path>,
	{
		let mut output = Vec::new();
		let vfs = self.vfs_manager.get_vfs()?;
		let mut connection = self.db.connect()?;

		if virtual_path.as_ref().components().count() == 0 {
			// Browse top-level
			let real_directories: Vec<Directory> = directories::table
				.filter(directories::parent.is_null())
				.load(&mut connection)?;
			let virtual_directories = real_directories
				.into_iter()
				.filter_map(|d| d.virtualize(&vfs));
			output.extend(virtual_directories.map(CollectionFile::Directory));
		} else {
			// Browse sub-directory
			let real_path = vfs.virtual_to_real(virtual_path)?;
			let real_path_string = real_path.as_path().to_string_lossy().into_owned();

			let real_directories: Vec<Directory> = directories::table
				.filter(directories::parent.eq(&real_path_string))
				.order(sql::<sql_types::Bool>("path COLLATE NOCASE ASC"))
				.load(&mut connection)?;
			let virtual_directories = real_directories
				.into_iter()
				.filter_map(|d| d.virtualize(&vfs));
			output.extend(virtual_directories.map(CollectionFile::Directory));

			println!("Browse: {}", real_path_string);
			let real_songs: Vec<Song> = songs::table
				.filter(songs::parent.eq(&real_path_string))
				.order(sql::<sql_types::Bool>("path COLLATE NOCASE ASC"))
				.load(&mut connection)?;
			let virtual_songs = real_songs.into_iter().filter_map(|s| s.virtualize(&vfs));
			output.extend(virtual_songs.map(CollectionFile::Song));
		}

		Ok(output)
	}

	pub fn flatten<P>(&self, virtual_path: P) -> Result<Vec<Song>, QueryError>
	where
		P: AsRef<Path>,
	{
		use self::songs::dsl::*;
		let vfs = self.vfs_manager.get_vfs()?;
		let mut connection = self.db.connect()?;

		let real_songs: Vec<Song> = if virtual_path.as_ref().parent().is_some() {
			let real_path = vfs.virtual_to_real(virtual_path)?;
			let song_path_filter = {
				let mut path_buf = real_path;
				path_buf.push("%");
				path_buf.as_path().to_string_lossy().into_owned()
			};
			songs
				.filter(path.like(&song_path_filter))
				.order(path)
				.load(&mut connection)?
		} else {
			songs.order(path).load(&mut connection)?
		};

		let virtual_songs = real_songs.into_iter().filter_map(|s| s.virtualize(&vfs));
		Ok(virtual_songs.collect::<Vec<_>>())
	}

	pub fn get_random_albums(&self, count: i64) -> Result<Vec<Directory>, QueryError> {
		use self::directories::dsl::*;
		let vfs = self.vfs_manager.get_vfs()?;
		let mut connection = self.db.connect()?;
		let real_directories: Vec<Directory> = directories
			.filter(album.is_not_null())
			.limit(count)
			.order(random())
			.load(&mut connection)?;
		let virtual_directories = real_directories
			.into_iter()
			.filter_map(|d| d.virtualize(&vfs));
		Ok(virtual_directories.collect::<Vec<_>>())
	}

	pub fn get_recent_albums(&self, count: i64) -> Result<Vec<Directory>, QueryError> {
		use self::directories::dsl::*;
		let vfs = self.vfs_manager.get_vfs()?;
		let mut connection = self.db.connect()?;
		let real_directories: Vec<Directory> = directories
			.filter(album.is_not_null())
			.order(date_added.desc())
			.limit(count)
			.load(&mut connection)?;
		let virtual_directories = real_directories
			.into_iter()
			.filter_map(|d| d.virtualize(&vfs));
		Ok(virtual_directories.collect::<Vec<_>>())
	}

	pub fn generic_search(&self, query: &str) -> Result<Vec<CollectionFile>, QueryError> {
		let vfs = self.vfs_manager.get_vfs()?;
		let mut connection = self.db.connect()?;
		let like_test = format!("%{}%", query);
		let mut output = Vec::new();

		// Find dirs with matching path and parent not matching
		{
			use self::directories::dsl::*;
			let real_directories: Vec<Directory> = directories
				.filter(path.like(&like_test))
				.filter(parent.not_like(&like_test))
				.load(&mut connection)?;

			let virtual_directories = real_directories
				.into_iter()
				.filter_map(|d| d.virtualize(&vfs));

			output.extend(virtual_directories.map(CollectionFile::Directory));
		}

		// Find songs with matching title/album/artist and non-matching parent
		{
			use self::songs::dsl::*;
			let real_songs: Vec<Song> = songs
				.filter(
					path.like(&like_test)
						.or(title.like(&like_test))
						.or(album.like(&like_test))
						.or(artist.like(&like_test))
						.or(album_artist.like(&like_test))
						.or(composer.like(&like_test))
						.or(lyricist.like(&like_test))
						.or(genre.like(&like_test)),
				)
				.filter(parent.not_like(&like_test))
				.load(&mut connection)?;

			let virtual_songs = real_songs.into_iter().filter_map(|d| d.virtualize(&vfs));

			output.extend(virtual_songs.map(CollectionFile::Song));
		}

		Ok(output)
	}

	fn field_search(&self, fields: &QueryFields) -> Result<Vec<CollectionFile>, QueryError> {
		let vfs = self.vfs_manager.get_vfs()?;
		let mut connection = self.db.connect()?;
		let mut output = Vec::new();

		// Find songs with matching title/album/artist and non-matching parent
		{
			use self::songs::dsl::*;
			let mut filter = songs.into_boxed();
			if let Some(title_name) = fields.title.as_ref() {
				filter = filter.filter(title.like(title_name))
			}

			if let Some(artist_name) = fields.artist.as_ref() {
				filter = filter.filter(artist.like(artist_name))
			}

			if let Some(album_artist_name) = fields.album_artist.as_ref() {
				filter = filter.filter(album_artist.like(album_artist_name))
			}

			if let Some(album_name) = fields.album.as_ref() {
				filter = filter.filter(album.like(album_name))
			}

			if let Some(lyricist_name) = fields.lyricist.as_ref() {
				filter = filter.filter(lyricist.like(lyricist_name))
			}

			if let Some(composer_name) = fields.composer.as_ref() {
				filter = filter.filter(composer.like(composer_name))
			}

			if let Some(genre_name) = fields.genre.as_ref() {
				filter = filter.filter(genre.like(genre_name))
			}

			if let Some(years) = fields.years.as_ref() {
				filter = filter
					.filter(year.ge(years.start))
					.filter(year.lt(years.end))
			}

			let real_songs: Vec<Song> = filter.load(&mut connection)?;
			let virtual_songs = real_songs.into_iter().filter_map(|d| d.virtualize(&vfs));

			output.extend(virtual_songs.map(CollectionFile::Song));
		}
		Ok(output)
	}

	pub fn search(&self, query: &str) -> Result<Vec<CollectionFile>, QueryError> {
		let parsed_query = parse_query(query);
		let tmp = QueryFields {
			general_query: Some(parsed_query.general_query.as_ref().unwrap().to_string()),
			..Default::default()
		};
		if parsed_query == tmp {
			return self.generic_search(parsed_query.general_query.as_ref().unwrap());
		}
		self.field_search(&parsed_query)
	}

	pub fn get_song(&self, virtual_path: &Path) -> Result<Song, QueryError> {
		let vfs = self.vfs_manager.get_vfs()?;
		let mut connection = self.db.connect()?;

		let real_path = vfs.virtual_to_real(virtual_path)?;
		let real_path_string = real_path.as_path().to_string_lossy();

		use self::songs::dsl::*;
		let real_song: Song = songs
			.filter(path.eq(real_path_string))
			.get_result(&mut connection)?;

		match real_song.virtualize(&vfs) {
			Some(s) => Ok(s),
			None => Err(QueryError::SongNotFound(real_path)),
		}
	}
}
