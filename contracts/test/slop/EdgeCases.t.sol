// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

import {Test} from "forge-std/Test.sol";
import {MarchMadness} from "../../src/MarchMadness.sol";

/// @title Edge case tests
contract EdgeCasesTest is Test {
    MarchMadness mm;
    address owner;
    address alice = address(0xA11CE);
    address bob = address(0xB0B);
    uint256 constant ENTRY_FEE = 1 ether;
    uint256 constant DEADLINE = 1000;

    bytes8 constant BRACKET = bytes8(0xFFFFFFFFFFFFFFFF);
    bytes8 constant RESULTS = bytes8(0xFFFFFFFFFFFFFFFF);

    function setUp() public {
        vm.warp(100);
        owner = address(this);
        mm = new MarchMadness(2026, ENTRY_FEE, DEADLINE);
        vm.deal(alice, 10 ether);
        vm.deal(bob, 10 ether);
    }

    // ── Duplicate submission ───────────────────────────────────────────────

    function test_cannotSubmitTwice() public {
        vm.prank(alice);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(BRACKET));

        vm.prank(alice);
        vm.expectRevert("Already submitted");
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(BRACKET));
    }

    // ── Update without submission ──────────────────────────────────────────

    function test_cannotUpdateWithoutSubmission() public {
        vm.prank(alice);
        vm.expectRevert("No bracket submitted");
        mm.updateBracket(sbytes8(BRACKET));
    }

    // ── Update after deadline ──────────────────────────────────────────────

    function test_cannotUpdateAfterDeadline() public {
        vm.prank(alice);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(BRACKET));

        vm.warp(DEADLINE + 1);
        vm.prank(alice);
        vm.expectRevert("Submission deadline passed");
        mm.updateBracket(sbytes8(BRACKET));
    }

    // ── Submit after deadline ──────────────────────────────────────────────

    function test_cannotSubmitAfterDeadline() public {
        vm.warp(DEADLINE + 1);
        vm.prank(alice);
        vm.expectRevert("Submission deadline passed");
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(BRACKET));
    }

    // ── Score before results ───────────────────────────────────────────────

    function test_cannotScoreBeforeResults() public {
        vm.prank(alice);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(BRACKET));

        vm.expectRevert("Results not posted");
        mm.scoreBracket(alice);
    }

    // ── Collect before scoring window closed ───────────────────────────────

    function test_cannotCollectWinningsBeforeScoringWindowClosed() public {
        vm.prank(alice);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(BRACKET));
        vm.prank(bob);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(BRACKET));

        vm.warp(DEADLINE + 1);
        mm.submitResults(RESULTS);
        mm.scoreBracket(alice);
        mm.scoreBracket(bob);

        vm.prank(alice);
        vm.expectRevert("Scoring window still open");
        mm.collectWinnings();
    }

    // ── Results posting ────────────────────────────────────────────────────

    function test_onlyOwnerCanPostResults() public {
        vm.warp(DEADLINE + 1);
        vm.prank(alice);
        vm.expectRevert("Only owner");
        mm.submitResults(RESULTS);
    }

    function test_cannotPostResultsBeforeDeadline() public {
        vm.expectRevert("Submission phase not over");
        mm.submitResults(RESULTS);
    }

    function test_cannotPostResultsTwice() public {
        vm.warp(DEADLINE + 1);
        mm.submitResults(RESULTS);

        vm.expectRevert("Results already posted");
        mm.submitResults(RESULTS);
    }

    // ── Update bracket works correctly ─────────────────────────────────────

    function test_updateBracketChangesStoredBracket() public {
        vm.prank(alice);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(BRACKET));

        bytes8 newBracket = bytes8(0x8000000000000000);
        vm.prank(alice);
        mm.updateBracket(sbytes8(newBracket));

        vm.prank(alice);
        bytes8 stored = mm.getBracket(alice);
        assertEq(stored, newBracket);
    }

    // ── Entry count tracking ───────────────────────────────────────────────

    function test_updateDoesNotIncrementEntryCount() public {
        vm.prank(alice);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(BRACKET));
        assertEq(mm.numEntries(), 1);

        bytes8 newBracket = bytes8(0x8000000000000000);
        vm.prank(alice);
        mm.updateBracket(sbytes8(newBracket));
        assertEq(mm.numEntries(), 1); // still 1
    }

    // ── Tag storage ────────────────────────────────────────────────────────

    function test_tagIsStoredViaSetTag() public {
        vm.prank(alice);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(BRACKET));
        vm.prank(alice);
        mm.setTag("alice-tag");
        assertEq(mm.getTag(alice), "alice-tag");
    }

    function test_noTagByDefault() public {
        vm.prank(alice);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(BRACKET));
        assertEq(bytes(mm.getTag(alice)).length, 0);
    }

    // ── View functions ─────────────────────────────────────────────────────

    function test_getEntryCount() public {
        assertEq(mm.getEntryCount(), 0);
        vm.prank(alice);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(BRACKET));
        assertEq(mm.getEntryCount(), 1);
    }

    function test_getScoreAndIsScored() public {
        vm.prank(alice);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(BRACKET));

        assertEq(mm.getScore(alice), 0);
        assertFalse(mm.getIsScored(alice));

        vm.warp(DEADLINE + 1);
        mm.submitResults(RESULTS);
        mm.scoreBracket(alice);

        assertTrue(mm.getIsScored(alice));
        assertEq(mm.getScore(alice), 192); // perfect score
    }

    // ── Wrong entry fee ────────────────────────────────────────────────────

    function test_rejectsTooLittleFee() public {
        vm.prank(alice);
        vm.expectRevert("Incorrect entry fee");
        mm.submitBracket{value: 0.5 ether}(sbytes8(BRACKET));
    }

    function test_rejectsTooMuchFee() public {
        vm.prank(alice);
        vm.expectRevert("Incorrect entry fee");
        mm.submitBracket{value: 2 ether}(sbytes8(BRACKET));
    }
}
