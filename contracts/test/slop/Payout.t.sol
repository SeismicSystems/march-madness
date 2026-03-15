// SPDX-License-Identifier: MIT
pragma solidity ^0.8.30;

import {Test} from "forge-std/Test.sol";
import {MarchMadness} from "../../src/MarchMadness.sol";
import {IMarchMadness} from "../../src/IMarchMadness.sol";

/// @title Payout tests for collectWinnings with single and multiple winners
contract PayoutTest is Test {
    MarchMadness mm;
    address alice = address(0xA11CE);
    address bob = address(0xB0B);
    address charlie = address(0xC4A2);
    uint256 constant ENTRY_FEE = 1 ether;
    uint256 constant DEADLINE = 1000;

    bytes8 constant RESULTS = bytes8(0xFFFFFFFFFFFFFFFF);

    function setUp() public {
        vm.warp(100);
        mm = new MarchMadness(2026, ENTRY_FEE, DEADLINE);
        vm.deal(alice, 10 ether);
        vm.deal(bob, 10 ether);
        vm.deal(charlie, 10 ether);
    }

    function test_singleWinnerGetsFullPool() public {
        _submitEntry(alice, 0xFFFFFFFFFFFFFFFF); // perfect bracket
        _submitEntry(bob, 0x8000000000000000); // bad bracket

        vm.warp(DEADLINE + 1);
        mm.submitResults(RESULTS);

        mm.scoreBracket(alice);
        mm.scoreBracket(bob);

        // Warp past scoring window
        vm.warp(mm.resultsPostedAt() + mm.SCORING_DURATION());

        uint256 aliceBefore = alice.balance;
        vm.prank(alice);
        mm.collectWinnings();
        assertEq(alice.balance - aliceBefore, 2 ether); // full pool
    }

    function test_twoWinnersSplitPool() public {
        _submitEntry(alice, 0xFFFFFFFFFFFFFFFF);
        _submitEntry(bob, 0xFFFFFFFFFFFFFFFF); // same bracket, ties
        _submitEntry(charlie, 0x8000000000000000); // loser

        vm.warp(DEADLINE + 1);
        mm.submitResults(RESULTS);

        mm.scoreBracket(alice);
        mm.scoreBracket(bob);
        mm.scoreBracket(charlie);

        // Warp past scoring window
        vm.warp(mm.resultsPostedAt() + mm.SCORING_DURATION());

        uint256 expectedWinnings = (3 * ENTRY_FEE) / 2;

        uint256 aliceBefore = alice.balance;
        vm.prank(alice);
        mm.collectWinnings();
        assertEq(alice.balance - aliceBefore, expectedWinnings);

        uint256 bobBefore = bob.balance;
        vm.prank(bob);
        mm.collectWinnings();
        assertEq(bob.balance - bobBefore, expectedWinnings);
    }

    function test_allWinnersSameScoreSplitEvenly() public {
        // All same bracket → all same score → all winners
        _submitEntry(alice, 0xFFFFFFFFFFFFFFFF);
        _submitEntry(bob, 0xFFFFFFFFFFFFFFFF);
        _submitEntry(charlie, 0xFFFFFFFFFFFFFFFF);

        vm.warp(DEADLINE + 1);
        mm.submitResults(RESULTS);

        mm.scoreBracket(alice);
        mm.scoreBracket(bob);
        mm.scoreBracket(charlie);

        // Warp past scoring window
        vm.warp(mm.resultsPostedAt() + mm.SCORING_DURATION());

        assertEq(mm.numWinners(), 3);
        uint256 expectedWinnings = ENTRY_FEE; // 3 ETH / 3 winners

        uint256 aliceBefore = alice.balance;
        vm.prank(alice);
        mm.collectWinnings();
        assertEq(alice.balance - aliceBefore, expectedWinnings);
    }

    function test_cannotCollectWinningsTwice() public {
        _submitEntry(alice, 0xFFFFFFFFFFFFFFFF);

        vm.warp(DEADLINE + 1);
        mm.submitResults(RESULTS);
        mm.scoreBracket(alice);

        // Warp past scoring window
        vm.warp(mm.resultsPostedAt() + mm.SCORING_DURATION());

        vm.prank(alice);
        mm.collectWinnings();

        vm.prank(alice);
        vm.expectRevert(IMarchMadness.AlreadyCollected.selector);
        mm.collectWinnings();
    }

    function test_loserCannotCollectWinnings() public {
        _submitEntry(alice, 0xFFFFFFFFFFFFFFFF);
        _submitEntry(bob, 0x8000000000000000);

        vm.warp(DEADLINE + 1);
        mm.submitResults(RESULTS);
        mm.scoreBracket(alice);
        mm.scoreBracket(bob);

        // Warp past scoring window
        vm.warp(mm.resultsPostedAt() + mm.SCORING_DURATION());

        vm.prank(bob);
        vm.expectRevert(IMarchMadness.NotAWinner.selector);
        mm.collectWinnings();
    }

    function _submitEntry(address account, uint64 gameBits) internal {
        bytes8 bracket = bytes8(gameBits);
        vm.prank(account);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(bracket));
    }
}
