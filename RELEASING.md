# Release Process

This project uses [`cargo-release`](https://github.com/crate-ci/cargo-release) to automate the release process for the workspace.

## Prerequisites

Install `cargo-release`:

```bash
cargo install cargo-release
```

## Release Workflow

### 1. Prepare for Release

Before releasing, ensure:
- All tests pass: `cargo test --all --all-features`
- Code is formatted: `cargo fmt --all`
- Lints pass: `cargo clippy --all --all-features`
- Documentation builds: `cargo doc --all --all-features --no-deps`
- The working directory is clean (commit any pending changes)

### 2. Dry Run

Always do a dry run first to see what will happen:

```bash
# Patch release (0.0.x)
cargo release --workspace patch --dry-run

# Minor release (0.x.0)
cargo release --workspace minor --dry-run

# Major release (x.0.0)
cargo release --workspace major --dry-run
```

Review the output carefully. It will show:
- Which crates will be updated
- What the new versions will be
- What commits and tags will be created
- The publishing order

### 3. Execute Release

Once you're satisfied with the dry run:

```bash
# Patch release
cargo release --workspace patch --execute

# Minor release
cargo release --workspace minor --execute

# Major release
cargo release --workspace major --execute
```

This will:
1. Update version numbers in all `Cargo.toml` files
2. Update dependency versions between workspace crates
3. Create a git commit with the version changes
4. Create git tags (one per crate: `v<version>`)
5. Publish crates to crates.io in dependency order

### 4. Push to GitHub

The release process does NOT automatically push (configured in `release.toml`). After reviewing the commits and tags locally:

```bash
# Push commits
git push origin master

# Push all tags
git push origin --tags
```

### 5. Create GitHub Release (Optional)

You can manually create a GitHub release from the tags, or use the GitHub CLI:

```bash
gh release create v11.0.2 --generate-notes
```

## Troubleshooting

### Publishing Failed Midway

If publishing fails for a crate after some crates have already been published:

1. Fix the issue (e.g., update metadata, fix tests)
2. Re-run `cargo release` - it will skip already-published crates
3. Or manually publish the remaining crates: `cargo publish -p <crate-name>`

### Wrong Version Released

If you published the wrong version:

1. You cannot unpublish from crates.io (versions are immutable)
2. You can yank the version: `cargo yank --vers <version> <crate-name>`
3. Release a new corrected version

### Undo Before Publishing

If you created commits/tags but haven't pushed or published yet:

```bash
# Remove the last commit (keeps changes)
git reset --soft HEAD~1

# Delete local tags
git tag -d v11.0.2  # repeat for each tag
```

## Release Strategy

### Version Coordination

This workspace uses **coordinated versioning** for core crates:
- `varlink`, `varlink_generator`, `varlink_stdinterfaces` share major version numbers
- `varlink_parser` has independent versioning (currently v5.x)
- `varlink_derive` has independent versioning (currently v10.x)

### When to Release

**Patch Release (x.y.Z):**
- Bug fixes
- Documentation updates
- Performance improvements (no API changes)

**Minor Release (x.Y.0):**
- New features (backward compatible)
- New async functionality
- Deprecations (but not removals)

**Major Release (X.0.0):**
- Breaking API changes
- Removal of deprecated features
- Major architectural changes

## Configuration Files

- `release.toml` - Main cargo-release configuration
- `Cargo.toml` (workspace root) - Workspace-level release metadata
- Each crate's `Cargo.toml` - Individual crate versions and dependencies
