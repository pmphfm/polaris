use anyhow::*;
use anyhow::{bail, Result};
use image::ImageOutputFormat;
use image::{DynamicImage, GenericImage, GenericImageView, ImageBuffer};
use std::cmp;
use std::collections::hash_map::DefaultHasher;
use std::fs::{self, File};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use crate::utils;
use crate::utils::AudioFormat;

#[derive(Debug, Hash)]
pub struct Options {
	pub max_dimension: Option<u32>,
	pub resize_if_almost_square: bool,
	pub pad_to_square: bool,
}

impl Default for Options {
	fn default() -> Self {
		Self {
			max_dimension: Some(400),
			resize_if_almost_square: true,
			pad_to_square: true,
		}
	}
}
#[derive(Clone)]
pub struct Manager {
	thumbnails_dir_path: PathBuf,
}

impl Manager {
	pub fn new(thumbnails_dir_path: PathBuf) -> Self {
		Self {
			thumbnails_dir_path,
		}
	}

	pub fn get_thumbnail(&self, image_path: &Path, thumbnailoptions: &Options) -> Result<PathBuf> {
		match self.retrieve_thumbnail(image_path, thumbnailoptions) {
			Some(path) => Ok(path),
			None => self.create_thumbnail(image_path, thumbnailoptions),
		}
	}

	fn get_thumbnail_path(&self, image_path: &Path, thumbnailoptions: &Options) -> PathBuf {
		let hash = Manager::hash(image_path, thumbnailoptions);
		let mut thumbnail_path = self.thumbnails_dir_path.clone();
		thumbnail_path.push(format!("{}.jpg", hash));
		thumbnail_path
	}

	fn retrieve_thumbnail(&self, image_path: &Path, thumbnailoptions: &Options) -> Option<PathBuf> {
		let path = self.get_thumbnail_path(image_path, thumbnailoptions);
		if path.exists() {
			Some(path)
		} else {
			None
		}
	}

	fn create_thumbnail(&self, image_path: &Path, thumbnailoptions: &Options) -> Result<PathBuf> {
		let thumbnail = generate_thumbnail(image_path, thumbnailoptions)?;
		let quality = 80;

		fs::create_dir_all(&self.thumbnails_dir_path)?;
		let path = self.get_thumbnail_path(image_path, thumbnailoptions);
		let mut out_file = File::create(&path)?;
		thumbnail.write_to(&mut out_file, ImageOutputFormat::Jpeg(quality))?;
		Ok(path)
	}

	fn hash(path: &Path, thumbnailoptions: &Options) -> u64 {
		let mut hasher = DefaultHasher::new();
		path.hash(&mut hasher);
		thumbnailoptions.hash(&mut hasher);
		hasher.finish()
	}
}

pub fn generate_thumbnail(image_path: &Path, options: &Options) -> Result<DynamicImage> {
	let source_image = DynamicImage::ImageRgb8(read(image_path)?.into_rgb8());
	let (source_width, source_height) = source_image.dimensions();
	let largest_dimension = cmp::max(source_width, source_height);
	let out_dimension = cmp::min(
		options.max_dimension.unwrap_or(largest_dimension),
		largest_dimension,
	);

	let source_aspect_ratio: f32 = source_width as f32 / source_height as f32;
	let is_almost_square = source_aspect_ratio > 0.8 && source_aspect_ratio < 1.2;

	let mut final_image;
	if is_almost_square && options.resize_if_almost_square {
		final_image = source_image.thumbnail_exact(out_dimension, out_dimension);
	} else if options.pad_to_square {
		let scaled_image = source_image.thumbnail(out_dimension, out_dimension);
		let (scaled_width, scaled_height) = scaled_image.dimensions();
		let background = image::Rgb([255, 255_u8, 255_u8]);
		final_image = DynamicImage::ImageRgb8(ImageBuffer::from_pixel(
			out_dimension,
			out_dimension,
			background,
		));
		final_image.copy_from(
			&scaled_image,
			(out_dimension - scaled_width) / 2,
			(out_dimension - scaled_height) / 2,
		)?;
	} else {
		final_image = source_image.thumbnail(out_dimension, out_dimension);
	}

	Ok(final_image)
}

