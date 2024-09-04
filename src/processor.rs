use crate::error::{EscrowError, throw_and_log};
use crate::state::{Escrow, DATA_LEN, SEED};
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::program::{invoke, invoke_signed};
use solana_program::program_pack::Pack;
use solana_program::pubkey::{Pubkey};
use solana_program::sysvar::{rent::Rent, Sysvar};
use solana_program::{msg, system_instruction, system_program};
use solana_program::instruction::AccountMeta;
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
        amount_expected: u32,
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
        2 => {
            Ok(EscrowInstruction::Execute {
                amount_expected: 10
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
    // TODO: Add additional checks as rent exemption for deposit account
    let token_account_data = spl_token::state::Account::unpack(&token_account_info.data.borrow())?;

    // Check if the owner matches the expected owner
    if token_account_data.owner != *owner_account_info.key {
        msg!("The provided wallet is not the owner of this token account.");
        return Err(ProgramError::IllegalOwner);
    }

    let owner_change_ix = set_authority(
        token_program.key,
        token_account_info.key,
        Some(&pda_account_info.key),
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

fn execute(accounts: &[AccountInfo], program_id: &Pubkey, amount_expected: u32) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let payer_account_info = next_account_info(account_info_iter)?;
    let pda_account_info = next_account_info(account_info_iter)?;
    let owner_account_info = next_account_info(account_info_iter)?;
    let token_account_info = next_account_info(account_info_iter)?;
    let token_program = next_account_info(account_info_iter)?;
    let token_expected = next_account_info(account_info_iter)?;
    let deposit_account_info = next_account_info(account_info_iter)?;

    assert!(deposit_account_info.is_writable);
    assert!(token_account_info.is_writable);

    let mut escrow = Escrow::unpack_from_slice(*pda_account_info.try_borrow_data()?)?;
    let token_account_data = spl_token::state::Account::unpack(&token_account_info.data.borrow())?;
    //TODO: remove payer from PDA calculation
    let (pda, bump_seed) = Pubkey::find_program_address(&[payer_account_info.key.as_ref(), SEED], program_id);

    if pda != *pda_account_info.key {
        msg!("Incorrect PDA account provided to the instruction");
        return Err(ProgramError::InvalidAccountData);
    }

    let deposit_account_data = spl_token::state::Account::unpack(&deposit_account_info.data.borrow())?;

    if deposit_account_data.owner != pda {
        msg!("The provided owner is not the real owner of user token account.");
        return Err(ProgramError::IllegalOwner);
    }

    if !escrow.active {
        return Err(throw_and_log(EscrowError::NotInitialized));
    }

    // Actors' expectations checks
    if escrow.amount_expected != token_account_data.amount as u32 {
        msg!("Error: Depositor and executor expectations are not met");
        msg!("Depositor expected: {} tokens", escrow.amount_expected);
        msg!("Executor provided: {} tokens", token_account_data.amount as u32);
        return Err(throw_and_log(EscrowError::ExecutorTokenAmtMismatch));
    }

    if deposit_account_data.amount as u32 != amount_expected {
        msg!("Error: Depositor and executor expectations are not met");
        msg!("Executor expected: {} tokens", amount_expected);
        msg!("Depositor provided: {} tokens", deposit_account_data.amount as u32);
        return Err(throw_and_log(EscrowError::DepositTokenAmtMismatch));
    }

    if escrow.token_expected != token_account_data.mint {
        msg!("Error: Depositor and executor expectations are not met");
        msg!("Depositor expected: {} mint", escrow.token_expected);
        msg!("Executor provided: {} mint", token_account_data.mint);
        return Err(throw_and_log(EscrowError::ExecutorTokenMintMismatch));
    }

    if deposit_account_data.mint != *token_expected.key {
        msg!("Error: Depositor and executor expectations are not met");
        msg!("Executor expected: {} mint", *token_expected.key);
        msg!("Depositor provided: {} mint", deposit_account_data.mint);
        return Err(throw_and_log(EscrowError::DepositTokenMintMismatch));
    }

    // Callee account transfer
    let owner_change_ix = set_authority(
        token_program.key,
        token_account_info.key,
        Some(&escrow.owner_account),
        spl_token::instruction::AuthorityType::AccountOwner,
        owner_account_info.key,
        &[&owner_account_info.key],
    )?;

    msg!("Calling the token program to transfer callee token account ownership...");
    invoke(
        &owner_change_ix,
        &[
            token_account_info.clone(),
            owner_account_info.clone(),
            token_program.clone(),
        ],
    )?;

    // Deposit account transfer
    let deposit_owner_change_ix = set_authority(
        token_program.key,
        &deposit_account_info.key,
        Some(&owner_account_info.key),
        spl_token::instruction::AuthorityType::AccountOwner,
        &pda,
        &[],
    )?;

    msg!("Calling the token program to transfer depositor token account ownership...");
    invoke_signed(
        &deposit_owner_change_ix,
        &[
            deposit_account_info.clone(),
            pda_account_info.clone(),
            token_program.clone(),
        ],
        &[&[payer_account_info.key.as_ref(), SEED, &[bump_seed]]],
    )?;

    msg!("Swap passed successfully!");
    msg!("Depositor gets an account: {} with {} tokens of {} mint", token_account_info.key, token_account_data.amount, token_account_data.mint);
    msg!("Executor gets an account: {} with {} tokens of {} mint", deposit_account_info.key, deposit_account_data.amount, deposit_account_data.mint);

    escrow.reset();
    escrow.pack_into_slice(&mut pda_account_info.try_borrow_mut_data()?);

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
        EscrowInstruction::Execute {
            amount_expected,
        } => {
            msg!("Execute escrow request...");
            return execute(accounts, program_id, amount_expected);
        }
        EscrowInstruction::Cancel => {
            msg!("Escrow account is closed, tokens returned to");
            // TODO: print return account
        }
    }

    Ok(())
}
