// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

/// @title IMarchMadness — minimal interface for composing contracts
/// @notice BracketGroups imports this instead of the full MarchMadness contract,
///         so it only needs the deployed address (not the full artifact) at deploy time.
interface IMarchMadness {
    function hasEntry(address account) external view returns (bool);
    function results() external view returns (bytes8);
    function scoringMask() external view returns (uint64);
    function resultsPostedAt() external view returns (uint256);
    function getBracket(address account) external view returns (bytes8);
}
