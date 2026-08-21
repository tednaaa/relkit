use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};

const RELEASE_TAG_GLOB: &str = "v[0-9]*.[0-9]*.[0-9]*";

fn run(args: &[&str]) -> Result<String> {
	let output = Command::new("git").args(args).output().context("failed to spawn `git` — is it installed?")?;

	if !output.status.success() {
		let stderr = String::from_utf8_lossy(&output.stderr);

		bail!("`git {}` failed: {}", args.join(" "), stderr.trim());
	}

	Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn run_on(args: &[&str], paths: &[&Path]) -> Result<String> {
	let paths: Vec<_> = paths.iter().map(|path| path.to_string_lossy().into_owned()).collect();
	let mut args = args.to_vec();

	args.push("--");
	args.extend(paths.iter().map(String::as_str));

	run(&args)
}

fn output_of(args: &[&str]) -> Option<String> {
	run(args).ok().filter(|output| !output.is_empty())
}

pub fn ensure_repository() -> Result<()> {
	run(&["rev-parse", "--git-dir"]).context("not a git repository")?;

	Ok(())
}

pub fn previous_release_tag() -> Option<String> {
	output_of(&["describe", "--tags", "--abbrev=0", "--match", RELEASE_TAG_GLOB])
}

pub fn remote_url() -> Option<String> {
	url_of_remote("origin").or_else(|| url_of_remote(&first_remote()?))
}

fn first_remote() -> Option<String> {
	Some(output_of(&["remote"])?.lines().next()?.to_owned())
}

fn url_of_remote(name: &str) -> Option<String> {
	output_of(&["config", "--get", &format!("remote.{name}.url")])
}

pub fn log(range: &str, excluded_subjects: &str, format: &str) -> Result<String> {
	let grep = format!("--grep={excluded_subjects}");
	let pretty = format!("--pretty=format:{format}");

	run(&["log", range, "--no-merges", "--invert-grep", &grep, &pretty])
}

pub fn is_dirty(path: &Path) -> Result<bool> {
	Ok(!run_on(&["status", "--porcelain"], &[path])?.is_empty())
}

pub fn add(paths: &[&Path]) -> Result<()> {
	run_on(&["add"], paths)?;

	Ok(())
}

pub fn unstage(path: &Path) {
	drop(run_on(&["reset", "--quiet"], &[path]));
}

pub fn commit(message: &str, paths: &[&Path]) -> Result<()> {
	run_on(&["commit", "--message", message], paths)?;

	Ok(())
}

pub fn amend() -> Result<()> {
	run(&["commit", "--amend", "--no-edit"])?;

	Ok(())
}

pub fn tag(name: &str, message: &str) -> Result<()> {
	run(&["tag", "--annotate", name, "--message", message])?;

	Ok(())
}

pub fn push() -> Result<()> {
	run(&["push", "--follow-tags"])?;

	Ok(())
}
