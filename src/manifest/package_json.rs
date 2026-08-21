use std::fs;
use std::ops::Range;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::manifest::Manifest;

const VERSION_KEY: &str = "version";

pub struct PackageJson {
	path: PathBuf,
}

impl PackageJson {
	pub fn new(path: PathBuf) -> Self {
		Self { path }
	}

	fn read(&self) -> Result<String> {
		fs::read_to_string(&self.path).with_context(|| format!("failed to read {}", self.path.display()))
	}

	fn version_span(&self, source: &str) -> Result<Range<usize>> {
		Scanner::new(source)
			.top_level_string_value(VERSION_KEY)
			.with_context(|| format!("{} has no top-level \"{VERSION_KEY}\" field", self.path.display()))
	}
}

impl Manifest for PackageJson {
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
		source.replace_range(span, version);

		fs::write(&self.path, source).with_context(|| format!("failed to write {}", self.path.display()))
	}
}

struct Scanner<'a> {
	source: &'a str,
	index: usize,
	depth: usize,
}

impl<'a> Scanner<'a> {
	fn new(source: &'a str) -> Self {
		Self { source, index: 0, depth: 0 }
	}

	fn top_level_string_value(mut self, key: &str) -> Option<Range<usize>> {
		while let Some(byte) = self.peek() {
			match byte {
				b'{' | b'[' => {
					self.depth += 1;
					self.index += 1;
				},
				b'}' | b']' => {
					self.depth = self.depth.saturating_sub(1);
					self.index += 1;
				},
				b'"' => {
					let depth = self.depth;
					let token = self.read_string()?;

					if depth == 1 && self.slice(&token) == key && self.accept(b':') {
						return self.read_value();
					}
				},
				_ => self.index += 1,
			}
		}

		None
	}

	fn read_value(&mut self) -> Option<Range<usize>> {
		self.skip_whitespace();

		if self.peek() != Some(b'"') {
			return None;
		}

		self.read_string()
	}

	fn read_string(&mut self) -> Option<Range<usize>> {
		let start = self.index + 1;
		let mut index = start;
		let bytes = self.source.as_bytes();

		loop {
			match *bytes.get(index)? {
				b'\\' => index += 2,
				b'"' => {
					self.index = index + 1;

					return Some(start..index);
				},
				_ => index += 1,
			}
		}
	}

	fn accept(&mut self, byte: u8) -> bool {
		self.skip_whitespace();

		if self.peek() != Some(byte) {
			return false;
		}

		self.index += 1;

		true
	}

	fn skip_whitespace(&mut self) {
		while self.peek().is_some_and(|byte| byte.is_ascii_whitespace()) {
			self.index += 1;
		}
	}

	fn peek(&self) -> Option<u8> {
		self.source.as_bytes().get(self.index).copied()
	}

	fn slice(&self, span: &Range<usize>) -> &str {
		&self.source[span.start..span.end]
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn version_of(source: &str) -> Option<String> {
		Scanner::new(source).top_level_string_value(VERSION_KEY).map(|span| source[span].to_owned())
	}

	#[test]
	fn reads_the_top_level_version() {
		assert_eq!(version_of(r#"{"name":"pkg","version":"1.2.3"}"#).as_deref(), Some("1.2.3"));
		assert_eq!(version_of("{\n\t\"version\" : \"1.2.3\"\n}\n").as_deref(), Some("1.2.3"));
	}

	#[test]
	fn ignores_nested_and_look_alike_fields() {
		let source = r#"{"name":"version","scripts":{"version":"echo"},"version":"1.2.3"}"#;

		assert_eq!(version_of(source).as_deref(), Some("1.2.3"));
	}

	#[test]
	fn is_not_confused_by_braces_and_escapes_inside_strings() {
		let source = r#"{"scripts":{"build":"echo \"{ version }\""},"version":"1.2.3"}"#;

		assert_eq!(version_of(source).as_deref(), Some("1.2.3"));
	}

	#[test]
	fn reports_a_missing_version() {
		assert_eq!(version_of(r#"{"name":"pkg"}"#), None);
		assert_eq!(version_of(r#"{"version":1}"#), None);
	}

	#[test]
	fn rewrites_only_the_version_and_keeps_formatting() {
		let source = "{\n\t\"name\": \"pkg\",\n\t\"version\": \"1.2.3\",\n\t\"private\": true\n}\n";
		let span = Scanner::new(source).top_level_string_value(VERSION_KEY).unwrap();
		let mut updated = source.to_owned();
		updated.replace_range(span, "1.3.0");

		assert_eq!(updated, "{\n\t\"name\": \"pkg\",\n\t\"version\": \"1.3.0\",\n\t\"private\": true\n}\n");
	}
}
