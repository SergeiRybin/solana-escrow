use crate::error::EscrowError;
use crate::state::{Escrow, DATA_LEN, SEED};
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::program::invoke_signed;
use solana_program::program_error::ProgramError;
use solana_program::program_pack::Pack;
use solana_program::pubkey::{Pubkey, PubkeyError};
use solana_program::system_instruction::create_account_with_seed;
use solana_program::sysvar::instructions::load_current_index_checked;
use solana_program::sysvar::{rent::Rent, Sysvar};
use solana_program::{msg, system_instruction};

enum EscrowInstruction {
    Init {
        amount_put: u32,
        amount_expected: u32,
    },
    Execute {
        amount: u32,
    },
    Cancel,
}

fn parse_data(instruction_data: &[u8]) -> EscrowInstruction {
    EscrowInstruction::Init {
        amount_put: 0,
        amount_expected: 0,
    }
}
fn init_escrow(
    accounts: &[AccountInfo],
    program_id: &Pubkey,
    amount_put: u32,
    amount_expected: u32,
) -> ProgramResult {
    //TODO: Make checks

    // Find PDA account
    let account_info_iter = &mut accounts.iter();
    let payer_account_info = next_account_info(account_info_iter)?;
    let pda_account_info = next_account_info(account_info_iter)?;
    if pda_account_info.data_is_empty() {
        return Err(EscrowError::PdaExists.into());
    } else {
        let escrow_data = pda_account_info.try_borrow_data()?;
        let escrow = Escrow::unpack_from_slice(*escrow_data)?;
    }
    // Transfer tokens
    // Return escrow id
    Ok(())
}

fn init_program(accounts: &[AccountInfo], program_id: &Pubkey) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let payer_account_info = next_account_info(account_info_iter)?;
    let escrow_account_info = next_account_info(account_info_iter)?;
    if !escrow_account_info.data_is_empty() {
        return Err(EscrowError::PdaExists.into());
    }
    let rent_sysvar_account_info = &Rent::from_account_info(next_account_info(account_info_iter)?)?;

    // find space and minimum rent required for account
    let space = DATA_LEN;
    let bump = SEED;
    let rent_lamports = rent_sysvar_account_info.minimum_balance(space.into());

    invoke_signed(
        &system_instruction::create_account(
            &payer_account_info.key,
            &escrow_account_info.key,
            rent_lamports,
            space as u64,
            program_id,
        ),
        &[payer_account_info.clone(), escrow_account_info.clone()],
        &[&[&payer_account_info.key.as_ref(), bump]],
    )?;

    let escrow_account = Escrow::default();

    escrow_account.pack_into_slice(&mut escrow_account_info.try_borrow_mut_data()?);
    msg!("PDA account is created: {}", escrow_account_info.key);
    Ok(())
}

fn process(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    // unpack instruction
    let instruction = parse_data(instruction_data);
    //

    match instruction {
        EscrowInstruction::Init {
            amount_put,
            amount_expected,
        } => {
            println!("Init escrow request...");
            return init_escrow(accounts, program_id, amount_put, amount_expected);
        }
        EscrowInstruction::Execute { .. } => {
            println!("Execute escrow request...");
        }
        EscrowInstruction::Cancel => {
            println!("Escrow account is closed, tokens returned to");
            // TODO: print return account
        }
    }

    Ok(())
}
