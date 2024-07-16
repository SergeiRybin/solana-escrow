use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use solana_program::program_error::ProgramError;
use solana_program::program_pack::{Pack, Sealed};
use solana_program::pubkey::Pubkey;
use std::mem;

pub const SEED: &[u8; 6] = b"escrow";
pub const DATA_LEN: usize = 69; //mem::size_of::<Escrow>();

// TODO: make multiple accounts
// pub struct EscrowCollection([Escrow; 10]);

#[derive(Default)]
pub struct Escrow {
    active: bool,
    amount_expected: u32,
    token_expected: Pubkey,
    existing_account: Pubkey,
}

impl Sealed for Escrow {}
impl Pack for Escrow {
    const LEN: usize = DATA_LEN;

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, Escrow::LEN];
        let (active_dst, amount_expected_dst, token_expected_dst, existing_account_dst) =
            mut_array_refs![dst, 1, 4, 32, 32];

        active_dst[0] = self.active as u8;
        *amount_expected_dst = self.amount_expected.to_le_bytes();
        token_expected_dst.copy_from_slice(self.token_expected.as_ref());
        existing_account_dst.copy_from_slice(self.existing_account.as_ref());
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let src = array_ref![src, 0, Escrow::LEN];
        let (active_src, amount_expected_src, token_expected_src, existing_account_src) =
            array_refs![src, 1, 4, 32, 32];
        Ok(Self {
            active: active_src[0] != 0,
            amount_expected: u32::from_le_bytes(*amount_expected_src),
            token_expected: Pubkey::new_from_array(*token_expected_src),
            existing_account: Pubkey::new_from_array(*existing_account_src),
        })
    }
}
