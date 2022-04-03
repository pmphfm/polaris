use std::ffi::OsStr;
use std::process::Command;
use std::{env, path::Path};

// Expects mdbook command in the environment PATH
fn build_user_guide() {
	let mdbook_rel_path = Path::new("docs").join("user-guide-src");
	println!("cargo:rerun-if-changed={:?}", mdbook_rel_path.as_os_str());
	let mdbook_dir = env::var_os("CARGO_MANIFEST_DIR").unwrap();
	let mdbook_src_path = Path::new(&mdbook_dir).join(mdbook_rel_path);
	let mdbook_output_dir = Path::new(&mdbook_dir).join("docs").join("user-guide");
	Command::new("mdbook")
		.args(&[
			OsStr::new("build"),
			mdbook_src_path.as_os_str(),
			OsStr::new("-d"),
			OsStr::new(&mdbook_output_dir),
		])
		.status()
		.unwrap();
}

#[cfg(windows)]
fn main() {
	build_user_guide();
	let mut res = winres::WindowsResource::new();
	res.set_icon("./res/windows/application/icon_polaris_512.ico");
	res.compile().unwrap();
}

#[cfg(unix)]
fn main() {
	build_user_guide();
}
