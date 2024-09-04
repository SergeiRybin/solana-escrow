use solana_program::program_error::ProgramError;

pub enum EscrowInstruction<'a> {
    Init { seed: &'a [u8], bump_seed: u8 },
    Deposit { amount_expected: u32 },
    Execute { amount_expected: u32 },
    Reclaim,
}

pub fn parse_data(instruction_data: &[u8]) -> Result<EscrowInstruction, ProgramError> {
    let data_len = instruction_data.len();
    assert!(data_len > 0);
    let instruction = instruction_data[0];
    match instruction {
        0 => Ok(EscrowInstruction::Init {
            seed: &instruction_data[1..data_len - 1],
            bump_seed: instruction_data[data_len - 1],
        }),
        1 => {
            let arr: [u8; 4] = instruction_data[1..5]
                .try_into()
                .map_err(|_| ProgramError::InvalidInstructionData)?;
            // Todo: make transmute
            Ok(EscrowInstruction::Deposit {
                amount_expected: unsafe { std::mem::transmute::<[u8; 4], u32>(arr) },
            })
        }
        2 => {
            let arr: [u8; 4] = instruction_data[1..5]
                .try_into()
                .map_err(|_| ProgramError::InvalidInstructionData)?;
            Ok(EscrowInstruction::Execute {
                amount_expected: unsafe { std::mem::transmute::<[u8; 4], u32>(arr) },
            })
        }
        3 => Ok(EscrowInstruction::Reclaim),
        _ => Err(ProgramError::InvalidInstructionData),
    }
}
