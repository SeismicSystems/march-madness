// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

import {Test} from "forge-std/Test.sol";
import {MarchMadness} from "../../src/MarchMadness.sol";

/// @title MarchMadness tests — ported from jimpo's MarchMadness.js
/// @dev Adapted for Seismic's direct shielded submission (no commit-reveal).
contract MarchMadnessJimpoTest is Test {
    MarchMadness mm;

    address owner = address(this);
    address alice = address(0xA11CE);
    address bob = address(0xB0B);
    address charlie = address(0xC4A2);

    uint256 constant ENTRY_FEE = 1 ether;
    uint256 constant DEADLINE = 1000;

    // Results: 0x8000000000000000 — only MSB set
    bytes8 constant RESULTS = bytes8(0x8000000000000000);

    function setUp() public {
        vm.warp(100); // well before deadline
        mm = new MarchMadness(2026, ENTRY_FEE, DEADLINE);
        vm.deal(alice, 10 ether);
        vm.deal(bob, 10 ether);
        vm.deal(charlie, 10 ether);
    }

    // ── Constructor tests ──────────────────────────────────────────────────

    function test_constructor_setsInitialState() public view {
        assertEq(mm.entryFee(), ENTRY_FEE);
        assertEq(mm.submissionDeadline(), DEADLINE);
        assertEq(mm.owner(), owner);
    }

    // ── submitBracket tests (ported from jimpo's #submitBracket) ───────────

    function test_submitBracket_acceptsWithEntryFee() public {
        bytes8 bracket = bytes8(0xFFFFFFFFFFFFFFFF);
        vm.prank(alice);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(bracket));
        assertEq(mm.numEntries(), 1);
    }

    function test_submitBracket_rejectsWithoutEntryFee() public {
        bytes8 bracket = bytes8(0xFFFFFFFFFFFFFFFF);
        vm.prank(alice);
        vm.expectRevert("Incorrect entry fee");
        mm.submitBracket{value: 0}(sbytes8(bracket));
    }

    function test_submitBracket_rejectsAfterDeadline() public {
        vm.warp(DEADLINE + 1);
        bytes8 bracket = bytes8(0xFFFFFFFFFFFFFFFF);
        vm.prank(alice);
        vm.expectRevert("Submission deadline passed");
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(bracket));
    }

    function test_submitBracket_rejectsResubmission() public {
        bytes8 bracket = bytes8(0xFFFFFFFFFFFFFFFF);
        vm.prank(alice);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(bracket));

        vm.prank(alice);
        vm.expectRevert("Already submitted");
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(bracket));
    }

    // ── scoreBracket tests (ported from jimpo's #scoreBracket) ─────────────

    function test_scoreBracket_rejectsBeforeResults() public {
        _submitEntry(alice, 0xC000000000000000);
        vm.warp(DEADLINE + 1);

        vm.expectRevert("Results not posted");
        mm.scoreBracket(alice);
    }

    function test_scoreBracket_assignsScores() public {
        // results = 0x8000000000000000 — only MSB set, rest 0
        // bracket 0xC000000000000000: bits 63 & 62 set; bit 63 matches results (MSB match),
        //   bit 62 doesn't match (result has 0, bracket has 1)
        // bracket 0xF000000000000000: bits 63-60 set
        _submitEntry(alice, 0xC000000000000000);
        _submitEntry(bob, 0xF000000000000000);
        _submitEntry(charlie, 0xC000000000000000);

        vm.warp(DEADLINE + 1);
        mm.submitResults(RESULTS);

        mm.scoreBracket(alice);
        mm.scoreBracket(bob);
        mm.scoreBracket(charlie);

        // jimpo expected: entries[0] = 32*5 = 160, entries[1] = 32*4 = 128, entries[2] = 160
        assertEq(mm.scores(alice), 160);
        assertEq(mm.scores(bob), 128);
        assertEq(mm.scores(charlie), 160);
    }

    function test_scoreBracket_tracksWinners() public {
        _submitEntry(alice, 0xC000000000000000);
        _submitEntry(bob, 0xF000000000000000);
        _submitEntry(charlie, 0xC000000000000000);

        vm.warp(DEADLINE + 1);
        mm.submitResults(RESULTS);

        mm.scoreBracket(alice);
        assertEq(mm.winningScore(), 160);
        assertEq(mm.numWinners(), 1);

        mm.scoreBracket(bob);
        // bob scores lower, numWinners stays 1
        assertEq(mm.numWinners(), 1);

        mm.scoreBracket(charlie);
        // charlie ties alice at 160
        assertEq(mm.numWinners(), 2);
    }

    // ── collectWinnings tests (ported from jimpo's #collectWinnings) ───────

    function test_collectWinnings_rejectsBeforeScoringWindowClosed() public {
        _submitEntry(alice, 0xC000000000000000);
        _submitEntry(bob, 0xF000000000000000);

        vm.warp(DEADLINE + 1);
        mm.submitResults(RESULTS);
        mm.scoreBracket(alice);
        mm.scoreBracket(bob);

        vm.prank(alice);
        vm.expectRevert("Scoring window still open");
        mm.collectWinnings();
    }

    function test_collectWinnings_splitsAmongWinners() public {
        _submitEntry(alice, 0xC000000000000000);
        _submitEntry(bob, 0xF000000000000000);
        _submitEntry(charlie, 0xC000000000000000);

        vm.warp(DEADLINE + 1);
        mm.submitResults(RESULTS);

        mm.scoreBracket(alice);
        mm.scoreBracket(bob);
        mm.scoreBracket(charlie);

        // Warp past scoring window
        vm.warp(mm.resultsPostedAt() + mm.SCORING_DURATION());

        uint256 expectedWinnings = (3 * ENTRY_FEE) / 2; // 3 entries, 2 winners

        uint256 aliceBefore = alice.balance;
        vm.prank(alice);
        mm.collectWinnings();
        assertEq(alice.balance - aliceBefore, expectedWinnings);

        uint256 charlieBefore = charlie.balance;
        vm.prank(charlie);
        mm.collectWinnings();
        assertEq(charlie.balance - charlieBefore, expectedWinnings);

        // bob is not a winner
        vm.prank(bob);
        vm.expectRevert("Not a winner");
        mm.collectWinnings();
    }

    // ── Helpers ────────────────────────────────────────────────────────────

    function _makeBracket(uint64 gameBits) internal pure returns (bytes8) {
        return bytes8(gameBits);
    }

    function _submitEntry(address account, uint64 gameBits) internal {
        bytes8 bracket = _makeBracket(gameBits);
        vm.prank(account);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(bracket));
    }
}
