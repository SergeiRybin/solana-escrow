# Solana Escrow Program

The **Solana Escrow Program** is a decentralized application (dApp) on the Solana blockchain that enables users to swap tokens in a secure and trustful environment.

## Features

- **Token Swap**: Allow users to securely swap tokens with a counterparty without the need for a trusted third party.
- **Requirements Verification**: Exchange won't happen till the on-chain program verifies all the parties requirements aligned.
- **Decentralized**: Fully on-chain logic ensures that swaps are executed transparently and without intermediaries.
- **Reclaim Option**: Initiators of a swap have the option to cancel the operation if the counterparty has not yet deposited their assets.

## How It Works

### 1. Prepare Account
First, users need to prepare an account with the tokens they wish to exchange. This account will specify the amount and the specific token mint.

### 2. Deposit Instruction
The initiator of the swap will then pass the account information (including token amount and mint) along with their requirements for the counterparty's assets by calling the `Deposit` instruction. This initiates the swap process.

### 3. Counterparty Preparation
The counterparty, upon agreeing to the terms, will prepare their own account with the assets and then call the `Execute` instruction. This instruction must include the requirements for the first user's assets.

### 4. Execution
The on-chain program checks if all requirements are met. If both parties' requirements align, the program swaps the accounts, allowing each user to gain ownership of the desired assets.

### 5. Reclaim Assets
If the initiator wishes to cancel the swap before the counterparty has deposited their assets, they can call the `Reclaim` instruction to retrieve their account and assets.

## Getting Started

To interact with the Solana Escrow Program, you will need:
- Solana SDK or
- Solana CLI tools
- Connection to Solana devnet/testnet/mainnet

## Test run
```shell
cargo build-bpf
BPF_OUT_DIR=<path_to_program_binary_file> cargo test

```
