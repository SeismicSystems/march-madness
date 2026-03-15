// SPDX-License-Identifier: MIT
pragma solidity ^0.8.30;

import {Test} from "forge-std/Test.sol";
import {ByteBracket} from "../../src/ByteBracket.sol";

/// @title ByteBracket tests — ported from jimpo's ByteBracket.js
/// @dev We expose the library functions via a wrapper since libraries with internal functions
///      cannot be called directly in tests.
contract ByteBracketWrapper {
    function getBracketScore(bytes8 bracket, bytes8 results, uint64 filter) external pure returns (uint8) {
        return ByteBracket.getBracketScore(bracket, results, filter);
    }

    function getScoringMask(bytes8 results) external pure returns (uint64) {
        return ByteBracket.getScoringMask(results);
    }
}

contract ByteBracketTest is Test {
    ByteBracketWrapper wrapper;

    function setUp() public {
        wrapper = new ByteBracketWrapper();
    }

    // ── getBracketScore tests (from jimpo's ByteBracket.js) ────────────────

    /// @dev jimpo test: "correctly calculates bracket scores"
    ///      results = 0xFFFFFFFFFFFFFFFF (all higher seeds win)
    function test_getBracketScore_allCorrect() public view {
        bytes8 results = bytes8(0xFFFFFFFFFFFFFFFF);
        uint64 filter = wrapper.getScoringMask(results);

        // Bracket: perfect bracket — all 1s match results
        bytes8 bracket0 = bytes8(0xFFFFFFFFFFFFFFFF);
        assertEq(wrapper.getBracketScore(bracket0, results, filter), 192);
    }

    function test_getBracketScore_partialMatch() public view {
        bytes8 results = bytes8(0xFFFFFFFFFFFFFFFF);
        uint64 filter = wrapper.getScoringMask(results);

        // Bracket: 0x80000000FFFFFFFF — only bottom 32 bits correct
        bytes8 bracket1 = bytes8(0x80000000FFFFFFFF);
        assertEq(wrapper.getBracketScore(bracket1, results, filter), 32);
    }

    function test_getBracketScore_halfRegionsCorrect() public view {
        bytes8 results = bytes8(0xFFFFFFFFFFFFFFFF);
        uint64 filter = wrapper.getScoringMask(results);

        // Bracket: 0xFFFF0000FFFFFFFF — top 16 bits wrong in first half, bottom 32 correct
        bytes8 bracket2 = bytes8(0xFFFF0000FFFFFFFF);
        assertEq(wrapper.getBracketScore(bracket2, results, filter), 32);
    }

    function test_getBracketScore_alternatingBitsOdd() public view {
        bytes8 results = bytes8(0xFFFFFFFFFFFFFFFF);
        uint64 filter = wrapper.getScoringMask(results);

        // Bracket: 0xFFFF5555FFFFFFFF — alternating bits in first-round region, rest correct
        bytes8 bracket3 = bytes8(0xFFFF5555FFFFFFFF);
        assertEq(wrapper.getBracketScore(bracket3, results, filter), 192 - 2 * 8);
    }

    function test_getBracketScore_alternatingBitsEven() public view {
        bytes8 results = bytes8(0xFFFFFFFFFFFFFFFF);
        uint64 filter = wrapper.getScoringMask(results);

        // Bracket: 0xFFFFaaaaFFFFFFFF
        bytes8 bracket4 = bytes8(0xFFFFaaaaFFFFFFFF);
        assertEq(wrapper.getBracketScore(bracket4, results, filter), 32 + 2 * 8);
    }

    // ── getScoringMask tests (from jimpo's ByteBracket.js) ─────────────────

    function test_getScoringMask_allOnes() public view {
        bytes8 results = bytes8(0xFFFFFFFFFFFFFFFF);
        uint64 mask = wrapper.getScoringMask(results);
        assertEq(mask, 0x1555555555555555);
    }

    function test_getScoringMask_msb_only() public view {
        bytes8 results = bytes8(0x8000000000000000);
        uint64 mask = wrapper.getScoringMask(results);
        assertEq(mask, 0x2aaaaaaaaaaaaaaa);
    }

    function test_getScoringMask_halfOnes() public view {
        bytes8 results = bytes8(0xFFFF000000000000);
        uint64 mask = wrapper.getScoringMask(results);
        assertEq(mask, 0x15555555aaaaaaaa);
    }
}
