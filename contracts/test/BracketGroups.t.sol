// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

import {Test} from "forge-std/Test.sol";
import {MarchMadness} from "../src/MarchMadness.sol";
import {BracketGroups} from "../src/BracketGroups.sol";

/// @title BracketGroups tests — manual groups, linked groups, scoring, payouts
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
    //  MANUAL GROUP TESTS
    // ════════════════════════════════════════════════════════════════════

    function test_createManualGroup() public {
        vm.prank(admin);
        uint256 groupId = bg.createManualGroup("yahoo-pool", "Yahoo Fantasy Pool");

        assertEq(groupId, 1);
        assertEq(bg.getGroupBySlug("yahoo-pool"), 1);

        BracketGroups.Group memory g = bg.getGroup(groupId);
        assertEq(uint8(g.groupType), uint8(BracketGroups.GroupType.Manual));
        assertEq(g.slug, "yahoo-pool");
        assertEq(g.displayName, "Yahoo Fantasy Pool");
        assertEq(g.admin, admin);
        assertEq(g.entryCount, 0);
        assertEq(g.entryFee, 0);
    }

    function test_addManualEntries() public {
        vm.prank(admin);
        uint256 groupId = bg.createManualGroup("pool", "Pool");

        vm.startPrank(admin);
        bg.addManualEntry(groupId, PERFECT, "Alice");
        bg.addManualEntry(groupId, BAD, "Bob");
        vm.stopPrank();

        BracketGroups.Group memory g = bg.getGroup(groupId);
        assertEq(g.entryCount, 2);

        BracketGroups.ManualEntry memory e0 = bg.getManualEntry(groupId, 0);
        assertEq(e0.name, "Alice");
        assertEq(e0.bracket, PERFECT);

        BracketGroups.ManualEntry memory e1 = bg.getManualEntry(groupId, 1);
        assertEq(e1.name, "Bob");
        assertEq(e1.bracket, BAD);
    }

    function test_getManualEntries_batch() public {
        vm.prank(admin);
        uint256 groupId = bg.createManualGroup("pool", "Pool");

        vm.startPrank(admin);
        bg.addManualEntry(groupId, PERFECT, "Alice");
        bg.addManualEntry(groupId, BAD, "Bob");
        vm.stopPrank();

        BracketGroups.ManualEntry[] memory entries = bg.getManualEntries(groupId);
        assertEq(entries.length, 2);
        assertEq(entries[0].name, "Alice");
        assertEq(entries[1].name, "Bob");
    }

    function test_removeManualEntry_swapAndPop() public {
        vm.prank(admin);
        uint256 groupId = bg.createManualGroup("pool", "Pool");

        vm.startPrank(admin);
        bg.addManualEntry(groupId, PERFECT, "Alice");
        bg.addManualEntry(groupId, BAD, "Bob");
        bg.addManualEntry(groupId, PERFECT, "Charlie");

        // Remove index 0 (Alice) — Bob stays at 1, Charlie moves to 0
        bg.removeManualEntry(groupId, 0);
        vm.stopPrank();

        BracketGroups.Group memory g = bg.getGroup(groupId);
        assertEq(g.entryCount, 2);

        // Charlie swapped into index 0
        BracketGroups.ManualEntry memory e0 = bg.getManualEntry(groupId, 0);
        assertEq(e0.name, "Charlie");

        // Bob unchanged at index 1
        BracketGroups.ManualEntry memory e1 = bg.getManualEntry(groupId, 1);
        assertEq(e1.name, "Bob");
    }

    function test_removeManualEntry_lastElement() public {
        vm.prank(admin);
        uint256 groupId = bg.createManualGroup("pool", "Pool");

        vm.startPrank(admin);
        bg.addManualEntry(groupId, PERFECT, "Alice");
        bg.addManualEntry(groupId, BAD, "Bob");

        // Remove last element
        bg.removeManualEntry(groupId, 1);
        vm.stopPrank();

        assertEq(bg.getGroup(groupId).entryCount, 1);
        assertEq(bg.getManualEntry(groupId, 0).name, "Alice");
    }

    function test_updateManualBracket() public {
        vm.prank(admin);
        uint256 groupId = bg.createManualGroup("pool", "Pool");

        vm.startPrank(admin);
        bg.addManualEntry(groupId, PERFECT, "Alice");
        bg.updateManualBracket(groupId, 0, BAD);
        vm.stopPrank();

        assertEq(bg.getManualEntry(groupId, 0).bracket, BAD);
    }

    function test_updateManualBracket_blockedAfterResults() public {
        vm.prank(admin);
        uint256 groupId = bg.createManualGroup("pool", "Pool");

        vm.prank(admin);
        bg.addManualEntry(groupId, PERFECT, "Alice");

        // Post results on main contract
        vm.warp(DEADLINE + 1);
        mm.submitResults(RESULTS);

        vm.prank(admin);
        vm.expectRevert("Results already posted");
        bg.updateManualBracket(groupId, 0, BAD);
    }

    function test_updateEntryName() public {
        vm.prank(admin);
        uint256 groupId = bg.createManualGroup("pool", "Pool");

        vm.prank(admin);
        bg.addManualEntry(groupId, PERFECT, "Alice");

        vm.prank(admin);
        bg.updateEntryName(groupId, 0, "Alice (updated)");

        assertEq(bg.getManualEntry(groupId, 0).name, "Alice (updated)");
    }

    function test_manualEntryScoring() public {
        vm.prank(admin);
        uint256 groupId = bg.createManualGroup("pool", "Pool");

        vm.startPrank(admin);
        bg.addManualEntry(groupId, PERFECT, "Alice");
        bg.addManualEntry(groupId, BAD, "Bob");
        vm.stopPrank();

        // Post results
        vm.warp(DEADLINE + 1);
        mm.submitResults(RESULTS);

        // Perfect bracket should score 192
        assertEq(bg.getManualEntryScore(groupId, 0), 192);

        // Bad bracket scores much lower
        uint8 bobScore = bg.getManualEntryScore(groupId, 1);
        assertTrue(bobScore < 192);
    }

    function test_manualGroupScores_batch() public {
        vm.prank(admin);
        uint256 groupId = bg.createManualGroup("pool", "Pool");

        vm.startPrank(admin);
        bg.addManualEntry(groupId, PERFECT, "Alice");
        bg.addManualEntry(groupId, BAD, "Bob");
        vm.stopPrank();

        vm.warp(DEADLINE + 1);
        mm.submitResults(RESULTS);

        uint8[] memory scores = bg.getManualGroupScores(groupId);
        assertEq(scores.length, 2);
        assertEq(scores[0], 192);
        assertTrue(scores[1] < 192);
    }

    function test_manualEntryScore_revertsBeforeResults() public {
        vm.prank(admin);
        uint256 groupId = bg.createManualGroup("pool", "Pool");

        vm.prank(admin);
        bg.addManualEntry(groupId, PERFECT, "Alice");

        vm.expectRevert("Results not posted");
        bg.getManualEntryScore(groupId, 0);
    }

    function test_setPrizeDescription() public {
        vm.prank(admin);
        uint256 groupId = bg.createManualGroup("pool", "Pool");

        vm.prank(admin);
        bg.setPrizeDescription(groupId, "$500 Amazon gift card");

        BracketGroups.Group memory g = bg.getGroup(groupId);
        assertEq(g.prizeDescription, "$500 Amazon gift card");
    }

    function test_setPrizeDescription_onlyAdmin() public {
        vm.prank(admin);
        uint256 groupId = bg.createManualGroup("pool", "Pool");

        vm.prank(alice);
        vm.expectRevert("Not group admin");
        bg.setPrizeDescription(groupId, "hack");
    }

    // ════════════════════════════════════════════════════════════════════
    //  LINKED GROUP TESTS
    // ════════════════════════════════════════════════════════════════════

    function test_createLinkedGroup() public {
        vm.prank(admin);
        uint256 groupId = bg.createLinkedGroup("side-bet", "Side Bet", 0.1 ether);

        BracketGroups.Group memory g = bg.getGroup(groupId);
        assertEq(uint8(g.groupType), uint8(BracketGroups.GroupType.Linked));
        assertEq(g.entryFee, 0.1 ether);
        assertEq(g.slug, "side-bet");
    }

    function test_joinLinkedGroup() public {
        vm.prank(admin);
        uint256 groupId = bg.createLinkedGroup("bet", "Bet", 0.1 ether);

        // Alice submits bracket to main contract
        vm.prank(alice);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(PERFECT));

        // Alice joins group
        vm.prank(alice);
        bg.joinGroup{value: 0.1 ether}(groupId);

        assertTrue(bg.isMemberOf(groupId, alice));
        assertEq(bg.getGroup(groupId).entryCount, 1);
    }

    function test_joinLinkedGroupWithName() public {
        vm.prank(admin);
        uint256 groupId = bg.createLinkedGroup("bet", "Bet", 0);

        vm.prank(alice);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(PERFECT));

        vm.prank(alice);
        bg.joinGroupWithName(groupId, "Alice");

        BracketGroups.LinkedMember[] memory members = bg.getLinkedMembers(groupId);
        assertEq(members.length, 1);
        assertEq(members[0].name, "Alice");
        assertEq(members[0].addr, alice);
    }

    function test_joinLinkedGroup_rejectDuplicate() public {
        vm.prank(admin);
        uint256 groupId = bg.createLinkedGroup("bet", "Bet", 0);

        vm.prank(alice);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(PERFECT));

        vm.prank(alice);
        bg.joinGroup(groupId);

        vm.prank(alice);
        vm.expectRevert("Already a member");
        bg.joinGroup(groupId);
    }

    function test_joinLinkedGroup_rejectWithoutMainBracket() public {
        vm.prank(admin);
        uint256 groupId = bg.createLinkedGroup("bet", "Bet", 0);

        vm.prank(alice);
        vm.expectRevert("No bracket in main contract");
        bg.joinGroup(groupId);
    }

    function test_joinLinkedGroup_rejectWrongFee() public {
        vm.prank(admin);
        uint256 groupId = bg.createLinkedGroup("bet", "Bet", 0.1 ether);

        vm.prank(alice);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(PERFECT));

        vm.prank(alice);
        vm.expectRevert("Incorrect entry fee");
        bg.joinGroup{value: 0.05 ether}(groupId);
    }

    function test_joinLinkedGroup_rejectAfterResults() public {
        vm.prank(admin);
        uint256 groupId = bg.createLinkedGroup("bet", "Bet", 0);

        vm.prank(alice);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(PERFECT));

        vm.warp(DEADLINE + 1);
        mm.submitResults(RESULTS);

        vm.prank(alice);
        vm.expectRevert("Results already posted");
        bg.joinGroup(groupId);
    }

    function test_leaveLinkedGroup_refund() public {
        vm.prank(admin);
        uint256 groupId = bg.createLinkedGroup("bet", "Bet", 0.5 ether);

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

    function test_leaveLinkedGroup_swapAndPop() public {
        vm.prank(admin);
        uint256 groupId = bg.createLinkedGroup("bet", "Bet", 0);

        // Submit brackets for alice, bob, charlie
        vm.prank(alice);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(PERFECT));
        vm.prank(bob);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(BAD));
        vm.prank(charlie);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(PERFECT));

        // All join
        vm.prank(alice);
        bg.joinGroup(groupId);
        vm.prank(bob);
        bg.joinGroup(groupId);
        vm.prank(charlie);
        bg.joinGroup(groupId);

        // Alice leaves (index 0) — charlie should swap in
        vm.prank(alice);
        bg.leaveGroup(groupId);

        assertEq(bg.getGroup(groupId).entryCount, 2);
        assertFalse(bg.isMemberOf(groupId, alice));

        BracketGroups.LinkedMember[] memory members = bg.getLinkedMembers(groupId);
        assertEq(members[0].addr, charlie);
        assertEq(members[1].addr, bob);
    }

    function test_leaveLinkedGroup_blockedAfterResults() public {
        vm.prank(admin);
        uint256 groupId = bg.createLinkedGroup("bet", "Bet", 0);

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

    function test_setGroupEntryName() public {
        vm.prank(admin);
        uint256 groupId = bg.createLinkedGroup("bet", "Bet", 0);

        vm.prank(alice);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(PERFECT));

        vm.prank(alice);
        bg.joinGroup(groupId);

        vm.prank(alice);
        bg.setGroupEntryName(groupId, "Alice the Great");

        BracketGroups.LinkedMember[] memory members = bg.getLinkedMembers(groupId);
        assertEq(members[0].name, "Alice the Great");
    }

    // ════════════════════════════════════════════════════════════════════
    //  LINKED GROUP — SCORING + PAYOUTS
    // ════════════════════════════════════════════════════════════════════

    function test_scoreGroupEntry() public {
        vm.prank(admin);
        uint256 groupId = bg.createLinkedGroup("bet", "Bet", 0);

        vm.prank(alice);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(PERFECT));
        vm.prank(alice);
        bg.joinGroup(groupId);

        vm.warp(DEADLINE + 1);
        mm.submitResults(RESULTS);

        bg.scoreGroupEntry(groupId, 0);

        assertEq(bg.getLinkedMemberScore(groupId, 0), 192);
    }

    function test_scoreGroupEntry_revertsBeforeResults() public {
        vm.prank(admin);
        uint256 groupId = bg.createLinkedGroup("bet", "Bet", 0);

        vm.prank(alice);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(PERFECT));
        vm.prank(alice);
        bg.joinGroup(groupId);

        vm.expectRevert("Results not posted");
        bg.scoreGroupEntry(groupId, 0);
    }

    function test_scoreGroupEntry_revertsAlreadyScored() public {
        vm.prank(admin);
        uint256 groupId = bg.createLinkedGroup("bet", "Bet", 0);

        vm.prank(alice);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(PERFECT));
        vm.prank(alice);
        bg.joinGroup(groupId);

        vm.warp(DEADLINE + 1);
        mm.submitResults(RESULTS);

        bg.scoreGroupEntry(groupId, 0);

        vm.expectRevert("Already scored");
        bg.scoreGroupEntry(groupId, 0);
    }

    function test_collectGroupWinnings_singleWinner() public {
        uint256 groupFee = 0.5 ether;
        vm.prank(admin);
        uint256 groupId = bg.createLinkedGroup("bet", "Bet", groupFee);

        // Alice (perfect) and Bob (bad) join
        vm.prank(alice);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(PERFECT));
        vm.prank(bob);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(BAD));

        vm.prank(alice);
        bg.joinGroup{value: groupFee}(groupId);
        vm.prank(bob);
        bg.joinGroup{value: groupFee}(groupId);

        // Post results and score
        vm.warp(DEADLINE + 1);
        mm.submitResults(RESULTS);

        bg.scoreGroupEntry(groupId, 0); // alice
        bg.scoreGroupEntry(groupId, 1); // bob

        // Warp past scoring window
        vm.warp(mm.resultsPostedAt() + bg.SCORING_DURATION());

        uint256 aliceBefore = alice.balance;
        vm.prank(alice);
        bg.collectGroupWinnings(groupId);

        // Alice gets full pool: 2 * 0.5 = 1 ETH
        assertEq(alice.balance - aliceBefore, 2 * groupFee);
    }

    function test_collectGroupWinnings_twoWinnersSplit() public {
        uint256 groupFee = 0.5 ether;
        vm.prank(admin);
        uint256 groupId = bg.createLinkedGroup("bet", "Bet", groupFee);

        // Alice and Bob both have perfect brackets, Charlie has bad
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

        bg.scoreGroupEntry(groupId, 0);
        bg.scoreGroupEntry(groupId, 1);
        bg.scoreGroupEntry(groupId, 2);

        vm.warp(mm.resultsPostedAt() + bg.SCORING_DURATION());

        uint256 expectedPayout = (3 * groupFee) / 2;

        uint256 aliceBefore = alice.balance;
        vm.prank(alice);
        bg.collectGroupWinnings(groupId);
        assertEq(alice.balance - aliceBefore, expectedPayout);

        uint256 bobBefore = bob.balance;
        vm.prank(bob);
        bg.collectGroupWinnings(groupId);
        assertEq(bob.balance - bobBefore, expectedPayout);
    }

    function test_collectGroupWinnings_nonWinnerReverts() public {
        uint256 groupFee = 0.5 ether;
        vm.prank(admin);
        uint256 groupId = bg.createLinkedGroup("bet", "Bet", groupFee);

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

        bg.scoreGroupEntry(groupId, 0);
        bg.scoreGroupEntry(groupId, 1);

        vm.warp(mm.resultsPostedAt() + bg.SCORING_DURATION());

        vm.prank(bob);
        vm.expectRevert("Not a winner");
        bg.collectGroupWinnings(groupId);
    }

    function test_collectGroupWinnings_cannotCollectTwice() public {
        uint256 groupFee = 0.5 ether;
        vm.prank(admin);
        uint256 groupId = bg.createLinkedGroup("bet", "Bet", groupFee);

        vm.prank(alice);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(PERFECT));
        vm.prank(alice);
        bg.joinGroup{value: groupFee}(groupId);

        vm.warp(DEADLINE + 1);
        mm.submitResults(RESULTS);
        bg.scoreGroupEntry(groupId, 0);

        vm.warp(mm.resultsPostedAt() + bg.SCORING_DURATION());

        vm.prank(alice);
        bg.collectGroupWinnings(groupId);

        vm.prank(alice);
        vm.expectRevert("Already collected");
        bg.collectGroupWinnings(groupId);
    }

    function test_collectGroupWinnings_revertsBeforeScoringWindow() public {
        uint256 groupFee = 0.5 ether;
        vm.prank(admin);
        uint256 groupId = bg.createLinkedGroup("bet", "Bet", groupFee);

        vm.prank(alice);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(PERFECT));
        vm.prank(alice);
        bg.joinGroup{value: groupFee}(groupId);

        vm.warp(DEADLINE + 1);
        mm.submitResults(RESULTS);
        bg.scoreGroupEntry(groupId, 0);

        // Don't warp past scoring window
        vm.prank(alice);
        vm.expectRevert("Scoring window still open");
        bg.collectGroupWinnings(groupId);
    }

    // ════════════════════════════════════════════════════════════════════
    //  EDGE CASES
    // ════════════════════════════════════════════════════════════════════

    function test_duplicateSlugReverts() public {
        vm.prank(admin);
        bg.createManualGroup("pool", "Pool");

        vm.prank(admin);
        vm.expectRevert("Slug already taken");
        bg.createManualGroup("pool", "Other Pool");
    }

    function test_emptySlugReverts() public {
        vm.prank(admin);
        vm.expectRevert("Slug cannot be empty");
        bg.createManualGroup("", "Pool");
    }

    function test_longSlugReverts() public {
        vm.prank(admin);
        vm.expectRevert("Slug too long");
        bg.createManualGroup("this-slug-is-way-too-long-and-exceeds-the-32-byte-limit", "Pool");
    }

    function test_nonAdminCannotModifyManualGroup() public {
        vm.prank(admin);
        uint256 groupId = bg.createManualGroup("pool", "Pool");

        vm.prank(alice);
        vm.expectRevert("Not group admin");
        bg.addManualEntry(groupId, PERFECT, "Alice");

        // First add an entry as admin
        vm.prank(admin);
        bg.addManualEntry(groupId, PERFECT, "Alice");

        vm.prank(alice);
        vm.expectRevert("Not group admin");
        bg.removeManualEntry(groupId, 0);

        vm.prank(alice);
        vm.expectRevert("Not group admin");
        bg.updateManualBracket(groupId, 0, BAD);

        vm.prank(alice);
        vm.expectRevert("Not group admin");
        bg.updateEntryName(groupId, 0, "hacked");
    }

    function test_invalidSentinelReverts() public {
        vm.prank(admin);
        uint256 groupId = bg.createManualGroup("pool", "Pool");

        bytes8 noSentinel = bytes8(0x0000000000000001);
        vm.prank(admin);
        vm.expectRevert("Invalid sentinel byte");
        bg.addManualEntry(groupId, noSentinel, "Bad");
    }

    function test_cannotJoinManualGroup() public {
        vm.prank(admin);
        uint256 groupId = bg.createManualGroup("pool", "Pool");

        vm.prank(alice);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(PERFECT));

        vm.prank(alice);
        vm.expectRevert("Not a linked group");
        bg.joinGroup(groupId);
    }

    function test_cannotAddManualEntryToLinkedGroup() public {
        vm.prank(admin);
        uint256 groupId = bg.createLinkedGroup("bet", "Bet", 0);

        vm.prank(admin);
        vm.expectRevert("Not a manual group");
        bg.addManualEntry(groupId, PERFECT, "Alice");
    }

    function test_nonexistentGroupReverts() public {
        vm.expectRevert("Group does not exist");
        bg.getGroup(999);
    }

    function test_slugLookupNonexistent() public {
        vm.expectRevert("Group not found");
        bg.getGroupBySlug("nope");
    }

    function test_getIsMember() public {
        vm.prank(admin);
        uint256 groupId = bg.createLinkedGroup("bet", "Bet", 0);

        assertFalse(bg.getIsMember(groupId, alice));

        vm.prank(alice);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(PERFECT));
        vm.prank(alice);
        bg.joinGroup(groupId);

        assertTrue(bg.getIsMember(groupId, alice));
    }

    function test_multipleGroups() public {
        vm.prank(admin);
        uint256 g1 = bg.createManualGroup("manual1", "Manual 1");
        vm.prank(admin);
        uint256 g2 = bg.createLinkedGroup("linked1", "Linked 1", 0);
        vm.prank(alice);
        uint256 g3 = bg.createManualGroup("manual2", "Manual 2");

        assertEq(g1, 1);
        assertEq(g2, 2);
        assertEq(g3, 3);

        assertEq(bg.getGroup(g3).admin, alice);
    }

    function test_freeLinkedGroup() public {
        // Entry fee = 0 linked group (no payouts, just tracking)
        vm.prank(admin);
        uint256 groupId = bg.createLinkedGroup("free", "Free", 0);

        vm.prank(alice);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(PERFECT));
        vm.prank(alice);
        bg.joinGroup(groupId);

        // Scoring works
        vm.warp(DEADLINE + 1);
        mm.submitResults(RESULTS);
        bg.scoreGroupEntry(groupId, 0);
        assertEq(bg.getLinkedMemberScore(groupId, 0), 192);

        // But collecting reverts (no entry fee)
        vm.warp(mm.resultsPostedAt() + bg.SCORING_DURATION());
        vm.prank(alice);
        vm.expectRevert("No entry fee");
        bg.collectGroupWinnings(groupId);
    }

    function test_manualGroupCannotPayOut() public {
        // Manual groups never hold ETH and have no payout mechanism
        vm.prank(admin);
        uint256 groupId = bg.createManualGroup("pool", "Pool");

        vm.prank(admin);
        bg.addManualEntry(groupId, PERFECT, "Alice");

        // Cannot call collectGroupWinnings on manual group
        vm.warp(DEADLINE + 1);
        mm.submitResults(RESULTS);
        vm.warp(mm.resultsPostedAt() + bg.SCORING_DURATION());

        vm.prank(admin);
        vm.expectRevert("Not a linked group");
        bg.collectGroupWinnings(groupId);

        // Cannot call scoreGroupEntry on manual group
        vm.prank(admin);
        vm.expectRevert("Not a linked group");
        bg.scoreGroupEntry(groupId, 0);

        // Contract holds zero ETH from manual groups
        assertEq(address(bg).balance, 0);
    }

    function test_scoreGroupEntry_revertsAfterScoringWindow() public {
        vm.prank(admin);
        uint256 groupId = bg.createLinkedGroup("bet", "Bet", 0);

        vm.prank(alice);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(PERFECT));
        vm.prank(alice);
        bg.joinGroup(groupId);

        vm.warp(DEADLINE + 1);
        mm.submitResults(RESULTS);

        // Warp past scoring window
        vm.warp(mm.resultsPostedAt() + bg.SCORING_DURATION());

        vm.expectRevert("Scoring window closed");
        bg.scoreGroupEntry(groupId, 0);
    }

    function test_indexOutOfBounds() public {
        vm.prank(admin);
        uint256 groupId = bg.createManualGroup("pool", "Pool");

        vm.prank(admin);
        vm.expectRevert("Index out of bounds");
        bg.removeManualEntry(groupId, 0);

        vm.prank(admin);
        vm.expectRevert("Index out of bounds");
        bg.updateManualBracket(groupId, 0, PERFECT);

        vm.prank(admin);
        vm.expectRevert("Index out of bounds");
        bg.updateEntryName(groupId, 0, "name");
    }
}
