use solana_program::account_info::AccountInfo;
use solana_program::{entrypoint, msg};
use solana_program::entrypoint::ProgramResult;
use solana_program::pubkey::Pubkey;
use crate::processor::process;

entrypoint!(process_instruction);

fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    process(
        program_id,
        accounts,
        instruction_data
    )
}
