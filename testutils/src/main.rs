use glob::glob;
use solana_program::hash::Hash;
use solana_program::instruction::AccountMeta;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;
use solana_program_test::{tokio, BanksClient, ProgramTest};
use solana_sdk::signature::{read_keypair_file, EncodableKey, Keypair, Signer};
use solana_sdk::transaction::Transaction;
use solana_sdk::{system_instruction, system_program};
use spl_token::{
    instruction,
    state::{Account, Mint},
};
use std::io;
use std::io::Error;
use std::path::{Path, PathBuf};

fn generate_custom_keypair(startswith: &str) -> Keypair {
    let mut counter = 0;
    let mut keypair = Keypair::new();
    while !keypair.pubkey().to_string().starts_with(startswith) {
        keypair = Keypair::new();
        if counter % 1000 == 0 {
            println!("Pairs checked: {counter}")
        }
        counter += 1;
    }
    println!("Keypair found: {}", keypair.pubkey().to_string());
    keypair
}

fn check_file_existence(startswith: &str) -> Result<PathBuf, Error> {
    let results = glob(startswith)
        .expect("Failed to read glob pattern")
        .next();
    match results {
        Some(path) => Ok(path.unwrap()),
        None => Err(Error::from(io::ErrorKind::NotFound)),
    }
}

fn read_or_generate_keypair(startswith: &str) -> Keypair {
    match check_file_existence(format!("./keys/{startswith}*").as_str()) {
        Ok(path) => read_keypair_file(path.as_path()).expect("Unable to open keypair file"),
        Err(_) => {
            let kp = generate_custom_keypair(startswith);
            kp.write_to_file(Path::new(
                format!("./keys/{}.json", kp.pubkey().to_string()).as_str(),
            ))
            .expect("Cannot write new keypair");
            kp
        }
    }
}

async fn create_mint_account(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    mint_account: &Keypair,
    mint_authority: &Keypair,
    recent_blockhash: &Hash,
) {
    let rent = banks_client.get_rent().await.expect("Unable to read rent");
    let mint_rent = rent.minimum_balance(Mint::LEN);

    let create_mint_instruction = system_instruction::create_account(
        &payer.pubkey(),
        &mint_account.pubkey(),
        mint_rent,
        Mint::LEN as u64,
        &spl_token::id(),
    );
    let init_mint_instruction = instruction::initialize_mint(
        &spl_token::id(),
        &mint_account.pubkey(),
        &mint_authority.pubkey(),
        None,
        9,
    )
    .expect("Unable to init mint account");
    let mint_tx = Transaction::new_signed_with_payer(
        &[create_mint_instruction, init_mint_instruction],
        Some(&payer.pubkey()),
        &[&payer, &mint_account],
        *recent_blockhash,
    );
    match banks_client.process_transaction(mint_tx).await {
        Err(e) => println!("Unable to send transaction: {}", e),
        _ => (),
    }
}

async fn create_user_account(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    user_account: &Keypair,
    recent_blockhash: &Hash,
) {
    let rent = banks_client.get_rent().await.expect("Unable to read rent");
    let account_rent = rent.minimum_balance(Account::LEN);
    let create_user_account_ix = system_instruction::create_account(
        &payer.pubkey(),
        &user_account.pubkey(),
        account_rent,
        Account::LEN as u64,
        &spl_token::id(),
    );

    let create_user_account_tx = Transaction::new_signed_with_payer(
        &[create_user_account_ix],
        Some(&payer.pubkey()),
        &[&payer, &user_account],
        *recent_blockhash,
    );
    match banks_client
        .process_transaction(create_user_account_tx)
        .await
    {
        Err(e) => println!("Unable to send transaction: {}", e),
        _ => (),
    }
}

async fn create_ata(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    user_account: &Keypair,
    user_owner_pk: &Pubkey,
    mint_account_pk: &Pubkey,
    recent_blockhash: &Hash,
) {
    //  TODO: remove Alice everywhere
    let rent = banks_client.get_rent().await.expect("Unable to read rent");
    let account_rent = rent.minimum_balance(Account::LEN);
    let create_alice_account_ix = system_instruction::create_account(
        &payer.pubkey(),
        &user_account.pubkey(),
        account_rent,
        Account::LEN as u64,
        &spl_token::id(),
    );

    let init_user_account_ix = instruction::initialize_account(
        &spl_token::id(),
        &user_account.pubkey(),
        mint_account_pk,
        user_owner_pk,
    )
    .expect("Unable to init Alice's account");

    let create_user_account_tx = Transaction::new_signed_with_payer(
        &[create_alice_account_ix, init_user_account_ix],
        Some(&payer.pubkey()),
        &[&payer, &user_account],
        *recent_blockhash,
    );
    match banks_client
        .process_transaction(create_user_account_tx)
        .await
    {
        Err(e) => println!("Unable to send transaction: {}", e),
        _ => (),
    }
}

