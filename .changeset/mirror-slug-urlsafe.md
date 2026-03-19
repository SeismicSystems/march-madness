---
"march-madness": minor
---

### Enforce URL-safe slugs in BracketMirror and BracketGroups contracts

- **Problem**: Mirror and group slugs accepted any UTF-8 string, including spaces and special characters (e.g. "Barracks Ballers"), making them unsuitable for URLs.
- **Fix**: Added `_validateSlugChars()` internal function to both BracketMirror and BracketGroups contracts. Slugs now must be lowercase alphanumeric + hyphens only (`[a-z0-9-]`), with no leading or trailing hyphens. New `SlugNotUrlSafe` error. Entry slugs in BracketMirror (`addEntry`, `updateEntrySlug`) are also validated.
- **Breaking**: Existing mirrors/groups with non-URL-safe slugs cannot be modified. New mirrors must use URL-safe slugs.