pub fn read(image_path: &Path) -> Result<DynamicImage> {
	match utils::get_audio_format(image_path) {
		Some(AudioFormat::AIFF) => read_aiff(image_path),
		Some(AudioFormat::APE) => read_ape(image_path),
		Some(AudioFormat::FLAC) => read_flac(image_path),
		Some(AudioFormat::MP3) => read_mp3(image_path),
		Some(AudioFormat::MP4) => read_mp4(image_path),
		Some(AudioFormat::MPC) => read_ape(image_path),
		Some(AudioFormat::OGG) => read_vorbis(image_path),
		Some(AudioFormat::OPUS) => read_opus(image_path),
		Some(AudioFormat::WAVE) => read_wave(image_path),
		None => Ok(image::open(image_path)?),
	}
}

fn read_ape(_: &Path) -> Result<DynamicImage> {
	bail!("Embedded images are not supported in APE files");
}

fn read_flac(path: &Path) -> Result<DynamicImage> {
	let tag = metaflac::Tag::read_from_path(path)?;

	if let Some(p) = tag.pictures().next() {
		return Ok(image::load_from_memory(&p.data)?);
	}

	bail!(
		"Embedded flac artwork not found for file: {}",
		path.display()
	);
}

fn read_mp3(path: &Path) -> Result<DynamicImage> {
	let tag = id3::Tag::read_from_path(path)?;

	read_id3(path, &tag)
}

fn read_aiff(path: &Path) -> Result<DynamicImage> {
	let tag = id3::Tag::read_from_aiff_path(path)?;

	read_id3(path, &tag)
}

fn read_wave(path: &Path) -> Result<DynamicImage> {
	let tag = id3::Tag::read_from_wav_path(path)?;

	read_id3(path, &tag)
}

fn read_id3(path: &Path, tag: &id3::Tag) -> Result<DynamicImage> {
	if let Some(p) = tag.pictures().next() {
		return Ok(image::load_from_memory(&p.data)?);
	}

	bail!(
		"Embedded id3 artwork not found for file: {}",
		path.display()
	);
}

fn read_mp4(path: &Path) -> Result<DynamicImage> {
	let tag = mp4ameta::Tag::read_from_path(path)?;

	match tag.artwork().map(|d| d.data) {
		Some(v) => Ok(image::load_from_memory(v)?),
		_ => bail!(
			"Embedded mp4 artwork not found for file: {}",
			path.display()
		),
	}
}

fn read_vorbis(_: &Path) -> Result<DynamicImage> {
	bail!("Embedded images are not supported in Vorbis files");
}

fn read_opus(_: &Path) -> Result<DynamicImage> {
	bail!("Embedded images are not supported in Opus files");
}

#[test]
fn can_read_artwork_data() {
	let ext_img = image::open("test-data/artwork/Folder.png")
		.unwrap()
		.to_rgb8();
	let embedded_img = image::open("test-data/artwork/Embedded.png")
		.unwrap()
		.to_rgb8();

	let folder_img = read(Path::new("test-data/artwork/Folder.png"))
		.unwrap()
		.to_rgb8();
	assert_eq!(folder_img, ext_img);

	let aiff_img = read(Path::new("test-data/artwork/sample.aif"))
		.unwrap()
		.to_rgb8();
	assert_eq!(aiff_img, embedded_img);

	let ape_img = read(Path::new("test-data/artwork/sample.ape"))
		.map(|d| d.to_rgb8())
		.ok();
	assert_eq!(ape_img, None);

	let flac_img = read(Path::new("test-data/artwork/sample.flac"))
		.unwrap()
		.to_rgb8();
	assert_eq!(flac_img, embedded_img);

	let mp3_img = read(Path::new("test-data/artwork/sample.mp3"))
		.unwrap()
		.to_rgb8();
	assert_eq!(mp3_img, embedded_img);

	let m4a_img = read(Path::new("test-data/artwork/sample.m4a"))
		.unwrap()
		.to_rgb8();
	assert_eq!(m4a_img, embedded_img);

	let ogg_img = read(Path::new("test-data/artwork/sample.ogg"))
		.map(|d| d.to_rgb8())
		.ok();
	assert_eq!(ogg_img, None);

	let opus_img = read(Path::new("test-data/artwork/sample.opus"))
		.map(|d| d.to_rgb8())
		.ok();
	assert_eq!(opus_img, None);

	let wave_img = read(Path::new("test-data/artwork/sample.wav"))
		.unwrap()
		.to_rgb8();
	assert_eq!(wave_img, embedded_img);
}
