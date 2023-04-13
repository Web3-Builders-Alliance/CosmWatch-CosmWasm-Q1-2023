# CosmWasm Escrow Contract with Milestones

`cw-escrow-milestones` is a CosmWasm smart contract that extends the functionality of the `cw-escrow` contract to support multiple milestones for payments using native and CW20 tokens. The contract allows users to create escrows with multiple milestones and release funds to the recipient only when the specified conditions for each milestone are met.

## Table of Contents

- [Features](#features)
- [Quick Start](#quick-start)
- [ContractError](#contracterror)
- [Contract Functions](#contract-functions)
  - [Instantiate](#instantiate)
  - [Create Escrow](#create-escrow)
  - [Approve Milestone](#approve-milestone)
  - [Release Milestone](#release-milestone)
  - [Refund Escrow](#refund-escrow)
  - [Query Functions](#query-functions)
    - [Get Escrow](#get-escrow)
    - [Get Milestone](#get-milestone)

## Features

- Create escrows with multiple milestones.
- Support for whitelisted CW20 tokens.
- Approve milestones individually.
- Release funds for approved milestones.
- Refund remaining balance if escrow expires.

## Quick Start

1. Install Rust and the required dependencies as described in the [CosmWasm documentation](https://book.cosmwasm.com/setting-up-env.html).
2. Clone this repository.
3. Navigate to `CosmWatch-CosmWasm-Q1-2023/Capstone/cw-escrow-milestones`
4. Run unit and integration tests via: `cargo test`
5. Build the contract via (assumes you are in the root of the cw-escrow-milestones directory): 
    ```bash
    docker run --rm -v "$(pwd)":/code \
    --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
    --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
    --platform linux/amd64 \
    cosmwasm/rust-optimizer:0.12.3

    ls -l artifacts/cw20_escrow_milestones.wasm

    sha256sum artifacts/cw20_escrow_milestones.wasm
    ```
6. Deploy the contract to your desired chain. Use a tool like [cosmwasm.tools](https://cosmwasm.tools/) to reduce friction and speed up the process.
7. Interact with the contract using the available functions seen in the [Contract Functions](#contract-functions) section.



## Contract Functions

### **Instantiate**

No arguments are required to instantiate the contract.

### **Execute Messages**

**Create**

- **CreateMsg**: Create a new escrow with milestones.
    - **id**: Unique identifier for the escrow.
    - **arbiter**: Address of the arbiter who can approve or refund milestones.
    - **recipient**: Optional recipient address.
    - **milestones**: List of milestones with details.
    - **end_height**: Optional escrow expiration height.
    - **end_time**: Optional escrow expiration time.

**CreateMilestone**
- **CreateMilestoneMsg**: Add a new milestone to an existing escrow.
    - **escrow_id**: The ID of the escrow to add the milestone to.
    - **amount**: The amount to be released upon milestone completion.
    - **description**: Description of the milestone.
    - **end_height**: Optional milestone expiration height.
    - **end_time**: Optional milestone expiration time.

**SetRecipient**
- **SetRecipient**: Set the recipient for an existing escrow.
    - **id**: The ID of the escrow.
    - **recipient**: The recipient address.

**ApproveMilestone**
- **ApproveMilestone**: Approve a milestone, releasing funds to the recipient.
    - **id**: The ID of the escrow.
    - **milestone_id**: The ID of the milestone to approve.

**ExtendMilestone**
- **ExtendMilestone**: Extend the deadline of a milestone.
    - **id**: The ID of the escrow.
    - **milestone_id**: The ID of the milestone to extend.
    - **end_height**: New milestone expiration height (optional).
    - **end_time**: New milestone expiration time (optional).

**Refund**
- **Refund**: Refund the remaining escrow balance to the sender.
    - **id**: The ID of the escrow.

### **Query Messages**
**List**
- **List**: Retrieve a list of all escrow IDs.

**Details**
- **Details**: Retrieve escrow details.
    - **id**: The ID of the escrow.

**ListMilestones**
- **ListMilestones**: Retrieve a list of all milestones for an escrow.
    - **id**: The ID of the escrow.

### **Contract Errors**

- **Std**: Wraps a standard error from the cosmwasm_std library.
- **Unauthorized**: Error when an unauthorized action is attempted.
- **NotInWhitelist**: Error when a token is not in the whitelist.
- **Expired**: Error when an escrow has expired.
- **NotFound**: Error when an escrow is not found.
- **InvalidAddress**: Error when an address is invalid.
- **EmptyBalance**: Error when an escrow is created with an empty balance.
- **FundsMismatch**: Error when the funds sent do not equal the total amount of all milestones.
- **AlreadyInUse**: Error when an escrow ID is already in use.
- **RecipientNotSet**: Error when a recipient is not set.
- **MilestoneNotFound**: Error when a milestone is not found.
- **MilestoneExpired**: Error when a milestone has expired.
- **EmptyMilestones**: Error when milestones are empty.
