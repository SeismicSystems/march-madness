// SPDX-License-Identifier: MIT
pragma solidity ^0.8.30;

import {Test} from "forge-std/Test.sol";
import {MarchMadness} from "../../src/MarchMadness.sol";

/// @title Sentinel bit validation tests
contract SentinelTest is Test {
    MarchMadness mm;
    address alice = address(0xA11CE);
    uint256 constant ENTRY_FEE = 1 ether;
    uint256 constant DEADLINE = 1000;

    function setUp() public {
        vm.warp(100);
        mm = new MarchMadness(2026, ENTRY_FEE, DEADLINE);
        vm.deal(alice, 10 ether);
    }

    function test_rejectsBracketWithoutSentinel() public {
        // No sentinel bit (MSB is 0)
        bytes8 bad = bytes8(0x0000000000000001);
        vm.prank(alice);
        vm.expectRevert(MarchMadness.InvalidSentinelByte.selector);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(bad));
    }

    function test_rejectsBracketWithMSBClear() public {
        // MSB clear, other bits set
        bytes8 bad = bytes8(0x7FFFFFFFFFFFFFFF);
        vm.prank(alice);
        vm.expectRevert(MarchMadness.InvalidSentinelByte.selector);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(bad));
    }

    function test_acceptsBracketWithMSBSet() public {
        bytes8 good = bytes8(0xFFFFFFFFFFFFFFFF);
        vm.prank(alice);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(good));
        assertEq(mm.numEntries(), 1);
    }

    function test_acceptsMinimalBracketWithMSBSet() public {
        // Only MSB set, all game bits 0
        bytes8 good = bytes8(0x8000000000000000);
        vm.prank(alice);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(good));
        assertEq(mm.numEntries(), 1);
    }

    function test_rejectsResultsWithoutSentinel() public {
        // Submit a bracket first
        bytes8 bracket = bytes8(0xFFFFFFFFFFFFFFFF);
        vm.prank(alice);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(bracket));

        vm.warp(DEADLINE + 1);
        bytes8 badResults = bytes8(0x0000000000000001); // MSB not set
        vm.expectRevert(MarchMadness.InvalidSentinelByte.selector);
        mm.submitResults(badResults);
    }

    function test_updateBracketRejectsBadSentinel() public {
        bytes8 good = bytes8(0xFFFFFFFFFFFFFFFF);
        vm.prank(alice);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(good));

        bytes8 bad = bytes8(0x0AAAAAAAAAAAAAAA); // MSB not set
        vm.prank(alice);
        vm.expectRevert(MarchMadness.InvalidSentinelByte.selector);
        mm.updateBracket(sbytes8(bad));
    }

    function test_sentinelDistinguishesUninitialized() public {
        // Uninitialized mapping entry should return bytes8(0), which has MSB clear
        vm.warp(DEADLINE + 1); // after deadline so we can read
        bytes8 b = mm.getBracket(alice);
        assertEq(b, bytes8(0));
        assertFalse(b[0] & 0x80 != 0);
    }
}
