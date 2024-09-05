mod test_utils;

use std::panic::panic_any;
use solana_program::instruction::AccountMeta;
use solana_program::pubkey::Pubkey;
use solana_program_test::{tokio, ProgramTest};
use solana_program_test::BanksClientError::TransactionError;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::system_program;
use solana_sdk::transaction::Transaction;
use solana_sdk::instruction::InstructionError;
use test_utils::*;

#[tokio::test]
async fn negative_test() {
    let mint_authority_kp = Keypair::new();
    let escrow_program_kp = Keypair::new();
    let alice_token_amount: u32 = 10u32;
    let bob_token_amount: u32 = 5u32;
    let david_token_amount: u32 = 6u32;

    let mut test_program = ProgramTest::default();
    test_program.add_program("solana_escrow", escrow_program_kp.pubkey(), None);
    let (mut banks_client, payer, recent_blockhash) = test_program.start().await;

    let alice = UserAccounts::prepare(
        &mut banks_client,
        &recent_blockhash,
        &payer,
        &mint_authority_kp,
        alice_token_amount,
    ).await;

    let bob = UserAccounts::prepare(
        &mut banks_client,
        &recent_blockhash,
        &payer,
        &mint_authority_kp,
        bob_token_amount,
    ).await;

    let david = UserAccounts::prepare(
        &mut banks_client,
        &recent_blockhash,
        &payer,
        &mint_authority_kp,
        david_token_amount,
    ).await;

    let (pda_account_pk, bump_seed) =
        Pubkey::find_program_address(&[SEED], &escrow_program_kp.pubkey());

    let escrow_init_ix = solana_sdk::instruction::Instruction {
        program_id: escrow_program_kp.pubkey(),
        accounts: vec![
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new(pda_account_pk, false),
            AccountMeta::new(system_program::id(), false),
        ],
        data: [[0].as_slice(), SEED.as_slice(), [bump_seed].as_slice()].concat(),
    };

    let init_escrow_tx = Transaction::new_signed_with_payer(
        &[escrow_init_ix],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );

    banks_client
        .process_transaction(init_escrow_tx)
        .await
        .expect("Unable to init an escrow program");

    // Make a deposit by the first party user
    let alice_desired_amount = unsafe { std::mem::transmute::<u32, [u8; 4]>(bob_token_amount) };
    let deposit_ix = solana_sdk::instruction::Instruction {
        program_id: escrow_program_kp.pubkey(),
        accounts: vec![
            AccountMeta::new(pda_account_pk, false),
            AccountMeta::new_readonly(alice.wallet_account.pubkey(), true),
            AccountMeta::new(alice.token_account.pubkey(), true),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(david.mint_account.pubkey(), false),
        ],
        data: [[1].as_slice(), alice_desired_amount.as_slice()].concat(),
    };

    let deposit_tx = Transaction::new_signed_with_payer(
        &[deposit_ix],
        Some(&payer.pubkey()),
        &[&payer, &alice.wallet_account, &alice.token_account],
        recent_blockhash,
    );

    banks_client
        .process_transaction(deposit_tx)
        .await
        .expect("Unable to make a deposit");

    // Bob's try failed due to his mint mismatch
    let bob_desired_amount = unsafe { std::mem::transmute::<u32, [u8; 4]>(alice_token_amount) };
    let execute_ix = solana_sdk::instruction::Instruction {
        program_id: escrow_program_kp.pubkey(),
        accounts: vec![
            AccountMeta::new(pda_account_pk, false),
            AccountMeta::new_readonly(bob.wallet_account.pubkey(), true),
            AccountMeta::new(bob.token_account.pubkey(), true),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(alice.mint_account.pubkey(), false),
            AccountMeta::new(alice.token_account.pubkey(), false),
        ],
        data: [[2].as_slice(), bob_desired_amount.as_slice()].concat(),
    };

    let execute_tx = Transaction::new_signed_with_payer(
        &[execute_ix],
        Some(&payer.pubkey()),
        &[&payer, &bob.wallet_account, &bob.token_account],
        recent_blockhash,
    );

    match banks_client
        .process_transaction(execute_tx)
        .await {
        Err(e) => {
            match e {
                TransactionError(te) => {match te {
                    solana_sdk::transaction::TransactionError::InstructionError(_, ie) => { match ie {
                        InstructionError::Custom(code) => assert_eq!(code, 6u32),
                        _ => panic_any("Wrong error type"),
                    }}
                    _ => panic_any("Wrong error type"),
                }}
                _ => panic_any("Wrong error type"),
            }
        },
        Ok(_) => panic_any("Error is expected!"),
    }

    // David's try failed due to token amount mismatch
    let david_desired_amount = unsafe { std::mem::transmute::<u32, [u8; 4]>(alice_token_amount) };
    let execute_ix = solana_sdk::instruction::Instruction {
        program_id: escrow_program_kp.pubkey(),
        accounts: vec![
            AccountMeta::new(pda_account_pk, false),
            AccountMeta::new_readonly(david.wallet_account.pubkey(), true),
            AccountMeta::new(david.token_account.pubkey(), true),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(alice.mint_account.pubkey(), false),
            AccountMeta::new(alice.token_account.pubkey(), false),
        ],
        data: [[2].as_slice(), david_desired_amount.as_slice()].concat(),
    };

    let execute_tx = Transaction::new_signed_with_payer(
        &[execute_ix],
        Some(&payer.pubkey()),
        &[&payer, &david.wallet_account, &david.token_account],
        recent_blockhash,
    );

    match banks_client
        .process_transaction(execute_tx)
        .await {
        Err(e) => {
            match e {
                TransactionError(te) => {match te {
                    solana_sdk::transaction::TransactionError::InstructionError(_, ie) => { match ie {
                        InstructionError::Custom(code) => assert_eq!(code, 5u32),
                        _ => panic_any("Wrong error type"),
                    }}
                    _ => panic_any("Wrong error type"),
                }}
                _ => panic_any("Wrong error type"),
            }
        },
        Ok(_) => panic_any("Error is expected!"),
    }

}
