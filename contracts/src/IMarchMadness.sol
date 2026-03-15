// SPDX-License-Identifier: MIT
pragma solidity ^0.8.30;

/// @title IMarchMadness — public interface for the MarchMadness bracket contest
/// @notice Defines the full public API for MarchMadness. Implementation details
///         (shielded storage, ByteBracket internals) live in the contract itself.
interface IMarchMadness {
    // ── Errors ───────────────────────────────────────────────────────────
    error IncorrectEntryFee(uint256 expected, uint256 actual);
    error SubmissionDeadlinePassed();
    error InvalidSentinelByte();
    error AlreadySubmitted();
    error NoBracketSubmitted();
    error CannotReadBracketBeforeDeadline();
    error OnlyOwner();
    error ResultsAlreadyPosted();
    error SubmissionPhaseNotOver();
    error ResultsNotPosted();
    error AlreadyScored();
    error ScoringWindowClosed();
    error ScoringWindowStillOpen();
    error NotAWinner();
    error AlreadyCollected();
    error TransferFailed();

    // ── Events ───────────────────────────────────────────────────────────
    event BracketSubmitted(address indexed account);
    event TagSet(address indexed account, string tag);
    event BracketScored(address indexed account, uint8 score);
    event ResultsPosted(bytes8 results);
    event WinningsCollected(address indexed account, uint256 amount);

    // ── Tournament parameters ────────────────────────────────────────────
    function owner() external view returns (address);
    function year() external view returns (uint16);
    function entryFee() external view returns (uint256);
    function submissionDeadline() external view returns (uint256);

    // ── Bracket submission ───────────────────────────────────────────────
    function submitBracket(sbytes8 bracket) external payable;
    function updateBracket(sbytes8 bracket) external;
    function setTag(string calldata tag) external;

    // ── Bracket reading ──────────────────────────────────────────────────
    function getBracket(address account) external view returns (bytes8);

    // ── State ────────────────────────────────────────────────────────────
    function hasEntry(address account) external view returns (bool);
    function tags(address account) external view returns (string memory);
    function numEntries() external view returns (uint32);
    function getEntryCount() external view returns (uint32);
    function getTag(address account) external view returns (string memory);

    // ── Results + scoring ────────────────────────────────────────────────
    function submitResults(bytes8 results) external;
    function scoreBracket(address account) external;
    function results() external view returns (bytes8);
    function scoringMask() external view returns (uint64);
    function resultsPostedAt() external view returns (uint256);
    function scores(address account) external view returns (uint8);
    function isScored(address account) external view returns (bool);
    function getScore(address account) external view returns (uint8);
    function getIsScored(address account) external view returns (bool);
    function numScored() external view returns (uint256);

    // ── Payouts ──────────────────────────────────────────────────────────
    function collectWinnings() external;
    function winningScore() external view returns (uint8);
    function numWinners() external view returns (uint256);
    function hasCollectedWinnings(address account) external view returns (bool);

    // ── Constants ────────────────────────────────────────────────────────
    function SCORING_DURATION() external view returns (uint256);
}
