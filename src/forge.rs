#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Forge {
	GitHub,
	GitLab,
	Bitbucket,
	Gitea,
	SourceHut,
	Unknown,
}

impl Forge {
	fn of(host: &str) -> Self {
		let host = host.to_ascii_lowercase();

		if host.contains("github") {
			Self::GitHub
		} else if host.contains("gitlab") {
			Self::GitLab
		} else if host.contains("bitbucket") {
			Self::Bitbucket
		} else if host.contains("codeberg") || host.contains("gitea") || host.contains("forgejo") {
			Self::Gitea
		} else if host.contains("sr.ht") {
			Self::SourceHut
		} else {
			Self::Unknown
		}
	}

	pub fn label(self) -> &'static str {
		match self {
			Self::GitHub => "GitHub",
			Self::GitLab => "GitLab",
			Self::Bitbucket => "Bitbucket",
			Self::Gitea => "Gitea",
			Self::SourceHut => "SourceHut",
			Self::Unknown => "unknown forge",
		}
	}

	fn commit_path(self) -> &'static str {
		match self {
			Self::GitLab => "-/commit",
			Self::Bitbucket => "commits",
			Self::GitHub | Self::Gitea | Self::SourceHut | Self::Unknown => "commit",
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Remote {
	web_url: String,
	host: String,
	forge: Forge,
}

impl Remote {
	pub fn parse(url: &str) -> Option<Self> {
		let location = without_scheme(url.trim())?;
		let (authority, path) = location.split_once('/')?;
		let host = host_of(authority);
		let path = repository_path(path);

		if host.is_empty() || path.is_empty() {
			return None;
		}

		Some(Self { web_url: format!("https://{host}/{path}"), host: host.to_owned(), forge: Forge::of(host) })
	}

	pub fn host(&self) -> &str {
		&self.host
	}

	pub fn forge(&self) -> Forge {
		self.forge
	}

	pub fn commit_url(&self, sha: &str) -> String {
		format!("{}/{}/{sha}", self.web_url, self.forge.commit_path())
	}
}

fn without_scheme(url: &str) -> Option<String> {
	match url.split_once("://") {
		Some(("ssh" | "git" | "http" | "https", location)) => Some(location.to_owned()),
		Some(_) => None,
		None => url.split_once(':').map(|(authority, path)| format!("{authority}/{path}")),
	}
}

fn host_of(authority: &str) -> &str {
	let without_user = authority.rsplit_once('@').map_or(authority, |(_user, host)| host);

	without_user.split_once(':').map_or(without_user, |(host, _port)| host)
}

fn repository_path(path: &str) -> &str {
	let path = path.trim_matches('/');

	path.strip_suffix(".git").unwrap_or(path).trim_end_matches('/')
}

#[cfg(test)]
mod tests {
	use super::*;

	fn commit_url(remote: &str) -> String {
		Remote::parse(remote).unwrap().commit_url("abc123")
	}

	#[test]
	fn normalizes_remote_spellings() {
		let expected = "https://github.com/owner/repo";

		for remote in [
			"git@github.com:owner/repo.git",
			"ssh://git@github.com:22/owner/repo.git",
			"https://github.com/owner/repo.git",
			"https://user@github.com/owner/repo/",
			"git://github.com/owner/repo",
		] {
			assert_eq!(Remote::parse(remote).unwrap().web_url, expected, "{remote}");
		}
	}

	#[test]
	fn builds_forge_specific_commit_urls() {
		assert_eq!(commit_url("git@github.com:owner/repo.git"), "https://github.com/owner/repo/commit/abc123");
		assert_eq!(commit_url("git@gitlab.com:owner/repo.git"), "https://gitlab.com/owner/repo/-/commit/abc123");
		assert_eq!(
			commit_url("git@gitlab.self-hosted.dev:group/sub/repo.git"),
			"https://gitlab.self-hosted.dev/group/sub/repo/-/commit/abc123"
		);
		assert_eq!(commit_url("git@bitbucket.org:owner/repo.git"), "https://bitbucket.org/owner/repo/commits/abc123");
		assert_eq!(commit_url("https://codeberg.org/owner/repo"), "https://codeberg.org/owner/repo/commit/abc123");
		assert_eq!(commit_url("git@git.sr.ht:~owner/repo"), "https://git.sr.ht/~owner/repo/commit/abc123");
		assert_eq!(commit_url("git@git.internal:owner/repo.git"), "https://git.internal/owner/repo/commit/abc123");
	}

	#[test]
	fn detects_forges() {
		assert_eq!(Remote::parse("git@github.enterprise.io:o/r.git").unwrap().forge(), Forge::GitHub);
		assert_eq!(Remote::parse("git@git.internal:o/r.git").unwrap().forge(), Forge::Unknown);
	}

	#[test]
	fn rejects_remotes_without_a_web_location() {
		assert_eq!(Remote::parse("/srv/git/repo.git"), None);
		assert_eq!(Remote::parse("file:///srv/git/repo.git"), None);
	}
}
