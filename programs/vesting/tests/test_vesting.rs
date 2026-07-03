use {
    anchor_lang::{
        prelude::Pubkey,
        solana_program::{clock::Clock, instruction::Instruction, system_program},
        AccountDeserialize, InstructionData, ToAccountMetas,
    },
    anchor_spl::{
        associated_token::{self, get_associated_token_address_with_program_id},
        token,
    },
    litesvm::LiteSVM,
    litesvm_token::{
        get_spl_account, spl_token::state::Account, CreateAssociatedTokenAccount, CreateMint,
        MintTo,
    },
    solana_keypair::Keypair,
    solana_message::{Message, VersionedMessage},
    solana_signer::Signer,
    solana_transaction::versioned::VersionedTransaction,
    vesting::VESTING_SEED,
};

#[test]
fn test_vesting() {
    let program_id = vesting::id();
    let mut svm = LiteSVM::new();
    let bytes = include_bytes!(concat!(
        env!("CARGO_TARGET_TMPDIR"),
        "/../deploy/vesting.so"
    ));
    svm.add_program(program_id, bytes).unwrap();

    let creator = Keypair::new();
    svm.airdrop(&creator.pubkey(), 1_000_000_000).unwrap();

    let beneficiary = Keypair::new();
    svm.airdrop(&beneficiary.pubkey(), 1_000_000_000).unwrap();

    let current_time = svm.get_sysvar::<Clock>().unix_timestamp;

    let id = 1u64;
    let amount = 100_000_000_000u64;
    let start = current_time;
    let cliff = current_time + 60 * 60;
    let duration = 60 * 60 * 4;

    let mint = CreateMint::new(&mut svm, &creator)
        .authority(&creator.pubkey())
        .decimals(9)
        .send()
        .unwrap();
    let creator_token_ata = CreateAssociatedTokenAccount::new(&mut svm, &creator, &mint)
        .owner(&creator.pubkey())
        .send()
        .unwrap();
    MintTo::new(&mut svm, &creator, &mint, &creator_token_ata, amount)
        .owner(&creator)
        .send()
        .unwrap();

    let (vesting, vesting_bump) = Pubkey::find_program_address(
        &[
            VESTING_SEED,
            creator.pubkey().as_ref(),
            beneficiary.pubkey().as_ref(),
            mint.as_ref(),
            &id.to_le_bytes(),
        ],
        &program_id,
    );
    let vesting_vault = get_associated_token_address_with_program_id(&vesting, &mint, &token::ID);
    let beneficiary_token_ata =
        get_associated_token_address_with_program_id(&beneficiary.pubkey(), &mint, &token::ID);

    let instruction = Instruction::new_with_bytes(
        program_id,
        &vesting::instruction::Create {
            id,
            amount,
            start,
            cliff,
            duration,
        }
        .data(),
        vesting::accounts::Create {
            creator: creator.pubkey(),
            beneficiary: beneficiary.pubkey(),
            mint,
            creator_token_ata,
            vesting,
            vesting_vault,
            associated_token_program: associated_token::ID,
            token_program: token::ID,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );
    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&[instruction], Some(&creator.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[&creator]).unwrap();
    svm.send_transaction(tx).unwrap();
    svm.expire_blockhash();
    let vesting_account = svm.get_account(&vesting).unwrap();
    let mut data: &[u8] = &vesting_account.data;
    let vesting_state = vesting::state::Vesting::try_deserialize(&mut data).unwrap();
    assert_eq!(vesting_state.id, id);
    assert_eq!(vesting_state.creator, creator.pubkey());
    assert_eq!(vesting_state.beneficiary, beneficiary.pubkey());
    assert_eq!(vesting_state.mint, mint);
    assert_eq!(vesting_state.total_amount, amount);
    assert_eq!(vesting_state.total_claimed, 0);
    assert_eq!(vesting_state.start, start);
    assert_eq!(vesting_state.cliff, cliff);
    assert_eq!(vesting_state.duration, duration);
    assert_eq!(vesting_state.bump, vesting_bump);

    let instruction = Instruction::new_with_bytes(
        program_id,
        &vesting::instruction::Claim { id }.data(),
        vesting::accounts::Claim {
            beneficiary: beneficiary.pubkey(),
            creator: creator.pubkey(),
            mint,
            beneficiary_token_ata,
            vesting,
            vesting_vault,
            associated_token_program: associated_token::ID,
            token_program: token::ID,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );
    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&[instruction], Some(&beneficiary.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[&beneficiary]).unwrap();
    let res = svm.send_transaction(tx);
    svm.expire_blockhash();
    assert!(res.is_err());

    let mut clock = svm.get_sysvar::<Clock>();
    clock.unix_timestamp = cliff + 1;
    svm.set_sysvar(&clock);

    let instruction = Instruction::new_with_bytes(
        program_id,
        &vesting::instruction::Claim { id }.data(),
        vesting::accounts::Claim {
            beneficiary: beneficiary.pubkey(),
            creator: creator.pubkey(),
            mint,
            beneficiary_token_ata,
            vesting,
            vesting_vault,
            associated_token_program: associated_token::ID,
            token_program: token::ID,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );
    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&[instruction], Some(&beneficiary.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[&beneficiary]).unwrap();
    svm.send_transaction(tx).unwrap();
    svm.expire_blockhash();
    let beneficiary_token_ata_account: Account =
        get_spl_account(&svm, &beneficiary_token_ata).unwrap();
    assert!(beneficiary_token_ata_account.amount > 0);

    let mut clock = svm.get_sysvar::<Clock>();
    clock.unix_timestamp = cliff + duration as i64 + 1;
    svm.set_sysvar(&clock);

    let instruction = Instruction::new_with_bytes(
        program_id,
        &vesting::instruction::Claim { id }.data(),
        vesting::accounts::Claim {
            beneficiary: beneficiary.pubkey(),
            creator: creator.pubkey(),
            mint,
            beneficiary_token_ata,
            vesting,
            vesting_vault,
            associated_token_program: associated_token::ID,
            token_program: token::ID,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );
    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&[instruction], Some(&beneficiary.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[&beneficiary]).unwrap();
    svm.send_transaction(tx).unwrap();
    svm.expire_blockhash();
    let beneficiary_token_ata_account: Account =
        get_spl_account(&svm, &beneficiary_token_ata).unwrap();
    assert_eq!(beneficiary_token_ata_account.amount, amount);
}
