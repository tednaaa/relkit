# Changelog

## v0.5.0 (2026-09-02)

- fix(cli): roll back the release commit when the changelog review is declined [`7b8d02ac`](https://github.com/tednaaa/relkit/commit/7b8d02ac61e1c4c3b8d91c4ac865c3ef511ab1f4)
- fix(cli): require a git remote before starting a release [`9957b290`](https://github.com/tednaaa/relkit/commit/9957b290abcdb15ded706dd531a1108190545907)
- fix(git): fail with a clear message when the repository has no commits [`a36afc9c`](https://github.com/tednaaa/relkit/commit/a36afc9c772c620a49c52254505e9f4f6881a541)
- fix(manifest): fall back to the latest git tag when no manifest has a version [`57801e6a`](https://github.com/tednaaa/relkit/commit/57801e6af5a450e76227868a5f7e004f1bc1f0eb)
- refactor(cli): drop the release commit confirmation prompt [`c6a33f11`](https://github.com/tednaaa/relkit/commit/c6a33f118258570b5519064d51e27c83535ca402)

## v0.4.0 (2026-08-31)

- feat(cli): add --completions to generate shell completion scripts [`6527ebe6`](https://github.com/tednaaa/relkit/commit/6527ebe6f0d4de9e1ffb04b84c80a8f54aac72a4)

## v0.3.0 (2026-08-31)

- feat(cli): add --no-manifest to release from tags without bumping manifests [`202ade59`](https://github.com/tednaaa/relkit/commit/202ade59bea6a773ca938e15c3c3d25112f7e0fc)

## v0.2.0 (2026-08-28)

- fix(manifest): skip comments when scanning json manifests [`eb947c37`](https://github.com/tednaaa/relkit/commit/eb947c3773b31b453d520ab37cbd1366730af208)
- feat(manifest): support browser extension manifest.json [`f2e8e8df`](https://github.com/tednaaa/relkit/commit/f2e8e8dff21b7ee5fc4d75208b6d798f9fe8130b)

## v0.1.2 (2026-08-22)

- add --version and --help flags via clap [`a7055963`](https://github.com/tednaaa/relkit/commit/a70559634f76ef151f20acd6957463ffc2891c36)

## v0.1.1 (2026-08-22)

- fix changelog body paragraph and spacing rendering [`e84f8be4`](https://github.com/tednaaa/relkit/commit/e84f8be44e35a0c5d40dbfcd838f29ee127b42d1)

## v0.1.0 (2026-08-21)

- add Cargo.toml manifest support with lockfile syncing [`e38fd9fc`](https://github.com/tednaaa/relkit/commit/e38fd9fc94d149d2ef2b2e7143679f99ebfd877d)
- add crates.io package metadata [`6886f0b5`](https://github.com/tednaaa/relkit/commit/6886f0b59d05ad251441d387e7397b38ddb0bc35)
- add github workflows [`84a59272`](https://github.com/tednaaa/relkit/commit/84a59272ebcc4785f06a92a1baadeb95014d86b3)
- draft implementation [`726b0476`](https://github.com/tednaaa/relkit/commit/726b0476d8564a218cb574a390827285b345b3be)
- init [`dde6ef86`](https://github.com/tednaaa/relkit/commit/dde6ef86b9b2652f97d5fc0beabf621ebb62350a)
- add license [`cd7989d3`](https://github.com/tednaaa/relkit/commit/cd7989d397e579aa01a7a8386f334ce638134a04)

