use std::fs;
use std::ops::Range;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::manifest::Manifest;

const VERSION_KEY: &str = "version";

pub struct Json {
	path: PathBuf,
}

impl Json {
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

impl Manifest for Json {
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
				b'/' => {
					if !self.skip_comment() {
						self.index += 1;
					}
				},
				_ => self.index += 1,
			}
		}

		None
	}

	fn read_value(&mut self) -> Option<Range<usize>> {
		self.skip_trivia();

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
		self.skip_trivia();

		if self.peek() != Some(byte) {
			return false;
		}

		self.index += 1;

		true
	}

	fn skip_trivia(&mut self) {
		loop {
			while self.peek().is_some_and(|byte| byte.is_ascii_whitespace()) {
				self.index += 1;
			}

			if !self.skip_comment() {
				return;
			}
		}
	}

	fn skip_comment(&mut self) -> bool {
		if self.peek() != Some(b'/') {
			return false;
		}

		self.index = match self.source.as_bytes().get(self.index + 1) {
			Some(b'/') => self.end_of(self.index + 2, "\n"),
			Some(b'*') => self.end_of(self.index + 2, "*/"),
			_ => return false,
		};

		true
	}

	fn end_of(&self, start: usize, terminator: &str) -> usize {
		self.source[start..].find(terminator).map_or(self.source.len(), |end| start + end + terminator.len())
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

	#[test]
	fn reads_and_rewrites_a_chrome_extension_manifest() {
		let source = r#"{
	"manifest_version": 3,
	"name": "СОН — сбор обратных номеров",
	"version": "0.1.0",
	"permissions": ["sidePanel", "tabs"],
	"background": { "service_worker": "background.js", "type": "module" },
	"icons": { "16": "icons/16.png", "128": "icons/128.png" }
}"#;

		assert_eq!(version_of(source).as_deref(), Some("0.1.0"));

		let span = Scanner::new(source).top_level_string_value(VERSION_KEY).unwrap();
		let mut updated = source.to_owned();
		updated.replace_range(span, "0.2.0");

		assert!(updated.contains("\"version\": \"0.2.0\""), "{updated}");
		assert!(updated.contains("СОН — сбор обратных номеров"), "{updated}");
	}

	#[test]
	fn reads_the_version_of_a_firefox_extension_manifest() {
		let source = r#"{
	"manifest_version": 2,
	"name": "Better Trading",
	"version": "2.0.1",
	"browser_specific_settings": { "gecko": { "id": "{c097f8f9-aec1-43cf-b6da-a88eff70a918}" } },
	"content_scripts": [{ "matches": ["*://*.tradingview.com/*"], "js": ["content.js"] }]
}"#;

		assert_eq!(version_of(source).as_deref(), Some("2.0.1"));
	}

	#[test]
	fn ignores_a_version_nested_in_an_extension_manifest() {
		let source = r#"{
	"manifest_version": 3,
	"browser_specific_settings": { "gecko": { "version": "9.9.9" } },
	"version": "0.1.0"
}"#;

		assert_eq!(version_of(source).as_deref(), Some("0.1.0"));
	}

	#[test]
	fn ignores_versions_hidden_in_comments() {
		let source = "{\n\t// \"version\": \"9.9.9\",\n\t/* \"version\": \"8.8.8\" */\n\t\"version\": \"0.1.0\"\n}\n";

		assert_eq!(version_of(source).as_deref(), Some("0.1.0"));
	}

	#[test]
	fn is_not_confused_by_braces_inside_comments() {
		let source = "{\n\t// { [ \"version\": \"9.9.9\"\n\t/* } ] */\n\t\"version\": \"0.1.0\"\n}\n";

		assert_eq!(version_of(source).as_deref(), Some("0.1.0"));
	}

	#[test]
	fn reads_a_version_separated_from_its_key_by_a_comment() {
		let source = "{\n\t\"version\" /* pinned */ : // release\n\t\t\"0.1.0\"\n}\n";

		assert_eq!(version_of(source).as_deref(), Some("0.1.0"));
	}

	#[test]
	fn survives_unterminated_comments() {
		assert_eq!(version_of("{\n\t\"version\": \"0.1.0\"\n} // trailing"), Some("0.1.0".to_owned()));
		assert_eq!(version_of("{ /* \"version\": \"9.9.9\""), None);
	}
}
