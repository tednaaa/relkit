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
	fn versioned(candidates: Vec<Box<dyn Manifest>>) -> Option<Self> {
		let manifests: Vec<_> = candidates.into_iter().filter(|manifest| manifest.read_version().is_ok()).collect();

		(!manifests.is_empty()).then_some(Self { manifests })
	}

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

pub fn discover(directory: &Path) -> Option<Manifests> {
	Manifests::versioned(
		SUPPORTED
			.iter()
			.map(|(file_name, open)| (directory.join(file_name), open))
			.filter(|(path, _)| path.is_file())
			.map(|(path, open)| open(path))
			.collect(),
	)
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
		version: RefCell<Option<String>>,
	}

	impl Fake {
		fn new(name: &str, version: &str) -> Self {
			Self { path: PathBuf::from(name), version: RefCell::new(Some(version.to_owned())) }
		}

		fn unversioned(name: &str) -> Self {
			Self { path: PathBuf::from(name), version: RefCell::new(None) }
		}
	}

	impl Manifest for Fake {
		fn path(&self) -> &Path {
			&self.path
		}

		fn read_version(&self) -> Result<String> {
			self.version.borrow().clone().with_context(|| format!("{} has no version", self.name()))
		}

		fn write_version(&self, version: &str) -> Result<()> {
			*self.version.borrow_mut() = Some(version.to_owned());

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
	fn finds_nothing_in_a_directory_without_a_manifest() {
		assert!(discover(Path::new("/nonexistent")).is_none());
	}

	#[test]
	fn keeps_only_the_manifests_that_carry_a_version() {
		let candidates: Vec<Box<dyn Manifest>> =
			vec![Box::new(Fake::unversioned("Cargo.toml")), Box::new(Fake::new("package.json", "0.1.0"))];

		assert_eq!(Manifests::versioned(candidates).unwrap().name(), "package.json");
	}

	#[test]
	fn finds_nothing_when_no_manifest_carries_a_version() {
		assert!(Manifests::versioned(vec![Box::new(Fake::unversioned("Cargo.toml"))]).is_none());
	}
}
