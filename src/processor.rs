use crate::error::{throw_and_log, EscrowError};
use crate::instruction_parser::{parse_data, EscrowInstruction};
use crate::state::{Escrow, DATA_LEN, SEED};
use crate::utils::{verify_pda, verify_rent_exemption};
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::program::{invoke, invoke_signed};
use solana_program::program_error::ProgramError;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;
use solana_program::sysvar::rent::Rent;
use solana_program::{msg, system_instruction, system_program};
use spl_token::instruction::set_authority;

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

    verify_pda(pda_account_info, program_id)?;

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
                    program_id,
                ),
                &[
                    payer_account_info.clone(),
                    pda_account_info.clone(),
                    system_account.clone(),
                ],
                &[&[seed, &[bump_seed]]],
            )?;
        }
        _ => {
            msg!("PDA account already exists");
            return Err(throw_and_log(EscrowError::PdaExists));
        }
    }

    Ok(())
}

fn deposit(accounts: &[AccountInfo], program_id: &Pubkey, amount_expected: u32) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let pda_account_info = next_account_info(account_info_iter)?;
    let owner_account_info = next_account_info(account_info_iter)?;
    let token_account_info = next_account_info(account_info_iter)?;
    let token_program = next_account_info(account_info_iter)?;
    let token_expected = next_account_info(account_info_iter)?;

    assert!(owner_account_info.is_signer);
    assert_eq!(*token_account_info.owner, spl_token::id());
    assert!(token_account_info.is_writable);
    verify_rent_exemption(token_account_info)?;

    if pda_account_info.owner != program_id {
        return Err(ProgramError::IncorrectProgramId);
    }

    let token_account_data = spl_token::state::Account::unpack(&token_account_info.data.borrow())?;

    // Check if the owner matches the expected owner
    if token_account_data.owner != *owner_account_info.key {
        msg!("The provided owner is not the real owner of this token account.");
        return Err(ProgramError::IllegalOwner);
    }

    verify_pda(pda_account_info, program_id)?;

    let escrow_account = Escrow {
        active: true,
        token_expected: *token_expected.key,
        amount_expected,
        holding_account: *token_account_info.key,
        owner_account: *owner_account_info.key,
    };

    escrow_account.pack_into_slice(&mut pda_account_info.try_borrow_mut_data()?);

    let owner_change_ix = set_authority(
        token_program.key,
        token_account_info.key,
        Some(pda_account_info.key),
        spl_token::instruction::AuthorityType::AccountOwner,
        owner_account_info.key,
        &[owner_account_info.key],
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

    msg!("Account deposited successfully! You can retrieve it using Revoke instruction.");
    Ok(())
}

fn execute(accounts: &[AccountInfo], program_id: &Pubkey, amount_expected: u32) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let pda_account_info = next_account_info(account_info_iter)?;
    let owner_account_info = next_account_info(account_info_iter)?;
    let token_account_info = next_account_info(account_info_iter)?;
    let token_program = next_account_info(account_info_iter)?;
    let token_expected = next_account_info(account_info_iter)?;
    let deposit_account_info = next_account_info(account_info_iter)?;

    assert!(deposit_account_info.is_writable);
    assert!(token_account_info.is_writable);
    verify_rent_exemption(token_account_info)?;

    let mut escrow_account = Escrow::unpack_from_slice(*pda_account_info.try_borrow_data()?)?;
    let token_account_data = spl_token::state::Account::unpack(&token_account_info.data.borrow())?;

    let (pda, bump_seed) = verify_pda(pda_account_info, program_id)?;
    let deposit_account_data =
        spl_token::state::Account::unpack(&deposit_account_info.data.borrow())?;

    if deposit_account_data.owner != pda {
        msg!("The provided owner is not the real owner of user token account.");
        return Err(ProgramError::IllegalOwner);
    }

    if !escrow_account.active {
        return Err(throw_and_log(EscrowError::NotInitialized));
    }

    // Actors' expectations checks
    if escrow_account.amount_expected != token_account_data.amount as u32 {
        msg!("Error: Depositor and executor expectations are not met");
        msg!(
            "Depositor expected: {} tokens",
            escrow_account.amount_expected
        );
        msg!(
            "Executor provided: {} tokens",
            token_account_data.amount as u32
        );
        return Err(throw_and_log(EscrowError::ExecutorTokenAmtMismatch));
    }

    if deposit_account_data.amount as u32 != amount_expected {
        msg!("Error: Depositor and executor expectations are not met");
        msg!("Executor expected: {} tokens", amount_expected);
        msg!(
            "Depositor provided: {} tokens",
            deposit_account_data.amount as u32
        );
        return Err(throw_and_log(EscrowError::DepositTokenAmtMismatch));
    }

    if escrow_account.token_expected != token_account_data.mint {
        msg!("Error: Depositor and executor expectations are not met");
        msg!("Depositor expected: {} mint", escrow_account.token_expected);
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
        Some(&escrow_account.owner_account),
        spl_token::instruction::AuthorityType::AccountOwner,
        owner_account_info.key,
        &[owner_account_info.key],
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
        deposit_account_info.key,
        Some(owner_account_info.key),
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
        &[&[SEED, &[bump_seed]]],
    )?;

    msg!("Swap passed successfully!");
    msg!(
        "Depositor gets an account: {} with {} tokens of {} mint",
        token_account_info.key,
        token_account_data.amount,
        token_account_data.mint
    );
    msg!(
        "Executor gets an account: {} with {} tokens of {} mint",
        deposit_account_info.key,
        deposit_account_data.amount,
        deposit_account_data.mint
    );

    escrow_account.reset();
    escrow_account.pack_into_slice(&mut pda_account_info.try_borrow_mut_data()?);

    Ok(())
}

