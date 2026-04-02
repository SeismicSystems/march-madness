// SPDX-License-Identifier: MIT
pragma solidity ^0.8.30;

import {ByteBracket} from "./ByteBracket.sol";
import {MarchMadness} from "./MarchMadness.sol";

/// @title MarchMadnessV2 — Migration-capable extension of MarchMadness
/// @notice Inherits all V1 logic unchanged. Adds owner-only bracket import,
///         tag import, ETH funding, and a non-mutating scoring preview.
///
/// @dev Migration workflow:
///   1. Deploy with same constructor params as V1 (deadline already in the past).
///   2. Fund with `fund()` to match the V1 prize pool balance.
///   3. Import corrected entries via `batchImportEntries` (bit-reversed from V1 brackets).
///   4. Import tags via `importTag` / batch loops.
///   5. Call `previewScore` to validate encoding before calling `submitResults`.
///
/// Storage layout: inherits all V1 slots in order. No new persistent state added.
contract MarchMadnessV2 is MarchMadness {
    // ── Constructor ────────────────────────────────────────────────────────
    constructor(uint16 _year, uint256 _entryFee, uint256 _submissionDeadline)
        MarchMadness(_year, _entryFee, _submissionDeadline)
    {}

    // ── Access Control ─────────────────────────────────────────────────────
    modifier onlyOwner() {
        if (msg.sender != owner) revert OnlyOwner();
        _;
    }

    // ── Entry Import ───────────────────────────────────────────────────────

    /// @notice Import a single corrected bracket entry. Owner only.
    /// @dev    Caller is responsible for applying `reverse_63_game_bits` to the
    ///         original on-chain bracket before passing it here. Bracket must have
    ///         the sentinel bit set (MSB = 1).
    /// @param account  The original entrant's address.
    /// @param bracket  The corrected bracket in contract-correct encoding.
    function importEntry(address account, bytes8 bracket) external onlyOwner {
        if (hasEntry[account]) revert AlreadySubmitted();
        if (bracket[0] & 0x80 == 0) revert InvalidSentinelByte();

        brackets[account] = sbytes8(bracket);
        hasEntry[account] = true;
        if (numEntries >= type(uint32).max) revert EntryCountOverflow();
        numEntries++;

        emit BracketSubmitted(account);
    }

    /// @notice Batch-import corrected bracket entries. Owner only.
    /// @dev    Idempotent: silently skips accounts that already have entries.
    ///         Recommended chunk size: 50 entries per tx to stay within gas limits.
    /// @param accounts   Array of entrant addresses.
    /// @param bracketList  Array of corrected brackets (contract-correct encoding), same length.
    function batchImportEntries(address[] calldata accounts, bytes8[] calldata bracketList)
        external
        onlyOwner
    {
        require(accounts.length == bracketList.length, "length mismatch");
        for (uint256 i = 0; i < accounts.length; i++) {
            if (hasEntry[accounts[i]]) continue;
            bytes8 bracket = bracketList[i];
            if (bracket[0] & 0x80 == 0) revert InvalidSentinelByte();
            brackets[accounts[i]] = sbytes8(bracket);
            hasEntry[accounts[i]] = true;
            if (numEntries >= type(uint32).max) revert EntryCountOverflow();
            numEntries++;
            emit BracketSubmitted(accounts[i]);
        }
    }

    // ── Tag Import ─────────────────────────────────────────────────────────

    /// @notice Import a display tag for an already-imported entry. Owner only.
    /// @param account  The entrant's address (must already have an entry).
    /// @param tag      The display name to associate.
    function importTag(address account, string calldata tag) external onlyOwner {
        if (!hasEntry[account]) revert NoBracketSubmitted();
        tags[account] = tag;
        emit TagSet(account, tag);
    }

    // ── ETH Funding ────────────────────────────────────────────────────────

    /// @notice Fund the contract with ETH to restore the prize pool balance.
    function fund() external payable {}

    /// @notice Accept ETH transfers directly.
    receive() external payable {}

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
