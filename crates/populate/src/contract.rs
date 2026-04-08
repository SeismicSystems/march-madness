//! V1 and V2 contract ABI definitions.
//!
//! V1 signatures for reading source data (entries, tags, groups, members).
//! V2 signatures for writing migration target (from PR #281).

use alloy_sol_types::sol;

// ── V1 MarchMadness (source) ────────────────────────────────────────

sol! {
    #[sol(rpc)]
    contract MarchMadness {
        function getBracket(address account) external view returns (bytes8);
        function getTag(address account) external view returns (string);
        function entryFee() external view returns (uint256);

        event BracketSubmitted(address indexed account);
    }
}

// ── V1 BracketGroups (source) ───────────────────────────────────────

sol! {
    #[sol(rpc)]
    contract BracketGroups {
        struct Group {
            string slug;
            string displayName;
            address creator;
            uint32 entryCount;
            uint256 entryFee;
            bool hasPassword;
        }

        struct Member {
            address addr;
            string name;
            uint8 score;
            bool isScored;
        }

        function marchMadness() external view returns (address);
        function getGroup(uint32 groupId) external view returns (Group memory);
        function getMembers(uint32 groupId) external view returns (Member[] memory);

        event GroupCreated(uint32 indexed groupId, string slug, string displayName, address creator, bool hasPassword);
    }
}

// ── V2 MarchMadnessV2 (target) ─────────────────────────────────────

sol! {
    #[sol(rpc)]
    contract MarchMadnessV2 {
        function entryFee() external view returns (uint256);

        /// Payable: msg.value must equal accounts.length * entryFee.
        function batchImportEntries(address[] calldata accounts, bytes8[] calldata bracketList) external payable;
        function importTag(address account, string calldata tag) external;
    }
}

// ── V2 BracketGroupsV2 (target) ────────────────────────────────────

sol! {
    #[sol(rpc)]
    contract BracketGroupsV2 {
        function marchMadness() external view returns (address);

        function importGroup(
            string calldata slug,
            string calldata displayName,
            uint256 entryFee,
            address creator
        ) external returns (uint32 groupId);

        /// Payable: msg.value must equal addrs.length * group.entryFee.
        function batchImportMembers(
            uint32 groupId,
            address[] calldata addrs,
            string[] calldata names
        ) external payable;

        struct Group {
            string slug;
            string displayName;
            address creator;
            uint32 entryCount;
            uint256 entryFee;
            bool hasPassword;
        }

        function getGroupBySlug(string calldata slug) external view returns (uint32, Group memory);
    }
}
