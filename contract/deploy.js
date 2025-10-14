const { ethers, upgrades } = require("hardhat");

async function main() {
    const ValidatorSetManagerV1 = await ethers.getContractFactory("ValidatorSetManagerV1");

    const consensusAddresses = [
        "0x6DC44Cc1eAEF40776f07529DB710e630FD71809f",
        "0x816CB06248bA969a6dbb23c5A2f3059AdfF94ECf",
        "0x9F1069B39df29bbf8b6cbD5600069430EE894447"
    ];
    const operatorAddresses = [
        "0x0754445aedA0441230D3ab099B0942181915186C",
        "0x3f8F2908B1B5B6Ef3eEC1968fCdF8340A6beC221",
        "0x9Ab1A8B89460fCcd8Eb6739352300988915c71fe"
    ];
    const initialPowers = [1, 1, 1];
    const initialPublicKeys = [
        "0x97007a7ab3b4ca24f8b88e6dceb764fe8bff810bf45fc16ef7bf0941fcbd7a27",
        "0xdac4b2f85de5e04c301a077b08256f659dddf36a39578361b1999df56237ab8e",
        "0x1b494a5bc634bfa140c1f5b8f765c7c0203a5d3a73883542ec3dd0daafc36157"
    ];
    const epochLength = 100;

    const proxy = await upgrades.deployProxy(ValidatorSetManagerV1, [
        consensusAddresses,
        operatorAddresses,
        initialPowers,
        initialPublicKeys,
        epochLength
    ], { kind: 'uups' });

    await proxy.deployed();
    console.log("ValidatorSetManagerV1 UUPS proxy deployed to:", proxy.address);
}

main();
