mod package_json;

use std::path::{Path, PathBuf};

use anyhow::{Result, bail};

use crate::manifest::package_json::PackageJson;

pub trait Manifest {
	fn path(&self) -> &Path;
	fn read_version(&self) -> Result<String>;
	fn write_version(&self, version: &str) -> Result<()>;

	fn name(&self) -> String {
		self.path().file_name().unwrap_or_default().to_string_lossy().into_owned()
	}
}

type Open = fn(PathBuf) -> Box<dyn Manifest>;

const SUPPORTED: &[(&str, Open)] = &[("package.json", |path| Box::new(PackageJson::new(path)))];

pub fn discover(directory: &Path) -> Result<Box<dyn Manifest>> {
	for (file_name, open) in SUPPORTED {
		let path = directory.join(file_name);

		if path.is_file() {
			return Ok(open(path));
		}
	}

	let supported: Vec<_> = SUPPORTED.iter().map(|(file_name, _)| *file_name).collect();

	bail!("no supported manifest in {} (looked for: {})", directory.display(), supported.join(", "))
}
