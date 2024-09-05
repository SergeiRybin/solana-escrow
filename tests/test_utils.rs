use solana_program::hash::Hash;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;
use solana_program::system_instruction;
use solana_program_test::BanksClient;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;
use spl_token::instruction;
use spl_token::state::{Account, Mint};
pub const SEED: &[u8; 6] = b"escrow";

pub struct UserAccounts {
    pub mint_account: Keypair,
    pub wallet_account: Keypair,
    pub token_account: Keypair,
}

impl UserAccounts {
    pub async fn prepare(
        client: &mut BanksClient,
        blockhash: &Hash,
        payer: &Keypair,
        mint_authority: &Keypair,
        token_amount: u32,
    ) -> Self {
        let user = UserAccounts {
            mint_account: Keypair::new(),
            wallet_account: Keypair::new(),
            token_account: Keypair::new(),
        };

        create_mint_account(
            client,
            &payer,
            &user.mint_account,
            &mint_authority,
            &blockhash,
        )
        .await;

        create_ata(
            client,
            &payer,
            &user.token_account,
            &user.wallet_account.pubkey(),
            &user.mint_account.pubkey(),
            &blockhash,
        )
        .await;
        mint_to_user_account(
            client,
            &payer,
            &user.mint_account.pubkey(),
            &mint_authority,
            &user.token_account.pubkey(),
            &blockhash,
            token_amount.into(),
        )
        .await;

        user
    }
}

pub async fn check_account_property<F>(client: &mut BanksClient, account: &Pubkey, f: F)
where
    F: Fn(Account),
{
    let fetched_account = client
        .get_account(*account)
        .await
        .unwrap()
        .expect("Unable to read account");
    let account_data = Account::unpack(&fetched_account.data).unwrap();
    f(account_data);
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

#[allow(dead_code)]
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
