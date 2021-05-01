use anyhow::*;
use std::path::Path;

use crate::app::{index, rj::error::ParseError};

#[derive(Debug, Default)]
struct SongInfo {
	prev: Option<index::Song>,
	next: Option<index::Song>,
	next_next: Option<index::Song>,
}

fn get_path_announcement(
	index: &index::Index,
	path: &Option<String>,
	present_tense: bool,
) -> Result<String, ParseError> {
	let path = match path {
		Some(s) => s,
		None => return Ok("".to_string()),
	};
	let song = index
		.get_song(Path::new(path))
		.map_err(|op| ParseError::FailedToBuild(op.to_string()))?;
	index
		.rj_manager
		.read()
		.unwrap()
		.get_announcement(&song, present_tense)
}

pub fn get_announcement(
	index: &index::Index,
	request: index::RjRequest,
) -> Result<(String, Vec<u8>), ParseError> {
	let mut announcement = get_path_announcement(index, &request.prev, false)?;
	let natural_pause = ". ".to_owned();
	announcement += &(natural_pause.clone() + &get_path_announcement(index, &request.next, true)?);
	announcement += &(natural_pause + &get_path_announcement(index, &request.next_next, true)?);
	announcement = String::from_utf8(announcement.into_bytes())
		.map_err(|op| ParseError::FailedToBuild(op.to_string()))?;

	// TODO: String rarely contains a null byte which is causing tts server to panic.
	// Root cause the issue.
	// This is a workaround for that issue.
	announcement = str::replace(&announcement, "\0", " ");
	announcement = index.rj_manager.read().unwrap().build_packet(announcement);
	index.rj_manager.read().unwrap().get_speech(&announcement)
}
