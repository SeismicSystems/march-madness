// SPDX-License-Identifier: MIT
pragma solidity ^0.8.30;

/// @title BracketMirror — admin-managed off-chain bracket pool mirror
/// @notice Stores brackets + slugs from external pools (e.g. Yahoo Fantasy) on-chain
///         for display purposes. No money, no scoring, no composition with MarchMadness.
///         All winner computation happens off-chain.
contract BracketMirror {
    // ── Errors ───────────────────────────────────────────────────────────
    error MirrorDoesNotExist();
    error NotMirrorAdmin();
    error SlugCannotBeEmpty();
    error SlugTooLong();
    error SlugAlreadyTaken();
    error InvalidSentinelByte();
    error EntrySlugAlreadyTaken();
    error IndexOutOfBounds();
    error MirrorNotFound();
    error EntryNotFound();
    error SlugNotUrlSafe();

    // ── Types ───────────────────────────────────────────────────────────
    struct Mirror {
        string slug;
        string displayName;
        uint32 entryFee; // off-chain entry fee amount (display only, e.g. 25 for "$25 USD")
        string entryCurrency; // currency label (e.g. "USD", "ETH")
        address admin;
    }

    struct MirrorEntry {
        bytes8 bracket;
        string slug; // display identifier for this entry (e.g. player name), unique within mirror
    }

    // ── State ───────────────────────────────────────────────────────────
    uint256 public nextMirrorId = 1;

    mapping(uint256 => Mirror) internal _mirrors;
    mapping(bytes32 => uint256) public slugToMirrorId;
    mapping(uint256 => MirrorEntry[]) internal _entries;

    // Entry slug lookup: _entrySlugIndex[mirrorId][slugHash] = entryIndex + 1 (0 = not found)
    mapping(uint256 => mapping(bytes32 => uint256)) internal _entrySlugIndex;

    // ── Constants ───────────────────────────────────────────────────────
    uint256 public constant MAX_SLUG_LENGTH = 32;

    // ── Internal helpers ────────────────────────────────────────────────

    /// @dev Revert if slug contains non-URL-safe characters.
    ///      Allowed: a-z, 0-9, hyphen. No leading/trailing hyphens.
    function _validateSlugChars(bytes memory s) internal pure {
        if (s[0] == 0x2D || s[s.length - 1] == 0x2D) revert SlugNotUrlSafe();
        for (uint256 i; i < s.length; i++) {
            bytes1 b = s[i];
            if (
                !(b >= 0x61 && b <= 0x7A) // a-z
                    && !(b >= 0x30 && b <= 0x39) // 0-9
                    && b != 0x2D // -
            ) revert SlugNotUrlSafe();
        }
    }

    // ── Events ──────────────────────────────────────────────────────────
    event MirrorCreated(uint256 indexed mirrorId, string slug, string displayName, address admin);
    event EntryAdded(uint256 indexed mirrorId, string slug);
    event EntryRemoved(uint256 indexed mirrorId, string slug);
    event BracketUpdated(uint256 indexed mirrorId, string slug);

    // ── Modifiers ───────────────────────────────────────────────────────
    modifier onlyAdmin(uint256 mirrorId) {
        if (_mirrors[mirrorId].admin == address(0)) revert MirrorDoesNotExist();
        if (msg.sender != _mirrors[mirrorId].admin) revert NotMirrorAdmin();
        _;
    }

    modifier mirrorExists(uint256 mirrorId) {
        if (_mirrors[mirrorId].admin == address(0)) revert MirrorDoesNotExist();
        _;
    }

    // ════════════════════════════════════════════════════════════════════
    //  MIRROR LIFECYCLE
    // ════════════════════════════════════════════════════════════════════

    /// @notice Create a new mirror pool. Caller becomes admin.
    function createMirror(string calldata slug, string calldata displayName) external returns (uint256 mirrorId) {
        bytes memory slugBytes = bytes(slug);
        if (slugBytes.length == 0) revert SlugCannotBeEmpty();
        if (slugBytes.length > MAX_SLUG_LENGTH) revert SlugTooLong();
        _validateSlugChars(slugBytes);

        bytes32 slugHash = keccak256(slugBytes);
        if (slugToMirrorId[slugHash] != 0) revert SlugAlreadyTaken();

        mirrorId = nextMirrorId++;

        _mirrors[mirrorId] =
            Mirror({slug: slug, displayName: displayName, entryFee: 0, entryCurrency: "", admin: msg.sender});

        slugToMirrorId[slugHash] = mirrorId;

        emit MirrorCreated(mirrorId, slug, displayName, msg.sender);
    }

    /// @notice Set or update the entry fee display info. Admin only.
    function setEntryFee(uint256 mirrorId, uint32 fee, string calldata currency) external onlyAdmin(mirrorId) {
        _mirrors[mirrorId].entryFee = fee;
        _mirrors[mirrorId].entryCurrency = currency;
    }

    // ════════════════════════════════════════════════════════════════════
    //  ENTRY MANAGEMENT — admin only
    // ════════════════════════════════════════════════════════════════════

    /// @notice Add a bracket entry (bracket + slug). Admin only. Slug must be unique within mirror.
    function addEntry(uint256 mirrorId, bytes8 bracket, string calldata slug) external onlyAdmin(mirrorId) {
        if (bracket[0] & 0x80 == 0) revert InvalidSentinelByte();

        bytes memory slugBytes = bytes(slug);
        if (slugBytes.length == 0) revert SlugCannotBeEmpty();
        _validateSlugChars(slugBytes);

        bytes32 entrySlugHash = keccak256(slugBytes);
        if (_entrySlugIndex[mirrorId][entrySlugHash] != 0) revert EntrySlugAlreadyTaken();

        _entries[mirrorId].push(MirrorEntry({bracket: bracket, slug: slug}));
        _entrySlugIndex[mirrorId][entrySlugHash] = _entries[mirrorId].length; // index + 1

        emit EntryAdded(mirrorId, slug);
    }

    /// @notice Remove an entry (swap-and-pop). Admin only.
    function removeEntry(uint256 mirrorId, uint256 entryIndex) external onlyAdmin(mirrorId) {
        MirrorEntry[] storage entries = _entries[mirrorId];
        if (entryIndex >= entries.length) revert IndexOutOfBounds();

        uint256 lastIndex = entries.length - 1;

        // Capture slug before swap-and-pop
        string memory removedSlug = entries[entryIndex].slug;

        // Update slug mappings before swap
        bytes32 removedSlugHash = keccak256(bytes(removedSlug));
        delete _entrySlugIndex[mirrorId][removedSlugHash];

        if (entryIndex != lastIndex) {
            bytes32 lastSlugHash = keccak256(bytes(entries[lastIndex].slug));
            _entrySlugIndex[mirrorId][lastSlugHash] = entryIndex + 1;
            entries[entryIndex] = entries[lastIndex];
        }
        entries.pop();

        emit EntryRemoved(mirrorId, removedSlug);
    }

    /// @notice Update the bracket for an entry. Admin only.
    function updateBracket(uint256 mirrorId, uint256 entryIndex, bytes8 bracket) external onlyAdmin(mirrorId) {
        if (entryIndex >= _entries[mirrorId].length) revert IndexOutOfBounds();
        if (bracket[0] & 0x80 == 0) revert InvalidSentinelByte();

        _entries[mirrorId][entryIndex].bracket = bracket;

        emit BracketUpdated(mirrorId, _entries[mirrorId][entryIndex].slug);
    }

    /// @notice Update the slug for an entry. Admin only. New slug must be unique within mirror.
    function updateEntrySlug(uint256 mirrorId, uint256 entryIndex, string calldata slug) external onlyAdmin(mirrorId) {
        if (entryIndex >= _entries[mirrorId].length) revert IndexOutOfBounds();

        bytes memory slugBytes = bytes(slug);
        if (slugBytes.length == 0) revert SlugCannotBeEmpty();
        _validateSlugChars(slugBytes);

        bytes32 oldSlugHash = keccak256(bytes(_entries[mirrorId][entryIndex].slug));
        bytes32 newSlugHash = keccak256(slugBytes);

        if (oldSlugHash != newSlugHash) {
            if (_entrySlugIndex[mirrorId][newSlugHash] != 0) revert EntrySlugAlreadyTaken();
            delete _entrySlugIndex[mirrorId][oldSlugHash];
            _entrySlugIndex[mirrorId][newSlugHash] = entryIndex + 1;
        }

        _entries[mirrorId][entryIndex].slug = slug;
    }

    // ════════════════════════════════════════════════════════════════════
    //  VIEW FUNCTIONS
    // ════════════════════════════════════════════════════════════════════

    function getMirrorBySlug(string calldata slug) external view returns (uint256) {
        bytes32 slugHash = keccak256(bytes(slug));
        uint256 mirrorId = slugToMirrorId[slugHash];
        if (mirrorId == 0) revert MirrorNotFound();
        return mirrorId;
    }

    function getMirror(uint256 mirrorId) external view mirrorExists(mirrorId) returns (Mirror memory) {
        return _mirrors[mirrorId];
    }

    function getEntryCount(uint256 mirrorId) external view mirrorExists(mirrorId) returns (uint256) {
        return _entries[mirrorId].length;
    }

    function getEntry(uint256 mirrorId, uint256 index)
        external
        view
        mirrorExists(mirrorId)
        returns (MirrorEntry memory)
    {
        if (index >= _entries[mirrorId].length) revert IndexOutOfBounds();
        return _entries[mirrorId][index];
    }

    function getEntryBySlug(uint256 mirrorId, string calldata slug)
        external
        view
        mirrorExists(mirrorId)
        returns (MirrorEntry memory)
    {
        bytes32 slugHash = keccak256(bytes(slug));
        uint256 indexPlusOne = _entrySlugIndex[mirrorId][slugHash];
        if (indexPlusOne == 0) revert EntryNotFound();
        return _entries[mirrorId][indexPlusOne - 1];
    }

    function getEntries(uint256 mirrorId) external view mirrorExists(mirrorId) returns (MirrorEntry[] memory) {
        return _entries[mirrorId];
    }
}
