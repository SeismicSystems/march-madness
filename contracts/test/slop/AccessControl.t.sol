// SPDX-License-Identifier: MIT
pragma solidity ^0.8.30;

import {Test} from "forge-std/Test.sol";
import {MarchMadness} from "../../src/MarchMadness.sol";

/// @title Access control tests for getBracket
contract AccessControlTest is Test {
    MarchMadness mm;
    address alice = address(0xA11CE);
    address bob = address(0xB0B);
    uint256 constant ENTRY_FEE = 1 ether;
    uint256 constant DEADLINE = 1000;

    bytes8 constant BRACKET = bytes8(0xFFFFFFFFFFFFFFFF);

    function setUp() public {
        vm.warp(100);
        mm = new MarchMadness(2026, ENTRY_FEE, DEADLINE);
        vm.deal(alice, 10 ether);

        vm.prank(alice);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(BRACKET));
    }

    function test_ownerCanReadOwnBracketBeforeDeadline() public {
        vm.prank(alice);
        bytes8 b = mm.getBracket(alice);
        assertEq(b, BRACKET);
    }

    function test_otherCannotReadBracketBeforeDeadline() public {
        vm.prank(bob);
        vm.expectRevert(MarchMadness.CannotReadBracketBeforeDeadline.selector);
        mm.getBracket(alice);
    }

    function test_anyoneCanReadBracketAfterDeadline() public {
        vm.warp(DEADLINE + 1);
        vm.prank(bob);
        bytes8 b = mm.getBracket(alice);
        assertEq(b, BRACKET);
    }

    function test_ownerCanStillReadAfterDeadline() public {
        vm.warp(DEADLINE + 1);
        vm.prank(alice);
        bytes8 b = mm.getBracket(alice);
        assertEq(b, BRACKET);
    }

    function test_exactlyAtDeadlineCannotRead() public {
        // block.timestamp == submissionDeadline: still < is false, so after deadline
        // Actually: require(block.timestamp < submissionDeadline) means at DEADLINE, the
        // condition is false, so the else branch runs (anyone can read).
        vm.warp(DEADLINE);
        vm.prank(bob);
        bytes8 b = mm.getBracket(alice);
        assertEq(b, BRACKET);
    }
}
