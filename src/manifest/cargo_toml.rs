use std::fs;
use std::ops::Range;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::manifest::Manifest;

const LOCK_FILE: &str = "Cargo.lock";
const LOCK_PACKAGE_HEADER: &str = "[[package]]";
const PACKAGE_SECTION: &str = "package";
const NAME_KEY: &str = "name";
const SOURCE_KEY: &str = "source";
const VERSION_KEY: &str = "version";

pub struct CargoToml {
	path: PathBuf,
}

impl CargoToml {
	pub fn new(path: PathBuf) -> Self {
		Self { path }
	}

	fn lock_path(&self) -> PathBuf {
		self.path.with_file_name(LOCK_FILE)
	}

	fn read(&self) -> Result<String> {
		fs::read_to_string(&self.path).with_context(|| format!("failed to read {}", self.path.display()))
	}

	fn version_span(&self, source: &str) -> Result<Range<usize>> {
		if let Some(span) = package_field(source, VERSION_KEY) {
			return Ok(span);
		}

		if inherits_version_from_workspace(source) {
			bail!("{} inherits `version` from the workspace, which relkit cannot bump yet", self.path.display());
		}

		bail!("{} has no `version` under [package]", self.path.display())
	}

	fn write_lock_version(&self, package: &str, version: &str) -> Result<()> {
		let path = self.lock_path();
		let Ok(mut source) = fs::read_to_string(&path) else { return Ok(()) };

		let Some(span) = lock_version_span(&source, package) else {
			bail!("{} has no entry for `{package}` — run `cargo check` to refresh it", path.display());
		};

		source.replace_range(span, version);

		fs::write(&path, source).with_context(|| format!("failed to write {}", path.display()))
	}
}

impl Manifest for CargoToml {
	fn path(&self) -> &Path {
		&self.path
	}

	fn read_version(&self) -> Result<String> {
		let source = self.read()?;
		let span = self.version_span(&source)?;

		Ok(source[span].to_owned())
	}

	fn write_version(&self, version: &str) -> Result<()> {
		let mut source = self.read()?;
		let span = self.version_span(&source)?;
		let package = package_field(&source, NAME_KEY)
			.map(|span| source[span].to_owned())
			.with_context(|| format!("{} has no `name` under [package]", self.path.display()))?;

		source.replace_range(span, version);
		fs::write(&self.path, source).with_context(|| format!("failed to write {}", self.path.display()))?;

		self.write_lock_version(&package, version)
	}

	fn release_files(&self) -> Vec<PathBuf> {
		let lock = self.lock_path();

		if lock.is_file() { vec![self.path.clone(), lock] } else { vec![self.path.clone()] }
	}
}

fn package_field(source: &str, key: &str) -> Option<Range<usize>> {
	table_lines(source, PACKAGE_SECTION).find_map(|(offset, line)| Some(shift(string_value(line, key)?, offset)))
}

fn inherits_version_from_workspace(source: &str) -> bool {
	table_lines(source, PACKAGE_SECTION).any(|(_, line)| {
		let Some((key, value)) = line.split_once('=') else { return false };
		let key = key.trim();

		key.strip_suffix(".workspace").is_some_and(|key| key.trim_end() == VERSION_KEY)
			|| (key == VERSION_KEY && value.contains("workspace"))
	})
}

fn lock_version_span(source: &str, package: &str) -> Option<Range<usize>> {
	lock_packages(source).find_map(|(offset, block)| {
		let named = value_of(block, NAME_KEY).is_some_and(|span| block[span] == *package);
		let local = value_of(block, SOURCE_KEY).is_none();

		if !named || !local {
			return None;
		}

		Some(shift(value_of(block, VERSION_KEY)?, offset))
	})
}

fn lock_packages(source: &str) -> impl Iterator<Item = (usize, &str)> {
	let mut start = None;
	let mut offset = 0;

	source.split_inclusive('\n').chain([""]).filter_map(move |line| {
		let position = offset;
		offset += line.len();

		let header = line.trim() == LOCK_PACKAGE_HEADER;

		if !header && !line.is_empty() {
			return None;
		}

		let block = start.map(|begin| (begin, &source[begin..position]));
		start = header.then(|| position + line.len());

		block
	})
}

fn table_lines<'a>(source: &'a str, table: &'a str) -> impl Iterator<Item = (usize, &'a str)> {
	let mut section = "";
	let mut offset = 0;

	source.split_inclusive('\n').filter_map(move |line| {
		let position = offset;
		offset += line.len();

		if let Some(header) = table_header(line) {
			section = header;

			return None;
		}

		(section == table).then_some((position, line))
	})
}

fn table_header(line: &str) -> Option<&str> {
	let inner = line.trim().strip_prefix('[')?;

	if inner.starts_with('[') {
		return Some("");
	}

	Some(inner.split(']').next()?.trim())
}

