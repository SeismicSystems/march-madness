// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

import {Test} from "forge-std/Test.sol";
import {MarchMadness} from "../src/MarchMadness.sol";
import {BracketGroups} from "../src/BracketGroups.sol";

/// @title BracketGroups tests — linked sub-groups with optional password and entry fee
contract BracketGroupsTest is Test {
    MarchMadness mm;
    BracketGroups bg;

    address admin = address(0xAD);
    address alice = address(0xA11CE);
    address bob = address(0xB0B);
    address charlie = address(0xC4A2);

    uint256 constant ENTRY_FEE = 1 ether;
    uint256 constant DEADLINE = 1000;

    bytes8 constant PERFECT = bytes8(0xFFFFFFFFFFFFFFFF);
    bytes8 constant BAD = bytes8(0x8000000000000000);
    bytes8 constant RESULTS = bytes8(0xFFFFFFFFFFFFFFFF);

    // Password: keccak256("secret") stored as sbytes32
    bytes32 constant PASSWORD = keccak256("secret");

    function setUp() public {
        vm.warp(100);
        mm = new MarchMadness(ENTRY_FEE, DEADLINE);
        bg = new BracketGroups(address(mm));

        vm.deal(admin, 100 ether);
        vm.deal(alice, 100 ether);
        vm.deal(bob, 100 ether);
        vm.deal(charlie, 100 ether);
    }

    // ════════════════════════════════════════════════════════════════════
    //  GROUP CREATION
    // ════════════════════════════════════════════════════════════════════

    function test_createGroup() public {
        vm.prank(admin);
        uint256 groupId = bg.createGroup("side-bet", "Side Bet", 0.1 ether);

        assertEq(groupId, 1);
        assertEq(bg.getGroupBySlug("side-bet"), 1);

        BracketGroups.Group memory g = bg.getGroup(groupId);
        assertEq(g.slug, "side-bet");
        assertEq(g.displayName, "Side Bet");
        assertEq(g.admin, admin);
        assertEq(g.entryCount, 0);
        assertEq(g.entryFee, 0.1 ether);
        assertFalse(g.hasPassword);
    }

    function test_createGroupWithPassword() public {
        vm.prank(admin);
        uint256 groupId = bg.createGroupWithPassword("private", "Private Group", 0, sbytes32(PASSWORD));

        BracketGroups.Group memory g = bg.getGroup(groupId);
        assertTrue(g.hasPassword);
        assertEq(g.slug, "private");
    }

    function test_duplicateSlugReverts() public {
        vm.prank(admin);
        bg.createGroup("bet", "Bet", 0);

        vm.prank(admin);
        vm.expectRevert("Slug already taken");
        bg.createGroup("bet", "Other", 0);
    }

    function test_emptySlugReverts() public {
        vm.expectRevert("Slug cannot be empty");
        bg.createGroup("", "Bet", 0);
    }

    function test_longSlugReverts() public {
        vm.expectRevert("Slug too long");
        bg.createGroup("this-slug-is-way-too-long-and-exceeds-the-32-byte-limit", "Bet", 0);
    }

    function test_slugLookupNonexistent() public {
        vm.expectRevert("Group not found");
        bg.getGroupBySlug("nope");
    }

    function test_nonexistentGroupReverts() public {
        vm.expectRevert("Group does not exist");
        bg.getGroup(999);
    }

    function test_multipleGroups() public {
        vm.prank(admin);
        uint256 g1 = bg.createGroup("g1", "G1", 0);
        vm.prank(alice);
        uint256 g2 = bg.createGroup("g2", "G2", 0.5 ether);

        assertEq(g1, 1);
        assertEq(g2, 2);
        assertEq(bg.getGroup(g2).admin, alice);
    }

    // ════════════════════════════════════════════════════════════════════
    //  JOIN / LEAVE — PUBLIC GROUPS
    // ════════════════════════════════════════════════════════════════════

    function test_joinGroup() public {
        vm.prank(admin);
        uint256 groupId = bg.createGroup("bet", "Bet", 0.1 ether);

        vm.prank(alice);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(PERFECT));

        vm.prank(alice);
        bg.joinGroup{value: 0.1 ether}(groupId);

        assertTrue(bg.isMemberOf(groupId, alice));
        assertTrue(bg.getIsMember(groupId, alice));
        assertEq(bg.getGroup(groupId).entryCount, 1);
    }

    function test_joinGroupWithName() public {
        vm.prank(admin);
        uint256 groupId = bg.createGroup("bet", "Bet", 0);

        vm.prank(alice);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(PERFECT));

        vm.prank(alice);
        bg.joinGroupWithName{value: 0}(groupId, "Alice");

        BracketGroups.Member[] memory members = bg.getMembers(groupId);
        assertEq(members.length, 1);
        assertEq(members[0].name, "Alice");
        assertEq(members[0].addr, alice);
    }

    function test_joinGroup_rejectDuplicate() public {
        vm.prank(admin);
        uint256 groupId = bg.createGroup("bet", "Bet", 0);

        vm.prank(alice);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(PERFECT));
        vm.prank(alice);
        bg.joinGroup(groupId);

        vm.prank(alice);
        vm.expectRevert("Already a member");
        bg.joinGroup(groupId);
    }

    function test_joinGroup_rejectWithoutMainBracket() public {
        vm.prank(admin);
        uint256 groupId = bg.createGroup("bet", "Bet", 0);

        vm.prank(alice);
        vm.expectRevert("No bracket in main contract");
        bg.joinGroup(groupId);
    }

    function test_joinGroup_rejectWrongFee() public {
        vm.prank(admin);
        uint256 groupId = bg.createGroup("bet", "Bet", 0.1 ether);

        vm.prank(alice);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(PERFECT));

        vm.prank(alice);
        vm.expectRevert("Incorrect entry fee");
        bg.joinGroup{value: 0.05 ether}(groupId);
    }

    function test_joinGroup_rejectAfterResults() public {
        vm.prank(admin);
        uint256 groupId = bg.createGroup("bet", "Bet", 0);

        vm.prank(alice);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(PERFECT));

        vm.warp(DEADLINE + 1);
        mm.submitResults(RESULTS);

        vm.prank(alice);
        vm.expectRevert("Results already posted");
        bg.joinGroup(groupId);
    }

    function test_leaveGroup_refund() public {
        vm.prank(admin);
        uint256 groupId = bg.createGroup("bet", "Bet", 0.5 ether);

        vm.prank(alice);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(PERFECT));
        vm.prank(alice);
        bg.joinGroup{value: 0.5 ether}(groupId);

        uint256 balBefore = alice.balance;
        vm.prank(alice);
        bg.leaveGroup(groupId);

        assertEq(alice.balance - balBefore, 0.5 ether);
        assertFalse(bg.isMemberOf(groupId, alice));
        assertEq(bg.getGroup(groupId).entryCount, 0);
    }

    function test_leaveGroup_swapAndPop() public {
        vm.prank(admin);
        uint256 groupId = bg.createGroup("bet", "Bet", 0);

        vm.prank(alice);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(PERFECT));
        vm.prank(bob);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(BAD));
        vm.prank(charlie);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(PERFECT));

        vm.prank(alice);
        bg.joinGroup(groupId);
        vm.prank(bob);
        bg.joinGroup(groupId);
        vm.prank(charlie);
        bg.joinGroup(groupId);

        // Alice leaves (index 0) — charlie swaps in
        vm.prank(alice);
        bg.leaveGroup(groupId);

        assertEq(bg.getGroup(groupId).entryCount, 2);
        BracketGroups.Member[] memory members = bg.getMembers(groupId);
        assertEq(members[0].addr, charlie);
        assertEq(members[1].addr, bob);
    }

    function test_leaveGroup_blockedAfterResults() public {
        vm.prank(admin);
        uint256 groupId = bg.createGroup("bet", "Bet", 0);

        vm.prank(alice);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(PERFECT));
        vm.prank(alice);
        bg.joinGroup(groupId);

        vm.warp(DEADLINE + 1);
        mm.submitResults(RESULTS);

        vm.prank(alice);
        vm.expectRevert("Cannot leave after results posted");
        bg.leaveGroup(groupId);
    }

    function test_setEntryName() public {
        vm.prank(admin);
        uint256 groupId = bg.createGroup("bet", "Bet", 0);

        vm.prank(alice);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(PERFECT));
        vm.prank(alice);
        bg.joinGroup(groupId);

        vm.prank(alice);
        bg.setEntryName(groupId, "Alice the Great");

        assertEq(bg.getMembers(groupId)[0].name, "Alice the Great");
    }

    // ════════════════════════════════════════════════════════════════════
    //  JOIN — PASSWORD-PROTECTED GROUPS
    // ════════════════════════════════════════════════════════════════════

    function test_joinPasswordGroup() public {
        vm.prank(admin);
        uint256 groupId = bg.createGroupWithPassword("private", "Private", 0, sbytes32(PASSWORD));

        vm.prank(alice);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(PERFECT));

        vm.prank(alice);
        bg.joinGroupWithPassword(groupId, sbytes32(PASSWORD));

        assertTrue(bg.isMemberOf(groupId, alice));
    }

    function test_joinPasswordGroup_withName() public {
        vm.prank(admin);
        uint256 groupId = bg.createGroupWithPassword("private", "Private", 0, sbytes32(PASSWORD));

        vm.prank(alice);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(PERFECT));

        vm.prank(alice);
        bg.joinGroupWithPasswordAndName(groupId, sbytes32(PASSWORD), "Alice");

        assertEq(bg.getMembers(groupId)[0].name, "Alice");
    }

    function test_joinPasswordGroup_wrongPassword() public {
        vm.prank(admin);
        uint256 groupId = bg.createGroupWithPassword("private", "Private", 0, sbytes32(PASSWORD));

        vm.prank(alice);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(PERFECT));

        bytes32 wrongPw = keccak256("wrong");
        vm.prank(alice);
        vm.expectRevert("Wrong password");
        bg.joinGroupWithPassword(groupId, sbytes32(wrongPw));
    }

    function test_joinPasswordGroup_publicJoinReverts() public {
        vm.prank(admin);
        uint256 groupId = bg.createGroupWithPassword("private", "Private", 0, sbytes32(PASSWORD));

        vm.prank(alice);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(PERFECT));

        // Can't use joinGroup on password-protected group
        vm.prank(alice);
        vm.expectRevert("Password required");
        bg.joinGroup(groupId);
    }

    function test_joinPasswordGroup_withFee() public {
        vm.prank(admin);
        uint256 groupId = bg.createGroupWithPassword("private", "Private", 0.5 ether, sbytes32(PASSWORD));

        vm.prank(alice);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(PERFECT));

        vm.prank(alice);
        bg.joinGroupWithPassword{value: 0.5 ether}(groupId, sbytes32(PASSWORD));

        assertTrue(bg.isMemberOf(groupId, alice));
        assertEq(address(bg).balance, 0.5 ether);
    }

    function test_publicJoinOnPasswordlessGroup() public {
        // joinGroupWithPassword reverts on non-password group
        vm.prank(admin);
        uint256 groupId = bg.createGroup("public", "Public", 0);

        vm.prank(alice);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(PERFECT));

        vm.prank(alice);
        vm.expectRevert("Group is not password-protected");
        bg.joinGroupWithPassword(groupId, sbytes32(PASSWORD));
    }

    // ════════════════════════════════════════════════════════════════════
    //  SCORING + PAYOUTS
    // ════════════════════════════════════════════════════════════════════

    function test_scoreEntry() public {
        vm.prank(admin);
        uint256 groupId = bg.createGroup("bet", "Bet", 0);

        vm.prank(alice);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(PERFECT));
        vm.prank(alice);
        bg.joinGroup(groupId);

        vm.warp(DEADLINE + 1);
        mm.submitResults(RESULTS);

        bg.scoreEntry(groupId, 0);

        assertEq(bg.getMemberScore(groupId, 0), 192);
    }

    function test_scoreEntry_revertsBeforeResults() public {
        vm.prank(admin);
        uint256 groupId = bg.createGroup("bet", "Bet", 0);

        vm.prank(alice);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(PERFECT));
        vm.prank(alice);
        bg.joinGroup(groupId);

        vm.expectRevert("Results not posted");
        bg.scoreEntry(groupId, 0);
    }

    function test_scoreEntry_revertsAlreadyScored() public {
        vm.prank(admin);
        uint256 groupId = bg.createGroup("bet", "Bet", 0);

        vm.prank(alice);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(PERFECT));
        vm.prank(alice);
        bg.joinGroup(groupId);

        vm.warp(DEADLINE + 1);
        mm.submitResults(RESULTS);

        bg.scoreEntry(groupId, 0);
        vm.expectRevert("Already scored");
        bg.scoreEntry(groupId, 0);
    }

    function test_scoreEntry_revertsAfterScoringWindow() public {
        vm.prank(admin);
        uint256 groupId = bg.createGroup("bet", "Bet", 0);

        vm.prank(alice);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(PERFECT));
        vm.prank(alice);
        bg.joinGroup(groupId);

        vm.warp(DEADLINE + 1);
        mm.submitResults(RESULTS);
        vm.warp(mm.resultsPostedAt() + bg.SCORING_DURATION());

        vm.expectRevert("Scoring window closed");
        bg.scoreEntry(groupId, 0);
    }

    function test_collectWinnings_singleWinner() public {
        uint256 groupFee = 0.5 ether;
        vm.prank(admin);
        uint256 groupId = bg.createGroup("bet", "Bet", groupFee);

        vm.prank(alice);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(PERFECT));
        vm.prank(bob);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(BAD));

        vm.prank(alice);
        bg.joinGroup{value: groupFee}(groupId);
        vm.prank(bob);
        bg.joinGroup{value: groupFee}(groupId);

        vm.warp(DEADLINE + 1);
        mm.submitResults(RESULTS);

        bg.scoreEntry(groupId, 0);
        bg.scoreEntry(groupId, 1);

        vm.warp(mm.resultsPostedAt() + bg.SCORING_DURATION());

        uint256 aliceBefore = alice.balance;
        vm.prank(alice);
        bg.collectWinnings(groupId);
        assertEq(alice.balance - aliceBefore, 2 * groupFee);
    }

    function test_collectWinnings_twoWinnersSplit() public {
        uint256 groupFee = 0.5 ether;
        vm.prank(admin);
        uint256 groupId = bg.createGroup("bet", "Bet", groupFee);

        vm.prank(alice);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(PERFECT));
        vm.prank(bob);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(PERFECT));
        vm.prank(charlie);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(BAD));

        vm.prank(alice);
        bg.joinGroup{value: groupFee}(groupId);
        vm.prank(bob);
        bg.joinGroup{value: groupFee}(groupId);
        vm.prank(charlie);
        bg.joinGroup{value: groupFee}(groupId);

        vm.warp(DEADLINE + 1);
        mm.submitResults(RESULTS);

        bg.scoreEntry(groupId, 0);
        bg.scoreEntry(groupId, 1);
        bg.scoreEntry(groupId, 2);

        vm.warp(mm.resultsPostedAt() + bg.SCORING_DURATION());

        uint256 expectedPayout = (3 * groupFee) / 2;

        uint256 aliceBefore = alice.balance;
        vm.prank(alice);
        bg.collectWinnings(groupId);
        assertEq(alice.balance - aliceBefore, expectedPayout);

        uint256 bobBefore = bob.balance;
        vm.prank(bob);
        bg.collectWinnings(groupId);
        assertEq(bob.balance - bobBefore, expectedPayout);
    }

    function test_collectWinnings_nonWinnerReverts() public {
        uint256 groupFee = 0.5 ether;
        vm.prank(admin);
        uint256 groupId = bg.createGroup("bet", "Bet", groupFee);

        vm.prank(alice);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(PERFECT));
        vm.prank(bob);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(BAD));

        vm.prank(alice);
        bg.joinGroup{value: groupFee}(groupId);
        vm.prank(bob);
        bg.joinGroup{value: groupFee}(groupId);

        vm.warp(DEADLINE + 1);
        mm.submitResults(RESULTS);
        bg.scoreEntry(groupId, 0);
        bg.scoreEntry(groupId, 1);

        vm.warp(mm.resultsPostedAt() + bg.SCORING_DURATION());

        vm.prank(bob);
        vm.expectRevert("Not a winner");
        bg.collectWinnings(groupId);
    }

    function test_collectWinnings_cannotCollectTwice() public {
        uint256 groupFee = 0.5 ether;
        vm.prank(admin);
        uint256 groupId = bg.createGroup("bet", "Bet", groupFee);

        vm.prank(alice);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(PERFECT));
        vm.prank(alice);
        bg.joinGroup{value: groupFee}(groupId);

        vm.warp(DEADLINE + 1);
        mm.submitResults(RESULTS);
        bg.scoreEntry(groupId, 0);

        vm.warp(mm.resultsPostedAt() + bg.SCORING_DURATION());

        vm.prank(alice);
        bg.collectWinnings(groupId);
        vm.prank(alice);
        vm.expectRevert("Already collected");
        bg.collectWinnings(groupId);
    }

    function test_collectWinnings_revertsBeforeScoringWindow() public {
        uint256 groupFee = 0.5 ether;
        vm.prank(admin);
        uint256 groupId = bg.createGroup("bet", "Bet", groupFee);

        vm.prank(alice);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(PERFECT));
        vm.prank(alice);
        bg.joinGroup{value: groupFee}(groupId);

        vm.warp(DEADLINE + 1);
        mm.submitResults(RESULTS);
        bg.scoreEntry(groupId, 0);

        vm.prank(alice);
        vm.expectRevert("Scoring window still open");
        bg.collectWinnings(groupId);
    }

    function test_freeGroup_noPayouts() public {
        vm.prank(admin);
        uint256 groupId = bg.createGroup("free", "Free", 0);

        vm.prank(alice);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(PERFECT));
        vm.prank(alice);
        bg.joinGroup(groupId);

        vm.warp(DEADLINE + 1);
        mm.submitResults(RESULTS);
        bg.scoreEntry(groupId, 0);
        assertEq(bg.getMemberScore(groupId, 0), 192);

        vm.warp(mm.resultsPostedAt() + bg.SCORING_DURATION());
        vm.prank(alice);
        vm.expectRevert("No entry fee");
        bg.collectWinnings(groupId);
    }

    function test_indexOutOfBounds() public {
        vm.prank(admin);
        uint256 groupId = bg.createGroup("bet", "Bet", 0);

        vm.expectRevert("Index out of bounds");
        bg.scoreEntry(groupId, 0);

        vm.expectRevert("Index out of bounds");
        bg.getMember(groupId, 0);
    }
}
