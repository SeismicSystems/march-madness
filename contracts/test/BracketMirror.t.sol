// SPDX-License-Identifier: MIT
pragma solidity ^0.8.30;

import {Test} from "forge-std/Test.sol";
import {BracketMirror} from "../src/BracketMirror.sol";

/// @title BracketMirror tests — admin-managed off-chain bracket pool mirror
contract BracketMirrorTest is Test {
    BracketMirror bm;

    address admin = address(0xAD);
    address alice = address(0xA11CE);

    bytes8 constant PERFECT = bytes8(0xFFFFFFFFFFFFFFFF);
    bytes8 constant BAD = bytes8(0x8000000000000000);

    function setUp() public {
        bm = new BracketMirror();
    }

    // ── Mirror creation ─────────────────────────────────────────────────

    function test_createMirror() public {
        vm.prank(admin);
        uint256 mirrorId = bm.createMirror("yahoo-pool", "Yahoo Fantasy Pool");

        assertEq(mirrorId, 1);
        assertEq(bm.getMirrorBySlug("yahoo-pool"), 1);

        BracketMirror.Mirror memory m = bm.getMirror(mirrorId);
        assertEq(m.slug, "yahoo-pool");
        assertEq(m.displayName, "Yahoo Fantasy Pool");
        assertEq(m.admin, admin);
    }

    function test_duplicateSlugReverts() public {
        vm.prank(admin);
        bm.createMirror("pool", "Pool");

        vm.prank(admin);
        vm.expectRevert(BracketMirror.SlugAlreadyTaken.selector);
        bm.createMirror("pool", "Other Pool");
    }

    function test_emptySlugReverts() public {
        vm.expectRevert(BracketMirror.SlugCannotBeEmpty.selector);
        bm.createMirror("", "Pool");
    }

    function test_longSlugReverts() public {
        vm.expectRevert(BracketMirror.SlugTooLong.selector);
        bm.createMirror("this-slug-is-way-too-long-and-exceeds-the-32-byte-limit", "Pool");
    }

    function test_slugLookupNonexistent() public {
        vm.expectRevert(BracketMirror.MirrorNotFound.selector);
        bm.getMirrorBySlug("nope");
    }

    function test_nonexistentMirrorReverts() public {
        vm.expectRevert(BracketMirror.MirrorDoesNotExist.selector);
        bm.getMirror(999);
    }

    // ── Entry fee ─────────────────────────────────────────────────────────

    function test_setEntryFee() public {
        vm.prank(admin);
        uint256 mirrorId = bm.createMirror("pool", "Pool");

        vm.prank(admin);
        bm.setEntryFee(mirrorId, 25, "USD");

        BracketMirror.Mirror memory m = bm.getMirror(mirrorId);
        assertEq(m.entryFee, 25);
        assertEq(m.entryCurrency, "USD");
    }

    function test_setEntryFee_onlyAdmin() public {
        vm.prank(admin);
        uint256 mirrorId = bm.createMirror("pool", "Pool");

        vm.prank(alice);
        vm.expectRevert(BracketMirror.NotMirrorAdmin.selector);
        bm.setEntryFee(mirrorId, 100, "USD");
    }

    // ── Entry management ────────────────────────────────────────────────

    function test_addEntries() public {
        vm.prank(admin);
        uint256 mirrorId = bm.createMirror("pool", "Pool");

        vm.startPrank(admin);
        bm.addEntry(mirrorId, PERFECT, "alice");
        bm.addEntry(mirrorId, BAD, "bob");
        vm.stopPrank();

        assertEq(bm.getEntryCount(mirrorId), 2);

        BracketMirror.MirrorEntry memory e0 = bm.getEntry(mirrorId, 0);
        assertEq(e0.slug, "alice");
        assertEq(e0.bracket, PERFECT);

        BracketMirror.MirrorEntry memory e1 = bm.getEntry(mirrorId, 1);
        assertEq(e1.slug, "bob");
        assertEq(e1.bracket, BAD);
    }

    function test_addEntry_duplicateSlugReverts() public {
        vm.prank(admin);
        uint256 mirrorId = bm.createMirror("pool", "Pool");

        vm.startPrank(admin);
        bm.addEntry(mirrorId, PERFECT, "alice");

        vm.expectRevert(BracketMirror.EntrySlugAlreadyTaken.selector);
        bm.addEntry(mirrorId, BAD, "alice");
        vm.stopPrank();
    }

    function test_getEntryBySlug() public {
        vm.prank(admin);
        uint256 mirrorId = bm.createMirror("pool", "Pool");

        vm.startPrank(admin);
        bm.addEntry(mirrorId, PERFECT, "alice");
        bm.addEntry(mirrorId, BAD, "bob");
        vm.stopPrank();

        BracketMirror.MirrorEntry memory e = bm.getEntryBySlug(mirrorId, "alice");
        assertEq(e.bracket, PERFECT);
        assertEq(e.slug, "alice");

        BracketMirror.MirrorEntry memory e2 = bm.getEntryBySlug(mirrorId, "bob");
        assertEq(e2.bracket, BAD);
    }

    function test_getEntryBySlug_notFound() public {
        vm.prank(admin);
        uint256 mirrorId = bm.createMirror("pool", "Pool");

        vm.expectRevert(BracketMirror.EntryNotFound.selector);
        bm.getEntryBySlug(mirrorId, "nope");
    }

    function test_getEntries_batch() public {
        vm.prank(admin);
        uint256 mirrorId = bm.createMirror("pool", "Pool");

        vm.startPrank(admin);
        bm.addEntry(mirrorId, PERFECT, "alice");
        bm.addEntry(mirrorId, BAD, "bob");
        vm.stopPrank();

        BracketMirror.MirrorEntry[] memory entries = bm.getEntries(mirrorId);
        assertEq(entries.length, 2);
        assertEq(entries[0].slug, "alice");
        assertEq(entries[1].slug, "bob");
    }

    function test_removeEntry_swapAndPop() public {
        vm.prank(admin);
        uint256 mirrorId = bm.createMirror("pool", "Pool");

        vm.startPrank(admin);
        bm.addEntry(mirrorId, PERFECT, "alice");
        bm.addEntry(mirrorId, BAD, "bob");
        bm.addEntry(mirrorId, PERFECT, "charlie");

        // Remove index 0 (alice) — charlie moves to 0
        bm.removeEntry(mirrorId, 0);
        vm.stopPrank();

        assertEq(bm.getEntryCount(mirrorId), 2);
        assertEq(bm.getEntry(mirrorId, 0).slug, "charlie");
        assertEq(bm.getEntry(mirrorId, 1).slug, "bob");

        // Slug lookup should still work after swap
        BracketMirror.MirrorEntry memory e = bm.getEntryBySlug(mirrorId, "charlie");
        assertEq(e.bracket, PERFECT);

        // Removed entry's slug should be gone
        vm.expectRevert(BracketMirror.EntryNotFound.selector);
        bm.getEntryBySlug(mirrorId, "alice");
    }

    function test_removeEntry_lastElement() public {
        vm.prank(admin);
        uint256 mirrorId = bm.createMirror("pool", "Pool");

        vm.startPrank(admin);
        bm.addEntry(mirrorId, PERFECT, "alice");
        bm.addEntry(mirrorId, BAD, "bob");
        bm.removeEntry(mirrorId, 1);
        vm.stopPrank();

        assertEq(bm.getEntryCount(mirrorId), 1);
        assertEq(bm.getEntry(mirrorId, 0).slug, "alice");

        vm.expectRevert(BracketMirror.EntryNotFound.selector);
        bm.getEntryBySlug(mirrorId, "bob");
    }

    function test_updateBracket() public {
        vm.prank(admin);
        uint256 mirrorId = bm.createMirror("pool", "Pool");

        vm.startPrank(admin);
        bm.addEntry(mirrorId, PERFECT, "alice");
        bm.updateBracket(mirrorId, 0, BAD);
        vm.stopPrank();

        assertEq(bm.getEntry(mirrorId, 0).bracket, BAD);
    }

    function test_updateEntrySlug() public {
        vm.prank(admin);
        uint256 mirrorId = bm.createMirror("pool", "Pool");

        vm.prank(admin);
        bm.addEntry(mirrorId, PERFECT, "alice");

        vm.prank(admin);
        bm.updateEntrySlug(mirrorId, 0, "alice-updated");

        assertEq(bm.getEntry(mirrorId, 0).slug, "alice-updated");

        // Old slug gone, new slug works
        vm.expectRevert(BracketMirror.EntryNotFound.selector);
        bm.getEntryBySlug(mirrorId, "alice");

        BracketMirror.MirrorEntry memory e = bm.getEntryBySlug(mirrorId, "alice-updated");
        assertEq(e.bracket, PERFECT);
    }

    function test_updateEntrySlug_duplicateReverts() public {
        vm.prank(admin);
        uint256 mirrorId = bm.createMirror("pool", "Pool");

        vm.startPrank(admin);
        bm.addEntry(mirrorId, PERFECT, "alice");
        bm.addEntry(mirrorId, BAD, "bob");

        vm.expectRevert(BracketMirror.EntrySlugAlreadyTaken.selector);
        bm.updateEntrySlug(mirrorId, 0, "bob");
        vm.stopPrank();
    }

    function test_updateEntrySlug_sameSlugNoOp() public {
        vm.prank(admin);
        uint256 mirrorId = bm.createMirror("pool", "Pool");

        vm.startPrank(admin);
        bm.addEntry(mirrorId, PERFECT, "alice");
        // Updating to same slug should not revert
        bm.updateEntrySlug(mirrorId, 0, "alice");
        vm.stopPrank();

        assertEq(bm.getEntry(mirrorId, 0).slug, "alice");
    }

    function test_invalidSentinelReverts() public {
        vm.prank(admin);
        uint256 mirrorId = bm.createMirror("pool", "Pool");

        vm.prank(admin);
        vm.expectRevert(BracketMirror.InvalidSentinelByte.selector);
        bm.addEntry(mirrorId, bytes8(0x0000000000000001), "bad");
    }

    // ── Access control ──────────────────────────────────────────────────

    function test_nonAdminCannotModify() public {
        vm.prank(admin);
        uint256 mirrorId = bm.createMirror("pool", "Pool");

        vm.prank(admin);
        bm.addEntry(mirrorId, PERFECT, "alice");

        vm.prank(alice);
        vm.expectRevert(BracketMirror.NotMirrorAdmin.selector);
        bm.addEntry(mirrorId, BAD, "hack");

        vm.prank(alice);
        vm.expectRevert(BracketMirror.NotMirrorAdmin.selector);
        bm.removeEntry(mirrorId, 0);

        vm.prank(alice);
        vm.expectRevert(BracketMirror.NotMirrorAdmin.selector);
        bm.updateBracket(mirrorId, 0, BAD);

        vm.prank(alice);
        vm.expectRevert(BracketMirror.NotMirrorAdmin.selector);
        bm.updateEntrySlug(mirrorId, 0, "hack");
    }

    function test_indexOutOfBounds() public {
        vm.prank(admin);
        uint256 mirrorId = bm.createMirror("pool", "Pool");

        vm.prank(admin);
        vm.expectRevert(BracketMirror.IndexOutOfBounds.selector);
        bm.removeEntry(mirrorId, 0);

        vm.prank(admin);
        vm.expectRevert(BracketMirror.IndexOutOfBounds.selector);
        bm.updateBracket(mirrorId, 0, PERFECT);

        vm.prank(admin);
        vm.expectRevert(BracketMirror.IndexOutOfBounds.selector);
        bm.updateEntrySlug(mirrorId, 0, "name");
    }

    function test_multipleMirrors() public {
        vm.prank(admin);
        uint256 m1 = bm.createMirror("pool1", "Pool 1");
        vm.prank(alice);
        uint256 m2 = bm.createMirror("pool2", "Pool 2");

        assertEq(m1, 1);
        assertEq(m2, 2);
        assertEq(bm.getMirror(m2).admin, alice);
    }

    function test_noPayableFunctions() public {
        vm.prank(admin);
        bm.createMirror("pool", "Pool");

        assertEq(address(bm).balance, 0);
    }
}
