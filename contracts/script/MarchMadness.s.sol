// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

import {Script, console} from "forge-std/Script.sol";
import {MarchMadness} from "../src/MarchMadness.sol";

/// @title MarchMadness deploy script (production)
/// @dev ByteBracket is an internal library — it gets inlined by the compiler
///      into MarchMadness, so no separate deployment is needed.
contract MarchMadnessScript is Script {
    function run() public {
        vm.startBroadcast();

        MarchMadness mm = new MarchMadness(
            2026, // year
            1 ether, // entryFee
            1773853200 // submissionDeadline: March 18, 2026 12:00 PM EST
        );

        console.log("MarchMadness deployed at:", address(mm));

        vm.stopBroadcast();
    }
}
