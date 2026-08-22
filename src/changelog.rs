use crate::forge::Remote;

pub const PATH: &str = "CHANGELOG.md";
pub const RELEASE_COMMIT_PATTERN: &str = "^release: ";
pub const LOG_FORMAT: &str = "%H\u{1f}%s\u{1f}%b\u{1e}";

const HEADING: &str = "# Changelog";
const SECTION_PREFIX: &str = "## v";
const FIELD_SEPARATOR: char = '\u{1f}';
const RECORD_SEPARATOR: char = '\u{1e}';

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Commit {
	sha: String,
	subject: String,
	body: String,
}

pub fn parse(raw: &str) -> Vec<Commit> {
	raw
		.split(RECORD_SEPARATOR)
		.map(str::trim)
		.filter(|record| !record.is_empty())
		.map(|record| {
			let mut fields = record.split(FIELD_SEPARATOR);

			Commit {
				sha: fields.next().unwrap_or_default().to_owned(),
				subject: fields.next().unwrap_or_default().trim().to_owned(),
				body: fields.next().unwrap_or_default().to_owned(),
			}
		})
		.collect()
}

pub fn section(version: &str, date: &str, commits: &[Commit], remote: Option<&Remote>) -> String {
	let heading = format!("{SECTION_PREFIX}{version} ({date})\n");

	if commits.is_empty() {
		return heading;
	}

	let entries: Vec<_> = commits.iter().map(|commit| entry(commit, remote)).collect();

	format!("{heading}\n{}\n", entries.join("\n").trim_end())
}

pub fn render(existing: Option<&str>, version: &str, section: &str) -> String {
	let body = existing.map(|existing| without_section(without_heading(existing), version)).unwrap_or_default();

	format!("{HEADING}\n\n{section}\n{body}")
}

fn entry(commit: &Commit, remote: Option<&Remote>) -> String {
	let short_sha = commit.sha.get(..8).unwrap_or(&commit.sha);
	let reference = match remote {
		Some(remote) => format!("[`{short_sha}`]({})", remote.commit_url(&commit.sha)),
		None => format!("`{short_sha}`"),
	};

	let heading = format!("- {} {reference}", commit.subject);
	let paragraphs = body_paragraphs(&commit.body);

	if paragraphs.is_empty() { heading } else { format!("{heading}\n\n{}\n", paragraphs.join("\n\n")) }
}

fn body_paragraphs(body: &str) -> Vec<String> {
	let lines: Vec<_> = body.lines().map(str::trim).filter(|line| line.is_empty() || !is_trailer(line)).collect();

	lines
		.split(|line| line.is_empty())
		.map(|paragraph| paragraph.join(" "))
		.filter(|paragraph| !paragraph.is_empty())
		.map(|paragraph| format!("  {paragraph}"))
		.collect()
}

fn is_trailer(line: &str) -> bool {
	let Some((key, rest)) = line.split_once(':') else { return false };

	!key.is_empty()
		&& key.chars().all(|char| char.is_ascii_alphabetic() || char == '-')
		&& rest.starts_with(char::is_whitespace)
}

fn without_heading(existing: &str) -> &str {
	existing.strip_prefix(HEADING).map_or(existing, |rest| rest.trim_start_matches('\n'))
}

fn without_section(body: &str, version: &str) -> String {
	let dropped = format!("{SECTION_PREFIX}{version} ");
	let mut keep = true;

	body
		.split_inclusive('\n')
		.filter(|line| {
			if line.starts_with(SECTION_PREFIX) {
				keep = !line.starts_with(&dropped);
			}

			keep
		})
		.collect()
}

#[cfg(test)]
mod tests {
	use super::*;

	fn remote() -> Remote {
		Remote::parse("git@github.com:owner/repo.git").unwrap()
	}

	fn commit(sha: &str, subject: &str, body: &str) -> Commit {
		Commit { sha: sha.to_owned(), subject: subject.to_owned(), body: body.to_owned() }
	}

	#[test]
	fn parses_log_records() {
		let raw = "sha1\u{1f}first\u{1f}body line\u{1e}\nsha2\u{1f}second\u{1f}\u{1e}";

		assert_eq!(parse(raw), vec![commit("sha1", "first", "body line"), commit("sha2", "second", "")]);
	}

