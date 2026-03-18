---
---

Move group membership tracking from frontend localStorage to Redis. Add `mm:address_groups` reverse mapping (address → group IDs) maintained by the indexer on join/leave events. New server endpoint `GET /address/:address/groups`. Frontend now fetches membership from API; localStorage only stores passphrases (client-side secrets).
