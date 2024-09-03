use crate::error::EscrowError;
use crate::state::{Escrow, DATA_LEN, SEED};
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::program::{invoke, invoke_signed};
use solana_program::program_pack::Pack;
use solana_program::pubkey::{Pubkey};
use solana_program::sysvar::{rent::Rent, Sysvar};
use solana_program::{msg, system_instruction, system_program};
use solana_program::program_error::ProgramError;
use spl_token::instruction::set_authority;

enum EscrowInstruction<'a> {
    Init {
        seed: &'a [u8],
        bump_seed: u8
    },
    Deposit {
        amount_expected: u32,
    },
    Execute {
        amount: u32,
    },
    Cancel,
}

fn parse_data(instruction_data: &[u8]) -> Result<EscrowInstruction, ProgramError> {
    let data_len = instruction_data.len();
    assert!(data_len > 0);
    let instruction = instruction_data[0];
    match instruction {
        0 => Ok(EscrowInstruction::Init {
            seed: &instruction_data[1..data_len - 1],
            bump_seed: instruction_data[data_len - 1]
        }),
        1 => {
            // Todo: make transmute
            Ok(EscrowInstruction::Deposit {
                amount_expected: 5
            })
        }
        _ => Err(ProgramError::InvalidInstructionData)
    }
}
fn init_escrow(
    accounts: &[AccountInfo],
    program_id: &Pubkey,
    seed: &[u8],
    bump_seed: u8,
) -> ProgramResult {

    let account_info_iter = &mut accounts.iter();
    let payer_account_info = next_account_info(account_info_iter)?;
    let pda_account_info = next_account_info(account_info_iter)?;
    let system_account = next_account_info(account_info_iter)?;

    assert!(payer_account_info.is_signer);
    assert!(payer_account_info.is_writable);
    assert!(!pda_account_info.is_signer);
    assert!(pda_account_info.is_writable);
    assert!(system_program::check_id(system_account.key));

    match pda_account_info.data_is_empty() {
        true => {
            let space = DATA_LEN;
            let rent_lamports = Rent::default().minimum_balance(space);
            invoke_signed(
                &system_instruction::create_account(
                    payer_account_info.key,
                    pda_account_info.key,
                    rent_lamports,
                    space as u64,
                    program_id
                ),
                &[payer_account_info.clone(), pda_account_info.clone(), system_account.clone()],
                &[&[payer_account_info.key.as_ref(), seed, &[bump_seed]]],
            )?;
        }
        _ =>
            {
                msg!("PDA account already exists");
                return Err(EscrowError::PdaExists.into())
            }
    }
    // Transfer tokens
    // Return escrow id
    Ok(())
}

fn deposit(accounts: &[AccountInfo], program_id: &Pubkey, amount_expected: u32) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let payer_account_info = next_account_info(account_info_iter)?;
    let pda_account_info = next_account_info(account_info_iter)?;
    let owner_account_info = next_account_info(account_info_iter)?;
    let token_account_info = next_account_info(account_info_iter)?;
    let token_program = next_account_info(account_info_iter)?;
    let token_expected = next_account_info(account_info_iter)?;

    assert!(owner_account_info.is_signer);
    assert!(pda_account_info.is_writable);
    assert_eq!(*token_account_info.owner, spl_token::id());
    assert!(token_account_info.is_writable);

    let mut escrow_account = Escrow{
        active: true,
        token_expected: token_expected.key.clone(),
        amount_expected,
        holding_account: token_account_info.key.clone(),
        owner_account: owner_account_info.key.clone(),
    };

    if pda_account_info.owner != program_id {
        return Err(ProgramError::IncorrectProgramId);
    }

    escrow_account.pack_into_slice(&mut pda_account_info.try_borrow_mut_data()?);

    // TODO: Make assertions in one style
    let token_account_data = spl_token::state::Account::unpack(&token_account_info.data.borrow())?;

    // Check if the owner matches the expected owner
    if token_account_data.owner != *owner_account_info.key {
        msg!("The provided wallet is not the owner of this token account.");
        return Err(ProgramError::IllegalOwner);
    }

    let (pda, _nonce) = Pubkey::find_program_address(&[b"escrow"], program_id);

    let owner_change_ix = set_authority(
        token_program.key,
        token_account_info.key,
        Some(&pda),
        spl_token::instruction::AuthorityType::AccountOwner,
        owner_account_info.key,
        &[&owner_account_info.key],
    )?;

    msg!("Calling the token program to transfer token account ownership...");
    invoke(
        &owner_change_ix,
        &[
            token_account_info.clone(),
            owner_account_info.clone(),
            token_program.clone(),
        ],
    )?;

    Ok(())
}

pub fn process(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    // unpack instruction
    let instruction = parse_data(instruction_data)?;

    match instruction {
        EscrowInstruction::Init {
            seed,
            bump_seed,
        } => {
            msg!("Init escrow request...");
            return init_escrow(accounts, program_id, seed, bump_seed);
        }
        EscrowInstruction::Deposit {
            amount_expected,
        } => {
            msg!("Deposit instruction...");
            return deposit(accounts, program_id, amount_expected);
        }
        EscrowInstruction::Execute { .. } => {
            msg!("Execute escrow request...");
        }
        EscrowInstruction::Cancel => {
            msg!("Escrow account is closed, tokens returned to");
            // TODO: print return account
        }
    }

    Ok(())
}
