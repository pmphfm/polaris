#[derive(thiserror::Error, Debug)]
pub enum Error {
	#[error("User not found")]
	UserNotFound,
	#[error("Playlist not found: {0}")]
	PlaylistNotFound(String),
	#[error("Unspecified")]
	Unspecified,
}

impl From<anyhow::Error> for Error {
	fn from(_: anyhow::Error) -> Self {
		Error::Unspecified
	}
}
