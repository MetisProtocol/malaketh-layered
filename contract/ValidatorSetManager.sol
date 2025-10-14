// SPDX-License-Identifier: MIT
pragma solidity >=0.8.2 <0.9.0;

import "@openzeppelin/contracts-upgradeable/proxy/utils/Initializable.sol";
import "@openzeppelin/contracts-upgradeable/proxy/utils/UUPSUpgradeable.sol";
import "@openzeppelin/contracts-upgradeable/access/OwnableUpgradeable.sol";

contract ValidatorSetManagerV1 is Initializable, OwnableUpgradeable, UUPSUpgradeable {
    // Event definitions
    event ValidatorAdded(
        address indexed consensusAddress,
        address indexed operatorAddress,
        uint256 votingPower
    );
    event ValidatorRemoved(address indexed consensusAddress);

    // Struct definitions
    struct ValidatorInfo {
        address consensusAddress;
        address operatorAddress;
        uint256 votingPower;
        bytes32 publicKey;
    }

    // State variables
    mapping(address => ValidatorInfo) public validators;
    mapping(address => address) public consensusToOperator;
    mapping(uint256 => address[]) public epochValidators;
    address[] public activeValidators;
    uint256 public validatorNum;
    uint256 public epochLength;
    uint256 public updateHeight;
    address public admin;

    /// @custom:oz-upgrades-unsafe-allow constructor
    constructor() {
        _disableInitializers();
    }

    function initialize(
        address[] calldata consensusAddresses,
        address[] calldata operatorAddresses,
        uint256[] calldata initialPowers,
        bytes32[] calldata initialPublicKeys,
        uint256 _epochLength
    ) external initializer {
        __Ownable_init(msg.sender);
        admin = msg.sender;
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

    // UUPS 升级权限控制
    function _authorizeUpgrade(address) internal override onlyOwner {}

    // Query functions
    function getCurrentValidatorSetWithKeys() external view returns (
        address[] memory,
        address[] memory,
        uint256[] memory,
        bytes32[] memory
    ) {
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

    function getValidatorInfo(address validator) external view returns (ValidatorInfo memory) {
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
    function setEpochLength(uint256 newLength) external onlyOwner {
        require(newLength > 0, "Invalid epoch length");
        epochLength = newLength;
    }

    function setValidatorNum(uint256 newValidatorNum) external onlyOwner {
        require(newValidatorNum > 0, "Invalid validator number");
        validatorNum = newValidatorNum;
    }

    function setUpdateHeight(uint256 newHeight) external onlyOwner {
        updateHeight = newHeight;
    }

    function addValidator(
        address consensusAddress,
        address operatorAddress,
        uint256 votingPower,
        bytes32 publicKey
    ) external onlyOwner {
        _addValidator(consensusAddress, operatorAddress, votingPower, publicKey);
    }

    function removeValidator(address validator) external onlyOwner {
        _removeValidator(validator);
    }

    function _addValidator(
        address consensusAddress,
        address operatorAddress,
        uint256 votingPower,
        bytes32 publicKey
    ) internal {
        validators[consensusAddress] = ValidatorInfo({
            consensusAddress: consensusAddress,
            operatorAddress: operatorAddress,
            votingPower: votingPower,
            publicKey: publicKey
        });

        consensusToOperator[consensusAddress] = operatorAddress;
        updateHeight = block.number;
        activeValidators.push(consensusAddress);
        emit ValidatorAdded(consensusAddress, operatorAddress, votingPower);
    }

    function _removeValidator(address validator) internal {
        for (uint256 i = 0; i < activeValidators.length; i++) {
            if (activeValidators[i] == validator) {
                activeValidators[i] = activeValidators[activeValidators.length - 1];
                activeValidators.pop();
                break;
            }
        }
        updateHeight = block.number;
        emit ValidatorRemoved(validator);
    }
}
