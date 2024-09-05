mod test_utils;

use solana_program::instruction::AccountMeta;
use solana_program::pubkey::Pubkey;
use solana_program_test::{tokio, ProgramTest};
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::system_program;
use solana_sdk::transaction::Transaction;
use test_utils::*;

#[tokio::test]
async fn reclaim_test() {
    let mint_authority_kp = Keypair::new();
    let escrow_program_kp = Keypair::new();
    let alice_token_amount: u32 = 10u32;

    let mut test_program = ProgramTest::default();
    test_program.add_program("solana_escrow", escrow_program_kp.pubkey(), None);
    let (mut banks_client, payer, recent_blockhash) = test_program.start().await;

    let alice = UserAccounts::prepare(
        &mut banks_client,
        &recent_blockhash,
        &payer,
        &mint_authority_kp,
        alice_token_amount,
    )
        .await;

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
    let alice_desired_amount = unsafe { std::mem::transmute::<u32, [u8; 4]>(10u32) };
    let deposit_ix = solana_sdk::instruction::Instruction {
        program_id: escrow_program_kp.pubkey(),
        accounts: vec![
            AccountMeta::new(pda_account_pk, false),
            AccountMeta::new_readonly(alice.wallet_account.pubkey(), true),
            AccountMeta::new(alice.token_account.pubkey(), true),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(alice.mint_account.pubkey(), false),
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

    check_account_property(
        &mut banks_client,
        &alice.token_account.pubkey(),
        |account| assert_eq!(account.owner, pda_account_pk),
    )
        .await;

    let reclaim_ix = solana_sdk::instruction::Instruction {
        program_id: escrow_program_kp.pubkey(),
        accounts: vec![
            AccountMeta::new(pda_account_pk, false),
            AccountMeta::new_readonly(alice.wallet_account.pubkey(), true),
            AccountMeta::new(alice.token_account.pubkey(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: vec![3u8], // Reclaim instruction
    };

    let reclaim_tx = Transaction::new_signed_with_payer(
        &[reclaim_ix],
        Some(&payer.pubkey()),
        &[&payer, &alice.wallet_account],
        recent_blockhash,
    );

    banks_client
        .process_transaction(reclaim_tx)
        .await
        .expect("Unable to make reclaim");

    check_account_property(
        &mut banks_client,
        &alice.token_account.pubkey(),
        |account| assert_eq!(account.owner, alice.wallet_account.pubkey()),
    ).await;
}
