# New-project checklist

The template's checklist, with this project's status. Items marked `[x]` are done; the rest are open.

## Make it yours

- [x] Replace the README with the project's own leaflet.
- [x] Fill in [ROADMAP.md](../../ROADMAP.md), above all **What it is not**.
- [x] Licence: MIT, see [LICENSE-MIT](../../LICENSE-MIT).
- [x] Replace [Getting started](../user-guide/getting-started.md) with the project's first steps.
- [x] Write [Architecture](architecture.md).
- [x] File format page: removed, because the program saves no files. If that changes, add a page and start at `schema_version` 1.
- [x] Fill in [Tests](testing.md) with the real commands.
- [x] Add the project's own rules to [CLAUDE.md](../../CLAUDE.md).

## Make CI real

- [x] Stack recipe: none fits a Rust GUI tool, so the recipe folders were removed. The CI jobs for Rust are written in `.github/workflows/ci.yml`.
- [x] Test jobs run on Windows and Linux.
- [ ] On GitHub: **Settings → Branches**, protect `main` (require a pull request and a passing CI run). Only the repository owner can do this.
- [ ] Check that a pull request shows the CI result before it can be merged. The first CI run has not happened yet.
- [ ] Commit `Cargo.lock` after a first local `cargo build`. It is not in the project yet, because the author's environment could not download the crates.

## Before the first release

- [x] No data format, so no `schema_version`.
- [x] `version-files.json` lists `Cargo.toml`. Add `Cargo.lock` once it exists.
- [x] `CHANGELOG.md` is kept under *Unreleased*.
- [x] The issue form asks for the system, graphics card and monitor.
- [x] Where users' files are kept: nowhere, the program saves nothing (said in the user guide).

## Keep it healthy

- [ ] Every user-visible change updates the user guide in the same pull request.
- [ ] Every decision goes in [the decision log](decisions.md).
