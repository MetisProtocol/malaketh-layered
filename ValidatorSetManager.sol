pragma solidity >=0.8.2 <0.9.0;

contract ValidatorSetManager {
    // Event definitions
    event ValidatorAdded(
        address indexed consensusAddress,
        address indexed operatorAddress,
        uint256 votingPower
    );
    event ValidatorRemoved(address indexed consensusAddress);
    event ProxyUpgraded(
        address indexed oldImplementation,
        address indexed newImplementation
    );

    // Struct definitions
    struct ValidatorInfo {
        address consensusAddress; // Tendermint address for consensus
        address operatorAddress; // Ethereum address for operations
        uint256 votingPower;
        bytes32 publicKey; // Add public key field
    }

    // State variables
    mapping(address => ValidatorInfo) public validators; // consensus address => ValidatorInfo
    mapping(address => address) public consensusToOperator; // consensus address => operator address
    mapping(uint256 => address[]) public epochValidators;
    address[] public activeValidators;
    uint256 public validatorNum;
    uint256 public epochLength;
    uint256 public updateHeight;
    address public admin;
    address public implementation;
    address public proxyAdmin;

    constructor() {
        _addDefaultValidator(
            0x6DC44Cc1eAEF40776f07529DB710e630FD71809f,
            0x0754445aedA0441230D3ab099B0942181915186C,
            0x97007a7ab3b4ca24f8b88e6dceb764fe8bff810bf45fc16ef7bf0941fcbd7a27, // lwB6erO0yiT4uI5tzrdk/ov/gQv0X8Fu978JQfy9eic=
            1
        );

        _addDefaultValidator(
            0x816CB06248bA969a6dbb23c5A2f3059AdfF94ECf,
            0x3f8F2908B1B5B6Ef3eEC1968fCdF8340A6beC221,
            0xdac4b2f85de5e04c301a077b08256f659dddf36a39578361b1999df56237ab8e, // 2sSy+F3l4EwwGgd7CCVvZZ3d82o5V4NhsZmd9WI3q44=
            1
        );

        _addDefaultValidator(
            0x9F1069B39df29bbf8b6cbD5600069430EE894447,
            0x9Ab1A8B89460fCcd8Eb6739352300988915c71fe,
            0x1b494a5bc634bfa140c1f5b8f765c7c0203a5d3a73883542ec3dd0daafc36157, // G0lKW8Y0v6FAwfW492XHwCA6XTpziDVC7D3Q2q/DYVc=
            1
        );
        validatorNum = 10;
        epochLength = 100;
    }

    function _addDefaultValidator(
        address consensusAddress,
        address operatorAddress,
        bytes32 publicKey,
        uint256 votingPower
    ) private {
        _addValidator(
            consensusAddress,
            operatorAddress,
            votingPower,
            publicKey
        );
    }

    // // Modifiers
    modifier onlyAdmin() {
        require(msg.sender == admin, "Only admin");
        _;
    }

    modifier onlyProxyAdmin() {
        require(msg.sender == proxyAdmin, "Only proxy admin");
        _;
    }

    // Initialization functions
    function initialize(
        address[] calldata consensusAddresses,
        address[] calldata operatorAddresses,
        uint256[] calldata initialPowers,
        bytes32[] calldata initialPublicKeys,
        uint256 _epochLength
    ) external {
        require(admin == address(0), "Already initialized");
        admin = msg.sender;
        proxyAdmin = msg.sender;
        epochLength = _epochLength;
        validatorNum = 21;

        require(
            consensusAddresses.length == operatorAddresses.length &&
            consensusAddresses.length == initialPowers.length &&
            consensusAddresses.length == initialPublicKeys.length,
            "Invalid input"
        );
        require(consensusAddresses.length >= 3, "Need at least 3 validators");

        for (uint256 i = 0; i < consensusAddresses.length; i++) {
            _addValidator(
                consensusAddresses[i],
                operatorAddresses[i],
                initialPowers[i],
                initialPublicKeys[i]
            );
        }
    }

    // // Query functions
    // // Get validator set with public keys
    function getCurrentValidatorSetWithKeys() external view returns (
        address[] memory,
        address[] memory,
        uint256[] memory,
        bytes32[] memory
    )
    {
        address[] memory consensusAddresses = new address[](activeValidators.length);
        address[] memory operatorAddresses = new address[](activeValidators.length);
        uint256[] memory powers = new uint256[](activeValidators.length);
        bytes32[] memory publicKeys = new bytes32[](activeValidators.length);

        for (uint256 i = 0; i < activeValidators.length; i++) {
            consensusAddresses[i] = activeValidators[i];
            operatorAddresses[i] = validators[activeValidators[i]].operatorAddress;
            powers[i] = validators[activeValidators[i]].votingPower;
            publicKeys[i] = validators[activeValidators[i]].publicKey;
        }

        return (consensusAddresses, operatorAddresses, powers, publicKeys);
    }

    function getValidatorInfo(
        address validator
    ) external view returns (ValidatorInfo memory) {
        return validators[validator];
    }

    function getValidatorNum() external view returns (uint256) {
        return validatorNum;
    }

    function getValidatorCount() external view returns (uint256) {
        return activeValidators.length;
    }

    function getEpochLength() external view returns (uint256) {
        return epochLength;
    }

    function getUpdateHeight() external view returns (uint256) {
        return updateHeight;
    }

    // Management functions
    function setEpochLength(uint256 newLength) external onlyAdmin {
        require(newLength > 0, "Invalid epoch length");
        epochLength = newLength;
    }

    function setValidatorNum(uint256 newValidatorNum) external onlyAdmin {
        require(newValidatorNum > 0, "Invalid validator number");
        validatorNum = newValidatorNum;
    }

    function setUpdateHeight(uint256 newHeight) external onlyAdmin {
        updateHeight = newHeight;
    }

    // Proxy pattern implementation
    function upgradeTo(address newImplementation) external onlyProxyAdmin {
        require(newImplementation != address(0), "Invalid implementation");
        address oldImplementation = implementation;
        implementation = newImplementation;
        emit ProxyUpgraded(oldImplementation, newImplementation);
    }

    function setProxyAdmin(address newAdmin) external onlyProxyAdmin {
        require(newAdmin != address(0), "Invalid admin");
        proxyAdmin = newAdmin;
    }

    function AddValidator(
        address consensusAddress,
        address operatorAddress,
        uint256 votingPower,
        bytes32 publicKey
    ) external {
        _addValidator(
            consensusAddress,
            operatorAddress,
            votingPower,
            publicKey
        );
    }

    function RemoveValidator(address validator) external {
        _removeValidator(validator);
    }

    function _addValidator(
        address consensusAddress,
        address operatorAddress,
        uint256 votingPower,
        bytes32 publicKey
    ) internal {
        require(consensusAddress != address(0), "Invalid consensus address");
        require(operatorAddress != address(0), "Invalid operator address");
        require(validators[consensusAddress].consensusAddress == address(0), "Validator already exists");

        validators[consensusAddress] = ValidatorInfo({
            consensusAddress: consensusAddress,
            operatorAddress: operatorAddress,
            votingPower: votingPower,
            publicKey: publicKey
        });

        // Set mapping from consensus to operator address
        consensusToOperator[consensusAddress] = operatorAddress;

        // setUpdateHeight
        updateHeight = block.number;

        activeValidators.push(consensusAddress);
        emit ValidatorAdded(consensusAddress, operatorAddress, votingPower);
    }

    function _removeValidator(address validator) internal {
        require(validator != address(0), "Invalid validator address");
        require(validators[validator].consensusAddress != address(0), "Validator does not exist");

        // Remove from validators mapping
        delete validators[validator];
        delete consensusToOperator[validator];

        // Remove from activeValidators array
        for (uint256 i = 0; i < activeValidators.length; i++) {
            if (activeValidators[i] == validator) {
                activeValidators[i] = activeValidators[activeValidators.length - 1];
                activeValidators.pop();
                break;
            }
        }

        // setUpdateHeight
        updateHeight = block.number;

        emit ValidatorRemoved(validator);
    }
}
