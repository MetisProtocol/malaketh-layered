pragma solidity >=0.8.2 <0.9.0;

contract ValidatorSetManager {
    // Event definitions
    event ValidatorAdded(address indexed validator, uint256 votingPower);
    event ValidatorRemoved(address indexed validator);
    event ValidatorUpdated(
        address indexed validator,
        uint256 oldPower,
        uint256 newPower
    );
    event EpochUpdated(uint256 indexed epoch, address[] validators);
    event Slash(address indexed validator, uint256 amount, string reason);
    event FeeDistributed(address indexed validator, uint256 amount);
    event ProxyUpgraded(
        address indexed oldImplementation,
        address indexed newImplementation
    );

    // Struct definitions
    struct ValidatorInfo {
        address validator;
        uint256 votingPower;
        uint256 stakedAmount;
        bool isActive;
        uint256 lastUpdateEpoch;
        uint256 totalRewards;
        uint256 slashCount;
        bytes32 publicKey; // Add public key field
    }

    // State variables
    mapping(address => ValidatorInfo) public validators;
    mapping(uint256 => address[]) public epochValidators;
    address[] public activeValidators;
    uint256 public currentEpoch;
    uint256 public epochLength;
    uint256 public minStakeAmount;
    uint256 public totalStaked;
    address public admin;
    address public implementation;
    address public proxyAdmin;

    // Modifiers
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
        address[] calldata initialValidators,
        uint256[] calldata initialPowers,
        bytes32[] calldata initialPublicKeys,
        uint256 _epochLength,
        uint256 _minStakeAmount
    ) external {
        require(admin == address(0), "Already initialized");
        admin = msg.sender;
        proxyAdmin = msg.sender;
        epochLength = _epochLength;
        minStakeAmount = _minStakeAmount;
        currentEpoch = 0;

        require(
            initialValidators.length == initialPowers.length &&
            initialValidators.length == initialPublicKeys.length,
            "Invalid input"
        );
        require(initialValidators.length >= 3, "Need at least 3 validators");

        for (uint256 i = 0; i < initialValidators.length; i++) {
            _addValidator(initialValidators[i], initialPowers[i], 0, initialPublicKeys[i]);
        }

        _updateEpochValidators();
    }

    // Staking functions
    function stake(bytes32 publicKey) external payable {
        require(msg.value >= minStakeAmount, "Insufficient stake amount");

        ValidatorInfo storage validator = validators[msg.sender];
        if (validator.validator == address(0)) {
            // New validator
            _addValidator(msg.sender, msg.value, msg.value, publicKey);
        } else {
            // Existing validator increases stake
            validator.stakedAmount += msg.value;
            validator.votingPower = validator.stakedAmount;
            validator.lastUpdateEpoch = currentEpoch;
        }

        totalStaked += msg.value;
        emit ValidatorUpdated(
            msg.sender,
            validator.votingPower - msg.value,
            validator.votingPower
        );
    }

    function unstake(uint256 amount) external {
        ValidatorInfo storage validator = validators[msg.sender];
        require(validator.validator != address(0), "Not a validator");
        require(amount <= validator.stakedAmount, "Insufficient staked amount");
        require(
            validator.stakedAmount - amount >= minStakeAmount ||
                amount == validator.stakedAmount,
            "Below minimum stake"
        );

        validator.stakedAmount -= amount;
        validator.votingPower = validator.stakedAmount;
        validator.lastUpdateEpoch = currentEpoch;
        totalStaked -= amount;

        if (validator.stakedAmount == 0) {
            _removeValidator(msg.sender);
        }

        payable(msg.sender).transfer(amount);
        emit ValidatorUpdated(
            msg.sender,
            validator.votingPower + amount,
            validator.votingPower
        );
    }

    // Validator Set management
    function updateValidatorSet() external {
        require(block.number % epochLength == 0, "Not epoch end");

        currentEpoch++;
        _updateEpochValidators();

        // Clean up inactive validators
        for (uint256 i = activeValidators.length; i > 0; i--) {
            address validator = activeValidators[i - 1];
            if (validators[validator].stakedAmount < minStakeAmount) {
                _removeValidator(validator);
            }
        }

        emit EpochUpdated(currentEpoch, activeValidators);
    }

    // Slashing mechanism
    function slashValidator(
        address validator,
        uint256 amount,
        string calldata reason
    ) external onlyAdmin {
        ValidatorInfo storage val = validators[validator];
        require(val.validator != address(0), "Validator not found");
        require(amount <= val.stakedAmount, "Slash amount too large");

        val.stakedAmount -= amount;
        val.votingPower = val.stakedAmount;
        val.slashCount++;
        val.lastUpdateEpoch = currentEpoch;
        totalStaked -= amount;

        if (val.stakedAmount < minStakeAmount) {
            _removeValidator(validator);
        }

        emit Slash(validator, amount, reason);
    }

    // Fee distribution
    function distributeFees() external payable {
        require(msg.value > 0, "No fees to distribute");
        require(activeValidators.length > 0, "No active validators");

        uint256 feePerValidator = msg.value / activeValidators.length;
        uint256 remainder = msg.value % activeValidators.length;

        for (uint256 i = 0; i < activeValidators.length; i++) {
            address validator = activeValidators[i];
            uint256 amount = feePerValidator;
            if (i == activeValidators.length - 1) {
                amount += remainder; // Give remainder to last validator
            }

            validators[validator].totalRewards += amount;
            payable(validator).transfer(amount);
            emit FeeDistributed(validator, amount);
        }
    }

    // Query functions
    function getCurrentValidatorSet()
        external
        view
        returns (address[] memory, uint256[] memory)
    {
        address[] memory validators_list = new address[](
            activeValidators.length
        );
        uint256[] memory powers = new uint256[](activeValidators.length);

        for (uint256 i = 0; i < activeValidators.length; i++) {
            validators_list[i] = activeValidators[i];
            powers[i] = validators[activeValidators[i]].votingPower;
        }

        return (validators_list, powers);
    }

    // Get validator set with public keys
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

    function getEpochLength() external view returns (uint256) {
        return epochLength;
    }

    function getActiveValidatorCount() external view returns (uint256) {
        return activeValidators.length;
    }

    function getTotalStaked() external view returns (uint256) {
        return totalStaked;
    }

    // Management functions
    function setEpochLength(uint256 newLength) external onlyAdmin {
        require(newLength > 0, "Invalid epoch length");
        epochLength = newLength;
    }

    function setMinStakeAmount(uint256 newAmount) external onlyAdmin {
        minStakeAmount = newAmount;
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

    // Internal functions
    function _addValidator(
        address validator,
        uint256 votingPower,
        uint256 stakedAmount,
        bytes32 publicKey
    ) internal {
        validators[validator] = ValidatorInfo({
            validator: validator,
            votingPower: votingPower,
            stakedAmount: stakedAmount,
            isActive: true,
            lastUpdateEpoch: currentEpoch,
            totalRewards: 0,
            slashCount: 0,
            publicKey: publicKey
        });

        activeValidators.push(validator);
        emit ValidatorAdded(validator, votingPower);
    }

    function _removeValidator(address validator) internal {
        ValidatorInfo storage val = validators[validator];
        val.isActive = false;
        val.lastUpdateEpoch = currentEpoch;

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

        emit ValidatorRemoved(validator);
    }

    function _updateEpochValidators() internal {
        epochValidators[currentEpoch] = activeValidators;
    }
}
