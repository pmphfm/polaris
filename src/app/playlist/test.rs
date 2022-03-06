use std::path::{Path, PathBuf};
use std::str::FromStr;

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
