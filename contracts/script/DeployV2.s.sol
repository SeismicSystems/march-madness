// SPDX-License-Identifier: MIT
pragma solidity ^0.8.30;

import {Script, console} from "forge-std/Script.sol";
import {MarchMadnessV2} from "../src/MarchMadnessV2.sol";
import {BracketGroupsV2} from "../src/BracketGroupsV2.sol";

/// @title DeployV2 — Deploy MarchMadnessV2 + BracketGroupsV2 for the migration cutover.
/// @notice Deploys with the same production 2026 parameters as the V1 contracts.
///         The submission deadline (1773940500) is already in the past, so `submitBracket`
///         is naturally closed — no additional guard needed.
///
/// Usage (testnet):
///   DEPLOYER_PRIVATE_KEY=0x... bun deploy:v2
///
/// After deploy, update data/deployments.json with the printed addresses.
contract DeployV2Script is Script {
    function run() public {
        vm.startBroadcast();

        MarchMadnessV2 mm = new MarchMadnessV2(
            2026, // year
            0.1 ether, // entryFee
            1773940500 // submissionDeadline: March 19, 2026 12:15 PM EST (already past)
        );
        console.log("MarchMadnessV2 deployed at:", address(mm));

        BracketGroupsV2 bg = new BracketGroupsV2(address(mm));
        console.log("BracketGroupsV2 deployed at:", address(bg));

        vm.stopBroadcast();
    }
}
