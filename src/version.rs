use anyhow::{Context, Result, bail};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Version {
	major: u64,
	minor: u64,
	patch: u64,
}

impl Version {
	pub fn parse(raw: &str) -> Result<Self> {
		let without_suffix = raw.split(['-', '+']).next().unwrap_or(raw);
		let mut parts = without_suffix.split('.');

		let mut number = |name: &str| -> Result<u64> {
			parts
				.next()
				.filter(|part| !part.is_empty())
				.with_context(|| format!("version `{raw}` is missing a {name} component"))?
				.parse()
				.with_context(|| format!("version `{raw}` has a non-numeric {name} component"))
		};

		let (major, minor, patch) = (number("major")?, number("minor")?, number("patch")?);

		if parts.next().is_some() {
			bail!("version `{raw}` is not a `major.minor.patch` version");
		}

		Ok(Self { major, minor, patch })
	}

	pub fn bump(self, bump: Bump) -> Self {
		match bump {
			Bump::Patch => Self { patch: self.patch + 1, ..self },
			Bump::Minor => Self { minor: self.minor + 1, patch: 0, ..self },
			Bump::Major => Self { major: self.major + 1, minor: 0, patch: 0 },
		}
	}
}

impl std::fmt::Display for Version {
	fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(formatter, "{}.{}.{}", self.major, self.minor, self.patch)
	}
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Bump {
	#[default]
	Patch,
	Minor,
	Major,
}

impl Bump {
	pub const ALL: [Self; 3] = [Self::Patch, Self::Minor, Self::Major];

	pub fn label(self) -> &'static str {
		match self {
			Self::Patch => "patch",
			Self::Minor => "minor",
			Self::Major => "major",
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn bumped(current: &str, bump: Bump) -> String {
		Version::parse(current).unwrap().bump(bump).to_string()
	}

	#[test]
	fn bumps_each_component() {
		assert_eq!(bumped("1.2.3", Bump::Patch), "1.2.4");
		assert_eq!(bumped("1.2.3", Bump::Minor), "1.3.0");
		assert_eq!(bumped("1.2.3", Bump::Major), "2.0.0");
	}

	#[test]
	fn drops_prerelease_and_build_metadata() {
		assert_eq!(bumped("1.2.3-beta.1", Bump::Patch), "1.2.4");
		assert_eq!(bumped("1.2.3+build.5", Bump::Minor), "1.3.0");
	}

	#[test]
	fn rejects_malformed_versions() {
		for raw in ["1.2", "1.2.3.4", "1.2.x", "", "v1.2.3"] {
			assert!(Version::parse(raw).is_err(), "{raw}");
		}
	}
}
