use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use solana_program::program_error::ProgramError;
use solana_program::program_pack::{Pack, Sealed};
use solana_program::pubkey::Pubkey;

pub const SEED: &[u8; 6] = b"escrow";
const DATA_LEN: usize = 101;

const COLLECTION_WIDTH: usize = 5;

#[derive(Default, Copy, Clone)]
pub struct Escrow {
    pub active: bool,
    pub amount_expected: u32,
    pub token_expected: Pubkey,
    pub holding_account: Pubkey,
    pub owner_account: Pubkey,
}

pub struct EscrowCollection {
    pub items: [Escrow; COLLECTION_WIDTH],
}

impl EscrowCollection {
    pub fn find_by_token_account(&mut self, token_account: &Pubkey) -> Option<&mut Escrow> {
        self.items
            .as_mut()
            .iter_mut()
            .find(|item| item.active && item.holding_account == *token_account)
    }

    pub fn find_next_available(&mut self) -> Option<&mut Escrow> {
        self.items.iter_mut().find(|item| !item.active)
    }
}
impl Sealed for crate::state::EscrowCollection {}
impl Pack for crate::state::EscrowCollection {
    const LEN: usize = DATA_LEN * COLLECTION_WIDTH;
    fn pack_into_slice(&self, dst: &mut [u8]) {
        for (i, item) in self.items.iter().enumerate() {
            let subdst = &mut dst[i * DATA_LEN..(i + 1) * DATA_LEN];
            item.pack_into_slice(subdst)
        }
    }
    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let mut obj = Self {
            items: [Escrow::default(); COLLECTION_WIDTH],
        };
        for (i, item) in obj.items.iter_mut().enumerate() {
            let subdst = &src[i * DATA_LEN..(i + 1) * DATA_LEN];
            *item = Escrow::unpack_from_slice(subdst)?;
        }
        Ok(obj)
    }
}

/// Storage structure created for every escrow swap
impl Escrow {
    pub fn reset(&mut self) {
        self.active = false;
        self.amount_expected = 0;
        self.token_expected = Pubkey::default();
        self.holding_account = Pubkey::default();
        self.owner_account = Pubkey::default();
    }
}
impl Sealed for Escrow {}
impl Pack for Escrow {
    const LEN: usize = DATA_LEN;

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, Escrow::LEN];
        let (
            active_dst,
            amount_expected_dst,
            token_expected_dst,
            holding_account_dst,
            owner_account_dst,
        ) = mut_array_refs![dst, 1, 4, 32, 32, 32];

        active_dst[0] = self.active as u8;
        *amount_expected_dst = self.amount_expected.to_le_bytes();
        token_expected_dst.copy_from_slice(self.token_expected.as_ref());
        holding_account_dst.copy_from_slice(self.holding_account.as_ref());
        owner_account_dst.copy_from_slice(self.owner_account.as_ref());
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let src = array_ref![src, 0, Escrow::LEN];
        let (
            active_src,
            amount_expected_src,
            token_expected_src,
            holding_account_src,
            owner_account_src,
        ) = array_refs![src, 1, 4, 32, 32, 32];
        Ok(Self {
            active: active_src[0] != 0,
            amount_expected: u32::from_le_bytes(*amount_expected_src),
            token_expected: Pubkey::new_from_array(*token_expected_src),
            holding_account: Pubkey::new_from_array(*holding_account_src),
            owner_account: Pubkey::new_from_array(*owner_account_src),
        })
    }
}
