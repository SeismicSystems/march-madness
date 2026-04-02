// SPDX-License-Identifier: MIT
pragma solidity ^0.8.30;

import {Test} from "forge-std/Test.sol";
import {MarchMadness} from "../src/MarchMadness.sol";
import {MarchMadnessV2} from "../src/MarchMadnessV2.sol";
import {BracketGroups} from "../src/BracketGroups.sol";
import {BracketGroupsV2} from "../src/BracketGroupsV2.sol";
import {IMarchMadness} from "../src/IMarchMadness.sol";
import {ByteBracket} from "../src/ByteBracket.sol";

/// @title MarchMadnessV2 tests — migration import, funding, and preview scoring
contract MarchMadnessV2Test is Test {
    MarchMadnessV2 mm;

    address owner = address(this);
    address alice = address(0xA11CE);
    address bob = address(0xB0B);
    address nonOwner = address(0xDEAD);

    uint256 constant ENTRY_FEE = 0.1 ether;
    uint256 constant DEADLINE = 2000; // future deadline for privacy tests

    // Golden vectors from data/test-vectors/bracket-vectors.json
    bytes8 constant ALL_CHALK = bytes8(0xFFFFFFFFFFFFFFFF);
    bytes8 constant ALL_UPSETS = bytes8(0x8000000000000000);
    bytes8 constant CINDERELLA = bytes8(0xBFFFFFFFBFFFBFBA);
    bytes8 constant NO_SENTINEL = bytes8(0x7FFFFFFFFFFFFFFF); // invalid — no sentinel

    function setUp() public {
        // Start before deadline so we can test pre-deadline privacy.
        vm.warp(1000);
        mm = new MarchMadnessV2(2026, ENTRY_FEE, DEADLINE);
        vm.deal(address(this), 100 ether);
        vm.deal(alice, 10 ether);
        vm.deal(bob, 10 ether);
    }

    // ════════════════════════════════════════════════════════════════════
    //  importEntry
    // ════════════════════════════════════════════════════════════════════

    function test_importEntry_setsState() public {
        vm.expectEmit(true, false, false, false);
        emit IMarchMadness.BracketSubmitted(alice);

        mm.importEntry(alice, ALL_CHALK);

        assertTrue(mm.hasEntry(alice));
        assertEq(mm.numEntries(), 1);
    }

    function test_importEntry_rejectsDuplicate() public {
        mm.importEntry(alice, ALL_CHALK);
        vm.expectRevert(IMarchMadness.AlreadySubmitted.selector);
        mm.importEntry(alice, ALL_CHALK);
    }

    function test_importEntry_rejectsNoSentinel() public {
        vm.expectRevert(IMarchMadness.InvalidSentinelByte.selector);
        mm.importEntry(alice, NO_SENTINEL);
    }

    function test_importEntry_rejectsNonOwner() public {
        vm.prank(nonOwner);
        vm.expectRevert(IMarchMadness.OnlyOwner.selector);
        mm.importEntry(alice, ALL_CHALK);
    }

    // ════════════════════════════════════════════════════════════════════
    //  batchImportEntries
    // ════════════════════════════════════════════════════════════════════

    function test_batchImportEntries_happy() public {
        address[] memory accts = new address[](2);
        bytes8[] memory brackets = new bytes8[](2);
        accts[0] = alice;
        accts[1] = bob;
        brackets[0] = ALL_CHALK;
        brackets[1] = ALL_UPSETS;

        mm.batchImportEntries(accts, brackets);

        assertTrue(mm.hasEntry(alice));
        assertTrue(mm.hasEntry(bob));
        assertEq(mm.numEntries(), 2);
    }

    function test_batchImportEntries_idempotent() public {
        mm.importEntry(alice, ALL_CHALK);

        address[] memory accts = new address[](2);
        bytes8[] memory brackets = new bytes8[](2);
        accts[0] = alice; // already imported
        accts[1] = bob;
        brackets[0] = ALL_CHALK;
        brackets[1] = ALL_UPSETS;

        mm.batchImportEntries(accts, brackets); // must not revert

        assertEq(mm.numEntries(), 2); // alice counted once, bob added
    }

    function test_batchImportEntries_lengthMismatch_reverts() public {
        address[] memory accts = new address[](2);
        bytes8[] memory brackets = new bytes8[](1);
        accts[0] = alice;
        accts[1] = bob;
        brackets[0] = ALL_CHALK;
        vm.expectRevert();
        mm.batchImportEntries(accts, brackets);
    }

    // ════════════════════════════════════════════════════════════════════
    //  importTag
    // ════════════════════════════════════════════════════════════════════

    function test_importTag_happy() public {
        mm.importEntry(alice, ALL_CHALK);

        vm.expectEmit(true, false, false, true);
        emit IMarchMadness.TagSet(alice, "alice");
        mm.importTag(alice, "alice");

        assertEq(mm.getTag(alice), "alice");
    }

    function test_importTag_rejectsNoEntry() public {
        vm.expectRevert(IMarchMadness.NoBracketSubmitted.selector);
        mm.importTag(alice, "alice");
    }

    function test_importTag_rejectsNonOwner() public {
        mm.importEntry(alice, ALL_CHALK);
        vm.prank(nonOwner);
        vm.expectRevert(IMarchMadness.OnlyOwner.selector);
        mm.importTag(alice, "alice");
    }

    // ════════════════════════════════════════════════════════════════════
    //  getBracket — privacy semantics
    // ════════════════════════════════════════════════════════════════════

    function test_getBracket_postDeadline_anyoneCan() public {
        mm.importEntry(alice, ALL_CHALK);
        vm.warp(DEADLINE + 1); // past deadline

        bytes8 b = mm.getBracket(alice);
        assertEq(b, ALL_CHALK);
    }

    function test_getBracket_preDeadline_ownerCanReadOwn() public {
        mm.importEntry(alice, ALL_CHALK);
        // Still before deadline (warp is at 1000, deadline is 2000)

        vm.prank(alice);
        bytes8 b = mm.getBracket(alice);
        assertEq(b, ALL_CHALK);
    }

    function test_getBracket_preDeadline_otherRevertsWithError() public {
        mm.importEntry(alice, ALL_CHALK);
        // Still before deadline

        vm.prank(bob);
        vm.expectRevert(IMarchMadness.CannotReadBracketBeforeDeadline.selector);
        mm.getBracket(alice);
    }

    // ════════════════════════════════════════════════════════════════════
    //  previewScore
    // ════════════════════════════════════════════════════════════════════

    function test_previewScore_selfScore192() public {
        mm.importEntry(alice, ALL_CHALK);
        vm.warp(DEADLINE + 1);

        uint8 score = mm.previewScore(alice, ALL_CHALK);
        assertEq(score, 192);
    }

    function test_previewScore_noEntryReverts() public {
        vm.warp(DEADLINE + 1);
        vm.expectRevert(IMarchMadness.NoBracketSubmitted.selector);
        mm.previewScore(alice, ALL_CHALK);
    }

    function test_previewScore_noSentinelOnResultsReverts() public {
        mm.importEntry(alice, ALL_CHALK);
        vm.warp(DEADLINE + 1);
        vm.expectRevert(IMarchMadness.InvalidSentinelByte.selector);
        mm.previewScore(alice, NO_SENTINEL);
    }

    function test_previewScore_preservesPrivacyPreDeadline() public {
        mm.importEntry(alice, ALL_CHALK);
        // Before deadline — bob cannot preview alice's score
        vm.prank(bob);
        vm.expectRevert(IMarchMadness.CannotReadBracketBeforeDeadline.selector);
        mm.previewScore(alice, ALL_CHALK);
    }

    function test_previewScore_matchesScoreBracket() public {
        // Import alice with ALL_CHALK and post real results.
        // previewScore with those results should match the actual scoreBracket result.
        vm.warp(DEADLINE + 1);
        mm.importEntry(alice, ALL_CHALK);
        mm.submitResults(ALL_CHALK); // owner submits results

        uint8 preview = mm.previewScore(alice, ALL_CHALK);

        // Score alice on-chain
        mm.scoreBracket(alice);
        uint8 actual = mm.scores(alice);

        assertEq(preview, actual);
    }

    // ════════════════════════════════════════════════════════════════════
    //  scoreBracket — end-to-end via inherited logic
    // ════════════════════════════════════════════════════════════════════

    /// @notice Proves virtual dispatch: scoreBracket reads from `brackets[account]`
    ///         which is the shielded slot written by importEntry.
    function test_scoreBracket_worksOnImportedEntry() public {
        vm.warp(DEADLINE + 1);
        mm.importEntry(alice, ALL_CHALK);
        mm.submitResults(ALL_CHALK);

        mm.scoreBracket(alice);

        assertEq(mm.scores(alice), 192);
        assertTrue(mm.isScored(alice));
    }

    function test_scoreBracket_zeroScoreForAllUpsets() public {
        vm.warp(DEADLINE + 1);
        mm.importEntry(alice, ALL_UPSETS);
        mm.submitResults(ALL_CHALK); // chalk results, upset bracket = 0 score

        mm.scoreBracket(alice);
        assertEq(mm.scores(alice), 0);
    }

    // ════════════════════════════════════════════════════════════════════
    //  fund / receive
    // ════════════════════════════════════════════════════════════════════

    function test_fund_acceptsEth() public {
        mm.fund{value: 1 ether}();
        assertEq(address(mm).balance, 1 ether);
    }

    function test_receive_acceptsEth() public {
        (bool ok,) = address(mm).call{value: 0.5 ether}("");
        assertTrue(ok);
        assertEq(address(mm).balance, 0.5 ether);
    }
}

