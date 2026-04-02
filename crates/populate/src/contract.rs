//! V2 contract ABI definitions matching the actual deployed contracts.
//!
//! Signatures sourced from PR #279 (`cdai__v2-contracts-entry-migration`).

use alloy_sol_types::sol;

sol! {
    #[sol(rpc)]
    contract MarchMadnessV2 {
        // ── V1 read functions ───────────────────────────────────────
        function hasEntry(address account) external view returns (bool);
        function getEntryCount() external view returns (uint32);
        function getBracket(address account) external view returns (bytes8);

        // ── V2 migration imports (owner-only) ───────────────────────
        function importEntry(address account, bytes8 bracket) external;
        function importTag(address account, string calldata tag) external;

        // ── V2 batch import (owner-only, idempotent) ────────────────
        function batchImportEntries(address[] calldata accounts, bytes8[] calldata bracketList) external;

        // ── V2 scoring preview ──────────────────────────────────────
        function previewScore(address account, bytes8 rawResults) external view returns (uint8);

        // ── V2 funding ──────────────────────────────────────────────
        function fund() external payable;
    }
}

sol! {
    #[sol(rpc)]
    contract BracketGroupsV2 {
        // ── V2 migration imports (owner-only) ───────────────────────
        function importGroup(
            string calldata slug,
            string calldata displayName,
            uint256 entryFee,
            address creator
        ) external returns (uint32 groupId);

        function importMember(uint32 groupId, address addr, string calldata name) external;

        function batchImportMembers(
            uint32 groupId,
            address[] calldata addrs,
            string[] calldata names
        ) external;

        // ── V2 funding ──────────────────────────────────────────────
        function fund() external payable;
    }
}
