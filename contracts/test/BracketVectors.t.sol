// SPDX-License-Identifier: MIT
pragma solidity ^0.8.30;

import {Test} from "forge-std/Test.sol";
import {ByteBracket} from "../src/ByteBracket.sol";
import {MarchMadness} from "../src/MarchMadness.sol";
import {IMarchMadness} from "../src/IMarchMadness.sol";

/// @title ByteBracket library wrapper for testing internal functions
contract ByteBracketWrapper {
    function getBracketScore(bytes8 bracket, bytes8 results, uint64 filter) external pure returns (uint8) {
        return ByteBracket.getBracketScore(bracket, results, filter);
    }

    function getScoringMask(bytes8 results) external pure returns (uint64) {
        return ByteBracket.getScoringMask(results);
    }
}

/// @title Golden Vector Tests — cross-language consistency with data/test-vectors/bracket-vectors.json
/// @notice These tests verify that Solidity scoring matches the golden test vectors shared with
///         TypeScript and Rust. The contract is where real payouts happen, so these are the most
///         critical tests in the suite.
contract BracketVectorsTest is Test {
    ByteBracketWrapper wrapper;

    function setUp() public {
        wrapper = new ByteBracketWrapper();
    }

    // ── Bracket hex constants from golden vectors ───────────────────────────

    bytes8 constant ALL_CHALK = bytes8(0xFFFFFFFFFFFFFFFF);
    bytes8 constant ALL_UPSETS = bytes8(0x8000000000000000);
    bytes8 constant MOSTLY_CHALK_FEW_UPSETS = bytes8(0xF7F7F7F7DFFFFFFF);
    bytes8 constant CINDERELLA_RUN = bytes8(0xBFFFFFFFBFFFBFBA);
    bytes8 constant ALTERNATING_PICKS = bytes8(0xD555555555555555);
    bytes8 constant SPLIT_REGIONS = bytes8(0xFFFF80007F807865);
    bytes8 constant SINGLE_BIT_FLIP_CHAMPIONSHIP = bytes8(0xFFFFFFFFFFFFFFFE);
    bytes8 constant REGION_BOUNDARY_UPSETS = bytes8(0xFF7F7F7F7FFFFFFF);

    // ── Self-score: every bracket scored against itself = 192 ───────────────

    function test_selfScore_allChalk() public view {
        _assertSelfScore(ALL_CHALK, "all_chalk");
    }

    function test_selfScore_allUpsets() public view {
        _assertSelfScore(ALL_UPSETS, "all_upsets");
    }

    function test_selfScore_mostlyChalk() public view {
        _assertSelfScore(MOSTLY_CHALK_FEW_UPSETS, "mostly_chalk_few_upsets");
    }

    function test_selfScore_cinderella() public view {
        _assertSelfScore(CINDERELLA_RUN, "cinderella_run");
    }

    function test_selfScore_alternating() public view {
        _assertSelfScore(ALTERNATING_PICKS, "alternating_picks");
    }

    function test_selfScore_splitRegions() public view {
        _assertSelfScore(SPLIT_REGIONS, "split_regions");
    }

    function test_selfScore_singleBitFlip() public view {
        _assertSelfScore(SINGLE_BIT_FLIP_CHAMPIONSHIP, "single_bit_flip_championship");
    }

    function test_selfScore_regionBoundary() public view {
        _assertSelfScore(REGION_BOUNDARY_UPSETS, "region_boundary_upsets");
    }

    // ── Scoring against all_chalk results ───────────────────────────────────

    function test_score_allChalk_vs_allChalk() public view {
        _assertScore(ALL_CHALK, ALL_CHALK, 192);
    }

    function test_score_allUpsets_vs_allChalk() public view {
        _assertScore(ALL_UPSETS, ALL_CHALK, 0);
    }

    function test_score_mostlyChalk_vs_allChalk() public view {
        _assertScore(MOSTLY_CHALK_FEW_UPSETS, ALL_CHALK, 175);
    }

    function test_score_cinderella_vs_allChalk() public view {
        _assertScore(CINDERELLA_RUN, ALL_CHALK, 117);
    }

    function test_score_alternating_vs_allChalk() public view {
        _assertScore(ALTERNATING_PICKS, ALL_CHALK, 112);
    }

    function test_score_splitRegions_vs_allChalk() public view {
        _assertScore(SPLIT_REGIONS, ALL_CHALK, 18);
    }

    function test_score_singleBitFlip_vs_allChalk() public view {
        _assertScore(SINGLE_BIT_FLIP_CHAMPIONSHIP, ALL_CHALK, 129);
    }

    function test_score_regionBoundary_vs_allChalk() public view {
        _assertScore(REGION_BOUNDARY_UPSETS, ALL_CHALK, 183);
    }

    // ── Scoring against cinderella_run results ──────────────────────────────

    function test_score_allChalk_vs_cinderella() public view {
        _assertScore(ALL_CHALK, CINDERELLA_RUN, 117);
    }

    function test_score_allUpsets_vs_cinderella() public view {
        _assertScore(ALL_UPSETS, CINDERELLA_RUN, 5);
    }

    function test_score_mostlyChalk_vs_cinderella() public view {
        _assertScore(MOSTLY_CHALK_FEW_UPSETS, CINDERELLA_RUN, 102);
    }

    function test_score_cinderella_vs_cinderella() public view {
        _assertScore(CINDERELLA_RUN, CINDERELLA_RUN, 192);
    }

    function test_score_alternating_vs_cinderella() public view {
        _assertScore(ALTERNATING_PICKS, CINDERELLA_RUN, 45);
    }

    function test_score_splitRegions_vs_cinderella() public view {
        _assertScore(SPLIT_REGIONS, CINDERELLA_RUN, 11);
    }

    function test_score_singleBitFlip_vs_cinderella() public view {
        _assertScore(SINGLE_BIT_FLIP_CHAMPIONSHIP, CINDERELLA_RUN, 148);
    }

    function test_score_regionBoundary_vs_cinderella() public view {
        _assertScore(REGION_BOUNDARY_UPSETS, CINDERELLA_RUN, 112);
    }

    // ── Scoring symmetry: score(A, B) need not equal score(B, A) ────────────
    // But we verify the specific asymmetric pairs from the golden vectors.

    function test_scoring_asymmetry_chalk_cinderella() public view {
        // score(all_chalk, cinderella) = 117, score(cinderella, all_chalk) = 117
        // Happens to be symmetric in this case
        _assertScore(ALL_CHALK, CINDERELLA_RUN, 117);
        _assertScore(CINDERELLA_RUN, ALL_CHALK, 117);
    }

    function test_scoring_asymmetry_singleBit() public view {
        // score(single_bit_flip, all_chalk) = 129
        // score(all_chalk, single_bit_flip) — different since results change the mask
        uint64 filter = wrapper.getScoringMask(SINGLE_BIT_FLIP_CHAMPIONSHIP);
        uint8 score = wrapper.getBracketScore(ALL_CHALK, SINGLE_BIT_FLIP_CHAMPIONSHIP, filter);
        // Just verify it computes without error; the exact value is not in vectors
        assertTrue(score <= 192);
    }

    // ── Validation: sentinel bit checks ─────────────────────────────────────

    function test_validation_sentinelSet() public pure {
        // All valid brackets have MSB set
        assertTrue(ALL_CHALK[0] & 0x80 != 0);
        assertTrue(ALL_UPSETS[0] & 0x80 != 0);
        assertTrue(CINDERELLA_RUN[0] & 0x80 != 0);
        assertTrue(ALTERNATING_PICKS[0] & 0x80 != 0);
        assertTrue(SPLIT_REGIONS[0] & 0x80 != 0);
        assertTrue(SINGLE_BIT_FLIP_CHAMPIONSHIP[0] & 0x80 != 0);
        assertTrue(REGION_BOUNDARY_UPSETS[0] & 0x80 != 0);
        assertTrue(MOSTLY_CHALK_FEW_UPSETS[0] & 0x80 != 0);
    }

    function test_validation_noSentinel() public pure {
        // 0x7FFFFFFFFFFFFFFF has MSB=0 — invalid
        bytes8 noSentinel = bytes8(0x7FFFFFFFFFFFFFFF);
        assertTrue(noSentinel[0] & 0x80 == 0);

        // 0x0000000000000000 — invalid
        bytes8 zero = bytes8(0x0000000000000000);
        assertTrue(zero[0] & 0x80 == 0);

        // 0x1234567890ABCDEF — invalid (first nibble 0x1 < 0x8)
        bytes8 lowFirst = bytes8(0x1234567890ABCDEF);
        assertTrue(lowFirst[0] & 0x80 == 0);
    }

    // ── Helper functions ────────────────────────────────────────────────────

    function _assertSelfScore(bytes8 bracket, string memory name) internal view {
        uint64 filter = wrapper.getScoringMask(bracket);
        uint8 score = wrapper.getBracketScore(bracket, bracket, filter);
        assertEq(score, 192, string.concat("Self-score should be 192 for ", name));
    }

    function _assertScore(bytes8 bracket, bytes8 results, uint8 expected) internal view {
        uint64 filter = wrapper.getScoringMask(results);
        uint8 score = wrapper.getBracketScore(bracket, results, filter);
        assertEq(score, expected);
    }
}

