---
"march-madness": patch
---

### 2026-03-18 — Use PUSH_TOKEN in merge-changesets workflow

- **CI**: merge-changesets workflow now checks out with `PUSH_TOKEN` (fine-grained PAT) instead of the default `GITHUB_TOKEN`, allowing it to push to `main` past branch protection rulesets.
