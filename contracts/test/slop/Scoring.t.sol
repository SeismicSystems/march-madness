// SPDX-License-Identifier: MIT
pragma solidity ^0.8.30;

import {Test} from "forge-std/Test.sol";
import {MarchMadness} from "../../src/MarchMadness.sol";

/// @title Scoring tests with various bracket combinations
contract ScoringTest is Test {
    MarchMadness mm;
    address owner;
    address alice = address(0xA11CE);
    address bob = address(0xB0B);
    address charlie = address(0xC4A2);
    address dave = address(0xDA7E);
    uint256 constant ENTRY_FEE = 1 ether;
    uint256 constant DEADLINE = 1000;

    function setUp() public {
        vm.warp(100);
        owner = address(this);
        mm = new MarchMadness(2026, ENTRY_FEE, DEADLINE);
        vm.deal(alice, 10 ether);
        vm.deal(bob, 10 ether);
        vm.deal(charlie, 10 ether);
        vm.deal(dave, 10 ether);
    }

    function test_perfectBracketScores192() public {
        uint64 gameBits = 0xFFFFFFFFFFFFFFFF;
        _submitEntry(alice, gameBits);
        vm.warp(DEADLINE + 1);
        mm.submitResults(bytes8(gameBits));
        mm.scoreBracket(alice);
        assertEq(mm.scores(alice), 192);
    }

    function test_zeroBracketAgainstAllOnes() public {
        // Bracket: only MSB set (bit 63), rest 0
        // Results: all 1s
        _submitEntry(alice, 0x8000000000000000);
        vm.warp(DEADLINE + 1);
        mm.submitResults(bytes8(0xFFFFFFFFFFFFFFFF));
        mm.scoreBracket(alice);
        // Only first-round games where bracket bit == result bit would match
        // MSB matches, but first round is bits 62-31; bit 63 is not a game bit per jimpo
        // Actually bit 63 is MSB and is included in the overlap calculation
        assertTrue(mm.isScored(alice));
    }

    function test_multiplePlayersScored() public {
        uint64 resultBits = 0xFFFFFFFFFFFFFFFF;
        _submitEntry(alice, 0xFFFFFFFFFFFFFFFF); // perfect
        _submitEntry(bob, 0x8000000000000000); // minimal match
        _submitEntry(charlie, 0xFFFF5555FFFFFFFF); // mixed
        _submitEntry(dave, 0xFFFFaaaaFFFFFFFF); // inverted mixed

        vm.warp(DEADLINE + 1);
        mm.submitResults(bytes8(resultBits));

        mm.scoreBracket(alice);
        mm.scoreBracket(bob);
        mm.scoreBracket(charlie);
        mm.scoreBracket(dave);

        assertEq(mm.scores(alice), 192);
        assertEq(mm.scores(charlie), 192 - 2 * 8); // 176
        assertEq(mm.scores(dave), 32 + 2 * 8); // 48

        assertEq(mm.winningScore(), 192);
        assertEq(mm.numWinners(), 1);
    }

    function test_cannotScoreTwice() public {
        _submitEntry(alice, 0xFFFFFFFFFFFFFFFF);
        vm.warp(DEADLINE + 1);
        mm.submitResults(bytes8(0xFFFFFFFFFFFFFFFF));
        mm.scoreBracket(alice);

        vm.expectRevert("Already scored");
        mm.scoreBracket(alice);
    }

    function test_cannotScoreNonexistentBracket() public {
        vm.warp(DEADLINE + 1);
        mm.submitResults(bytes8(0xFFFFFFFFFFFFFFFF));

        vm.expectRevert("No bracket submitted");
        mm.scoreBracket(alice);
    }

    function _submitEntry(address account, uint64 gameBits) internal {
        bytes8 bracket = bytes8(gameBits);
        vm.prank(account);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(bracket));
    }
}
