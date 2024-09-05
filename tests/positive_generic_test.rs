mod test_utils;

use solana_program::instruction::AccountMeta;
use solana_program::pubkey::Pubkey;
use solana_program_test::{tokio, ProgramTest};
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::system_program;
use solana_sdk::transaction::Transaction;
use test_utils::*;

// Replace with devnet or whichever cluster you're currently on
// let rpc = RpcClient::new(Cluster::Localnet.url());
// let sig = rpc.request_airdrop(pubkey, lamports).unwrap();
#[tokio::test]
async fn positive_generic_flow_test() {
    let mint_authority_kp = Keypair::new();
    let escrow_program_kp = Keypair::new();
    let alice_token_amount: u32 = 10u32;
    let bob_token_amount: u32 = 5u32;

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
    let bob = UserAccounts::prepare(
        &mut banks_client,
        &recent_blockhash,
        &payer,
        &mint_authority_kp,
        bob_token_amount,
    )
    .await;

    check_account_property(
        &mut banks_client,
        &alice.token_account.pubkey(),
        |account| assert_eq!(account.amount, alice_token_amount as u64),
    )
    .await;

    check_account_property(&mut banks_client, &bob.token_account.pubkey(), |account| {
        assert_eq!(account.amount, bob_token_amount as u64)
    })
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

    let pda_account = banks_client
        .get_account(pda_account_pk)
        .await
        .unwrap()
        .expect("Unable to read PDA account");
    assert_eq!(pda_account.owner, escrow_program_kp.pubkey());

    // Make a deposit by the first party user
    let alice_desired_amount = unsafe { std::mem::transmute::<u32, [u8; 4]>(bob_token_amount) };
    let deposit_ix = solana_sdk::instruction::Instruction {
        program_id: escrow_program_kp.pubkey(),
        accounts: vec![
            AccountMeta::new(pda_account_pk, false),
            AccountMeta::new_readonly(alice.wallet_account.pubkey(), true),
            AccountMeta::new(alice.token_account.pubkey(), true),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(bob.mint_account.pubkey(), false),
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

    // Execute escrow swap
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

    banks_client
        .process_transaction(execute_tx)
        .await
        .expect("Unable to make an escrow execution");

    // Verify if accounts are swapped and belong to new owners
    check_account_property(
        &mut banks_client,
        &alice.token_account.pubkey(),
        |account| assert_eq!(account.owner, bob.wallet_account.pubkey()),
    )
    .await;
    check_account_property(&mut banks_client, &bob.token_account.pubkey(), |account| {
        assert_eq!(account.owner, alice.wallet_account.pubkey())
    })
    .await;
}
