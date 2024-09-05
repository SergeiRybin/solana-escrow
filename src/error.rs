use solana_program::msg;
use solana_program::program_error::ProgramError;
use thiserror::Error;

#[derive(Error, Debug, Copy, Clone)]
pub enum EscrowError {
    /// Invalid instruction
    #[error("Invalid Instruction")]
    InvalidInstruction,

    #[error("PDA already exists")]
    PdaExists,

    #[error("Escrow program is not initialized")]
    NotInitialized,

    #[error("Executor expects another amount of token deposited")]
    DepositTokenAmtMismatch,

    #[error("Executor expects another token mint from deposit")]
    DepositTokenMintMismatch,

    #[error("Depositor expected another amount of token passed by the executor")]
    ExecutorTokenAmtMismatch,

    #[error("Depositor expected another token mint passed by the executor")]
    ExecutorTokenMintMismatch,

    #[error("No available escrow accounts")]
    NoAvailableEscrowAccounts,
}

pub fn throw_and_log(error: EscrowError) -> ProgramError {
    msg!("{}", error);
    error.into()
}

impl From<EscrowError> for ProgramError {
    fn from(value: EscrowError) -> Self {
        ProgramError::Custom(value as u32)
    }
}