fn value_of(region: &str, key: &str) -> Option<Range<usize>> {
	let mut offset = 0;

	region.split_inclusive('\n').find_map(|line| {
		let position = offset;
		offset += line.len();

		Some(shift(string_value(line, key)?, position))
	})
}

fn string_value(line: &str, key: &str) -> Option<Range<usize>> {
	if line.trim_start().starts_with('#') {
		return None;
	}

	let (raw_key, value) = line.split_once('=')?;

	if raw_key.trim() != key {
		return None;
	}

	let offset = line.len() - value.len();
	let opening = value.find(['"', '\''])?;
	let quote = value[opening..].chars().next()?;
	let start = opening + quote.len_utf8();
	let end = start + value[start..].find(quote)?;

	Some(shift(start..end, offset))
}

fn shift(span: Range<usize>, offset: usize) -> Range<usize> {
	offset + span.start..offset + span.end
}

#[cfg(test)]
mod tests {
	use super::*;

	const MANIFEST: &str = "[package]\nname = \"relkit\"\nversion = \"0.1.0\"\nedition = \"2024\"\nrust-version = \"1.80\"\n\n[dependencies]\nanyhow = \"1.0.104\"\ncliclack = { version = \"0.5.6\" }\n\n[lints.clippy]\npedantic = { level = \"warn\", priority = -1 }\n";

	const LOCK: &str = "version = 4\n\n[[package]]\nname = \"anyhow\"\nversion = \"1.0.104\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\nchecksum = \"abc\"\n\n[[package]]\nname = \"relkit\"\nversion = \"0.1.0\"\ndependencies = [\n \"anyhow\",\n]\n";

	fn version_of(source: &str) -> Option<String> {
		package_field(source, VERSION_KEY).map(|span| source[span].to_owned())
	}

	#[test]
	fn reads_the_package_version() {
		assert_eq!(version_of(MANIFEST).as_deref(), Some("0.1.0"));
		assert_eq!(package_field(MANIFEST, NAME_KEY).map(|span| &MANIFEST[span]), Some("relkit"));
	}

	#[test]
	fn ignores_versions_outside_the_package_table() {
		let source = "[dependencies]\nserde = \"1.0\"\nversion = \"9.9.9\"\n\n[package]\nversion = \"0.1.0\"\n";

		assert_eq!(version_of(source).as_deref(), Some("0.1.0"));
	}

	#[test]
	fn ignores_look_alike_keys_and_comments() {
		let source = "[package]\n# version = \"9.9.9\"\nrust-version = \"1.80\"\nversion = \"0.1.0\"\n";

		assert_eq!(version_of(source).as_deref(), Some("0.1.0"));
	}

	#[test]
	fn is_not_confused_by_array_tables_or_multiline_arrays() {
		let source = "[package]\nkeywords = [\n  \"release\",\n]\nversion = \"0.1.0\"\n\n[[bin]]\nname = \"other\"\nversion = \"9.9.9\"\n";

		assert_eq!(version_of(source).as_deref(), Some("0.1.0"));
	}

	#[test]
	fn reads_single_quoted_values() {
		assert_eq!(version_of("[package]\nversion = '0.1.0'\n").as_deref(), Some("0.1.0"));
	}

	#[test]
	fn detects_workspace_inheritance() {
		assert!(inherits_version_from_workspace("[package]\nname = \"member\"\nversion.workspace = true\n"));
		assert!(inherits_version_from_workspace("[package]\nversion = { workspace = true }\n"));
		assert!(!inherits_version_from_workspace(MANIFEST));
	}

	#[test]
	fn rewrites_only_the_version_and_keeps_formatting() {
		let span = package_field(MANIFEST, VERSION_KEY).unwrap();
		let mut updated = MANIFEST.to_owned();
		updated.replace_range(span, "0.2.0");

		assert_eq!(updated, MANIFEST.replace("version = \"0.1.0\"", "version = \"0.2.0\""));
		assert!(updated.contains("anyhow = \"1.0.104\""));
	}

	#[test]
	fn finds_the_local_package_in_the_lockfile() {
		let span = lock_version_span(LOCK, "relkit").unwrap();

		assert_eq!(&LOCK[span.clone()], "0.1.0");

		let mut updated = LOCK.to_owned();
		updated.replace_range(span, "0.2.0");

		assert!(updated.contains("name = \"relkit\"\nversion = \"0.2.0\""), "{updated}");
		assert!(updated.contains("name = \"anyhow\"\nversion = \"1.0.104\""), "{updated}");
	}

	#[test]
	fn skips_registry_packages_that_share_the_local_name() {
		let lock = "[[package]]\nname = \"relkit\"\nversion = \"0.0.1\"\nsource = \"registry+https://example\"\n\n[[package]]\nname = \"relkit\"\nversion = \"0.1.0\"\n";
		let span = lock_version_span(lock, "relkit").unwrap();

		assert_eq!(&lock[span], "0.1.0");
	}

	#[test]
	fn reports_a_lockfile_without_the_local_package() {
		assert_eq!(lock_version_span("version = 4\n", "relkit"), None);
	}
}
