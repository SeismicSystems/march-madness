// SPDX-License-Identifier: MIT
pragma solidity ^0.8.30;

import {Script, console} from "forge-std/Script.sol";
import {MarchMadness} from "../src/MarchMadness.sol";
import {BracketGroups} from "../src/BracketGroups.sol";
import {BracketMirror} from "../src/BracketMirror.sol";

/// @title Deploy all contracts (production)
/// @dev Deploys MarchMadness, BracketGroups (linked to MM), and BracketMirror (standalone).
///      ByteBracket is an internal library — inlined by the compiler, no separate deploy.
contract DeployAllScript is Script {
    function run() public {
        vm.startBroadcast();

        MarchMadness mm = new MarchMadness(
            2026, // year
            1 ether, // entryFee
            1773939600 // submissionDeadline: March 19, 2026 12:00 PM EST
        );
        console.log("MarchMadness deployed at:", address(mm));

        BracketGroups bg = new BracketGroups(address(mm));
        console.log("BracketGroups deployed at:", address(bg));

        BracketMirror bm = new BracketMirror();
        console.log("BracketMirror deployed at:", address(bm));

        vm.stopBroadcast();
    }
}
