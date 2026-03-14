// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

import {Script, console} from "forge-std/Script.sol";
import {MarchMadness} from "../src/MarchMadness.sol";

/// @title MarchMadness deploy script (local development)
/// @dev Pass deadline offset in seconds via DEADLINE_OFFSET env var (default: 3600 = 1 hour).
///      Usage: sforge script script/MarchMadnessLocal.s.sol --rpc-url ... --broadcast
///      Custom: DEADLINE_OFFSET=300 sforge script ...  (5 min deadline)
///
///      ByteBracket is an internal library — inlined by the compiler, no separate deploy needed.
contract MarchMadnessLocalScript is Script {
    function run() public {
        uint256 deadlineOffset = vm.envOr("DEADLINE_OFFSET", uint256(3600));

        vm.startBroadcast();

        MarchMadness mm = new MarchMadness(
            1 ether,
            block.timestamp + deadlineOffset
        );

        console.log("MarchMadness (local) deployed at:", address(mm));
        console.log("Submission deadline:", mm.submissionDeadline());
        console.log("Deadline offset (seconds):", deadlineOffset);

        vm.stopBroadcast();
    }
}
