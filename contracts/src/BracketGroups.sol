// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

import {ByteBracket} from "./ByteBracket.sol";
import {MarchMadness} from "./MarchMadness.sol";

/// @title BracketGroups — sub-groups for the main MarchMadness bracket contest
/// @notice Users create groups, optionally password-protected (sbytes32) and with an entry fee.
///         Members self-join by linking their main-contract bracket. Winners of each group
///         split the group's prize pool after the scoring window.
contract BracketGroups {
    // ── References ──────────────────────────────────────────────────────
    MarchMadness public immutable mainContract;

    // ── Types ───────────────────────────────────────────────────────────
    struct Group {
        string slug;
        string displayName;
        address admin;
        uint32 entryCount;
        uint256 entryFee;
        bool hasPassword; // true if password-protected
        bool exists;
    }

    struct Member {
        address addr;
        string name;
        uint8 score;
        bool isScored;
    }

    struct GroupPayout {
        uint8 winningScore;
        uint256 numWinners;
        uint256 numScored;
    }

    // ── State ───────────────────────────────────────────────────────────
    uint256 public nextGroupId = 1;

    mapping(uint256 => Group) internal _groups;
    mapping(bytes32 => uint256) public slugToGroupId;

    // Password storage: shielded so nodes won't reveal it
    mapping(uint256 => sbytes32) internal _passwords;

    // Members
    mapping(uint256 => mapping(uint32 => Member)) internal _members;
    mapping(uint256 => mapping(address => bool)) public isMemberOf;
    mapping(uint256 => mapping(address => uint32)) internal _memberIndex;

    // Payouts
    mapping(uint256 => GroupPayout) public payouts;
    mapping(uint256 => mapping(address => bool)) public hasCollectedWinnings;

    // ── Constants ───────────────────────────────────────────────────────
    uint256 public constant MAX_SLUG_LENGTH = 32;
    uint256 public constant SCORING_DURATION = 7 days;

    // ── Events ──────────────────────────────────────────────────────────
    event GroupCreated(uint256 indexed groupId, string slug, string displayName, address admin, bool hasPassword);
    event MemberJoined(uint256 indexed groupId, address indexed addr);
    event MemberLeft(uint256 indexed groupId, address indexed addr);
    event EntryScored(uint256 indexed groupId, uint32 memberIndex, address indexed addr, uint8 score);
    event WinningsCollected(uint256 indexed groupId, address indexed addr, uint256 amount);

    // ── Constructor ─────────────────────────────────────────────────────
    constructor(address _mainContract) {
        mainContract = MarchMadness(_mainContract);
    }

    // ── Modifiers ───────────────────────────────────────────────────────
    modifier groupExists(uint256 groupId) {
        require(_groups[groupId].exists, "Group does not exist");
        _;
    }

    // ════════════════════════════════════════════════════════════════════
    //  GROUP LIFECYCLE
    // ════════════════════════════════════════════════════════════════════

    /// @notice Create a public group (no password).
    function createGroup(string calldata slug, string calldata displayName, uint256 entryFee)
        external
        returns (uint256 groupId)
    {
        groupId = _createGroup(slug, displayName, entryFee, false);
    }

    /// @notice Create a password-protected group. Password is stored shielded (sbytes32).
    ///         Frontend converts user's string password to bytes32 (e.g. keccak256) before sending.
    function createGroupWithPassword(
        string calldata slug,
        string calldata displayName,
        uint256 entryFee,
        sbytes32 password
    ) external returns (uint256 groupId) {
        groupId = _createGroup(slug, displayName, entryFee, true);
        _passwords[groupId] = password;
    }

    function _createGroup(string calldata slug, string calldata displayName, uint256 entryFee, bool hasPassword)
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
            slug: slug,
            displayName: displayName,
            admin: msg.sender,
            entryCount: 0,
            entryFee: entryFee,
            hasPassword: hasPassword,
            exists: true
        });

        slugToGroupId[slugHash] = groupId;

        emit GroupCreated(groupId, slug, displayName, msg.sender, hasPassword);
    }

    // ════════════════════════════════════════════════════════════════════
    //  JOIN / LEAVE
    // ════════════════════════════════════════════════════════════════════

    /// @notice Join a public group. Must have a bracket in the main contract.
    function joinGroup(uint256 groupId) external payable groupExists(groupId) {
        require(!_groups[groupId].hasPassword, "Password required");
        _joinGroup(groupId, "");
    }

    /// @notice Join a public group with a display name.
    function joinGroupWithName(uint256 groupId, string calldata name) external payable groupExists(groupId) {
        require(!_groups[groupId].hasPassword, "Password required");
        _joinGroup(groupId, name);
    }

    /// @notice Join a password-protected group. Caller provides the password as sbytes32.
    function joinGroupWithPassword(uint256 groupId, sbytes32 password) external payable groupExists(groupId) {
        require(_groups[groupId].hasPassword, "Group is not password-protected");
        require(bytes32(password) == bytes32(_passwords[groupId]), "Wrong password");
        _joinGroup(groupId, "");
    }

    /// @notice Join a password-protected group with a display name.
    function joinGroupWithPasswordAndName(uint256 groupId, sbytes32 password, string calldata name)
        external
        payable
        groupExists(groupId)
    {
        require(_groups[groupId].hasPassword, "Group is not password-protected");
        require(bytes32(password) == bytes32(_passwords[groupId]), "Wrong password");
        _joinGroup(groupId, name);
    }

    function _joinGroup(uint256 groupId, string memory name) internal {
        Group storage g = _groups[groupId];
        require(msg.value == g.entryFee, "Incorrect entry fee");
        require(!isMemberOf[groupId][msg.sender], "Already a member");
        require(mainContract.hasEntry(msg.sender), "No bracket in main contract");
        require(mainContract.results() == bytes8(0), "Results already posted");

        uint32 idx = g.entryCount;
        _members[groupId][idx] = Member({addr: msg.sender, name: name, score: 0, isScored: false});
        isMemberOf[groupId][msg.sender] = true;
        _memberIndex[groupId][msg.sender] = idx;
        g.entryCount++;

        emit MemberJoined(groupId, msg.sender);
    }

    /// @notice Leave a group. Only before results are posted. Refunds entry fee.
    function leaveGroup(uint256 groupId) external groupExists(groupId) {
        require(isMemberOf[groupId][msg.sender], "Not a member");
        require(mainContract.results() == bytes8(0), "Cannot leave after results posted");

        Group storage g = _groups[groupId];
        uint32 idx = _memberIndex[groupId][msg.sender];
        uint32 lastIndex = g.entryCount - 1;

        // Swap-and-pop
        if (idx != lastIndex) {
            Member storage last = _members[groupId][lastIndex];
            _members[groupId][idx] = last;
            _memberIndex[groupId][last.addr] = idx;
        }
        delete _members[groupId][lastIndex];
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

    /// @notice Update your display name in a group.
    function setEntryName(uint256 groupId, string calldata name) external groupExists(groupId) {
        require(isMemberOf[groupId][msg.sender], "Not a member");
        uint32 idx = _memberIndex[groupId][msg.sender];
        _members[groupId][idx].name = name;
    }

    // ════════════════════════════════════════════════════════════════════
    //  SCORING + PAYOUTS
    // ════════════════════════════════════════════════════════════════════

    /// @notice Score a group member's bracket. Anyone can call.
    function scoreEntry(uint256 groupId, uint32 memberIndex) external groupExists(groupId) {
        Group storage g = _groups[groupId];
        require(memberIndex < g.entryCount, "Index out of bounds");

        Member storage member = _members[groupId][memberIndex];
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
        GroupPayout storage payout = payouts[groupId];
        payout.numScored++;
        if (score > payout.winningScore) {
            payout.winningScore = score;
            payout.numWinners = 1;
        } else if (score == payout.winningScore) {
            payout.numWinners++;
        }

        emit EntryScored(groupId, memberIndex, member.addr, score);
    }

    /// @notice Collect winnings. Winners split the group's prize pool equally.
    function collectWinnings(uint256 groupId) external groupExists(groupId) {
        Group storage g = _groups[groupId];
        require(g.entryFee > 0, "No entry fee");
        require(isMemberOf[groupId][msg.sender], "Not a member");

        uint256 resultsPostedAt = mainContract.resultsPostedAt();
        require(resultsPostedAt > 0, "Results not posted");
        require(block.timestamp >= resultsPostedAt + SCORING_DURATION, "Scoring window still open");

        GroupPayout storage payout = payouts[groupId];
        require(payout.numWinners > 0, "No entries scored");

        uint32 idx = _memberIndex[groupId][msg.sender];
        Member storage member = _members[groupId][idx];
        require(member.isScored, "Not scored");
        require(member.score == payout.winningScore, "Not a winner");
        require(!hasCollectedWinnings[groupId][msg.sender], "Already collected");

        hasCollectedWinnings[groupId][msg.sender] = true;
        uint256 amount = (uint256(g.entryCount) * g.entryFee) / payout.numWinners;

        emit WinningsCollected(groupId, msg.sender, amount);

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

    function getMembers(uint256 groupId) external view groupExists(groupId) returns (Member[] memory) {
        uint32 count = _groups[groupId].entryCount;
        Member[] memory members = new Member[](count);
        for (uint32 i = 0; i < count; i++) {
            members[i] = _members[groupId][i];
        }
        return members;
    }

    function getMember(uint256 groupId, uint32 index) external view groupExists(groupId) returns (Member memory) {
        require(index < _groups[groupId].entryCount, "Index out of bounds");
        return _members[groupId][index];
    }

    function getMemberScore(uint256 groupId, uint32 index) external view groupExists(groupId) returns (uint8) {
        require(index < _groups[groupId].entryCount, "Index out of bounds");
        return _members[groupId][index].score;
    }

    function getIsMember(uint256 groupId, address addr) external view returns (bool) {
        return isMemberOf[groupId][addr];
    }
}
