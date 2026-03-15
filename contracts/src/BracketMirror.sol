// SPDX-License-Identifier: MIT
pragma solidity ^0.8.30;

/// @title BracketMirror — admin-managed off-chain bracket pool mirror
/// @notice Stores brackets + slugs from external pools (e.g. Yahoo Fantasy) on-chain
///         for display purposes. No money, no scoring, no composition with MarchMadness.
///         All winner computation happens off-chain.
contract BracketMirror {
    // ── Types ───────────────────────────────────────────────────────────
    struct Mirror {
        string slug;
        string displayName;
        string prizeDescription; // off-chain prize info (e.g. "$500 Amazon gift card")
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

    // ── Events ──────────────────────────────────────────────────────────
    event MirrorCreated(uint256 indexed mirrorId, string slug, string displayName, address admin);
    event EntryAdded(uint256 indexed mirrorId, uint256 entryIndex, string slug);
    event EntryRemoved(uint256 indexed mirrorId, uint256 entryIndex);
    event BracketUpdated(uint256 indexed mirrorId, uint256 entryIndex);

    // ── Modifiers ───────────────────────────────────────────────────────
    modifier onlyAdmin(uint256 mirrorId) {
        require(_mirrors[mirrorId].admin != address(0), "Mirror does not exist");
        require(msg.sender == _mirrors[mirrorId].admin, "Not mirror admin");
        _;
    }

    modifier mirrorExists(uint256 mirrorId) {
        require(_mirrors[mirrorId].admin != address(0), "Mirror does not exist");
        _;
    }

    // ════════════════════════════════════════════════════════════════════
    //  MIRROR LIFECYCLE
    // ════════════════════════════════════════════════════════════════════

    /// @notice Create a new mirror pool. Caller becomes admin.
    function createMirror(string calldata slug, string calldata displayName) external returns (uint256 mirrorId) {
        bytes memory slugBytes = bytes(slug);
        require(slugBytes.length > 0, "Slug cannot be empty");
        require(slugBytes.length <= MAX_SLUG_LENGTH, "Slug too long");

        bytes32 slugHash = keccak256(slugBytes);
        require(slugToMirrorId[slugHash] == 0, "Slug already taken");

        mirrorId = nextMirrorId++;

        _mirrors[mirrorId] = Mirror({slug: slug, displayName: displayName, prizeDescription: "", admin: msg.sender});

        slugToMirrorId[slugHash] = mirrorId;

        emit MirrorCreated(mirrorId, slug, displayName, msg.sender);
    }

    /// @notice Set or update the prize description. Admin only.
    function setPrizeDescription(uint256 mirrorId, string calldata description) external onlyAdmin(mirrorId) {
        _mirrors[mirrorId].prizeDescription = description;
    }

    // ════════════════════════════════════════════════════════════════════
    //  ENTRY MANAGEMENT — admin only
    // ════════════════════════════════════════════════════════════════════

    /// @notice Add a bracket entry (bracket + slug). Admin only. Slug must be unique within mirror.
    function addEntry(uint256 mirrorId, bytes8 bracket, string calldata slug) external onlyAdmin(mirrorId) {
        require(bracket[0] & 0x80 != 0, "Invalid sentinel byte");

        bytes32 entrySlugHash = keccak256(bytes(slug));
        require(_entrySlugIndex[mirrorId][entrySlugHash] == 0, "Entry slug already taken");

        _entries[mirrorId].push(MirrorEntry({bracket: bracket, slug: slug}));
        _entrySlugIndex[mirrorId][entrySlugHash] = _entries[mirrorId].length; // index + 1

        emit EntryAdded(mirrorId, _entries[mirrorId].length - 1, slug);
    }

    /// @notice Remove an entry (swap-and-pop). Admin only.
    function removeEntry(uint256 mirrorId, uint256 entryIndex) external onlyAdmin(mirrorId) {
        MirrorEntry[] storage entries = _entries[mirrorId];
        require(entryIndex < entries.length, "Index out of bounds");

        uint256 lastIndex = entries.length - 1;

        // Update slug mappings before swap
        bytes32 removedSlugHash = keccak256(bytes(entries[entryIndex].slug));
        delete _entrySlugIndex[mirrorId][removedSlugHash];

        if (entryIndex != lastIndex) {
            bytes32 lastSlugHash = keccak256(bytes(entries[lastIndex].slug));
            _entrySlugIndex[mirrorId][lastSlugHash] = entryIndex + 1;
            entries[entryIndex] = entries[lastIndex];
        }
        entries.pop();

        emit EntryRemoved(mirrorId, entryIndex);
    }

    /// @notice Update the bracket for an entry. Admin only.
    function updateBracket(uint256 mirrorId, uint256 entryIndex, bytes8 bracket) external onlyAdmin(mirrorId) {
        require(entryIndex < _entries[mirrorId].length, "Index out of bounds");
        require(bracket[0] & 0x80 != 0, "Invalid sentinel byte");

        _entries[mirrorId][entryIndex].bracket = bracket;

        emit BracketUpdated(mirrorId, entryIndex);
    }

    /// @notice Update the slug for an entry. Admin only. New slug must be unique within mirror.
    function updateEntrySlug(uint256 mirrorId, uint256 entryIndex, string calldata slug) external onlyAdmin(mirrorId) {
        require(entryIndex < _entries[mirrorId].length, "Index out of bounds");

        bytes32 oldSlugHash = keccak256(bytes(_entries[mirrorId][entryIndex].slug));
        bytes32 newSlugHash = keccak256(bytes(slug));

        if (oldSlugHash != newSlugHash) {
            require(_entrySlugIndex[mirrorId][newSlugHash] == 0, "Entry slug already taken");
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
        require(mirrorId != 0, "Mirror not found");
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
        require(index < _entries[mirrorId].length, "Index out of bounds");
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
        require(indexPlusOne != 0, "Entry not found");
        return _entries[mirrorId][indexPlusOne - 1];
    }

    function getEntries(uint256 mirrorId) external view mirrorExists(mirrorId) returns (MirrorEntry[] memory) {
        return _entries[mirrorId];
    }
}
