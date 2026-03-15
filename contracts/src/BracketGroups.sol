// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

import {ByteBracket} from "./ByteBracket.sol";
import {MarchMadness} from "./MarchMadness.sol";

/// @title BracketGroups — side groups that compose with the main MarchMadness contract
/// @notice Two group types:
///   - Manual: admin enters external brackets (name + bytes8). No money, scores computed on-the-fly.
///   - Linked: users self-join with their main-contract bracket. Optional entry fee + payout.
contract BracketGroups {
    // ── References ──────────────────────────────────────────────────────
    MarchMadness public immutable mainContract;

    // ── Types ───────────────────────────────────────────────────────────
    enum GroupType {
        Manual,
        Linked
    }

    struct Group {
        GroupType groupType;
        string slug;
        string displayName;
        address admin;
        uint32 entryCount;
        uint256 entryFee; // only meaningful for Linked groups
        string prizeDescription; // admin-set off-chain prize info (e.g. "$500 Amazon gift card")
        bool exists;
    }

    struct ManualEntry {
        string name;
        bytes8 bracket;
    }

    struct LinkedMember {
        address addr;
        string name;
        uint8 score;
        bool isScored;
    }

    struct LinkedGroupPayout {
        uint8 winningScore;
        uint256 numWinners;
        uint256 numScored;
    }

    // ── State ───────────────────────────────────────────────────────────
    uint256 public nextGroupId = 1;

    mapping(uint256 => Group) internal _groups;
    mapping(bytes32 => uint256) public slugToGroupId;

    // Manual group storage
    mapping(uint256 => mapping(uint32 => ManualEntry)) internal _manualEntries;

    // Linked group storage
    mapping(uint256 => mapping(uint32 => LinkedMember)) internal _linkedMembers;
    mapping(uint256 => mapping(address => bool)) public isMemberOf;
    mapping(uint256 => mapping(address => uint32)) internal _memberIndex;
    mapping(uint256 => LinkedGroupPayout) public payouts;
    mapping(uint256 => mapping(address => bool)) public hasCollectedGroupWinnings;

    // ── Constants ───────────────────────────────────────────────────────
    uint256 public constant MAX_SLUG_LENGTH = 32;
    uint256 public constant SCORING_DURATION = 7 days;

    // ── Events ──────────────────────────────────────────────────────────
    event GroupCreated(uint256 indexed groupId, GroupType groupType, string slug, string displayName, address admin);
    event ManualEntryAdded(uint256 indexed groupId, uint32 entryIndex, string name);
    event ManualEntryRemoved(uint256 indexed groupId, uint32 entryIndex);
    event ManualBracketUpdated(uint256 indexed groupId, uint32 entryIndex);
    event MemberJoined(uint256 indexed groupId, address indexed addr);
    event MemberLeft(uint256 indexed groupId, address indexed addr);
    event GroupEntryScored(uint256 indexed groupId, uint32 memberIndex, address indexed addr, uint8 score);
    event GroupWinningsCollected(uint256 indexed groupId, address indexed addr, uint256 amount);

    // ── Constructor ─────────────────────────────────────────────────────
    constructor(address _mainContract) {
        mainContract = MarchMadness(_mainContract);
    }

    // ── Modifiers ───────────────────────────────────────────────────────
    modifier onlyGroupAdmin(uint256 groupId) {
        require(_groups[groupId].exists, "Group does not exist");
        require(msg.sender == _groups[groupId].admin, "Not group admin");
        _;
    }

    modifier groupExists(uint256 groupId) {
        require(_groups[groupId].exists, "Group does not exist");
        _;
    }

    // ════════════════════════════════════════════════════════════════════
    //  GROUP LIFECYCLE
    // ════════════════════════════════════════════════════════════════════

    /// @notice Create a manual group (admin-managed, no money).
    function createManualGroup(string calldata slug, string calldata displayName) external returns (uint256 groupId) {
        groupId = _createGroup(GroupType.Manual, slug, displayName, 0);
    }

    /// @notice Create a linked group (self-join, optional entry fee).
    function createLinkedGroup(string calldata slug, string calldata displayName, uint256 entryFee)
        external
        returns (uint256 groupId)
    {
        groupId = _createGroup(GroupType.Linked, slug, displayName, entryFee);
    }

    function _createGroup(GroupType groupType, string calldata slug, string calldata displayName, uint256 entryFee)
        internal
        returns (uint256 groupId)
    {
        bytes memory slugBytes = bytes(slug);
        require(slugBytes.length > 0, "Slug cannot be empty");
        require(slugBytes.length <= MAX_SLUG_LENGTH, "Slug too long");

        bytes32 slugHash = keccak256(slugBytes);
        require(slugToGroupId[slugHash] == 0, "Slug already taken");

        groupId = nextGroupId++;

        _groups[groupId] = Group({
            groupType: groupType,
            slug: slug,
            displayName: displayName,
            admin: msg.sender,
            entryCount: 0,
            entryFee: entryFee,
            prizeDescription: "",
            exists: true
        });

        slugToGroupId[slugHash] = groupId;

        emit GroupCreated(groupId, groupType, slug, displayName, msg.sender);
    }

    // ════════════════════════════════════════════════════════════════════
    //  MANUAL GROUP — admin enters external brackets (name + bytes8)
    // ════════════════════════════════════════════════════════════════════

    /// @notice Add an entry to a manual group. Admin only.
    function addManualEntry(uint256 groupId, bytes8 bracket, string calldata name) external onlyGroupAdmin(groupId) {
        require(_groups[groupId].groupType == GroupType.Manual, "Not a manual group");
        require(bracket[0] & 0x80 != 0, "Invalid sentinel byte");

        uint32 idx = _groups[groupId].entryCount;
        _manualEntries[groupId][idx] = ManualEntry({name: name, bracket: bracket});
        _groups[groupId].entryCount++;

        emit ManualEntryAdded(groupId, idx, name);
    }

    /// @notice Remove an entry from a manual group (swap-and-pop). Admin only.
    function removeManualEntry(uint256 groupId, uint32 entryIndex) external onlyGroupAdmin(groupId) {
        require(_groups[groupId].groupType == GroupType.Manual, "Not a manual group");
        uint32 count = _groups[groupId].entryCount;
        require(entryIndex < count, "Index out of bounds");

        uint32 lastIndex = count - 1;
        if (entryIndex != lastIndex) {
            _manualEntries[groupId][entryIndex] = _manualEntries[groupId][lastIndex];
        }
        delete _manualEntries[groupId][lastIndex];
        _groups[groupId].entryCount--;

        emit ManualEntryRemoved(groupId, entryIndex);
    }

    /// @notice Update the bracket for a manual entry. Admin only. Blocked after results.
    function updateManualBracket(uint256 groupId, uint32 entryIndex, bytes8 bracket) external onlyGroupAdmin(groupId) {
        require(_groups[groupId].groupType == GroupType.Manual, "Not a manual group");
        require(entryIndex < _groups[groupId].entryCount, "Index out of bounds");
        require(mainContract.results() == bytes8(0), "Results already posted");
        require(bracket[0] & 0x80 != 0, "Invalid sentinel byte");

        _manualEntries[groupId][entryIndex].bracket = bracket;

        emit ManualBracketUpdated(groupId, entryIndex);
    }

    /// @notice Update the display name for a manual entry. Admin only.
    function updateEntryName(uint256 groupId, uint32 entryIndex, string calldata name)
        external
        onlyGroupAdmin(groupId)
    {
        require(_groups[groupId].groupType == GroupType.Manual, "Not a manual group");
        require(entryIndex < _groups[groupId].entryCount, "Index out of bounds");

        _manualEntries[groupId][entryIndex].name = name;
    }

    /// @notice Set or update the prize description for a group. Admin only.
    ///         Use for off-chain bookkeeping (e.g. "$500 Amazon gift card").
    function setPrizeDescription(uint256 groupId, string calldata description) external onlyGroupAdmin(groupId) {
        _groups[groupId].prizeDescription = description;
    }

    // ════════════════════════════════════════════════════════════════════
    //  LINKED GROUP — self-join with main-contract bracket
    // ════════════════════════════════════════════════════════════════════

    /// @notice Join a linked group. Must have a bracket in the main contract.
    function joinGroup(uint256 groupId) external payable groupExists(groupId) {
        _joinGroup(groupId, "");
    }

    /// @notice Join a linked group with a display name.
    function joinGroupWithName(uint256 groupId, string calldata name) external payable groupExists(groupId) {
        _joinGroup(groupId, name);
    }

    function _joinGroup(uint256 groupId, string memory name) internal {
        Group storage g = _groups[groupId];
        require(g.groupType == GroupType.Linked, "Not a linked group");
        require(msg.value == g.entryFee, "Incorrect entry fee");
        require(!isMemberOf[groupId][msg.sender], "Already a member");
        require(mainContract.hasEntry(msg.sender), "No bracket in main contract");
        // Only allow joining before results are posted
        require(mainContract.results() == bytes8(0), "Results already posted");

        uint32 idx = g.entryCount;
        _linkedMembers[groupId][idx] = LinkedMember({addr: msg.sender, name: name, score: 0, isScored: false});
        isMemberOf[groupId][msg.sender] = true;
        _memberIndex[groupId][msg.sender] = idx;
        g.entryCount++;

        emit MemberJoined(groupId, msg.sender);
    }

    /// @notice Leave a linked group. Only before results are posted. Refunds entry fee.
    function leaveGroup(uint256 groupId) external groupExists(groupId) {
        Group storage g = _groups[groupId];
        require(g.groupType == GroupType.Linked, "Not a linked group");
        require(isMemberOf[groupId][msg.sender], "Not a member");
        require(mainContract.results() == bytes8(0), "Cannot leave after results posted");

        uint32 idx = _memberIndex[groupId][msg.sender];
        uint32 lastIndex = g.entryCount - 1;

        // Swap-and-pop
        if (idx != lastIndex) {
            LinkedMember storage last = _linkedMembers[groupId][lastIndex];
            _linkedMembers[groupId][idx] = last;
            _memberIndex[groupId][last.addr] = idx;
        }
        delete _linkedMembers[groupId][lastIndex];
        delete isMemberOf[groupId][msg.sender];
        delete _memberIndex[groupId][msg.sender];
        g.entryCount--;

        emit MemberLeft(groupId, msg.sender);

        // Refund entry fee
        if (g.entryFee > 0) {
            (bool success,) = msg.sender.call{value: g.entryFee}("");
            require(success, "Refund failed");
        }
    }

    /// @notice Update your display name in a linked group.
    function setGroupEntryName(uint256 groupId, string calldata name) external groupExists(groupId) {
        require(isMemberOf[groupId][msg.sender], "Not a member");
        uint32 idx = _memberIndex[groupId][msg.sender];
        _linkedMembers[groupId][idx].name = name;
    }

    // ════════════════════════════════════════════════════════════════════
    //  LINKED GROUP — SCORING + PAYOUTS
    // ════════════════════════════════════════════════════════════════════

    /// @notice Score a linked group member's bracket. Anyone can call.
    function scoreGroupEntry(uint256 groupId, uint32 memberIndex) external groupExists(groupId) {
        Group storage g = _groups[groupId];
        require(g.groupType == GroupType.Linked, "Not a linked group");
        require(memberIndex < g.entryCount, "Index out of bounds");

        LinkedMember storage member = _linkedMembers[groupId][memberIndex];
        require(!member.isScored, "Already scored");

        // Read results and scoring mask from main contract
        bytes8 results = mainContract.results();
        require(results != bytes8(0), "Results not posted");
        uint64 scoringMask = mainContract.scoringMask();
        uint256 resultsPostedAt = mainContract.resultsPostedAt();
        require(block.timestamp < resultsPostedAt + SCORING_DURATION, "Scoring window closed");

        // Read bracket from main contract (works post-deadline)
        bytes8 bracket = mainContract.getBracket(member.addr);
        require(bracket[0] & 0x80 != 0, "Member has no bracket");

        uint8 score = ByteBracket.getBracketScore(bracket, results, scoringMask);
        member.score = score;
        member.isScored = true;

        // Track payout state
        LinkedGroupPayout storage payout = payouts[groupId];
        payout.numScored++;
        if (score > payout.winningScore) {
            payout.winningScore = score;
            payout.numWinners = 1;
        } else if (score == payout.winningScore) {
            payout.numWinners++;
        }

        emit GroupEntryScored(groupId, memberIndex, member.addr, score);
    }

    /// @notice Collect winnings from a linked group. Winners split the prize pool equally.
    function collectGroupWinnings(uint256 groupId) external groupExists(groupId) {
        Group storage g = _groups[groupId];
        require(g.groupType == GroupType.Linked, "Not a linked group");
        require(g.entryFee > 0, "No entry fee");
        require(isMemberOf[groupId][msg.sender], "Not a member");

        uint256 resultsPostedAt = mainContract.resultsPostedAt();
        require(resultsPostedAt > 0, "Results not posted");
        require(block.timestamp >= resultsPostedAt + SCORING_DURATION, "Scoring window still open");

        LinkedGroupPayout storage payout = payouts[groupId];
        require(payout.numWinners > 0, "No entries scored");

        uint32 idx = _memberIndex[groupId][msg.sender];
        LinkedMember storage member = _linkedMembers[groupId][idx];
        require(member.isScored, "Not scored");
        require(member.score == payout.winningScore, "Not a winner");
        require(!hasCollectedGroupWinnings[groupId][msg.sender], "Already collected");

        hasCollectedGroupWinnings[groupId][msg.sender] = true;
        uint256 amount = (uint256(g.entryCount) * g.entryFee) / payout.numWinners;

        emit GroupWinningsCollected(groupId, msg.sender, amount);

        (bool success,) = msg.sender.call{value: amount}("");
        require(success, "Transfer failed");
    }

    // ════════════════════════════════════════════════════════════════════
    //  VIEW FUNCTIONS
    // ════════════════════════════════════════════════════════════════════

    function getGroupBySlug(string calldata slug) external view returns (uint256) {
        bytes32 slugHash = keccak256(bytes(slug));
        uint256 groupId = slugToGroupId[slugHash];
        require(groupId != 0, "Group not found");
        return groupId;
    }

    function getGroup(uint256 groupId) external view groupExists(groupId) returns (Group memory) {
        return _groups[groupId];
    }

    function getManualEntry(uint256 groupId, uint32 index)
        external
        view
        groupExists(groupId)
        returns (ManualEntry memory)
    {
        require(_groups[groupId].groupType == GroupType.Manual, "Not a manual group");
        require(index < _groups[groupId].entryCount, "Index out of bounds");
        return _manualEntries[groupId][index];
    }

    function getManualEntries(uint256 groupId) external view groupExists(groupId) returns (ManualEntry[] memory) {
        require(_groups[groupId].groupType == GroupType.Manual, "Not a manual group");
        uint32 count = _groups[groupId].entryCount;
        ManualEntry[] memory entries = new ManualEntry[](count);
        for (uint32 i = 0; i < count; i++) {
            entries[i] = _manualEntries[groupId][i];
        }
        return entries;
    }

    /// @notice Compute a manual entry's score on-the-fly against main contract results.
    function getManualEntryScore(uint256 groupId, uint32 index) external view groupExists(groupId) returns (uint8) {
        require(_groups[groupId].groupType == GroupType.Manual, "Not a manual group");
        require(index < _groups[groupId].entryCount, "Index out of bounds");

        bytes8 results = mainContract.results();
        require(results != bytes8(0), "Results not posted");

        return ByteBracket.getBracketScore(_manualEntries[groupId][index].bracket, results, mainContract.scoringMask());
    }

    /// @notice Batch-compute scores for all manual entries.
    function getManualGroupScores(uint256 groupId) external view groupExists(groupId) returns (uint8[] memory) {
        require(_groups[groupId].groupType == GroupType.Manual, "Not a manual group");

        bytes8 results = mainContract.results();
        require(results != bytes8(0), "Results not posted");
        uint64 mask = mainContract.scoringMask();

        uint32 count = _groups[groupId].entryCount;
        uint8[] memory scores = new uint8[](count);
        for (uint32 i = 0; i < count; i++) {
            scores[i] = ByteBracket.getBracketScore(_manualEntries[groupId][i].bracket, results, mask);
        }
        return scores;
    }

    function getLinkedMembers(uint256 groupId) external view groupExists(groupId) returns (LinkedMember[] memory) {
        require(_groups[groupId].groupType == GroupType.Linked, "Not a linked group");
        uint32 count = _groups[groupId].entryCount;
        LinkedMember[] memory members = new LinkedMember[](count);
        for (uint32 i = 0; i < count; i++) {
            members[i] = _linkedMembers[groupId][i];
        }
        return members;
    }

    function getLinkedMemberScore(uint256 groupId, uint32 index) external view groupExists(groupId) returns (uint8) {
        require(_groups[groupId].groupType == GroupType.Linked, "Not a linked group");
        require(index < _groups[groupId].entryCount, "Index out of bounds");
        return _linkedMembers[groupId][index].score;
    }

    function getIsMember(uint256 groupId, address addr) external view returns (bool) {
        return isMemberOf[groupId][addr];
    }
}
