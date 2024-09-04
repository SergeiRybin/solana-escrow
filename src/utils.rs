use solana_program::account_info::AccountInfo;
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::rent::Rent;
use spl_token::state::Account;
use crate::state::SEED;

pub fn verify_pda(pda_account_info: &AccountInfo, program_id: &Pubkey) -> Result<(Pubkey, u8), ProgramError> {
    let (pda, bump_seed) = Pubkey::find_program_address(&[SEED], program_id);

    if pda != *pda_account_info.key {
        msg!("Incorrect PDA account provided to the instruction");
        return Err(ProgramError::InvalidAccountData);
    }

    Ok((pda, bump_seed))
}

pub fn verify_rent_exemption(account_info: &AccountInfo) -> Result<(), ProgramError> {
    if !Rent::default().is_exempt(**account_info.lamports.borrow(), account_info.data_len()) {
        msg!("Token account requires to be rent-exempted!");
        return Err(ProgramError::AccountNotRentExempt);
    };
    Ok(())
}