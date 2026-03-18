---
name: checklist
description: Run the CLAUDE.md rules checklist to verify nothing was missed before pushing or opening a PR
user_invocable: true
---

If the user is invoking this skill, it means you have likely been sloppy about respecting the instructions in CLAUDE.md. Refresh your memory on the checklist below and work through every item carefully.

## Rules Checklist (mirrored from CLAUDE.md)

1. **After every change**, update `README.md` and `CLAUDE.md` if the change affects documented behavior, architecture, or setup.
2. **Every PR** must include a changeset file. Run `bunx changeset` to create one in `.changeset/`. Do NOT edit `docs/changeset.md` directly — it is auto-generated on merge by the `merge-changesets` workflow.
3. **Every prompt** from the user must be saved verbatim to `docs/prompts/<branch-name>/` as a `.txt` file. Filename format: `{timestamp-seconds}-{slug}.txt`. Organize by feature branch name.
4. **When submitting PRs**, write them in the chat for user review. User may leave comments here or on GitHub.
5. **Branch strategy**: Be intentional about what branch you're working off of. Usually `main`, but agents may stack on each other when dependencies exist.
6. **All git branches** must be prefixed with `cdai__` (e.g., `cdai__add-contracts`).
7. **Every task ends with a PR**. After completing work, push the branch and open a PR. GitHub is source of truth — no code goes to main without review.
8. **`scripts/ci.sh` and `.github/workflows/ci.yml` must stay in sync.** If you change one, update the other. The local script mirrors the GitHub workflow exactly so you can validate before pushing.
9. **Run `./scripts/ci.sh` locally before pushing any commits or opening PRs.** CI must pass locally first. No exceptions. If you break CI, fix it before pushing.

## How to use this checklist

Go through each item and verify compliance for the current branch/task:

- [ ] Read `CLAUDE.md` rules section fresh
- [ ] Check all user prompts in this conversation are saved to `docs/prompts/<branch-name>/`
- [ ] Check `.changeset/` has a changeset file for this PR (run `bunx changeset` if not)
- [ ] Check `README.md` and `CLAUDE.md` are updated if behavior/architecture/setup changed
- [ ] Verify branch name has `cdai__` prefix
- [ ] If `scripts/ci.sh` or `.github/workflows/ci.yml` was modified, verify they are in sync
- [ ] Run `./scripts/ci.sh` and confirm it passes
- [ ] Write the PR description in chat for user review before creating on GitHub
