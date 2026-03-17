// SPDX-License-Identifier: MIT
pragma solidity ^0.8.30;

import {Script, console} from "forge-std/Script.sol";
import {BracketMirror} from "../src/BracketMirror.sol";

/// @title Deploy BracketMirror only (for redeployment after contract changes)
contract DeployMirrorScript is Script {
    function run() public {
        vm.startBroadcast();

        BracketMirror bm = new BracketMirror();
        console.log("BracketMirror deployed at:", address(bm));

        vm.stopBroadcast();
    }
}
