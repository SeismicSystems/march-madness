---
"march-madness": patch
---

### 2026-03-18 — Switch to @changesets/cli workflow

- **Workflow**: PRs now add individual `.changeset/*.md` files instead of editing `docs/changeset.md` directly. On merge to main, the `merge-changesets` GitHub Action collects entries, prepends them to `docs/changeset.md`, and deletes the individual files. Eliminates changeset merge conflicts.
- **CI**: Changeset check now verifies a `.changeset/*.md` file was added AND that `docs/changeset.md` was not directly modified. Both `ci.yml` and `ci.sh` updated.
- **Deps**: Added `@changesets/cli` and `@changesets/changelog-github` as dev dependencies.
