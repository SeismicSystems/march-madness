// SPDX-License-Identifier: MIT
pragma solidity ^0.8.30;

import {Script, console} from "forge-std/Script.sol";
import {MarchMadness} from "../src/MarchMadness.sol";
import {BracketGroups} from "../src/BracketGroups.sol";
import {BracketMirror} from "../src/BracketMirror.sol";

/// @title Deploy all contracts (local development)
/// @dev Pass deadline offset in seconds via DEADLINE_OFFSET env var (default: 3600 = 1 hour).
contract DeployAllLocalScript is Script {
    function run() public {
        uint256 deadlineOffset = vm.envOr("DEADLINE_OFFSET", uint256(3600));

        vm.startBroadcast();

        uint16 year = uint16(vm.envOr("YEAR", uint256(2026)));

        MarchMadness mm = new MarchMadness(year, 1 ether, block.timestamp + deadlineOffset);
        console.log("MarchMadness deployed at:", address(mm));
        console.log("Submission deadline:", mm.submissionDeadline());
        console.log("Deadline offset (seconds):", deadlineOffset);

        BracketGroups bg = new BracketGroups(address(mm));
        console.log("BracketGroups deployed at:", address(bg));

        BracketMirror bm = new BracketMirror();
        console.log("BracketMirror deployed at:", address(bm));

        vm.stopBroadcast();
    }
}
