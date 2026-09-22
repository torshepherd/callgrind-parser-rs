# Publishing and binary releases

The project is experimental. This documentation is fully LLM-generated and
will receive a review and cleanup pass before 1.0.

Six crates are released: `callgrind-parser`, `callgrind-writer`, `pprof-profile`,
`callgrind-annotate-rs`, `pprof2callgrind` and `callgrind2pprof`. The two UI stubs
are excluded with `publish = false` and `dist = false`.

## Normal release workflow

Continue ordinary development on `master`. Release-plz 0.3.169 maintains a
release PR containing version and changelog changes. Review that PR, including
its semver suggestions, and merge it when ready to release. `release_always =
false` means ordinary development pushes do not authorize a release by
themselves. Versions currently inherit the shared workspace version.

`release-pr.yml` explicitly dispatches `ci.yml` and a binary dry run for the
release PR branch. This makes checks run even though the PR was opened with
the built-in `GITHUB_TOKEN`. No personal GitHub token or GitHub App is needed.

After the merged commit's full push CI succeeds, `release-plz.yml` checks the
exact SHA, the current `master` head and all three successful CI jobs. It then
publishes the changed crates with crates.io Trusted Publishing and creates
`PACKAGE-vVERSION` tags. Ordinary Cargo packaging verification remains enabled.
Release-plz does not create GitHub Releases; cargo-dist owns binary releases.

For each published CLI, the workflow explicitly dispatches `release.yml` at
that package's tag. The dispatch avoids GitHub's suppression of workflows
triggered by tags created with `GITHUB_TOKEN`. The source check requires an
existing tag at the checked-out commit and successful full CI for that commit.

Cargo-dist 0.33.0 builds archives and checksums for:

- Linux x86-64 and ARM64, using musl targets.
- macOS Intel and Apple Silicon, using native runners.

After uploading GitHub Release artifacts, the workflow installs the exact
version on all four platforms using `cargo binstall --strategies crate-meta-data`
and runs its help/version commands. Source compilation and third-party
quickinstall artifacts cannot mask a missing release binary in this check.

## First publication and authentication

The first publication of a new crate needs a normal crates.io login. Our
development wrapper uses `.dev/cargo`, which intentionally differs from the
normal `~/.cargo` where `cargo login` stores credentials. Use the pinned Cargo
with the normal Cargo home for that first publication; do not copy credentials
into the repository or print tokens.

Run `cargo publish --workspace --dry-run --locked` before the actual publish.
The pinned Cargo can package and verify interdependent unpublished workspace
crates together. Commit and push the complete source and wait for full CI
before uploading. Cargo publishes dependencies before their consumers.

For each crate, configure this Trusted Publisher on crates.io:

| Field | Value |
| --- | --- |
| Repository owner | `torshepherd` |
| Repository | `callgrind-parser-rs` |
| Workflow filename | `release-plz.yml` |
| Environment | `crates-io` |

The GitHub environment `crates-io` permits deployments from `master`. The
publishing job has `id-token: write`; release-plz exchanges its OIDC identity
for short-lived registry credentials. Do not add `CARGO_REGISTRY_TOKEN` to that
job: it would select token authentication instead of Trusted Publishing.

Enable GitHub Actions' permission to create release PRs. Set repository variable
`RELEASES_ENABLED=true` only after the initial crates and their trusted publisher
configurations exist. This switch controls both release-plz workflows.

## Validation and recovery

Run the normal repository gates and `python3 -m unittest discover -s tests/release -v`.
`cargo doc --workspace --no-deps --locked --offline` checks the crate API docs.
Inspect the packaged READMEs and license files as well as the working-tree docs.

Regenerate `.github/workflows/release.yml` with the pinned `dist generate` after
editing `dist-workspace.toml`; `dist plan` detects generated-workflow drift.
The other workflows are maintained directly. Test binary changes with:

```console
gh workflow run release.yml --ref master -f tag=dry-run
```

If a binary build fails after crates.io publication, fix the build setup and
retry against the intended release source; never overwrite a published crate
version or move an existing release tag. A repair that changes source needs a
new crate version. To retry an existing package release:

```console
gh workflow run release.yml --ref PACKAGE-vVERSION -f tag=PACKAGE-vVERSION
```

Record actual completed run URLs and publication results in `NOTES.md` and
`HANDOFF.md`. A source push or queued release job is not a successful release.