/// @title End-to-end scoring through MarchMadness contract with golden vectors
/// @notice Tests that scoring through the full contract (submit → results → score)
///         matches the golden test vectors. This is the most critical test since
///         the contract is where real payouts happen.
contract BracketVectorsE2ETest is Test {
    MarchMadness mm;
    address owner;
    uint256 constant ENTRY_FEE = 1 ether;
    uint256 constant DEADLINE = 1000;

    // Test accounts
    address alice = address(0xA11CE);
    address bob = address(0xB0B);
    address charlie = address(0xC4A2);
    address dave = address(0xDA7E);
    address eve = address(0xE7E);
    address frank = address(0xF2A4C);
    address grace = address(0x62ACE);
    address hank = address(0x4A4C);

    function setUp() public {
        vm.warp(100);
        owner = address(this);
        mm = new MarchMadness(2026, ENTRY_FEE, DEADLINE);

        // Fund all accounts
        vm.deal(alice, 10 ether);
        vm.deal(bob, 10 ether);
        vm.deal(charlie, 10 ether);
        vm.deal(dave, 10 ether);
        vm.deal(eve, 10 ether);
        vm.deal(frank, 10 ether);
        vm.deal(grace, 10 ether);
        vm.deal(hank, 10 ether);
    }

    /// @notice Submit all 8 golden vector brackets, post all_chalk results, verify scores
    function test_e2e_allVectors_vs_allChalkResults() public {
        bytes8 allChalk = bytes8(0xFFFFFFFFFFFFFFFF);

        // Submit brackets
        _submit(alice, 0xFFFFFFFFFFFFFFFF); // all_chalk
        _submit(bob, 0x8000000000000000); // all_upsets
        _submit(charlie, 0xF7F7F7F7DFFFFFFF); // mostly_chalk
        _submit(dave, 0xBFFFFFFFBFFFBFBA); // cinderella
        _submit(eve, 0xD555555555555555); // alternating
        _submit(frank, 0xFFFF80007F807865); // split_regions
        _submit(grace, 0xFFFFFFFFFFFFFFFE); // single_bit_flip
        _submit(hank, 0xFF7F7F7F7FFFFFFF); // region_boundary

        // Post results and score
        vm.warp(DEADLINE + 1);
        mm.submitResults(allChalk);

        mm.scoreBracket(alice);
        mm.scoreBracket(bob);
        mm.scoreBracket(charlie);
        mm.scoreBracket(dave);
        mm.scoreBracket(eve);
        mm.scoreBracket(frank);
        mm.scoreBracket(grace);
        mm.scoreBracket(hank);

        // Verify scores match golden vectors
        assertEq(mm.scores(alice), 192, "all_chalk vs all_chalk");
        assertEq(mm.scores(bob), 0, "all_upsets vs all_chalk");
        assertEq(mm.scores(charlie), 175, "mostly_chalk vs all_chalk");
        assertEq(mm.scores(dave), 117, "cinderella vs all_chalk");
        assertEq(mm.scores(eve), 112, "alternating vs all_chalk");
        assertEq(mm.scores(frank), 18, "split_regions vs all_chalk");
        assertEq(mm.scores(grace), 129, "single_bit_flip vs all_chalk");
        assertEq(mm.scores(hank), 183, "region_boundary vs all_chalk");

        // Verify winner tracking
        assertEq(mm.winningScore(), 192);
        assertEq(mm.numWinners(), 1);
    }

    /// @notice Submit all 8 golden vector brackets, post cinderella results, verify scores
    function test_e2e_allVectors_vs_cinderellaResults() public {
        bytes8 cinderellaResults = bytes8(0xBFFFFFFFBFFFBFBA);

        // Submit brackets
        _submit(alice, 0xFFFFFFFFFFFFFFFF); // all_chalk
        _submit(bob, 0x8000000000000000); // all_upsets
        _submit(charlie, 0xF7F7F7F7DFFFFFFF); // mostly_chalk
        _submit(dave, 0xBFFFFFFFBFFFBFBA); // cinderella
        _submit(eve, 0xD555555555555555); // alternating
        _submit(frank, 0xFFFF80007F807865); // split_regions
        _submit(grace, 0xFFFFFFFFFFFFFFFE); // single_bit_flip
        _submit(hank, 0xFF7F7F7F7FFFFFFF); // region_boundary

        // Post results and score
        vm.warp(DEADLINE + 1);
        mm.submitResults(cinderellaResults);

        mm.scoreBracket(alice);
        mm.scoreBracket(bob);
        mm.scoreBracket(charlie);
        mm.scoreBracket(dave);
        mm.scoreBracket(eve);
        mm.scoreBracket(frank);
        mm.scoreBracket(grace);
        mm.scoreBracket(hank);

        // Verify scores match golden vectors
        assertEq(mm.scores(alice), 117, "all_chalk vs cinderella");
        assertEq(mm.scores(bob), 5, "all_upsets vs cinderella");
        assertEq(mm.scores(charlie), 102, "mostly_chalk vs cinderella");
        assertEq(mm.scores(dave), 192, "cinderella vs cinderella");
        assertEq(mm.scores(eve), 45, "alternating vs cinderella");
        assertEq(mm.scores(frank), 11, "split_regions vs cinderella");
        assertEq(mm.scores(grace), 148, "single_bit_flip vs cinderella");
        assertEq(mm.scores(hank), 112, "region_boundary vs cinderella");

        // Verify winner tracking — cinderella bracket wins with 192
        assertEq(mm.winningScore(), 192);
        assertEq(mm.numWinners(), 1);
    }

    /// @notice Verify the contract rejects brackets without sentinel bit
    function test_e2e_rejectNoSentinel() public {
        vm.prank(alice);
        vm.expectRevert(IMarchMadness.InvalidSentinelByte.selector);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(bytes8(0x7FFFFFFFFFFFFFFF)));
    }

    /// @notice Verify the contract rejects zero bracket (no sentinel)
    function test_e2e_rejectZeroBracket() public {
        vm.prank(alice);
        vm.expectRevert(IMarchMadness.InvalidSentinelByte.selector);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(bytes8(0x0000000000000000)));
    }

    /// @notice Payout test: perfect bracket wins the entire pool
    function test_e2e_perfectBracketWinsPool() public {
        bytes8 allChalk = bytes8(0xFFFFFFFFFFFFFFFF);

        _submit(alice, 0xFFFFFFFFFFFFFFFF); // perfect
        _submit(bob, 0x8000000000000000); // worst possible

        vm.warp(DEADLINE + 1);
        mm.submitResults(allChalk);

        mm.scoreBracket(alice);
        mm.scoreBracket(bob);

        assertEq(mm.scores(alice), 192);
        assertEq(mm.scores(bob), 0);
        assertEq(mm.winningScore(), 192);
        assertEq(mm.numWinners(), 1);

        // Fast-forward past scoring window
        vm.warp(DEADLINE + 1 + 7 days + 1);

        uint256 balanceBefore = alice.balance;
        vm.prank(alice);
        mm.collectWinnings();
        uint256 balanceAfter = alice.balance;

        // Alice should receive the entire pool (2 entries × 1 ETH)
        assertEq(balanceAfter - balanceBefore, 2 ether);
    }

    /// @notice Two brackets tie for the win, split the pool
    function test_e2e_tiedWinnersSplitPool() public {
        bytes8 cinderellaResults = bytes8(0xBFFFFFFFBFFFBFBA);

        // Both submit the cinderella bracket — both will score 192
        _submit(alice, 0xBFFFFFFFBFFFBFBA);
        _submit(bob, 0xBFFFFFFFBFFFBFBA);
        _submit(charlie, 0x8000000000000000); // loser

        vm.warp(DEADLINE + 1);
        mm.submitResults(cinderellaResults);

        mm.scoreBracket(alice);
        mm.scoreBracket(bob);
        mm.scoreBracket(charlie);

        assertEq(mm.numWinners(), 2);

        vm.warp(DEADLINE + 1 + 7 days + 1);

        uint256 aliceBefore = alice.balance;
        vm.prank(alice);
        mm.collectWinnings();

        uint256 bobBefore = bob.balance;
        vm.prank(bob);
        mm.collectWinnings();

        // Each winner gets 3 ETH / 2 = 1.5 ETH
        assertEq(alice.balance - aliceBefore, 1.5 ether);
        assertEq(bob.balance - bobBefore, 1.5 ether);
    }

    function _submit(address account, uint64 gameBits) internal {
        bytes8 bracket = bytes8(gameBits);
        vm.prank(account);
        mm.submitBracket{value: ENTRY_FEE}(sbytes8(bracket));
    }
}