/// @title BracketGroupsV2 tests — migration import for groups and members
contract BracketGroupsV2Test is Test {
    MarchMadnessV2 mm;
    BracketGroupsV2 bg;

    address owner = address(this);
    address groupCreator = address(0xC2);
    address alice = address(0xA11CE);
    address bob = address(0xB0B);
    address nonOwner = address(0xDEAD);

    uint256 constant ENTRY_FEE = 0.1 ether;
    uint256 constant DEADLINE = 500;

    bytes8 constant ALL_CHALK = bytes8(0xFFFFFFFFFFFFFFFF);

    function setUp() public {
        vm.warp(600); // past deadline
        mm = new MarchMadnessV2(2026, ENTRY_FEE, DEADLINE);
        bg = new BracketGroupsV2(address(mm));

        // Import brackets so members are valid in the main contract
        mm.importEntry(alice, ALL_CHALK);
        mm.importEntry(bob, bytes8(0x8000000000000000));

        vm.deal(address(this), 10 ether);
    }

    // ════════════════════════════════════════════════════════════════════
    //  importGroup
    // ════════════════════════════════════════════════════════════════════

    function test_importGroup_happy() public {
        uint32 gId = bg.importGroup("test-group", "Test Group", 0.1 ether, groupCreator);

        assertEq(gId, 1);
        BracketGroups.Group memory g = bg.getGroup(gId);
        assertEq(g.slug, "test-group");
        assertEq(g.displayName, "Test Group");
        assertEq(g.creator, groupCreator);
        assertEq(g.entryCount, 0); // no auto-join
        assertEq(g.entryFee, 0.1 ether);
        assertFalse(g.hasPassword);
    }

    function test_importGroup_duplicateSlug_reverts() public {
        bg.importGroup("test-group", "Test Group", 0, groupCreator);
        vm.expectRevert(BracketGroups.SlugAlreadyTaken.selector);
        bg.importGroup("test-group", "Other", 0, groupCreator);
    }

    function test_importGroup_rejectsNonOwner() public {
        vm.prank(nonOwner);
        vm.expectRevert("not owner");
        bg.importGroup("test-group", "Test Group", 0, groupCreator);
    }

    // ════════════════════════════════════════════════════════════════════
    //  importMember / batchImportMembers
    // ════════════════════════════════════════════════════════════════════

    function test_importMember_happy() public {
        uint32 gId = bg.importGroup("grp", "Grp", 0, groupCreator);
        bg.importMember(gId, alice, "Alice");

        assertTrue(bg.isMemberOf(gId, alice));
        BracketGroups.Group memory g = bg.getGroup(gId);
        assertEq(g.entryCount, 1);
        BracketGroups.Member[] memory members = bg.getMembers(gId);
        assertEq(members[0].addr, alice);
        assertEq(members[0].name, "Alice");
    }

    function test_importMember_idempotent() public {
        uint32 gId = bg.importGroup("grp", "Grp", 0, groupCreator);
        bg.importMember(gId, alice, "Alice");
        bg.importMember(gId, alice, "Alice"); // should not revert or double-count
        assertEq(bg.getGroup(gId).entryCount, 1);
    }

    function test_importMember_groupNotFound_reverts() public {
        vm.expectRevert(BracketGroups.GroupDoesNotExist.selector);
        bg.importMember(99, alice, "Alice");
    }

    function test_batchImportMembers_happy() public {
        uint32 gId = bg.importGroup("grp", "Grp", 0, groupCreator);

        address[] memory addrs = new address[](2);
        string[] memory names = new string[](2);
        addrs[0] = alice;
        addrs[1] = bob;
        names[0] = "Alice";
        names[1] = "Bob";

        bg.batchImportMembers(gId, addrs, names);

        assertEq(bg.getGroup(gId).entryCount, 2);
        assertTrue(bg.isMemberOf(gId, alice));
        assertTrue(bg.isMemberOf(gId, bob));
    }

    function test_batchImportMembers_idempotent() public {
        uint32 gId = bg.importGroup("grp", "Grp", 0, groupCreator);
        bg.importMember(gId, alice, "Alice");

        address[] memory addrs = new address[](2);
        string[] memory names = new string[](2);
        addrs[0] = alice; // already imported
        addrs[1] = bob;
        names[0] = "Alice";
        names[1] = "Bob";

        bg.batchImportMembers(gId, addrs, names);
        assertEq(bg.getGroup(gId).entryCount, 2);
    }

    // ════════════════════════════════════════════════════════════════════
    //  ETH funding
    // ════════════════════════════════════════════════════════════════════

    function test_fund_acceptsEth() public {
        bg.fund{value: 1 ether}();
        assertEq(address(bg).balance, 1 ether);
    }

    function test_receive_acceptsEth() public {
        (bool ok,) = address(bg).call{value: 0.5 ether}("");
        assertTrue(ok);
        assertEq(address(bg).balance, 0.5 ether);
    }
}
