use crossbeam_channel::Receiver;
use diesel::prelude::*;
use log::error;
use std::mem::take;
use std::thread::spawn;
use std::thread::JoinHandle;

use crate::db::{directories, songs, DB};

const INDEX_BUILDING_INSERT_BUFFER_SIZE: usize = 1000; // Insertions in each transaction

#[derive(Debug, Insertable)]
#[diesel(table_name = songs)]
pub struct Song {
	pub path: String,
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

#[derive(Debug, Insertable)]
#[diesel(table_name = directories)]
pub struct Directory {
	pub path: String,
	pub parent: Option<String>,
	pub artist: Option<String>,
	pub year: Option<i32>,
	pub album: Option<String>,
	pub artwork: Option<String>,
	pub date_added: i32,
}

pub enum Item {
	Directory(Directory),
	Song(Song),
}

pub struct Inserter {
	receiver: Receiver<Item>,
	new_directories: Vec<Directory>,
	new_songs: Vec<Song>,
	db: DB,
	flush_thread: Option<JoinHandle<()>>,
}

impl Inserter {
	pub fn new(db: DB, receiver: Receiver<Item>) -> Self {
		let new_directories = Vec::with_capacity(INDEX_BUILDING_INSERT_BUFFER_SIZE);
		let new_songs = Vec::with_capacity(INDEX_BUILDING_INSERT_BUFFER_SIZE);
		Self {
			receiver,
			new_directories,
			new_songs,
			db,
			flush_thread: None,
		}
	}

	pub fn insert(&mut self) {
		while let Ok(item) = self.receiver.recv() {
			self.insert_item(item);
		}
	}

	fn wait_flush(&mut self) {
		if let Some(join_handle) = take(&mut self.flush_thread) {
			let _ = join_handle.join();
		}
	}

	fn queue_flush(&mut self, force: bool) {
		if self.new_directories.len() < INDEX_BUILDING_INSERT_BUFFER_SIZE
			&& self.new_songs.len() < INDEX_BUILDING_INSERT_BUFFER_SIZE
			&& !force
		{
			return;
		}

		if self.new_directories.len() >= INDEX_BUILDING_INSERT_BUFFER_SIZE || force {
			self.wait_flush();
			let db = self.db.clone();
			let new_directories = take(&mut self.new_directories);
			self.flush_thread = Some(spawn(|| Self::flush_directories(db, new_directories)));
		}
		if self.new_songs.len() >= INDEX_BUILDING_INSERT_BUFFER_SIZE || force {
			self.wait_flush();
			let db = self.db.clone();
			let new_songs = take(&mut self.new_songs);
			self.flush_thread = Some(spawn(|| Self::flush_songs(db, new_songs)));
		}
	}

	fn insert_item(&mut self, insert: Item) {
		match insert {
			Item::Directory(d) => {
				self.new_directories.push(d);
			}
			Item::Song(s) => {
				self.new_songs.push(s);
			}
		};

		self.queue_flush(false);
	}

	fn flush_directories(db: DB, new_directories: Vec<Directory>) {
		if new_directories.is_empty() {
			return;
		}

		let res = db.connect().ok().and_then(|mut connection| {
			diesel::insert_into(directories::table)
				.values(&new_directories)
				.execute(&mut *connection) // TODO https://github.com/diesel-rs/diesel/issues/1822
				.ok()
		});
		if res.is_none() {
			error!("Could not insert new directories in database");
		}
	}

	fn flush_songs(db: DB, new_songs: Vec<Song>) {
		if new_songs.is_empty() {
			return;
		}

		let res = db.connect().ok().and_then(|mut connection| {
			diesel::insert_into(songs::table)
				.values(&new_songs)
				.execute(&mut *connection) // TODO https://github.com/diesel-rs/diesel/issues/1822
				.ok()
		});
		if res.is_none() {
			error!("Could not insert new songs in database");
		}
	}
}

impl Drop for Inserter {
	fn drop(&mut self) {
		self.queue_flush(true);
		self.wait_flush();
	}
}
