// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

import {Test} from "forge-std/Test.sol";
import {MarchMadness} from "../../src/MarchMadness.sol";

/// @title Sentinel byte validation tests
contract SentinelTest is Test {
    MarchMadness mm;
    address alice = address(0xA11CE);
    uint256 constant ENTRY_FEE = 1 ether;
    uint256 constant DEADLINE = 1000;

    function setUp() public {
        vm.warp(100);
        mm = new MarchMadness(ENTRY_FEE, DEADLINE, "IPFS");
        vm.deal(alice, 10 ether);
    }

    function test_rejectsBracketWithoutSentinel() public {
        // No sentinel byte (last byte is 0x00)
        bytes32 bad = bytes32(uint256(0xFFFFFFFFFFFFFFFF) << 192);
        vm.prank(alice);
        vm.expectRevert("Invalid sentinel byte");
        mm.submitBracket{value: ENTRY_FEE}(sbytes32(bad), "");
    }

    function test_rejectsBracketWithWrongSentinel() public {
        // Wrong sentinel byte (0x02 instead of 0x01)
        bytes32 bad = bytes32(uint256(0xFFFFFFFFFFFFFFFF) << 192 | 0x02);
        vm.prank(alice);
        vm.expectRevert("Invalid sentinel byte");
        mm.submitBracket{value: ENTRY_FEE}(sbytes32(bad), "");
    }

    function test_acceptsBracketWithCorrectSentinel() public {
        bytes32 good = bytes32(uint256(0xFFFFFFFFFFFFFFFF) << 192 | 0x01);
        vm.prank(alice);
        mm.submitBracket{value: ENTRY_FEE}(sbytes32(good), "");
        assertEq(mm.numEntries(), 1);
    }

    function test_rejectsResultsWithoutSentinel() public {
        // Submit a bracket first
        bytes32 bracket = bytes32(uint256(0xFFFFFFFFFFFFFFFF) << 192 | 0x01);
        vm.prank(alice);
        mm.submitBracket{value: ENTRY_FEE}(sbytes32(bracket), "");

        vm.warp(DEADLINE + 1);
        bytes32 badResults = bytes32(uint256(0x8000000000000000) << 192); // no sentinel
        vm.expectRevert("Invalid sentinel byte");
        mm.submitResults(badResults);
    }

    function test_updateBracketRejectsBadSentinel() public {
        bytes32 good = bytes32(uint256(0xFFFFFFFFFFFFFFFF) << 192 | 0x01);
        vm.prank(alice);
        mm.submitBracket{value: ENTRY_FEE}(sbytes32(good), "");

        bytes32 bad = bytes32(uint256(0xAAAAAAAAAAAAAAAA) << 192); // no sentinel
        vm.prank(alice);
        vm.expectRevert("Invalid sentinel byte");
        mm.updateBracket(sbytes32(bad));
    }

    function test_sentinelDistinguishesUninitialized() public {
        // Uninitialized mapping entry should return bytes32(0), which has no sentinel
        vm.warp(DEADLINE + 1); // after deadline so we can read
        bytes32 b = mm.getBracket(alice);
        assertEq(b, bytes32(0));
        assertFalse(b[31] == bytes1(0x01));
    }
}
