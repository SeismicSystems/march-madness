// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

import {Test} from "forge-std/Test.sol";
import {MarchMadness} from "../../src/MarchMadness.sol";

/// @title NoContest tests — 28-day refund mechanism
contract NoContestTest is Test {
    MarchMadness mm;
    address alice = address(0xA11CE);
    address bob = address(0xB0B);
    uint256 constant ENTRY_FEE = 1 ether;
    uint256 constant DEADLINE = 1000;

    bytes8 constant RESULTS = bytes8(0xFFFFFFFFFFFFFFFF);

    function setUp() public {
        vm.warp(100);
        mm = new MarchMadness(ENTRY_FEE, DEADLINE, "IPFS");
        vm.deal(alice, 10 ether);
        vm.deal(bob, 10 ether);
    }

    function test_cannotCollectBeforeResultsPosted() public {
        _submitEntry(alice);
        vm.warp(DEADLINE + 29 days);

        vm.prank(alice);
        vm.expectRevert("Results not posted");
        mm.collectEntryFee();
    }

    function test_cannotCollectBeforeNoContestPeriod() public {
        _submitEntry(alice);
        vm.warp(DEADLINE + 1);
        mm.submitResults(RESULTS);

        vm.prank(alice);
        vm.expectRevert("No-contest period not reached");
        mm.collectEntryFee();
    }

    function test_cannotCollectIfAllBracketsScored() public {
        _submitEntry(alice);
        vm.warp(DEADLINE + 1);
        mm.submitResults(RESULTS);
        mm.scoreBracket(alice);

        vm.warp(mm.resultsPostedAt() + 28 days);
        vm.prank(alice);
        vm.expectRevert("All brackets scored, contest is valid");
        mm.collectEntryFee();
    }

    function test_canCollectAfterNoContestPeriodWhenNotAllScored() public {
        _submitEntry(alice);
        _submitEntry(bob);
        vm.warp(DEADLINE + 1);
        mm.submitResults(RESULTS);

        // Only score alice, not bob
        mm.scoreBracket(alice);

        vm.warp(mm.resultsPostedAt() + 28 days);

        uint256 aliceBefore = alice.balance;
        vm.prank(alice);
        mm.collectEntryFee();
        assertEq(alice.balance - aliceBefore, ENTRY_FEE);

        uint256 bobBefore = bob.balance;
        vm.prank(bob);
        mm.collectEntryFee();
        assertEq(bob.balance - bobBefore, ENTRY_FEE);
    }

    function test_cannotCollectEntryFeeTwice() public {
        _submitEntry(alice);
        _submitEntry(bob);
        vm.warp(DEADLINE + 1);
        mm.submitResults(RESULTS);
        mm.scoreBracket(alice); // only score alice, not bob

        vm.warp(mm.resultsPostedAt() + 28 days);

        vm.prank(alice);
        mm.collectEntryFee();

        vm.prank(alice);
        vm.expectRevert("Already collected");
        mm.collectEntryFee();
    }

    function test_nonSubmitterCannotCollectEntryFee() public {
        _submitEntry(alice);
        _submitEntry(bob);
        vm.warp(DEADLINE + 1);
        mm.submitResults(RESULTS);
        mm.scoreBracket(alice);

        vm.warp(mm.resultsPostedAt() + 28 days);

        address nobody = address(0x999);
        vm.prank(nobody);
        vm.expectRevert("No bracket submitted");
        mm.collectEntryFee();
    }

    function _submitEntry(address account) internal {
        bytes8 bracket = bytes8(0xFFFFFFFFFFFFFFFF);
        vm.prank(account);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(bracket));
    }
}
