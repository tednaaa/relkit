mod changelog;
mod date;
mod forge;
mod git;
mod manifest;
mod version;

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::{env, fs, io};

use anyhow::{Context, Result};
use clap::{CommandFactory, Parser};
use clap_complete::{Shell, generate};
use cliclack::{confirm, intro, log, note, outro, outro_cancel, select};

use crate::changelog::Commit;
use crate::forge::Remote;
use crate::manifest::Manifests;
use crate::version::{Bump, Version};

#[derive(Parser)]
#[command(version, about)]
struct Cli {
	#[arg(long, help = "Ignore manifest files and take the current version from the latest release tag")]
	no_manifest: bool,

	#[arg(long, value_name = "SHELL", help = "Print a completion script for the given shell to stdout")]
	completions: Option<Shell>,
}

const RELEASE_COMMIT_PREFIX: &str = "release: v";
const TAG_PREFIX: &str = "v";
const UNDO_HINT: &str = "Nothing was tagged or pushed. Undo the commit with: git reset --soft HEAD~1";

fn main() -> ExitCode {
	let cli = Cli::parse();

	if let Some(shell) = cli.completions {
		print_completions(shell);

		return ExitCode::SUCCESS;
	}

	let Err(error) = release(&cli) else { return ExitCode::SUCCESS };

	if let Some(cancelled) = error.downcast_ref::<Cancelled>() {
		let _ = outro_cancel(cancelled);

		return ExitCode::SUCCESS;
	}

	let _ = outro_cancel(format!("Release failed: {error:#}"));

	ExitCode::FAILURE
}

fn print_completions(shell: Shell) {
	let mut command = Cli::command();
	let name = command.get_name().to_owned();

	generate(shell, &mut command, name, &mut io::stdout());
}

fn release(cli: &Cli) -> Result<()> {
	intro("release")?;
	git::ensure_repository()?;

	let directory = env::current_dir().context("failed to resolve the current directory")?;
	let versioning = Versioning::discover(&directory, cli.no_manifest)?;
	let changelog_path = directory.join(changelog::PATH);
	let current = versioning.current_version()?;
	let remote_url = git::remote_url();
	let remote = remote_url.as_deref().and_then(Remote::parse);

	log::info(format!("{} at v{current}", versioning.source()))?;
	announce_remote(remote_url.as_deref(), remote.as_ref())?;

	let mut touched = versioning.release_files();
	touched.push(changelog_path.clone());

	let version = pick_version(current)?;
	ask(&format!("Release v{version} — update the changelog and create the release commit?"), None)?;

	let snapshots: Vec<_> = touched.iter().map(|path| Snapshot::take(path)).collect();

	if let Err(error) = create_release_commit(&versioning, &changelog_path, &touched, version, remote.as_ref()) {
		for snapshot in &snapshots {
			snapshot.restore();
		}

		log::warning(format!("Restored {} — nothing was released.", file_names(&touched)))?;

		return Err(error);
	}

	review_changelog(version)?;
	amend_changelog_edits(&changelog_path)?;

	let tag = format!("{TAG_PREFIX}{version}");
	git::tag(&tag, &format!("{RELEASE_COMMIT_PREFIX}{version}"))?;
	log::step(format!("Tagged {tag}."))?;

	git::push()?;
	outro(format!("Released {tag}."))?;

	Ok(())
}

fn announce_remote(url: Option<&str>, remote: Option<&Remote>) -> Result<()> {
	if let Some(remote) = remote {
		log::info(format!("Linking commits to {} ({}).", remote.host(), remote.forge().label()))?;

		return Ok(());
	}

	let reason = match url {
		Some(url) => format!("`{url}` has no web address"),
		None => "there is no git remote".to_owned(),
	};

	log::warning(format!("Changelog entries will not link to commits — {reason}."))?;

	Ok(())
}

fn pick_version(current: Version) -> Result<Version> {
	let mut prompt = select(format!("Select release version (current {current})"));

	for bump in Bump::ALL {
		let next = current.bump(bump);
		prompt = prompt.item(next, format!("{} → {next}", bump.label()), "");
	}

	cancel_on_interrupt(prompt.interact(), None)
}

fn create_release_commit(
	versioning: &Versioning,
	changelog_path: &Path,
	touched: &[PathBuf],
	version: Version,
	remote: Option<&Remote>,
) -> Result<()> {
	let version = version.to_string();
	versioning.write_version(&version)?;

	let commits = commits_since_previous_release()?;
	write_changelog(changelog_path, &version, &commits, remote)?;
	log::step(format!("Changelog updated for v{version} ({} entries).", commits.len()))?;

	let staged: Vec<_> = touched.iter().map(PathBuf::as_path).filter(|path| !git::is_ignored(path)).collect();
	git::add(&staged)?;
	git::commit(&format!("{RELEASE_COMMIT_PREFIX}{version}"), &staged)?;
	log::success(format!("Committed {RELEASE_COMMIT_PREFIX}{version}."))?;

	Ok(())
}

