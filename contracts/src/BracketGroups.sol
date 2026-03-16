// SPDX-License-Identifier: MIT
pragma solidity ^0.8.30;

import {IMarchMadness} from "./IMarchMadness.sol";

/// @title BracketGroups — sub-groups for the main MarchMadness bracket contest
/// @notice Users create groups, optionally password-protected (sbytes12) and with an entry fee.
///         Members self-join by linking their main-contract bracket. Winners of each group
///         split the group's prize pool after the scoring window.
contract BracketGroups {
    // ── Errors ───────────────────────────────────────────────────────────
    error GroupDoesNotExist();
    error SlugCannotBeEmpty();
    error SlugTooLong();
    error SlugAlreadyTaken();
    error PasswordRequired();
    error GroupIsNotPasswordProtected();
    error WrongPassword();
    error IncorrectEntryFee(uint256 expected, uint256 actual);
    error AlreadyAMember();
    error NoBracketInMainContract();
    error CannotJoinAfterDeadline();
    error NotAMember();
    error CannotLeaveAfterDeadline();
    error RefundFailed();
    error IndexOutOfBounds();
    error AlreadyScored();
    error NoEntryFee();
    error ResultsNotPosted();
    error ScoringWindowStillOpen();
    error NoEntriesScored();
    error NotScored();
    error NotAWinner();
    error AlreadyCollected();
    error TransferFailed();
    error GroupNotFound();

    // ── References ──────────────────────────────────────────────────────
    IMarchMadness public immutable marchMadness;

    // ── Types ───────────────────────────────────────────────────────────
    struct Group {
        string slug;
        string displayName;
        address creator;
        uint32 entryCount;
        uint256 entryFee;
        bool hasPassword;
    }

    struct Member {
        address addr;
        string name;
        uint8 score;
        bool isScored;
    }

    struct GroupPayout {
        uint8 winningScore;
        uint32 numWinners;
        uint32 numScored;
    }

    // ── State ───────────────────────────────────────────────────────────
    uint32 public nextGroupId = 1;

    mapping(uint32 => Group) internal _groups;
    mapping(bytes32 => uint32) public slugToGroupId;

    // Password storage: shielded so nodes won't reveal it
    mapping(uint32 => sbytes12) internal _passwords;

    // Members
    mapping(uint32 => mapping(uint32 => Member)) internal _members;
    mapping(uint32 => mapping(address => bool)) public isMemberOf;
    mapping(uint32 => mapping(address => uint32)) internal _memberIndex;

    // Payouts
    mapping(uint32 => GroupPayout) public payouts;
    mapping(uint32 => mapping(address => bool)) public hasCollectedWinnings;

    // ── Constants ───────────────────────────────────────────────────────
    uint256 public constant MAX_SLUG_LENGTH = 32;
    uint256 public constant SCORING_DURATION = 7 days;

    // ── Events ──────────────────────────────────────────────────────────
    event GroupCreated(uint32 indexed groupId, string slug, string displayName, address creator, bool hasPassword);
    event MemberJoined(uint32 indexed groupId, address indexed addr);
    event MemberLeft(uint32 indexed groupId, address indexed addr);
    event EntryScored(uint32 indexed groupId, uint32 memberIndex, address indexed addr, uint8 score);
    event WinningsCollected(uint32 indexed groupId, address indexed addr, uint256 amount);

    // ── Constructor ─────────────────────────────────────────────────────
    constructor(address _marchMadness) {
        marchMadness = IMarchMadness(_marchMadness);
    }

    // ── Modifiers ───────────────────────────────────────────────────────
    modifier groupExists(uint32 groupId) {
        if (_groups[groupId].creator == address(0)) revert GroupDoesNotExist();
        _;
    }

    // ════════════════════════════════════════════════════════════════════
    //  GROUP LIFECYCLE
    // ════════════════════════════════════════════════════════════════════

    /// @notice Create a public group (no password).
    function createGroup(string calldata slug, string calldata displayName, uint256 entryFee)
        external
        returns (uint32 groupId)
    {
        groupId = _createGroup(slug, displayName, entryFee, false);
    }

    /// @notice Create a password-protected group. Password is stored shielded (sbytes12).
    ///         Frontend converts user's string password to bytes12 (e.g. keccak256 truncated) before sending.
    function createGroupWithPassword(
        string calldata slug,
        string calldata displayName,
        uint256 entryFee,
        sbytes12 password
    ) external returns (uint32 groupId) {
        groupId = _createGroup(slug, displayName, entryFee, true);
        _passwords[groupId] = password;
    }

    function _createGroup(string calldata slug, string calldata displayName, uint256 entryFee, bool hasPassword)
        internal
        returns (uint32 groupId)
    {
        bytes memory slugBytes = bytes(slug);
        if (slugBytes.length == 0) revert SlugCannotBeEmpty();
        if (slugBytes.length > MAX_SLUG_LENGTH) revert SlugTooLong();

        bytes32 slugHash = keccak256(slugBytes);
        if (slugToGroupId[slugHash] != 0) revert SlugAlreadyTaken();

        groupId = nextGroupId++;

        _groups[groupId] = Group({
            slug: slug,
            displayName: displayName,
            creator: msg.sender,
            entryCount: 0,
            entryFee: entryFee,
            hasPassword: hasPassword
        });

        slugToGroupId[slugHash] = groupId;

        emit GroupCreated(groupId, slug, displayName, msg.sender, hasPassword);
    }

    // ════════════════════════════════════════════════════════════════════
    //  JOIN / LEAVE
    // ════════════════════════════════════════════════════════════════════

    /// @notice Join a public group with a display name.
    function joinGroup(uint32 groupId, string calldata name) external payable groupExists(groupId) {
        if (_groups[groupId].hasPassword) revert PasswordRequired();
        _joinGroup(groupId, name);
    }

    /// @notice Join a password-protected group with a display name.
    function joinGroupWithPassword(uint32 groupId, sbytes12 password, string calldata name)
        external
        payable
        groupExists(groupId)
    {
        if (!_groups[groupId].hasPassword) revert GroupIsNotPasswordProtected();
        if (password != _passwords[groupId]) revert WrongPassword();
        _joinGroup(groupId, name);
    }

    function _joinGroup(uint32 groupId, string memory name) internal {
        Group storage g = _groups[groupId];
        if (msg.value != g.entryFee) revert IncorrectEntryFee(g.entryFee, msg.value);
        if (isMemberOf[groupId][msg.sender]) revert AlreadyAMember();
        if (!marchMadness.hasEntry(msg.sender)) revert NoBracketInMainContract();
        if (block.timestamp >= marchMadness.submissionDeadline()) revert CannotJoinAfterDeadline();

        uint32 idx = g.entryCount;
        _members[groupId][idx] = Member({addr: msg.sender, name: name, score: 0, isScored: false});
        isMemberOf[groupId][msg.sender] = true;
        _memberIndex[groupId][msg.sender] = idx;
        g.entryCount++;

        emit MemberJoined(groupId, msg.sender);
    }

    /// @notice Leave a group. Only before the submission deadline. Refunds entry fee.
    function leaveGroup(uint32 groupId) external groupExists(groupId) {
        if (!isMemberOf[groupId][msg.sender]) revert NotAMember();
        if (block.timestamp >= marchMadness.submissionDeadline()) revert CannotLeaveAfterDeadline();

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
            if (!success) revert RefundFailed();
        }
    }

    /// @notice Update your display name in a group.
    function editEntryName(uint32 groupId, string calldata name) external groupExists(groupId) {
        if (!isMemberOf[groupId][msg.sender]) revert NotAMember();
        uint32 idx = _memberIndex[groupId][msg.sender];
        _members[groupId][idx].name = name;
    }

    // ════════════════════════════════════════════════════════════════════
    //  SCORING + PAYOUTS
    // ════════════════════════════════════════════════════════════════════

    /// @notice Score a group member's bracket. Anyone can call.
    ///         Delegates scoring to MarchMadness contract if not already scored there.
    function scoreEntry(uint32 groupId, uint32 memberIndex) external groupExists(groupId) {
        Group storage g = _groups[groupId];
        if (memberIndex >= g.entryCount) revert IndexOutOfBounds();

        uint256 resultsPostedAt = marchMadness.resultsPostedAt();
        uint256 scoringDuration = marchMadness.SCORING_DURATION();
        if (resultsPostedAt == 0) revert ResultsNotPosted();
        if (block.timestamp >= resultsPostedAt + scoringDuration) revert IMarchMadness.ScoringWindowClosed();

        Member storage member = _members[groupId][memberIndex];
        if (member.isScored) revert AlreadyScored();

        // Score on main contract if not already scored there
        if (!marchMadness.isScored(member.addr)) {
            marchMadness.scoreBracket(member.addr);
        }

        uint8 score = marchMadness.scores(member.addr);
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
    function collectWinnings(uint32 groupId) external groupExists(groupId) {
        Group storage g = _groups[groupId];
        if (g.entryFee == 0) revert NoEntryFee();
        if (!isMemberOf[groupId][msg.sender]) revert NotAMember();

        uint256 resultsPostedAt = marchMadness.resultsPostedAt();
        uint256 scoringDuration = marchMadness.SCORING_DURATION();
        if (resultsPostedAt == 0) revert ResultsNotPosted();
        if (block.timestamp < resultsPostedAt + scoringDuration) revert ScoringWindowStillOpen();

        GroupPayout storage payout = payouts[groupId];
        if (payout.numWinners == 0) revert NoEntriesScored();

        uint32 idx = _memberIndex[groupId][msg.sender];
        Member storage member = _members[groupId][idx];
        if (!member.isScored) revert NotScored();
        if (member.score != payout.winningScore) revert NotAWinner();
        if (hasCollectedWinnings[groupId][msg.sender]) revert AlreadyCollected();

        hasCollectedWinnings[groupId][msg.sender] = true;
        uint256 amount = (uint256(g.entryCount) * g.entryFee) / payout.numWinners;

        (bool success,) = msg.sender.call{value: amount}("");
        emit WinningsCollected(groupId, msg.sender, amount);
        if (!success) revert TransferFailed();
    }

    // ════════════════════════════════════════════════════════════════════
    //  VIEW FUNCTIONS
    // ════════════════════════════════════════════════════════════════════

    function getGroupBySlug(string calldata slug) external view returns (uint32, Group memory) {
        bytes32 slugHash = keccak256(bytes(slug));
        uint32 groupId = slugToGroupId[slugHash];
        if (groupId == 0) revert GroupNotFound();
        return (groupId, _groups[groupId]);
    }

    function getGroup(uint32 groupId) external view groupExists(groupId) returns (Group memory) {
        return _groups[groupId];
    }

    function getMembers(uint32 groupId) external view groupExists(groupId) returns (Member[] memory) {
        uint32 count = _groups[groupId].entryCount;
        Member[] memory members = new Member[](count);
        for (uint32 i = 0; i < count; i++) {
            members[i] = _members[groupId][i];
        }
        return members;
    }

    function getMember(uint32 groupId, uint32 index) external view groupExists(groupId) returns (Member memory) {
        if (index >= _groups[groupId].entryCount) revert IndexOutOfBounds();
        return _members[groupId][index];
    }

    function getMemberScore(uint32 groupId, uint32 index) external view groupExists(groupId) returns (uint8) {
        if (index >= _groups[groupId].entryCount) revert IndexOutOfBounds();
        return _members[groupId][index].score;
    }

    function getIsMember(uint32 groupId, address addr) external view returns (bool) {
        return isMemberOf[groupId][addr];
    }
}
