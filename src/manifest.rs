mod cargo_toml;
mod json;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::manifest::cargo_toml::CargoToml;
use crate::manifest::json::Json;

pub trait Manifest {
	fn path(&self) -> &Path;
	fn read_version(&self) -> Result<String>;
	fn write_version(&self, version: &str) -> Result<()>;

	fn name(&self) -> String {
		self.path().file_name().unwrap_or_default().to_string_lossy().into_owned()
	}

	fn release_files(&self) -> Vec<PathBuf> {
		vec![self.path().to_owned()]
	}
}

type Open = fn(PathBuf) -> Box<dyn Manifest>;

#[rustfmt::skip]
const SUPPORTED: &[(&str, Open)] = &[
	("Cargo.toml",    |path| Box::new(CargoToml::new(path))),
	("package.json",  |path| Box::new(Json::new(path))),
	("manifest.json", |path| Box::new(Json::new(path))),
];

pub struct Manifests {
	manifests: Vec<Box<dyn Manifest>>,
}

impl Manifests {
	pub fn name(&self) -> String {
		self.manifests.iter().map(|manifest| manifest.name()).collect::<Vec<_>>().join(", ")
	}

	pub fn read_version(&self) -> Result<String> {
		let versions = self.read_versions()?;
		let (_, version) = versions.first().context("no manifest to read a version from")?;

		if versions.iter().any(|(_, other)| other != version) {
			bail!("{} disagree on the current version — align them before releasing", listed(&versions));
		}

		Ok(version.clone())
	}

	pub fn write_version(&self, version: &str) -> Result<()> {
		self.manifests.iter().try_for_each(|manifest| manifest.write_version(version))
	}

	pub fn release_files(&self) -> Vec<PathBuf> {
		self.manifests.iter().flat_map(|manifest| manifest.release_files()).collect()
	}

	fn read_versions(&self) -> Result<Vec<(String, String)>> {
		self.manifests.iter().map(|manifest| Ok((manifest.name(), manifest.read_version()?))).collect()
	}
}

pub fn discover(directory: &Path) -> Result<Manifests> {
	let manifests: Vec<_> = SUPPORTED
		.iter()
		.map(|(file_name, open)| (directory.join(file_name), open))
		.filter(|(path, _)| path.is_file())
		.map(|(path, open)| open(path))
		.collect();

	if manifests.is_empty() {
		let supported: Vec<_> = SUPPORTED.iter().map(|(file_name, _)| *file_name).collect();

		bail!("no supported manifest in {} (looked for: {})", directory.display(), supported.join(", "));
	}

	Ok(Manifests { manifests })
}

fn listed(versions: &[(String, String)]) -> String {
	versions.iter().map(|(name, version)| format!("{name} at v{version}")).collect::<Vec<_>>().join(", ")
}

#[cfg(test)]
mod tests {
	use std::cell::RefCell;

	use super::*;

	struct Fake {
		path: PathBuf,
		version: RefCell<String>,
	}

	impl Fake {
		fn new(name: &str, version: &str) -> Self {
			Self { path: PathBuf::from(name), version: RefCell::new(version.to_owned()) }
		}
	}

	impl Manifest for Fake {
		fn path(&self) -> &Path {
			&self.path
		}

		fn read_version(&self) -> Result<String> {
			Ok(self.version.borrow().clone())
		}

		fn write_version(&self, version: &str) -> Result<()> {
			*self.version.borrow_mut() = version.to_owned();

			Ok(())
		}
	}

	fn extension(package: &str, manifest: &str) -> Manifests {
		Manifests {
			manifests: vec![Box::new(Fake::new("package.json", package)), Box::new(Fake::new("manifest.json", manifest))],
		}
	}

	#[test]
	fn reads_the_shared_version_of_every_manifest() {
		let manifests = extension("0.1.0", "0.1.0");

		assert_eq!(manifests.name(), "package.json, manifest.json");
		assert_eq!(manifests.read_version().unwrap(), "0.1.0");
	}

	#[test]
	fn refuses_to_release_manifests_that_disagree() {
		let error = extension("0.1.0", "0.0.9").read_version().unwrap_err().to_string();

		assert!(error.contains("package.json at v0.1.0, manifest.json at v0.0.9"), "{error}");
	}

	#[test]
	fn bumps_every_manifest_at_once() {
		let manifests = extension("0.1.0", "0.1.0");
		manifests.write_version("0.2.0").unwrap();

		assert_eq!(manifests.read_version().unwrap(), "0.2.0");
	}

	#[test]
	fn stages_the_files_of_every_manifest() {
		let files = extension("0.1.0", "0.1.0").release_files();

		assert_eq!(files, vec![PathBuf::from("package.json"), PathBuf::from("manifest.json")]);
	}

	#[test]
	fn reports_a_directory_without_a_manifest() {
		let Err(error) = discover(Path::new("/nonexistent")) else { panic!("a missing directory has no manifest") };

		assert!(error.to_string().contains("manifest.json"), "{error}");
	}
}