fn reclaim(accounts: &[AccountInfo], program_id: &Pubkey) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let pda_account_info = next_account_info(account_info_iter)?;
    let owner_account_info = next_account_info(account_info_iter)?;
    let token_account_info = next_account_info(account_info_iter)?;
    let token_program = next_account_info(account_info_iter)?;

    assert!(token_account_info.is_writable);
    let (_pda, bump_seed) = verify_pda(pda_account_info, program_id)?;

    let token_account_data = spl_token::state::Account::unpack(&token_account_info.data.borrow())?;

    // Check if the owner matches the expected owner
    if token_account_data.owner != *pda_account_info.key {
        msg!("The provided owner is not the real owner of this token account.");
        return Err(ProgramError::IllegalOwner);
    }

    let escrow_account = Escrow::unpack_from_slice(*pda_account_info.try_borrow_data()?)?;

    if !escrow_account.active {
        return Err(throw_and_log(EscrowError::NotInitialized));
    }

    // Actors' expectations checks
    if escrow_account.owner_account != *owner_account_info.key {
        msg!("Attempt to reclaim non-owned account");
        msg!("Claimed owner: {}", owner_account_info.key);
        msg!("Real owner: {}", escrow_account.owner_account);
        return Err(ProgramError::IllegalOwner);
    }

    let reclaim_ix = set_authority(
        token_program.key,
        token_account_info.key,
        Some(owner_account_info.key),
        spl_token::instruction::AuthorityType::AccountOwner,
        pda_account_info.key,
        &[],
    )?;

    msg!("Calling the token program to transfer depositor token account ownership...");
    invoke_signed(
        &reclaim_ix,
        &[
            token_account_info.clone(),
            pda_account_info.clone(),
            token_program.clone(),
        ],
        &[&[SEED, &[bump_seed]]],
    )?;

    msg!("Account deposited successfully! You can retrieve it using Revoke instruction.");

    escrow_account.pack_into_slice(&mut pda_account_info.try_borrow_mut_data()?);
    Ok(())
}

pub fn parse_execute_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    // unpack instruction
    let instruction = parse_data(instruction_data)?;

    match instruction {
        EscrowInstruction::Init { seed, bump_seed } => {
            msg!("Init escrow request...");
            init_escrow(accounts, program_id, seed, bump_seed)
        }
        EscrowInstruction::Deposit { amount_expected } => {
            msg!("Deposit instruction...");
            deposit(accounts, program_id, amount_expected)
        }
        EscrowInstruction::Execute { amount_expected } => {
            msg!("Execute escrow request...");
            execute(accounts, program_id, amount_expected)
        }
        EscrowInstruction::Reclaim => {
            msg!("Escrow account is closed, tokens returned to");
            reclaim(accounts, program_id)
        }
    }
}
