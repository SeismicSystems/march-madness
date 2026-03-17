// SPDX-License-Identifier: MIT
pragma solidity ^0.8.30;

import {Test} from "forge-std/Test.sol";
import {MarchMadness} from "../../src/MarchMadness.sol";
import {IMarchMadness} from "../../src/IMarchMadness.sol";

/// @title Tests for the 90-day results submission deadline
contract ResultsDeadlineTest is Test {
    MarchMadness mm;
    address alice = address(0xA11CE);
    uint256 constant ENTRY_FEE = 1 ether;
    uint256 constant DEADLINE = 1000;

    bytes8 constant BRACKET = bytes8(0xFFFFFFFFFFFFFFFF);
    bytes8 constant RESULTS = bytes8(0xFFFFFFFFFFFFFFFF);

    function setUp() public {
        vm.warp(100);
        mm = new MarchMadness(2026, ENTRY_FEE, DEADLINE);
        vm.deal(alice, 10 ether);

        vm.prank(alice);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(BRACKET));
    }

    function test_canSubmitResultsJustBeforeDeadline() public {
        vm.warp(DEADLINE + 90 days);
        mm.submitResults(RESULTS);
        assertEq(mm.results(), RESULTS);
    }

    function test_cannotSubmitResultsAfter90Days() public {
        vm.warp(DEADLINE + 90 days + 1);
        vm.expectRevert(IMarchMadness.ResultsSubmissionWindowClosed.selector);
        mm.submitResults(RESULTS);
    }

    function test_canSubmitResultsDayAfterDeadline() public {
        vm.warp(DEADLINE + 1 days);
        mm.submitResults(RESULTS);
        assertEq(mm.results(), RESULTS);
    }

    function test_collectEntryFeeAfterResultsWindowExpires() public {
        // Results window has passed with no results posted — users can reclaim
        vm.warp(DEADLINE + 90 days + 1);
        vm.prank(alice);
        mm.collectEntryFee();
        assertEq(alice.balance, 10 ether); // got refund back
    }

    function test_cannotCollectEntryFeeBeforeWindowExpires() public {
        vm.warp(DEADLINE + 30 days);
        vm.prank(alice);
        vm.expectRevert(IMarchMadness.ResultsWindowStillOpen.selector);
        mm.collectEntryFee();
    }

    function test_cannotCollectEntryFeeIfResultsPosted() public {
        vm.warp(DEADLINE + 1);
        mm.submitResults(RESULTS);

        vm.warp(DEADLINE + 90 days + 1);
        vm.prank(alice);
        vm.expectRevert(IMarchMadness.ResultsAlreadyPosted.selector);
        mm.collectEntryFee();
    }

    function test_cannotCollectEntryFeeTwice() public {
        vm.warp(DEADLINE + 90 days + 1);
        vm.prank(alice);
        mm.collectEntryFee();

        vm.prank(alice);
        vm.expectRevert(IMarchMadness.AlreadyCollected.selector);
        mm.collectEntryFee();
    }
}
