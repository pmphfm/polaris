use std::fmt::Write;

use super::*;
use crate::app::index::Song;

pub static M3U_HEADER: &str = "#EXTM3U";
pub static M3U_RMIM_FIELDS: &str = "#EXT-X-POLARIS:";
pub static M3U_COMMON_PATH: &str = "COMMON_PATH";

pub(crate) fn create_m3u_playlist(songs: &[Song]) -> Result<String, Error> {
	let (common_path, buffer) = strip_base_path(songs);
	let mut ret = String::new();
	writeln!(ret, "{}", M3U_HEADER).unwrap();
	if !common_path.is_empty() {
		writeln!(
			ret,
			"{} {}={}",
			M3U_RMIM_FIELDS, M3U_COMMON_PATH, common_path
		)
		.unwrap();
	}
	write!(ret, "{}", buffer).unwrap();
	Ok(ret)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn create_m3u_playlist_with_common_path() {
		assert_eq!(
			create_m3u_playlist(&[
				Song::test_only_from_path("a/bc/d/ef"),
				Song::test_only_from_path("a/bc/g/hi"),
				Song::test_only_from_path("a/bc/j/kl"),
			])
			.unwrap(),
			format!(
				"{}\n{} {}={}\n{}",
				M3U_HEADER, M3U_RMIM_FIELDS, M3U_COMMON_PATH, "a/bc/", "d/ef\ng/hi\nj/kl\n"
			),
		);
	}

	#[test]
	fn create_m3u_playlist_no_common_path() {
		assert_eq!(
			create_m3u_playlist(&[
				Song::test_only_from_path("a/bc/d/ef"),
				Song::test_only_from_path("ab/c/g/hi"),
				Song::test_only_from_path("abc/j/kl"),
			])
			.unwrap(),
			format!("{}\n{}", M3U_HEADER, "a/bc/d/ef\nab/c/g/hi\nabc/j/kl\n"),
		);
	}
}