async fn mint_to_user_account(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    mint_account: &Pubkey,
    mint_authority: &Keypair,
    user_account: &Pubkey,
    recent_blockhash: &Hash,
    amount: u64,
) {
    let mint_to_alice_ix = instruction::mint_to(
        &spl_token::id(),
        mint_account,
        user_account,
        &mint_authority.pubkey(),
        &[],
        amount,
    )
    .expect("Unable mint to Alice");
    let mint_to_alice_tx = Transaction::new_signed_with_payer(
        &[mint_to_alice_ix],
        Some(&payer.pubkey()),
        &[&payer, &mint_authority],
        *recent_blockhash,
    );
    match banks_client.process_transaction(mint_to_alice_tx).await {
        Err(e) => println!("Unable to send mint transaction: {}", e),
        _ => (),
    }
}

// Replace with devnet or whichever cluster you're currently on
// let rpc = RpcClient::new(Cluster::Localnet.url());
// let sig = rpc.request_airdrop(pubkey, lamports).unwrap();
#[tokio::test]
async fn test_case() {
    let mint_authority_kp = read_or_generate_keypair("ma");
    let alice_mint_account_kp = read_or_generate_keypair("am");
    let bob_mint_account_kp = read_or_generate_keypair("bm");
    let mut test_program = ProgramTest::default();
    let escrow_program_kp = Keypair::new();
    test_program.add_program("escrow-program", escrow_program_kp.pubkey(), None);
    let (mut banks_client, payer, recent_blockhash) = test_program.start().await;

    let alice_owner_kp = read_or_generate_keypair("ao");
    let bob_owner_kp = read_or_generate_keypair("bo");
    let alice_account_kp = read_or_generate_keypair("aa");
    let bob_account_kp = read_or_generate_keypair("ba");

    let alice_token_amount = 10u64;
    let bob_token_amount = 5u64;

    create_user_account(
        &mut banks_client,
        &payer,
        &alice_owner_kp,
        &recent_blockhash,
    ).await;

    create_mint_account(
        &mut banks_client,
        &payer,
        &alice_mint_account_kp,
        &mint_authority_kp,
        &recent_blockhash,
    )
    .await;

    create_ata(
        &mut banks_client,
        &payer,
        &alice_account_kp,
        &alice_owner_kp.pubkey(),
        &alice_mint_account_kp.pubkey(),
        &recent_blockhash,
    )
    .await;
    mint_to_user_account(
        &mut banks_client,
        &payer,
        &alice_mint_account_kp.pubkey(),
        &mint_authority_kp,
        &alice_account_kp.pubkey(),
        &recent_blockhash,
        alice_token_amount,
    )
    .await;

    create_mint_account(
        &mut banks_client,
        &payer,
        &bob_mint_account_kp,
        &mint_authority_kp,
        &recent_blockhash,
    )
    .await;
    create_ata(
        &mut banks_client,
        &payer,
        &bob_account_kp,
        &bob_owner_kp.pubkey(),
        &bob_mint_account_kp.pubkey(),
        &recent_blockhash,
    )
    .await;
    mint_to_user_account(
        &mut banks_client,
        &payer,
        &bob_mint_account_kp.pubkey(),
        &mint_authority_kp,
        &bob_account_kp.pubkey(),
        &recent_blockhash,
        bob_token_amount,
    )
    .await;

    let alice_token_account = banks_client
        .get_account(alice_account_kp.pubkey())
        .await
        .unwrap()
        .expect("Unable to read Alice's account");
    let account_data = Account::unpack(&alice_token_account.data).unwrap();
    assert_eq!(account_data.amount, alice_token_amount);

    let bob_account = banks_client
        .get_account(bob_account_kp.pubkey())
        .await
        .unwrap()
        .expect("Unable to read Alice's account");
    let account_data = Account::unpack(&bob_account.data).unwrap();
    assert_eq!(account_data.amount, bob_token_amount);

    let seed = b"escrow";
    let (pda_account_pk, bump_seed) = Pubkey::find_program_address(
        &[payer.pubkey().as_ref(), seed],
        &escrow_program_kp.pubkey(),
    );
    println!("PDA: {}", pda_account_pk);

    let escrow_init_ix = solana_sdk::instruction::Instruction {
        program_id: escrow_program_kp.pubkey(),
        accounts: vec![
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new(pda_account_pk, false),
            AccountMeta::new(system_program::id(), false),
        ],
        data: [[0].as_slice(), seed.as_slice(), [bump_seed].as_slice()].concat(),
    };

    let interact_with_escrow_tx = Transaction::new_signed_with_payer(
        &[escrow_init_ix],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );

    match banks_client
        .process_transaction(interact_with_escrow_tx)
        .await
    {
        Err(e) => println!("Unable to send a transaction: {}", e),
        _ => (),
    }

    let pda_account = banks_client
        .get_account(pda_account_pk)
        .await
        .unwrap()
        .expect("Unable to read PDA account");
    let account_data = Account::unpack(&bob_account.data).unwrap();
    assert_eq!(pda_account.owner, escrow_program_kp.pubkey());

    /// Deposit block
    let deposit_ix = solana_sdk::instruction::Instruction {
        program_id: escrow_program_kp.pubkey(),
        accounts: vec![
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new(pda_account_pk, false),
            AccountMeta::new_readonly(alice_owner_kp.pubkey(), true),
            AccountMeta::new(alice_account_kp.pubkey(), true),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(bob_mint_account_kp.pubkey(), false),
        ],
        data: vec![1],
    };

    let deposit_tx = Transaction::new_signed_with_payer(
        &[deposit_ix],
        Some(&payer.pubkey()),
        &[&payer, &alice_owner_kp, &alice_account_kp],
        recent_blockhash,
    );

    match banks_client
        .process_transaction(deposit_tx)
        .await
    {
        Err(e) => println!("Unable to make a deposit: {}", e),
        _ => (),
    }

    let deposit_account = banks_client
        .get_account(alice_account_kp.pubkey())
        .await
        .unwrap()
        .expect("Unable to read deposit account");
    let deposit_data = Account::unpack(&deposit_account.data).unwrap();
    assert_eq!(deposit_data.owner, pda_account_pk);

    /// Execute block
    let execute_ix = solana_sdk::instruction::Instruction {
        program_id: escrow_program_kp.pubkey(),
        accounts: vec![
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new(pda_account_pk, false),
            AccountMeta::new_readonly(bob_owner_kp.pubkey(), true),
            AccountMeta::new(bob_account_kp.pubkey(), true),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(alice_mint_account_kp.pubkey(), false),
            AccountMeta::new(alice_account_kp.pubkey(), false),
        ],
        data: vec![2],
    };

    let execute_tx = Transaction::new_signed_with_payer(
        &[execute_ix],
        Some(&payer.pubkey()),
        &[&payer, &bob_owner_kp, &bob_account_kp],
        recent_blockhash,
    );

    match banks_client
        .process_transaction(execute_tx)
        .await
    {
        Err(e) => println!("Unable to make an execution: {}", e),
        _ => (),
    }
    // let blockhash = banks_client.get_latest_blockhash().await.unwrap();
    //
    // let escrow_init_ix = solana_sdk::instruction::Instruction {
    //     program_id: escrow_program_kp.pubkey(),
    //     accounts: vec![AccountMeta::new(payer.pubkey(), true), AccountMeta::new(pda_account_pk, false), AccountMeta::new(system_program::id(), false)],
    //     data: vec![seed]
    // };
    // let interact_with_escrow_tx = Transaction::new_signed_with_payer(
    //     &[escrow_init_ix],
    //     Some(&payer.pubkey()),
    //     &[&payer],
    //     blockhash,
    // );
    //
    // match banks_client.process_transaction(interact_with_escrow_tx).await {
    //     Err(e) => println!("Unable to send a transaction: {}", e),
    //     _ => (),
    // }
    // println!("Balance: {}", banks_client.get_balance(pda_account_pk).await.unwrap());
    // Check if an authority kp exists
    // If not, create a new one
    // Make an airdrop
    // Check if 2 mint kp exists
    // If not, create new ones
    // Create new tokens
    // Create new accounts for Alice and Bob
    // Transfer tokens to their accounts

    // Alice initiates swap through escrow

    // Bob completes swap on his side
    println!("Hello world");
}

fn main() {}
