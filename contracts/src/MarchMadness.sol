// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

import {ByteBracket} from "./ByteBracket.sol";

/// @title MarchMadness — Seismic privacy-preserving bracket contest
/// @notice Brackets are stored as sbytes8 (shielded) and hidden until the submission deadline.
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
    mapping(address => sbytes8) private brackets; // SHIELDED bracket storage
    mapping(address => string) public tags;        // optional display name
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
    uint256 public winnings; // set once when last bracket is scored
    mapping(address => bool) public hasCollectedWinnings;
    mapping(address => bool) public hasCollectedEntryFee;

    // ── Constants ──────────────────────────────────────────────────────────
    uint256 public constant NO_CONTEST_PERIOD = 28 days;

    // ── Events ─────────────────────────────────────────────────────────────
    event BracketSubmitted(address indexed account);
    event BracketScored(address indexed account, uint8 score);
    event ResultsPosted(bytes8 results);
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
    /// @param bracket  The shielded bracket (MSB must be set as sentinel).
    function submitBracket(sbytes8 bracket) external payable {
        require(msg.value == entryFee, "Incorrect entry fee");
        require(block.timestamp < submissionDeadline, "Submission deadline passed");

        // Validate sentinel: MSB (bit 63) must be set
        bytes8 raw = bytes8(bracket);
        require(raw[0] & 0x80 != 0, "Invalid sentinel byte");

        // Check address hasn't already submitted (sentinel check on existing bracket)
        bytes8 existing = bytes8(brackets[msg.sender]);
        require(existing[0] & 0x80 == 0, "Already submitted");

        brackets[msg.sender] = bracket;
        require(numEntries < type(uint32).max, "Entry count overflow");
        numEntries++;

        emit BracketSubmitted(msg.sender);
    }

    /// @notice Set or update an optional display name (tag). Requires bracket already submitted.
    /// @param tag  The display name to associate with the sender's bracket.
    function setTag(string calldata tag) external {
        bytes8 existing = bytes8(brackets[msg.sender]);
        require(existing[0] & 0x80 != 0, "No bracket submitted");
        tags[msg.sender] = tag;
    }

    /// @notice Update an already-submitted bracket (no additional fee required).
    /// @param bracket  The new shielded bracket (MSB must be set as sentinel).
    function updateBracket(sbytes8 bracket) external {
        require(block.timestamp < submissionDeadline, "Submission deadline passed");

        // Require address HAS already submitted
        bytes8 existing = bytes8(brackets[msg.sender]);
        require(existing[0] & 0x80 != 0, "No bracket submitted");

        // Validate sentinel on new bracket
        bytes8 raw = bytes8(bracket);
        require(raw[0] & 0x80 != 0, "Invalid sentinel byte");

        brackets[msg.sender] = bracket;

        emit BracketSubmitted(msg.sender);
    }

    // ── Bracket Reading ────────────────────────────────────────────────────

    /// @notice Read a bracket. Before deadline: only the account owner can read (signed read).
    ///         After deadline: anyone can read.
    /// @dev THIS IS THE MOST SECURITY-CRITICAL FUNCTION.
    /// @param account  The address whose bracket to read.
    /// @return The bracket as bytes8 (unshielded).
    function getBracket(address account) public view returns (bytes8) {
        if (block.timestamp < submissionDeadline) {
            require(msg.sender == account, "Cannot read bracket before deadline");
        }
        return bytes8(brackets[account]);
    }

    // ── Results ────────────────────────────────────────────────────────────

    /// @notice Post tournament results. Owner only, once.
    /// @param _results  The tournament results as bytes8 (MSB must be set as sentinel).
    function submitResults(bytes8 _results) external {
        require(msg.sender == owner, "Only owner");
        require(results == bytes8(0), "Results already posted");
        require(block.timestamp >= submissionDeadline, "Submission phase not over");
        require(_results[0] & 0x80 != 0, "Invalid sentinel byte");

        results = _results;
        scoringMask = ByteBracket.getScoringMask(_results);
        resultsPostedAt = block.timestamp;

        emit ResultsPosted(_results);
    }

    // ── Scoring ────────────────────────────────────────────────────────────

    /// @notice Score a bracket against the posted results.
    /// @param account  The address whose bracket to score.
    function scoreBracket(address account) external {
        require(results != bytes8(0), "Results not posted");
        require(!isScored[account], "Already scored");

        bytes8 b = bytes8(brackets[account]);
        require(b[0] & 0x80 != 0, "No bracket submitted");

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

        bytes8 b = bytes8(brackets[msg.sender]);
        require(b[0] & 0x80 != 0, "No bracket submitted");
        require(!hasCollectedEntryFee[msg.sender], "Already collected");

        hasCollectedEntryFee[msg.sender] = true;

        emit EntryFeeCollected(msg.sender);

        (bool success,) = msg.sender.call{value: entryFee}("");
        require(success, "Transfer failed");
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
}
