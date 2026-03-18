---
"march-madness": patch
---

Fix public groups showing as free: add entry_fee to indexer → Redis → server API pipeline

The GroupCreated event doesn't include entryFee, so the indexer now reads it from the
contract via getGroup() after seeing the event. The field flows through GroupData (Redis),
GroupResponse (server API), and is consumed by the frontend's usePublicGroups hook.
