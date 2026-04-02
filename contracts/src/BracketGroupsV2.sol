// SPDX-License-Identifier: MIT
pragma solidity ^0.8.30;

import {BracketGroups} from "./BracketGroups.sol";

/// @title BracketGroupsV2 — Migration-capable extension of BracketGroups
/// @notice Inherits all V1 group logic unchanged. Adds:
///   - `owner` address with import-only privileges
///   - Owner-only group and member import for post-cutover migration
///   - ETH funding paths to restore group prize pool balances
///
/// @dev Migration workflow:
///   1. Deploy pointing at MarchMadnessV2 address.
///   2. For each old group: call `importGroup`, then `batchImportMembers`.
///   3. Fund with `fund()` to match the V1 total group balance.
///
/// Storage layout: inherits all BracketGroups V1 slots in order, then appends `owner`.
contract BracketGroupsV2 is BracketGroups {
    // ── Owner ──────────────────────────────────────────────────────────────
    address public owner;

    // ── Constructor ────────────────────────────────────────────────────────
    constructor(address _marchMadness) BracketGroups(_marchMadness) {
        owner = msg.sender;
    }

    // ── Access Control ─────────────────────────────────────────────────────
    modifier onlyOwner() {
        require(msg.sender == owner, "not owner");
        _;
    }

    // ── Group Import ───────────────────────────────────────────────────────

    /// @notice Import a group from the V1 contract. Owner only.
    /// @dev    Creates the group metadata without auto-joining the creator as a member.
    ///         All members are imported separately via `importMember` / `batchImportMembers`.
    ///         Password protection is not migrated — joins are closed post-deadline.
    /// @param slug         URL-safe slug (must be unique).
    /// @param displayName  Human-readable group name.
    /// @param entryFee     Group entry fee (wei) — for payout calculation only.
    /// @param creator      The original group creator's address.
    /// @return groupId     The newly assigned group ID.
    function importGroup(
        string calldata slug,
        string calldata displayName,
        uint256 entryFee,
        address creator
    ) external onlyOwner returns (uint32 groupId) {
        bytes memory slugBytes = bytes(slug);
        if (slugBytes.length == 0) revert SlugCannotBeEmpty();
        if (slugBytes.length > MAX_SLUG_LENGTH) revert SlugTooLong();

        bytes32 slugHash = keccak256(slugBytes);
        if (slugToGroupId[slugHash] != 0) revert SlugAlreadyTaken();

        groupId = nextGroupId++;

        _groups[groupId] = Group({
            slug: slug,
            displayName: displayName,
            creator: creator,
            entryCount: 0,
            entryFee: entryFee,
            hasPassword: false
        });

        slugToGroupId[slugHash] = groupId;

        emit GroupCreated(groupId, slug, displayName, creator, false);
    }

    // ── Member Import ──────────────────────────────────────────────────────

    /// @notice Import a single group member. Owner only.
    /// @dev    Does not validate that `addr` has a bracket in the main contract —
    ///         migration assumes the entry list is authoritative.
    ///         Silently skips if the address is already a member (idempotent).
    /// @param groupId  The group to add the member to.
    /// @param addr     The member's wallet address.
    /// @param name     The member's display name within the group.
    function importMember(uint32 groupId, address addr, string calldata name) external onlyOwner {
        if (_groups[groupId].creator == address(0)) revert GroupDoesNotExist();
        if (isMemberOf[groupId][addr]) return; // idempotent

        uint32 idx = _groups[groupId].entryCount;
        _members[groupId][idx] = Member({addr: addr, name: name, score: 0, isScored: false});
        _memberIndex[groupId][addr] = idx;
        isMemberOf[groupId][addr] = true;
        _groups[groupId].entryCount++;

        emit MemberJoined(groupId, addr);
    }

    /// @notice Batch-import group members. Owner only.
    /// @dev    Idempotent: already-present addresses are silently skipped.
    ///         Recommended chunk size: 50 members per tx.
    /// @param groupId  The target group ID.
    /// @param addrs    Array of member addresses.
    /// @param names    Array of display names, same length as `addrs`.
    function batchImportMembers(
        uint32 groupId,
        address[] calldata addrs,
        string[] calldata names
    ) external onlyOwner {
        require(addrs.length == names.length, "length mismatch");
        if (_groups[groupId].creator == address(0)) revert GroupDoesNotExist();

        for (uint256 i = 0; i < addrs.length; i++) {
            if (isMemberOf[groupId][addrs[i]]) continue;
            uint32 idx = _groups[groupId].entryCount;
            _members[groupId][idx] = Member({addr: addrs[i], name: names[i], score: 0, isScored: false});
            _memberIndex[groupId][addrs[i]] = idx;
            isMemberOf[groupId][addrs[i]] = true;
            _groups[groupId].entryCount++;
            emit MemberJoined(groupId, addrs[i]);
        }
    }

    // ── ETH Funding ────────────────────────────────────────────────────────

    /// @notice Fund the contract to restore group prize pool balances.
    function fund() external payable {}

    /// @notice Accept ETH transfers directly.
    receive() external payable {}
}