	#[test]
	fn parses_empty_log_as_no_commits() {
		assert!(parse("").is_empty());
	}

	#[test]
	fn links_commits_to_the_remote() {
		let commits = [commit("0123456789abcdef", "add thing", "")];
		let section = section("1.0.0", "2026-08-21", &commits, Some(&remote()));

		assert_eq!(
			section,
			"## v1.0.0 (2026-08-21)\n\n- add thing [`01234567`](https://github.com/owner/repo/commit/0123456789abcdef)\n"
		);
	}

	#[test]
	fn falls_back_to_a_bare_sha_without_a_remote() {
		let commits = [commit("0123456789abcdef", "add thing", "")];

		assert_eq!(section("1.0.0", "2026-08-21", &commits, None), "## v1.0.0 (2026-08-21)\n\n- add thing `01234567`\n");
	}

	#[test]
	fn indents_body_paragraphs_and_drops_trailers() {
		let body = "why it changed\n\nand a detail\nSigned-off-by: someone <a@b.c>";
		let commits = [commit("0123456789abcdef", "add thing", body)];
		let section = section("1.0.0", "2026-08-21", &commits, None);

		assert!(section.ends_with("- add thing `01234567`\n\n  why it changed\n\n  and a detail\n"), "{section}");
	}

	#[test]
	fn joins_wrapped_lines_into_one_paragraph() {
		let body = "a sentence that git\nwrapped across lines\n\na second paragraph";
		let commits = [commit("0123456789abcdef", "add thing", body)];
		let section = section("1.0.0", "2026-08-21", &commits, None);

		assert!(
			section
				.ends_with("- add thing `01234567`\n\n  a sentence that git wrapped across lines\n\n  a second paragraph\n"),
			"{section}"
		);
	}

	#[test]
	fn separates_a_described_entry_from_the_next_one() {
		let commits = [commit("aaaaaaaaaaaaaaaa", "described", "why it changed"), commit("bbbbbbbbbbbbbbbb", "bare", "")];

		assert_eq!(
			section("1.0.0", "2026-08-21", &commits, None),
			"## v1.0.0 (2026-08-21)\n\n- described `aaaaaaaa`\n\n  why it changed\n\n- bare `bbbbbbbb`\n"
		);
	}

	#[test]
	fn keeps_prose_that_only_looks_like_a_trailer() {
		assert!(is_trailer("Co-authored-by: someone"));
		assert!(!is_trailer("note:missing space"));
		assert!(!is_trailer("fix(core): scoped subject"));
	}

	#[test]
	fn writes_an_empty_section_when_nothing_changed() {
		assert_eq!(section("1.0.0", "2026-08-21", &[], None), "## v1.0.0 (2026-08-21)\n");
	}

	#[test]
	fn prepends_to_an_existing_changelog() {
		let existing = "# Changelog\n\n## v0.9.0 (2026-01-01)\n\n- old\n";
		let rendered = render(Some(existing), "1.0.0", "## v1.0.0 (2026-08-21)\n\n- new\n");

		assert_eq!(rendered, "# Changelog\n\n## v1.0.0 (2026-08-21)\n\n- new\n\n## v0.9.0 (2026-01-01)\n\n- old\n");
	}

	#[test]
	fn replaces_a_section_written_for_the_same_version() {
		let existing = "# Changelog\n\n## v1.0.0 (2026-08-20)\n\n- stale\n\n## v0.9.0 (2026-01-01)\n\n- old\n";
		let rendered = render(Some(existing), "1.0.0", "## v1.0.0 (2026-08-21)\n\n- new\n");

		assert_eq!(rendered, "# Changelog\n\n## v1.0.0 (2026-08-21)\n\n- new\n\n## v0.9.0 (2026-01-01)\n\n- old\n");
	}

	#[test]
	fn creates_a_changelog_when_none_exists() {
		assert_eq!(render(None, "1.0.0", "## v1.0.0 (2026-08-21)\n"), "# Changelog\n\n## v1.0.0 (2026-08-21)\n\n");
	}
}
