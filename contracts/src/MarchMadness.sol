// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

import {ByteBracket} from "./ByteBracket.sol";

/// @title MarchMadness — Seismic privacy-preserving bracket contest
/// @notice Brackets are stored as sbytes32 (shielded) and hidden until the submission deadline.
///         After the deadline, brackets become publicly readable. Scoring uses jimpo's ByteBracket
///         library. Winners split the prize pool equally.
contract MarchMadness {
    // ── Owner ──────────────────────────────────────────────────────────────
    address public owner;

    // ── Tournament parameters (set in constructor) ─────────────────────────
    uint256 public entryFee;
    uint256 public submissionDeadline; // unix timestamp
    string public tournamentDataIPFSHash;

    // ── State ──────────────────────────────────────────────────────────────
    mapping(address => sbytes32) private brackets; // SHIELDED bracket storage
    mapping(address => string) public tags;        // optional display name
    mapping(address => uint8) public scores;
    mapping(address => bool) public isScored;
    uint256 public numEntries;
    uint256 public numScored;

    // ── Results (only set by owner) ────────────────────────────────────────
    bytes32 public results;
    uint64 public scoringMask;
    uint256 public resultsPostedAt; // timestamp when results posted

    // ── Payout ─────────────────────────────────────────────────────────────
    uint8 public winningScore;
    uint256 public numWinners;
    uint256 public winnings; // set once when last bracket is scored
    mapping(address => bool) public hasCollectedWinnings;
    mapping(address => bool) public hasCollectedEntryFee;

    // ── Constants ──────────────────────────────────────────────────────────
    uint256 public constant NO_CONTEST_PERIOD = 28 days;
    bytes1 public constant SENTINEL = 0x01;

    // ── Events ─────────────────────────────────────────────────────────────
    event BracketSubmitted(address indexed account);
    event BracketScored(address indexed account, uint8 score);
    event ResultsPosted(bytes32 results);
    event WinningsCollected(address indexed account, uint256 amount);
    event EntryFeeCollected(address indexed account);

    // ── Constructor ────────────────────────────────────────────────────────
    constructor(
        uint256 _entryFee,
        uint256 _submissionDeadline,
        string memory _tournamentDataIPFSHash
    ) {
        owner = msg.sender;
        entryFee = _entryFee;
        submissionDeadline = _submissionDeadline;
        tournamentDataIPFSHash = _tournamentDataIPFSHash;
    }

    // ── Bracket Submission ─────────────────────────────────────────────────

    /// @notice Submit a shielded bracket with 1 ETH entry fee.
    /// @param bracket  The shielded bracket (last byte must be 0x01 sentinel).
    /// @param tag      Optional display name for the entrant.
    function submitBracket(sbytes32 bracket, string calldata tag) external payable {
        require(msg.value == entryFee, "Incorrect entry fee");
        require(block.timestamp < submissionDeadline, "Submission deadline passed");

        // Validate sentinel: last byte of bracket must be 0x01
        bytes32 raw = bytes32(bracket);
        require(raw[31] == SENTINEL, "Invalid sentinel byte");

        // Check address hasn't already submitted (sentinel check on existing bracket)
        bytes32 existing = bytes32(brackets[msg.sender]);
        require(existing[31] != SENTINEL, "Already submitted");

        brackets[msg.sender] = bracket;
        if (bytes(tag).length > 0) {
            tags[msg.sender] = tag;
        }
        numEntries++;

        emit BracketSubmitted(msg.sender);
    }

    /// @notice Update an already-submitted bracket (no additional fee required).
    /// @param bracket  The new shielded bracket (last byte must be 0x01 sentinel).
    function updateBracket(sbytes32 bracket) external {
        require(block.timestamp < submissionDeadline, "Submission deadline passed");

        // Require address HAS already submitted
        bytes32 existing = bytes32(brackets[msg.sender]);
        require(existing[31] == SENTINEL, "No bracket submitted");

        // Validate sentinel on new bracket
        bytes32 raw = bytes32(bracket);
        require(raw[31] == SENTINEL, "Invalid sentinel byte");

        brackets[msg.sender] = bracket;

        emit BracketSubmitted(msg.sender);
    }

    // ── Bracket Reading ────────────────────────────────────────────────────

    /// @notice Read a bracket. Before deadline: only the account owner can read (signed read).
    ///         After deadline: anyone can read.
    /// @dev THIS IS THE MOST SECURITY-CRITICAL FUNCTION.
    /// @param account  The address whose bracket to read.
    /// @return The bracket as bytes32 (unshielded).
    function getBracket(address account) public view returns (bytes32) {
        if (block.timestamp < submissionDeadline) {
            require(msg.sender == account, "Cannot read bracket before deadline");
        }
        return bytes32(brackets[account]);
    }

    // ── Results ────────────────────────────────────────────────────────────

    /// @notice Post tournament results. Owner only, once.
    /// @param _results  The tournament results as bytes32 (sentinel in last byte).
    function submitResults(bytes32 _results) external {
        require(msg.sender == owner, "Only owner");
        require(results == bytes32(0), "Results already posted");
        require(block.timestamp >= submissionDeadline, "Submission phase not over");
        require(_results[31] == SENTINEL, "Invalid sentinel byte");

        results = _results;
        scoringMask = ByteBracket.getScoringMask(_results);
        resultsPostedAt = block.timestamp;

        emit ResultsPosted(_results);
    }

    // ── Scoring ────────────────────────────────────────────────────────────

    /// @notice Score a bracket against the posted results.
    /// @param account  The address whose bracket to score.
    function scoreBracket(address account) external {
        require(results != bytes32(0), "Results not posted");
        require(!isScored[account], "Already scored");

        bytes32 b = bytes32(brackets[account]);
        require(b[31] == SENTINEL, "No bracket submitted");

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

        // When all brackets have been scored, calculate the per-winner payout
        if (numScored == numEntries) {
            winnings = (numEntries * entryFee) / numWinners;
        }

        emit BracketScored(account, score);
    }

    // ── Payout ─────────────────────────────────────────────────────────────

    /// @notice Collect winnings. Available once all brackets have been scored.
    function collectWinnings() external {
        require(numScored == numEntries, "Not all brackets scored");
        require(numEntries > 0, "No entries");
        require(scores[msg.sender] == winningScore, "Not a winner");
        require(isScored[msg.sender], "Not scored");
        require(!hasCollectedWinnings[msg.sender], "Already collected");

        hasCollectedWinnings[msg.sender] = true;

        emit WinningsCollected(msg.sender, winnings);

        (bool success,) = msg.sender.call{value: winnings}("");
        require(success, "Transfer failed");
    }

    /// @notice Collect entry fee refund if the contest is invalid.
    /// @dev Available if results were posted but not all brackets were scored within 28 days.
    function collectEntryFee() external {
        require(resultsPostedAt > 0, "Results not posted");
        require(block.timestamp >= resultsPostedAt + NO_CONTEST_PERIOD, "No-contest period not reached");
        require(numScored < numEntries, "All brackets scored, contest is valid");

        bytes32 b = bytes32(brackets[msg.sender]);
        require(b[31] == SENTINEL, "No bracket submitted");
        require(!hasCollectedEntryFee[msg.sender], "Already collected");

        hasCollectedEntryFee[msg.sender] = true;

        emit EntryFeeCollected(msg.sender);

        (bool success,) = msg.sender.call{value: entryFee}("");
        require(success, "Transfer failed");
    }

    // ── View Functions ─────────────────────────────────────────────────────

    function getEntryCount() external view returns (uint256) {
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
}
