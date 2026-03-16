// SPDX-License-Identifier: MIT
pragma solidity ^0.8.30;

import {Script, console} from "forge-std/Script.sol";
import {BracketGroups} from "../src/BracketGroups.sol";

/// @title Deploy BracketGroups only against an existing MarchMadness contract
contract DeployBracketGroupsScript is Script {
    function run() public {
        address marchMadness = vm.envAddress("MARCH_MADNESS_ADDRESS");

        vm.startBroadcast();

        BracketGroups bg = new BracketGroups(marchMadness);
        console.log("MarchMadness address:", marchMadness);
        console.log("BracketGroups deployed at:", address(bg));

        vm.stopBroadcast();
    }
}
