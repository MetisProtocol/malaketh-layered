// SPDX-License-Identifier: MIT
pragma solidity >=0.8.2 <0.9.0;

contract ValidatorSetManager {
    // Event definitions
    event ValidatorAdded(address indexed validator, uint256 votingPower);
    event ValidatorRemoved(address indexed validator);
    event ProxyUpgraded(
        address indexed oldImplementation,
        address indexed newImplementation
    );

    // Struct definitions
    struct ValidatorInfo {
        address validator;
        uint256 votingPower;
        bytes32 publicKey; // Add public key field
    }

    // State variables
    mapping(address => ValidatorInfo) public validators;
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
            0x0754445aedA0441230D3ab099B0942181915186C,
            "lwB6erO0yiT4uI5tzrdk/ov/gQv0X8Fu978JQfy9eic=",
            1
        );
        
        _addDefaultValidator(
            0x3f8F2908B1B5B6Ef3eEC1968fCdF8340A6beC221,
            "2sSy+F3l4EwwGgd7CCVvZZ3d82o5V4NhsZmd9WI3q44=",
            1
        );

        _addDefaultValidator(
            0x9Ab1A8B89460fCcd8Eb6739352300988915c71fe,
            "G0lKW8Y0v6FAwfW492XHwCA6XTpziDVC7D3Q2q/DYVc=",
            1
        );
    }

    function _addDefaultValidator(
        address validator, 
        string memory publicKey, 
        uint256 votingPower
    ) private {
        bytes32 publicKeyBytes = _base64ToBytes32(publicKey);
        _addValidator(validator, votingPower, publicKeyBytes);
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

    // // Initialization functions
    function initialize(
        address[] calldata initialValidators,
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
            initialValidators.length == initialPowers.length &&
            initialValidators.length == initialPublicKeys.length,
            "Invalid input"
        );
        require(initialValidators.length >= 3, "Need at least 3 validators");

        for (uint256 i = 0; i < initialValidators.length; i++) {
            _addValidator(initialValidators[i], initialPowers[i], initialPublicKeys[i]);
        }
    }

    // // Query functions
    // // Get validator set with public keys
    function getCurrentValidatorSetWithKeys()
        external
        view
        returns (address[] memory, uint256[] memory, bytes32[] memory)
    {
        address[] memory validators_list = new address[](
            activeValidators.length
        );
        uint256[] memory powers = new uint256[](activeValidators.length);
        bytes32[] memory publicKeys = new bytes32[](activeValidators.length);

        for (uint256 i = 0; i < activeValidators.length; i++) {
            validators_list[i] = activeValidators[i];
            powers[i] = validators[activeValidators[i]].votingPower;
            publicKeys[i] = validators[activeValidators[i]].publicKey;
        }

        return (validators_list, powers, publicKeys);
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

    // // Proxy pattern implementation
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

    function AddValidatorBase64(
        address validator,
        uint256 votingPower,
        string calldata publicKey
    ) external {
        bytes32 publicKeyBytes = _base64ToBytes32(publicKey);
        _addValidator(validator, votingPower, publicKeyBytes);
    }

    // Internal functions
    // Helper function to convert base64 string to bytes32
    function _base64ToBytes32(string memory base64String) internal pure returns (bytes32) {
        // This is a simplified implementation
        // In practice, you'd need a proper base64 decoder
        bytes memory data = bytes(base64String);
        require(data.length == 44, "Invalid base64 length"); // 32 bytes = 44 base64 chars

        // For now, we'll use a simple approach - hash the string
        // In a real implementation, you'd decode the base64 properly
        return keccak256(data);
    }
    
    function _addValidator(
        address validator,
        uint256 votingPower,
        bytes32 publicKey
    ) internal {
        validators[validator] = ValidatorInfo({
            validator: validator,
            votingPower: votingPower,
            publicKey: publicKey
        });

        // todo setUpdateHeight

        activeValidators.push(validator);
        emit ValidatorAdded(validator, votingPower);
    }

    function _removeValidator(address validator) internal {
        // Remove from activeValidators array
        for (uint256 i = 0; i < activeValidators.length; i++) {
            if (activeValidators[i] == validator) {
                activeValidators[i] = activeValidators[
                    activeValidators.length - 1
                ];
                activeValidators.pop();
                break;
            }
        }

        // todo setUpdateHeight

        emit ValidatorRemoved(validator);
    }
}