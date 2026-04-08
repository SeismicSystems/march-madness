// SPDX-License-Identifier: MIT
pragma solidity ^0.8.30;

import {ByteBracket} from "./ByteBracket.sol";
import {IMarchMadness} from "./IMarchMadness.sol";

/// @title MarchMadness — Seismic privacy-preserving bracket contest
/// @notice Brackets are stored as sbytes8 (shielded) and hidden until the submission deadline.
///         After the deadline, brackets become publicly readable. Scoring uses jimpo's ByteBracket
///         library. Winners split the prize pool equally.
contract MarchMadness is IMarchMadness {
    // ── Owner ──────────────────────────────────────────────────────────────
    address public owner;

    // ── Tournament parameters (set in constructor) ─────────────────────────
    uint16 public year; // tournament season (e.g. 2026)
    uint256 public entryFee;
    uint256 public submissionDeadline; // unix timestamp

    // ── State ──────────────────────────────────────────────────────────────
    mapping(address => sbytes8) internal brackets; // SHIELDED bracket storage
    mapping(address => bool) public hasEntry; // unshielded — publicly readable
    mapping(address => string) public tags; // optional display name
    mapping(address => uint8) public scores;
    mapping(address => bool) public isScored;
    uint32 public numEntries;
    uint256 public numScored;

    // ── Results (only set by owner) ────────────────────────────────────────
    bytes8 public results;
    uint64 public scoringMask;
    uint256 public resultsPostedAt; // timestamp when results posted

    // ── Payout ─────────────────────────────────────────────────────────────
    uint8 public winningScore;
    uint256 public numWinners;
    mapping(address => bool) public hasCollectedWinnings;
    mapping(address => bool) public hasCollectedEntryFee;

    // ── Constants ──────────────────────────────────────────────────────────
    uint256 public constant SCORING_DURATION = 7 days;
    uint256 public constant RESULTS_DEADLINE = 90 days;

    // ── Constructor ────────────────────────────────────────────────────────
    constructor(uint16 _year, uint256 _entryFee, uint256 _submissionDeadline) {
        owner = msg.sender;
        year = _year;
        entryFee = _entryFee;
        submissionDeadline = _submissionDeadline;
    }

    // ── Bracket Submission ─────────────────────────────────────────────────

    /// @notice Submit a shielded bracket with entry fee.
    /// @param bracket  The shielded bracket (MSB must be set as sentinel).
    function submitBracket(sbytes8 bracket) external payable {
        if (msg.value != entryFee) revert IncorrectEntryFee(entryFee, msg.value);
        if (block.timestamp >= submissionDeadline) revert SubmissionDeadlinePassed();

        // Validate sentinel: MSB (bit 63) must be set
        bytes8 raw = bytes8(bracket);
        if (raw[0] & 0x80 == 0) revert InvalidSentinelByte();

        // Check address hasn't already submitted (sentinel check on existing bracket)
        bytes8 existing = bytes8(brackets[msg.sender]);
        if (existing[0] & 0x80 != 0) revert AlreadySubmitted();

        brackets[msg.sender] = bracket;
        hasEntry[msg.sender] = true;
        if (numEntries >= type(uint32).max) revert EntryCountOverflow();
        numEntries++;

        emit BracketSubmitted(msg.sender);
    }

    /// @notice Set or update an optional display name (tag). Requires bracket already submitted.
    /// @param tag  The display name to associate with the sender's bracket.
    function setTag(string calldata tag) external {
        bytes8 existing = bytes8(brackets[msg.sender]);
        if (existing[0] & 0x80 == 0) revert NoBracketSubmitted();
        tags[msg.sender] = tag;

        emit TagSet(msg.sender, tag);
    }

    /// @notice Update an already-submitted bracket (no additional fee required).
    /// @param bracket  The new shielded bracket (MSB must be set as sentinel).
    function updateBracket(sbytes8 bracket) external {
        if (block.timestamp >= submissionDeadline) revert SubmissionDeadlinePassed();

        // Require address HAS already submitted
        bytes8 existing = bytes8(brackets[msg.sender]);
        if (existing[0] & 0x80 == 0) revert NoBracketSubmitted();

        // Validate sentinel on new bracket
        bytes8 raw = bytes8(bracket);
        if (raw[0] & 0x80 == 0) revert InvalidSentinelByte();

        brackets[msg.sender] = bracket;

        emit BracketSubmitted(msg.sender);
    }

    // ── Bracket Reading ────────────────────────────────────────────────────

    /// @notice Read a bracket. Before deadline: only the account owner can read (signed read).
    ///         After deadline: anyone can read.
    /// @dev THIS IS THE MOST SECURITY-CRITICAL FUNCTION.
    /// @param account  The address whose bracket to read.
    /// @return The bracket as bytes8 (unshielded).
    function getBracket(address account) public view virtual returns (bytes8) {
        if (block.timestamp < submissionDeadline) {
            if (msg.sender != account) revert CannotReadBracketBeforeDeadline();
        }
        return bytes8(brackets[account]);
    }

    // ── Results ────────────────────────────────────────────────────────────

    /// @notice Post tournament results. Owner only, once.
    /// @param _results  The tournament results as bytes8 (MSB must be set as sentinel).
    function submitResults(bytes8 _results) external {
        if (msg.sender != owner) revert OnlyOwner();
        if (results != bytes8(0)) revert ResultsAlreadyPosted();
        if (block.timestamp < submissionDeadline) revert SubmissionPhaseNotOver();
        if (block.timestamp > submissionDeadline + RESULTS_DEADLINE) revert ResultsSubmissionWindowClosed();
        if (_results[0] & 0x80 == 0) revert InvalidSentinelByte();

        results = _results;
        scoringMask = ByteBracket.getScoringMask(_results);
        resultsPostedAt = block.timestamp;

        emit ResultsPosted(_results);
    }

    // ── Scoring ────────────────────────────────────────────────────────────

    /// @notice Score a bracket against the posted results. Anyone can call this.
    /// @param account  The address whose bracket to score.
    function scoreBracket(address account) external {
        if (results == bytes8(0)) revert ResultsNotPosted();
        if (isScored[account]) revert AlreadyScored();

        bytes8 b = bytes8(brackets[account]);
        if (b[0] & 0x80 == 0) revert NoBracketSubmitted();

        uint8 score = ByteBracket.getBracketScore(b, results, scoringMask);
        scores[account] = score;
        isScored[account] = true;
        numScored++;

        // Update winning score and winner count
        if (score > winningScore) {
            winningScore = score;
            numWinners = 1;
        } else if (score == winningScore) {
            numWinners++;
        }

        emit BracketScored(account, score);
    }

    // ── Payout ─────────────────────────────────────────────────────────────

    /// @notice Collect winnings. Available once the scoring window has closed.
    ///         Winners are the entrants with the highest scored bracket.
    function collectWinnings() external {
        if (resultsPostedAt == 0) revert ResultsNotPosted();
        if (block.timestamp < resultsPostedAt + SCORING_DURATION) revert ScoringWindowStillOpen();
        if (numWinners == 0) revert NoBracketsScored();
        if (scores[msg.sender] != winningScore) revert NotAWinner();
        if (!isScored[msg.sender]) revert NotScored();
        if (hasCollectedWinnings[msg.sender]) revert AlreadyCollected();

        hasCollectedWinnings[msg.sender] = true;
        uint256 payout = (uint256(numEntries) * entryFee) / numWinners;

        emit WinningsCollected(msg.sender, payout);

        (bool success,) = msg.sender.call{value: payout}("");
        if (!success) revert TransferFailed();
    }

    /// @notice Reclaim entry fee if the owner failed to post results within the 90-day window.
    function collectEntryFee() external {
        if (results != bytes8(0)) revert ResultsAlreadyPosted();
        if (block.timestamp <= submissionDeadline + RESULTS_DEADLINE) revert ResultsWindowStillOpen();
        if (!hasEntry[msg.sender]) revert NoBracketSubmitted();
        if (hasCollectedEntryFee[msg.sender]) revert AlreadyCollected();

        hasCollectedEntryFee[msg.sender] = true;

        (bool success,) = msg.sender.call{value: entryFee}("");
        if (!success) revert TransferFailed();
    }

    // ── View Functions ─────────────────────────────────────────────────────

    function getEntryCount() external view returns (uint32) {
        return numEntries;
    }

    function getScore(address account) external view returns (uint8) {
        return scores[account];
    }

    function getIsScored(address account) external view returns (bool) {
        return isScored[account];
    }

    function getTag(address account) external view returns (string memory) {
        return tags[account];
    }

    // ── Scoring Preview ────────────────────────────────────────────────────

    /// @notice Compute the score an account would receive against candidate results,
    ///         without mutating any state. Use this to validate raw results bytes
    ///         before calling `submitResults` / `scoreBracket`.
    ///
    /// @dev    Pre-deadline privacy is preserved: `getBracket` enforces the
    ///         msg.sender == account check before the deadline. The sentinel on
    ///         `rawResults` is validated here to catch encoding mistakes early.
    ///
    /// @param account     The entrant to preview.
    /// @param rawResults  Candidate results bytes8 (must have sentinel bit set).
    /// @return            The score (0–192) the account would receive.
    function previewScore(address account, bytes8 rawResults) external view returns (uint8) {
        if (!hasEntry[account]) revert NoBracketSubmitted();
        if (rawResults[0] & 0x80 == 0) revert InvalidSentinelByte();
        bytes8 bracket = getBracket(account);
        if (bracket[0] & 0x80 == 0) revert NoBracketSubmitted();
        uint64 mask = ByteBracket.getScoringMask(rawResults);
        return ByteBracket.getBracketScore(bracket, rawResults, mask);
    }
}
