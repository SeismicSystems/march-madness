// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

import {Script, console} from "forge-std/Script.sol";
import {MarchMadness} from "../src/MarchMadness.sol";

/// @title MarchMadness deploy script (production)
contract MarchMadnessScript is Script {
    function run() public {
        vm.startBroadcast();

        MarchMadness mm = new MarchMadness(
            1 ether,        // entryFee
            1742313600,     // submissionDeadline: March 18, 2026 12:00 PM EST
            "QmTODO"        // IPFS hash placeholder
        );

        console.log("MarchMadness deployed at:", address(mm));

        vm.stopBroadcast();
    }
}
