---
"march-madness": minor
---

### Enforce URL-safe slugs in mirror and group client libraries

- **Problem**: Mirror and group slugs accepted any string, including spaces and special characters (e.g. "Barracks Ballers"), making them unsuitable for URLs.
- **Fix**: Added client-side slug validation in both layers:
  - **Rust**: `validate_slug()` in mirror-importer rejects non-URL-safe slugs before writing platform.json. Also fixed default slug from `YAHOO-{id}` (uppercase) to `yahoo-{id}`.
  - **TypeScript**: `assertUrlSafeSlug()` in the client library validates slugs in `createMirror`, `addEntry`, `updateEntrySlug`, `createGroup`, and `createGroupWithPassword` before sending transactions.
- Slugs must be lowercase alphanumeric + hyphens (`[a-z0-9-]`), no leading/trailing hyphens.
