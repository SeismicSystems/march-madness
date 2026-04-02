// SPDX-License-Identifier: MIT
pragma solidity ^0.8.30;

import {ByteBracket} from "./ByteBracket.sol";
import {MarchMadness} from "./MarchMadness.sol";

/// @title MarchMadnessV2 — Migration-capable extension of MarchMadness
/// @notice Inherits all V1 logic unchanged. Adds owner-only bracket import,
///         tag import, and a non-mutating scoring preview.
///
/// @dev Migration workflow:
///   1. Deploy with same constructor params as V1 (deadline already in the past).
///   2. Import corrected entries via `batchImportEntries` (bit-reversed from V1 brackets),
///      sending msg.value == accounts.length * entryFee to restore the prize pool.
///   3. Import tags via `importTag` / batch loops.
///   4. Call `previewScore` to validate encoding before calling `submitResults`.
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
    ///         the sentinel bit set (MSB = 1). msg.value must equal entryFee.
    /// @param account  The original entrant's address.
    /// @param bracket  The corrected bracket in contract-correct encoding.
    function importEntry(address account, bytes8 bracket) external payable onlyOwner {
        if (msg.value != entryFee) revert IncorrectEntryFee(entryFee, msg.value);
        if (hasEntry[account]) revert AlreadySubmitted();
        if (bracket[0] & 0x80 == 0) revert InvalidSentinelByte();

        brackets[account] = sbytes8(bracket);
        hasEntry[account] = true;
        if (numEntries >= type(uint32).max) revert EntryCountOverflow();
        numEntries++;

        emit BracketSubmitted(account);
    }

    /// @notice Batch-import corrected bracket entries. Owner only.
    /// @dev    msg.value must equal accounts.length * entryFee to correctly restore
    ///         the prize pool. Reverts if any account already has an entry.
    ///         Recommended chunk size: 50 entries per tx to stay within gas limits.
    /// @param accounts   Array of entrant addresses.
    /// @param bracketList  Array of corrected brackets (contract-correct encoding), same length.
    function batchImportEntries(address[] calldata accounts, bytes8[] calldata bracketList)
        external
        payable
        onlyOwner
    {
        require(accounts.length == bracketList.length, "length mismatch");
        require(msg.value == accounts.length * entryFee, "incorrect payment");
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
}
