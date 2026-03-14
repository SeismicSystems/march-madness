// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

/// @title ByteBracket — bracket scoring library
/// @notice Ported from jimpo's march-madness-dapp ByteBracket.sol
/// @dev Uses bytes32 instead of bytes8. Game bits occupy the first 8 bytes (same bit positions),
///      bytes 8-30 are zero, byte 31 is sentinel 0x01. Scoring operates on the uint64 extracted
///      from the first 8 bytes.
///
///      All bit manipulation functions use `unchecked` because jimpo's original code was written
///      for Solidity 0.5 (no overflow checks) and relies on intentional bit-level wrapping.
///
/// Algorithm description: https://drive.google.com/file/d/0BxHbbgrucCx2N1MxcnA1ZE1WQW8/view
/// Reference implementation: https://gist.github.com/pursuingpareto/b15f1197d96b1a2bbc48
library ByteBracket {
    /// @notice Score a bracket against the results using the precomputed scoring mask.
    /// @param bracket  A bytes32 bracket (game bits in first 8 bytes, sentinel in last byte).
    /// @param results  A bytes32 results (same layout).
    /// @param filter   The 64-bit scoring mask derived from `results` via `getScoringMask`.
    /// @return points  Total points scored (max 192).
    function getBracketScore(bytes32 bracket, bytes32 results, uint64 filter)
        internal
        pure
        returns (uint8 points)
    {
        unchecked {
            // Extract the first 8 bytes as uint64 for both bracket and results.
            uint64 bracketBits = uint64(uint256(bracket) >> 192);
            uint64 resultsBits = uint64(uint256(results) >> 192);

            uint8 roundNum = 0;
            uint8 numGames = 32;
            uint64 blacklist = (uint64(1) << numGames) - 1;
            uint64 overlap = uint64(~(bracketBits ^ resultsBits));

            while (numGames > 0) {
                uint64 scores = overlap & blacklist;
                points += popcount(scores) << roundNum;
                blacklist = pairwiseOr(scores & filter);
                overlap >>= numGames;
                filter >>= numGames;
                numGames /= 2;
                roundNum++;
            }
        }
    }

    /// @notice Compute the 64-bit scoring mask from a results bracket.
    /// @dev Extracts the first 8 bytes of the bytes32 as a bytes8, then mirrors jimpo's logic exactly.
    /// @param results  A bytes32 results bracket.
    /// @return mask    The 64-bit scoring mask.
    function getScoringMask(bytes32 results) internal pure returns (uint64 mask) {
        // Extract first 8 bytes as bytes8
        bytes8 r = bytes8(results);

        // Filter for the second most significant bit since MSB is ignored.
        bytes8 bitSelector = 0x4000000000000000;
        for (uint256 i = 0; i < 31; i++) {
            mask <<= 2;
            if (r & bitSelector != 0) {
                mask |= 1;
            } else {
                mask |= 2;
            }
            r <<= 1;
        }
    }

    /// @notice Returns a bitstring of half the length by taking bits two at a time and ORing them.
    /// @dev Separates the even and odd bits by repeatedly shuffling smaller segments of a bitstring.
    function pairwiseOr(uint64 bits) internal pure returns (uint64) {
        unchecked {
            uint64 tmp;
            tmp = (bits ^ (bits >> 1)) & 0x22222222;
            bits ^= (tmp ^ (tmp << 1));
            tmp = (bits ^ (bits >> 2)) & 0x0c0c0c0c;
            bits ^= (tmp ^ (tmp << 2));
            tmp = (bits ^ (bits >> 4)) & 0x00f000f0;
            bits ^= (tmp ^ (tmp << 4));
            tmp = (bits ^ (bits >> 8)) & 0x0000ff00;
            bits ^= (tmp ^ (tmp << 8));
            uint64 evens = bits >> 16;
            uint64 odds = bits % 0x10000;
            return evens | odds;
        }
    }

    /// @notice Counts the number of 1s in a bitstring.
    function popcount(uint64 bits) internal pure returns (uint8) {
        unchecked {
            bits -= (bits >> 1) & 0x5555555555555555;
            bits = (bits & 0x3333333333333333) + ((bits >> 2) & 0x3333333333333333);
            bits = (bits + (bits >> 4)) & 0x0f0f0f0f0f0f0f0f;
            return uint8(((bits * 0x0101010101010101) & 0xffffffffffffffff) >> 56);
        }
    }
}
