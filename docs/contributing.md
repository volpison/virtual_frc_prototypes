# Contributing

How changes are made, what CI does and how a release is made. The reasons are in [the conventions](../CONVENTIONS.md#4-workflow-small-checked-boring).

## The workflow

- Work happens on a branch, never directly on `main`. Branch names that Claude creates start with `claude/`.
- Each change is a **pull request** with one topic. The description says what changed, why, and what to check before merging (the pull request template asks for it).
- CI must be green (a ✓ in the pull request) before merging.
- A change that moves code without changing behaviour goes in its own pull request.
- Commit messages say what changed and why, in plain words.

Everyday steps:

```
git checkout -b my-change
git add .
git commit -m "What changed"
git push -u origin my-change
```

then open a pull request on GitHub.

## What CI is

A *workflow* is a recipe in `.github/workflows/` that GitHub runs on its own computers. `ci.yml` runs by itself on every push to `main` and every pull request; open the **Actions** tab to see a green ✓ or a red ✗, and click a red one to read the error. See [Tests](technical/testing.md).

## Making a release

The version number is written in several places (a manifest, a lock file, an installer setting). [scripts/version.mjs](../scripts/version.mjs) keeps them equal, and the **Release** workflow (`release.yml`) refuses a tag that disagrees with them.

1. **Choose the number.** New features raise the middle number, only fixes raise the last one (`1.2.0`, `1.2.1`). Change it everywhere at once:

   ```
   node scripts/version.mjs set 1.2.0
   ```

2. Move the *Unreleased* entries of [CHANGELOG.md](../CHANGELOG.md) under the new version and date. Send that change as a pull request and merge it when CI is green.
3. Tag the merged `main` and push the tag:

   ```
   git checkout main
   git pull
   git tag v1.2.0
   git push origin v1.2.0
   ```

4. Open **Actions** to watch the *Release* run. If its first step fails, the tag and the version disagree: delete the tag (`git push --delete origin v1.2.0` and `git tag -d v1.2.0`), fix the version and tag again.
5. Under **Releases**, a **draft** appears with notes made from the merged pull requests. Edit it and press **Publish**, or leave it as a draft if you do not publish binaries.

Useful commands: `node scripts/version.mjs show` prints the version in every file, `node scripts/version.mjs check` fails if they differ.

### Telling the script where the version lives

List the files in [version-files.json](../version-files.json). Each entry is a file and how to find the version in it:

```json
{
  "files": [
    { "file": "package.json", "kind": "json" },
    { "file": "Cargo.toml", "kind": "toml", "section": "package" },
    { "file": "pyproject.toml", "kind": "toml", "section": "project" },
    { "file": "mypackage/__init__.py", "kind": "regex", "pattern": "(__version__ = \")([^\"]+)(\")" }
  ]
}
```

- `json`: the first `"version": "…"` in the file (`package.json`, `tauri.conf.json`).
- `toml`: `version = "…"` inside the named `[section]` (`[package]` in `Cargo.toml`, `[project]` in `pyproject.toml`).
- `regex`: your own pattern with three groups, *(text before)(the version)(text after)*.

Lock files (`package-lock.json`, `Cargo.lock`) can be added the same way when they carry the project's own version. The script edits the text and never rewrites a file, so line endings and every other line stay as they were. The template ships with an empty list: until you fill it in, `check` says so and the release workflow stops.

## Keeping the docs healthy

- A user-facing change updates `docs/user-guide/`; a change to how the project is built or to its data format updates `docs/technical/`.
- A new page is added to [the index](index.md). `node scripts/check-docs.mjs` fails if a link or an index entry is broken, and CI runs it.
- The README stays short: what it is, what you can do, how to run it, links.
