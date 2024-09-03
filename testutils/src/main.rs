use glob::{glob};
use solana_program::program_pack::Pack;
use solana_sdk::signature::{read_keypair_file, EncodableKey, Keypair, Signer};
use solana_sdk::{system_instruction};
use std::io;
use std::io::Error;
use std::path::{Path, PathBuf};
use solana_program::hash::Hash;
use solana_program::pubkey::Pubkey;
use solana_program_test::{tokio, ProgramTest, BanksClient};
use solana_sdk::transaction::Transaction;
use spl_token::{
    instruction,
    state::{Account, Mint},
};

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

async fn create_mint_account(banks_client: &mut BanksClient, payer: &Keypair, mint_account: &Keypair, mint_authority: &Keypair, recent_blockhash: &Hash) {
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

async fn create_ata(banks_client: &mut BanksClient, payer: &Keypair, user_account: &Keypair, user_owner_pk: &Pubkey, mint_account_pk: &Pubkey, recent_blockhash: &Hash) {
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
    match banks_client.process_transaction(create_user_account_tx).await {
        Err(e) => println!("Unable to send transaction: {}", e),
        _ => (),
    }
}

async fn mint_to_user_account(banks_client: &mut BanksClient, payer: &Keypair, mint_account: &Pubkey, mint_authority: &Keypair, user_account: &Pubkey, recent_blockhash: &Hash, amount: u64) {
    let mint_to_alice_ix = instruction::mint_to(
        &spl_token::id(),
        mint_account,
        user_account,
        &mint_authority.pubkey(),
        &[],
        amount
    ).expect("Unable mint to Alice");
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
    let test_program = ProgramTest::default();
    let (mut banks_client, payer, recent_blockhash) = test_program.start().await;

    let alice_owner_kp = read_or_generate_keypair("ao");
    let bob_owner_kp = read_or_generate_keypair("bo");
    let alice_account_kp = read_or_generate_keypair("aa");
    let bob_account_kp = read_or_generate_keypair("ba");

    let alice_token_amount = 10u64;
    let bob_token_amount = 5u64;

    create_mint_account(&mut banks_client, &payer, &alice_mint_account_kp, &mint_authority_kp, &recent_blockhash).await;
    create_ata(&mut banks_client, &payer, &alice_account_kp, &alice_owner_kp.pubkey(), &alice_mint_account_kp.pubkey(), &recent_blockhash).await;
    mint_to_user_account(&mut banks_client, &payer, &alice_mint_account_kp.pubkey(), &mint_authority_kp, &alice_account_kp.pubkey(),&recent_blockhash, alice_token_amount).await;

    create_mint_account(&mut banks_client, &payer, &bob_mint_account_kp, &mint_authority_kp, &recent_blockhash).await;
    create_ata(&mut banks_client, &payer, &bob_account_kp, &bob_owner_kp.pubkey(), &bob_mint_account_kp.pubkey(), &recent_blockhash).await;
    mint_to_user_account(&mut banks_client, &payer, &bob_mint_account_kp.pubkey(), &mint_authority_kp, &bob_account_kp.pubkey(),&recent_blockhash, bob_token_amount).await;

    let alice_account = banks_client.get_account(alice_account_kp.pubkey()).await.unwrap().expect("Unable to read Alice's account");
    let account_data = Account::unpack(&alice_account.data).unwrap();
    assert_eq!(account_data.amount, alice_token_amount);

    let bob_account = banks_client.get_account(bob_account_kp.pubkey()).await.unwrap().expect("Unable to read Alice's account");
    let account_data = Account::unpack(&bob_account.data).unwrap();
    assert_eq!(account_data.amount, bob_token_amount);
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
