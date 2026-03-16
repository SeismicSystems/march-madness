//! ABI definitions for contracts using alloy's sol! macro.

use alloy_sol_types::sol;

// ── MarchMadness ─────────────────────────────────────────────────────
sol! {
    event BracketSubmitted(address indexed account);
    event TagSet(address indexed account, string tag);
    event BracketScored(address indexed account, uint8 score);
    event ResultsPosted(bytes8 results);
    event WinningsCollected(address indexed account, uint256 amount);

    function getEntryCount() external view returns (uint32);
    function getBracket(address account) external view returns (bytes8);
}

// ── BracketGroups ────────────────────────────────────────────────────
sol! {
    event GroupCreated(uint32 indexed groupId, string slug, string displayName, address creator, bool hasPassword);
    event MemberJoined(uint32 indexed groupId, address indexed addr);
    event MemberLeft(uint32 indexed groupId, address indexed addr);
}

// ── BracketMirror ────────────────────────────────────────────────────
sol! {
    event MirrorCreated(uint256 indexed mirrorId, string slug, string displayName, address admin);
    // Slug-based events (after contract update)
    event EntryAdded(uint256 indexed mirrorId, string slug);
    event EntryRemoved(uint256 indexed mirrorId, string slug);

    function getEntryBySlug(uint256 mirrorId, string slug) external view returns (bytes8 bracket, string entrySlug);
}
