use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum ParseError {
	#[error("self recursive dependency in name:({name:?}) fragment:{fragment:?}")]
	SelfRecursion { name: String, fragment: String },

	#[error(
        "invalid number({count:?}) of delimiters({delimiter:?}) in name:{name:?} fragment:{:?} word:{word:?}"
    )]
	OddNumberOfDelimiters {
		count: usize,
		delimiter: char,
		name: String,
		fragment: String,
		word: String,
	},

	#[error("interleaved delimiters({delimiter:?}) in name:{name:?} fragment:{:?} word:{word:?}")]
	InterleavedDelimiter {
		delimiter: char,
		name: String,
		fragment: String,
		word: String,
	},

	#[error("duplicate fragment found by name: {0}")]
	DuplicateFragment(String),

	#[error("fragment uses reserved name: {name:?}. Reserved names are {reserved:?}")]
	FragmentUsesReservedName { name: String, reserved: String },

	#[error("recursive dependency in name:({name:?}) fragment:{fragment:?}")]
	RecursiveDependency { name: String, fragment: String },

	#[error("expansion failed for name:({name:?}) field: {field:?} fragment:{fragment:?}")]
	ExpansionFailed {
		name: String,
		field: String,
		fragment: String,
	},

	#[error("either from recursion or limited depth({depth:?}) expansion failed for name:({name:?}) fragment:{fragment:?}")]
	TooDeep {
		depth: usize,
		name: String,
		fragment: String,
	},

	#[error("failed to deserialize: {0}")]
	FailedToDeserialize(String),

	#[error("failed to build: {0}")]
	FailedToBuild(String),

	#[error("failed to convert text to speech: {0}")]
	FailedToTTS(String),

	#[error("rj service is disable by the admin")]
	RjServiceDisabled,

	#[error("Invalid input: {0}")]
	InvalidInput(String),

	#[error("Delimiter({delimiter:?}) not allowed in conjunctions: {conjunction:?}")]
	DelimiterNotAllowed {
		delimiter: char,
		conjunction: String,
	},
}