fn file_names(paths: &[PathBuf]) -> String {
	let names: Vec<_> =
		paths.iter().filter_map(|path| path.file_name()).map(|name| name.to_string_lossy().into_owned()).collect();

	names.join(", ")
}

fn commits_since_previous_release() -> Result<Vec<Commit>> {
	let range = git::previous_release_tag().map_or_else(|| "HEAD".to_owned(), |tag| format!("{tag}..HEAD"));
	let raw = git::log(&range, changelog::RELEASE_COMMIT_PATTERN, changelog::LOG_FORMAT)?;

	Ok(changelog::parse(&raw))
}

fn write_changelog(path: &Path, version: &str, commits: &[Commit], remote: Option<&Remote>) -> Result<()> {
	let section = changelog::section(version, &date::today_utc(), commits, remote);
	let existing = fs::read_to_string(path).ok();

	fs::write(path, changelog::render(existing.as_deref(), version, &section))
		.with_context(|| format!("failed to write {}", path.display()))
}

fn review_changelog(version: Version) -> Result<()> {
	note(
		format!("Release commit for v{version} is local only"),
		format!(
			"Review {} and edit it if needed.\nUnstaged edits are amended into the release commit,\nor amend it yourself before continuing.",
			changelog::PATH
		),
	)?;

	ask("Changelog looks good — tag and push?", Some(UNDO_HINT))
}

fn amend_changelog_edits(path: &Path) -> Result<()> {
	if !git::is_dirty(path)? {
		log::step("No pending changelog edits.")?;

		return Ok(());
	}

	git::add(&[path])?;
	git::amend()?;
	log::success("Amended changelog edits into the release commit.")?;

	Ok(())
}

fn ask(message: &str, hint: Option<&str>) -> Result<()> {
	let confirmed = cancel_on_interrupt(confirm(message).initial_value(true).interact(), hint)?;

	if confirmed { Ok(()) } else { Err(Cancelled::new(hint).into()) }
}

fn cancel_on_interrupt<T>(result: io::Result<T>, hint: Option<&str>) -> Result<T> {
	match result {
		Err(error) if error.kind() == io::ErrorKind::Interrupted => Err(Cancelled::new(hint).into()),
		other => Ok(other?),
	}
}

enum Versioning {
	Manifests(Manifests),
	Tags,
}

impl Versioning {
	fn discover(directory: &Path, ignore_manifests: bool) -> Result<Self> {
		if ignore_manifests {
			return Ok(Self::Tags);
		}

		Ok(Self::Manifests(manifest::discover(directory)?))
	}

	fn source(&self) -> String {
		match self {
			Self::Manifests(manifests) => manifests.name(),
			Self::Tags => "git tags".to_owned(),
		}
	}

	fn current_version(&self) -> Result<Version> {
		match self {
			Self::Manifests(manifests) => Version::parse(&manifests.read_version()?),
			Self::Tags => latest_tagged_version(),
		}
	}

	fn write_version(&self, version: &str) -> Result<()> {
		match self {
			Self::Manifests(manifests) => manifests.write_version(version),
			Self::Tags => Ok(()),
		}
	}

	fn release_files(&self) -> Vec<PathBuf> {
		match self {
			Self::Manifests(manifests) => manifests.release_files(),
			Self::Tags => Vec::new(),
		}
	}
}

fn latest_tagged_version() -> Result<Version> {
	let Some(tag) = git::previous_release_tag() else { return Ok(Version::default()) };

	Version::parse(tag.trim_start_matches(TAG_PREFIX))
}

struct Snapshot {
	path: PathBuf,
	content: Option<String>,
}

impl Snapshot {
	fn take(path: &Path) -> Self {
		Self { path: path.to_owned(), content: fs::read_to_string(path).ok() }
	}

	fn restore(&self) {
		git::unstage(&self.path);

		let _ = match &self.content {
			Some(content) => fs::write(&self.path, content),
			None => fs::remove_file(&self.path),
		};
	}
}

#[derive(Debug)]
struct Cancelled {
	hint: Option<String>,
}

impl Cancelled {
	fn new(hint: Option<&str>) -> Self {
		Self { hint: hint.map(ToOwned::to_owned) }
	}
}

impl Display for Cancelled {
	fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
		match &self.hint {
			Some(hint) => write!(formatter, "Release cancelled. {hint}"),
			None => write!(formatter, "Release cancelled."),
		}
	}
}

impl Error for Cancelled {}
