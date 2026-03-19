---
"ncaa-feed": patch
"seismic-march-madness": patch
---

Add [ncaa] section to mappings.toml for NCAA API name aliases. The mapper now loads these aliases at startup, resolving mismatches like "South Fla." → "South Florida" and "Saint Mary's (CA)" → "Saint Mary's" that caused unresolved team name warnings.
