//! ABI definitions for the MarchMadness contract using alloy's sol! macro.

use alloy_sol_types::sol;

sol! {
    event BracketSubmitted(address indexed account);
    event TagSet(address indexed account, string tag);
    event BracketScored(address indexed account, uint8 score);
    event ResultsPosted(bytes8 results);
    event WinningsCollected(address indexed account, uint256 amount);

    function getEntryCount() external view returns (uint32);
    function getBracket(address account) external view returns (bytes8);
}
