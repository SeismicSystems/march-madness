// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

import {Script, console} from "forge-std/Script.sol";
import {MarchMadness} from "../src/MarchMadness.sol";

/// @title MarchMadness deploy script (local development)
/// @dev Uses a deadline 1 hour from deployment time for local testing.
contract MarchMadnessLocalScript is Script {
    function run() public {
        vm.startBroadcast();

        MarchMadness mm = new MarchMadness(
            1 ether,                    // entryFee
            block.timestamp + 1 hours   // submissionDeadline: 1 hour from now
        );

        console.log("MarchMadness (local) deployed at:", address(mm));
        console.log("Submission deadline:", mm.submissionDeadline());

        vm.stopBroadcast();
    }
}
