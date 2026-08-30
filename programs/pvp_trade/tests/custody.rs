use anchor_lang::{AccountDeserialize, InstructionData, ToAccountMetas};
use litesvm::LiteSVM;
use solana_account::Account;
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_program_option::COption;
use solana_program_pack::Pack;
use solana_signer::Signer;
use solana_transaction::Transaction;
use spl_token_interface::state::{Account as TokenAccount, AccountState, Mint};

const STAKE: u64 = 100_000_000;
const STARTING_BALANCE: u64 = 500_000_000;

struct TestContext {
    svm: LiteSVM,
    authority: Keypair,
    challenger: Keypair,
    opponent: Keypair,
    settlement_mint: solana_pubkey::Pubkey,
    challenger_source: solana_pubkey::Pubkey,
    opponent_source: solana_pubkey::Pubkey,
    protocol_config: solana_pubkey::Pubkey,
    swap_config: solana_pubkey::Pubkey,
}

fn program_instruction(
    accounts: Vec<solana_instruction::AccountMeta>,
    data: Vec<u8>,
) -> Instruction {
    Instruction {
        program_id: pvp_trade::ID,
        accounts,
        data,
    }
}

fn set_mint(svm: &mut LiteSVM, mint: solana_pubkey::Pubkey, decimals: u8) {
    let mint_state = Mint {
        mint_authority: COption::None,
        supply: STARTING_BALANCE * 2,
        decimals,
        is_initialized: true,
        freeze_authority: COption::None,
    };
    let mut data = vec![0; Mint::LEN];
    Mint::pack(mint_state, &mut data).unwrap();
    svm.set_account(
        mint,
        Account {
            lamports: 1_000_000_000,
            data,
            owner: spl_token_interface::ID,
            executable: false,
            rent_epoch: 0,
        },
    )
    .unwrap();
}

fn set_token_account(
    svm: &mut LiteSVM,
    address: solana_pubkey::Pubkey,
    mint: solana_pubkey::Pubkey,
    owner: solana_pubkey::Pubkey,
    amount: u64,
) {
    let token_state = TokenAccount {
        mint,
        owner,
        amount,
        delegate: COption::None,
        state: AccountState::Initialized,
        is_native: COption::None,
        delegated_amount: 0,
        close_authority: COption::None,
    };
    let mut data = vec![0; TokenAccount::LEN];
    TokenAccount::pack(token_state, &mut data).unwrap();
    svm.set_account(
        address,
        Account {
            lamports: 1_000_000_000,
            data,
            owner: spl_token_interface::ID,
            executable: false,
            rent_epoch: 0,
        },
    )
    .unwrap();
}

fn token_balance(svm: &LiteSVM, address: solana_pubkey::Pubkey) -> u64 {
    let account = svm.get_account(&address).expect("token account must exist");
    TokenAccount::unpack(&account.data).unwrap().amount
}

fn setup() -> TestContext {
    let mut svm = LiteSVM::new();
    let program_path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/deploy/pvp_trade.so");
    svm.add_program_from_file(pvp_trade::ID, program_path)
        .unwrap();
    let mock_program_path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/deploy/mock_swap.so");
    svm.add_program_from_file(mock_swap::ID, mock_program_path)
        .unwrap();

    let authority = Keypair::new();
    let challenger = Keypair::new();
    let opponent = Keypair::new();
    for signer in [&authority, &challenger, &opponent] {
        svm.airdrop(&signer.pubkey(), 10_000_000_000).unwrap();
    }

    let settlement_mint = solana_pubkey::Pubkey::new_unique();
    let challenger_source = solana_pubkey::Pubkey::new_unique();
    let opponent_source = solana_pubkey::Pubkey::new_unique();
    set_mint(&mut svm, settlement_mint, 6);
    set_token_account(
        &mut svm,
        challenger_source,
        settlement_mint,
        challenger.pubkey(),
        STARTING_BALANCE,
    );
    set_token_account(
        &mut svm,
        opponent_source,
        settlement_mint,
        opponent.pubkey(),
        STARTING_BALANCE,
    );

    let (protocol_config, _) =
        solana_pubkey::Pubkey::find_program_address(&[b"protocol"], &pvp_trade::ID);
    let initialize = program_instruction(
        pvp_trade::accounts::InitializeProtocol {
            protocol_config,
            settlement_mint,
            authority: authority.pubkey(),
            system_program: anchor_lang::system_program::ID,
        }
        .to_account_metas(None),
        pvp_trade::instruction::InitializeProtocol {
            max_settlement_fee_bps: 500,
            default_trading_lock_seconds: 300,
        }
        .data(),
    );
    let transaction = Transaction::new_signed_with_payer(
        &[initialize],
        Some(&authority.pubkey()),
        &[&authority],
        svm.latest_blockhash(),
    );
    svm.send_transaction(transaction).unwrap();

    let (swap_config, _) =
        solana_pubkey::Pubkey::find_program_address(&[b"swap_config"], &pvp_trade::ID);
    let initialize_swap = program_instruction(
        pvp_trade::accounts::InitializeSwapConfig {
            protocol_config,
            swap_config,
            authority: authority.pubkey(),
            swap_program: mock_swap::ID,
            system_program: anchor_lang::system_program::ID,
        }
        .to_account_metas(None),
        pvp_trade::instruction::InitializeSwapConfig {
            max_slippage_bps: 500,
        }
        .data(),
    );
    let transaction = Transaction::new_signed_with_payer(
        &[initialize_swap],
        Some(&authority.pubkey()),
        &[&authority],
        svm.latest_blockhash(),
    );
    svm.send_transaction(transaction).unwrap();

    TestContext {
        svm,
        authority,
        challenger,
        opponent,
        settlement_mint,
        challenger_source,
        opponent_source,
        protocol_config,
        swap_config,
    }
}

fn create_token_policy(
    context: &mut TestContext,
    mint: solana_pubkey::Pubkey,
    safe_enabled: bool,
    meme_enabled: bool,
) -> solana_pubkey::Pubkey {
    let (token_policy, _) = solana_pubkey::Pubkey::find_program_address(
        &[b"token_policy", mint.as_ref()],
        &pvp_trade::ID,
    );
    let instruction = program_instruction(
        pvp_trade::accounts::CreateTokenPolicy {
            protocol_config: context.protocol_config,
            token_policy,
            mint,
            authority: context.authority.pubkey(),
            system_program: anchor_lang::system_program::ID,
        }
        .to_account_metas(None),
        pvp_trade::instruction::CreateTokenPolicy {
            safe_enabled,
            meme_enabled,
        }
        .data(),
    );
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&context.authority.pubkey()),
        &[&context.authority],
        context.svm.latest_blockhash(),
    );
    context.svm.send_transaction(transaction).unwrap();
    token_policy
}

fn join_and_start(
    context: &mut TestContext,
    battle: solana_pubkey::Pubkey,
) -> solana_pubkey::Pubkey {
    let (opponent_vault, _) = solana_pubkey::Pubkey::find_program_address(
        &[
            b"vault",
            battle.as_ref(),
            context.opponent.pubkey().as_ref(),
        ],
        &pvp_trade::ID,
    );
    let join = program_instruction(
        pvp_trade::accounts::JoinBattle {
            protocol_config: context.protocol_config,
            battle,
            settlement_mint: context.settlement_mint,
            opponent_source: context.opponent_source,
            opponent_vault,
            opponent: context.opponent.pubkey(),
            token_program: spl_token_interface::ID,
            system_program: anchor_lang::system_program::ID,
        }
        .to_account_metas(None),
        pvp_trade::instruction::JoinBattle {}.data(),
    );
    let start = program_instruction(
        pvp_trade::accounts::BattleActor {
            protocol_config: context.protocol_config,
            battle,
            actor: context.challenger.pubkey(),
        }
        .to_account_metas(None),
        pvp_trade::instruction::StartBattle {}.data(),
    );
    let join_transaction = Transaction::new_signed_with_payer(
        &[join],
        Some(&context.opponent.pubkey()),
        &[&context.opponent],
        context.svm.latest_blockhash(),
    );
    context.svm.send_transaction(join_transaction).unwrap();
    let start_transaction = Transaction::new_signed_with_payer(
        &[start],
        Some(&context.challenger.pubkey()),
        &[&context.challenger],
        context.svm.latest_blockhash(),
    );
    context.svm.send_transaction(start_transaction).unwrap();
    opponent_vault
}

fn create_battle(
    context: &mut TestContext,
    battle_id: [u8; 32],
    source: solana_pubkey::Pubkey,
) -> (solana_pubkey::Pubkey, solana_pubkey::Pubkey) {
    let (battle, _) = solana_pubkey::Pubkey::find_program_address(
        &[b"battle", battle_id.as_ref()],
        &pvp_trade::ID,
    );
    let (challenger_vault, _) = solana_pubkey::Pubkey::find_program_address(
        &[
            b"vault",
            battle.as_ref(),
            context.challenger.pubkey().as_ref(),
        ],
        &pvp_trade::ID,
    );
    let instruction = program_instruction(
        pvp_trade::accounts::CreateBattle {
            protocol_config: context.protocol_config,
            battle,
            settlement_mint: context.settlement_mint,
            challenger_source: source,
            challenger_vault,
            challenger: context.challenger.pubkey(),
            token_program: spl_token_interface::ID,
            system_program: anchor_lang::system_program::ID,
        }
        .to_account_metas(None),
        pvp_trade::instruction::CreateBattle {
            battle_id,
            stake_micro_usdc: STAKE,
            duration_seconds: 86_400,
            trading_lock_seconds: 300,
            settlement_fee_bps: 200,
            arena: pvp_trade::Arena::Safe,
        }
        .data(),
    );
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&context.challenger.pubkey()),
        &[&context.challenger],
        context.svm.latest_blockhash(),
    );
    context.svm.send_transaction(transaction).unwrap();
    (battle, challenger_vault)
}

#[test]
fn equal_stakes_enter_isolated_player_vaults() {
    let mut context = setup();
    let battle_id = [7; 32];
    let challenger_source = context.challenger_source;
    let (battle, challenger_vault) = create_battle(&mut context, battle_id, challenger_source);
    let (opponent_vault, _) = solana_pubkey::Pubkey::find_program_address(
        &[
            b"vault",
            battle.as_ref(),
            context.opponent.pubkey().as_ref(),
        ],
        &pvp_trade::ID,
    );

    let join = program_instruction(
        pvp_trade::accounts::JoinBattle {
            protocol_config: context.protocol_config,
            battle,
            settlement_mint: context.settlement_mint,
            opponent_source: context.opponent_source,
            opponent_vault,
            opponent: context.opponent.pubkey(),
            token_program: spl_token_interface::ID,
            system_program: anchor_lang::system_program::ID,
        }
        .to_account_metas(None),
        pvp_trade::instruction::JoinBattle {}.data(),
    );
    let transaction = Transaction::new_signed_with_payer(
        &[join],
        Some(&context.opponent.pubkey()),
        &[&context.opponent],
        context.svm.latest_blockhash(),
    );
    context.svm.send_transaction(transaction).unwrap();

    assert_ne!(challenger_vault, opponent_vault);
    assert_eq!(token_balance(&context.svm, challenger_vault), STAKE);
    assert_eq!(token_balance(&context.svm, opponent_vault), STAKE);
    assert_eq!(
        token_balance(&context.svm, context.challenger_source),
        STARTING_BALANCE - STAKE
    );
    assert_eq!(
        token_balance(&context.svm, context.opponent_source),
        STARTING_BALANCE - STAKE
    );

    let raw_battle = context.svm.get_account(&battle).unwrap();
    let battle_state = pvp_trade::Battle::try_deserialize(&mut raw_battle.data.as_slice()).unwrap();
    assert_eq!(battle_state.status, pvp_trade::BattleStatus::Funded);
    assert_eq!(battle_state.challenger_vault, challenger_vault);
    assert_eq!(battle_state.opponent_vault, opponent_vault);
}

#[test]
fn cancelling_an_open_battle_refunds_the_challenger() {
    let mut context = setup();
    let challenger_source = context.challenger_source;
    let (battle, challenger_vault) = create_battle(&mut context, [8; 32], challenger_source);

    let cancel = program_instruction(
        pvp_trade::accounts::CancelBattle {
            battle,
            challenger: context.challenger.pubkey(),
            settlement_mint: context.settlement_mint,
            challenger_vault,
            challenger_refund: context.challenger_source,
            token_program: spl_token_interface::ID,
        }
        .to_account_metas(None),
        pvp_trade::instruction::CancelBattle {}.data(),
    );
    let transaction = Transaction::new_signed_with_payer(
        &[cancel],
        Some(&context.challenger.pubkey()),
        &[&context.challenger],
        context.svm.latest_blockhash(),
    );
    context.svm.send_transaction(transaction).unwrap();

    assert_eq!(token_balance(&context.svm, challenger_vault), 0);
    assert_eq!(
        token_balance(&context.svm, context.challenger_source),
        STARTING_BALANCE
    );
}

#[test]
fn wrong_mint_and_insufficient_balance_are_rejected_atomically() {
    let mut context = setup();
    let wrong_mint = solana_pubkey::Pubkey::new_unique();
    let wrong_source = solana_pubkey::Pubkey::new_unique();
    set_mint(&mut context.svm, wrong_mint, 6);
    set_token_account(
        &mut context.svm,
        wrong_source,
        wrong_mint,
        context.challenger.pubkey(),
        STARTING_BALANCE,
    );

    let wrong_mint_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        create_battle(&mut context, [9; 32], wrong_source)
    }));
    assert!(wrong_mint_result.is_err());

    let low_balance_source = solana_pubkey::Pubkey::new_unique();
    set_token_account(
        &mut context.svm,
        low_balance_source,
        context.settlement_mint,
        context.challenger.pubkey(),
        STAKE - 1,
    );
    let battle_id = [10; 32];
    let (battle, _) = solana_pubkey::Pubkey::find_program_address(
        &[b"battle", battle_id.as_ref()],
        &pvp_trade::ID,
    );
    let low_balance_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        create_battle(&mut context, battle_id, low_balance_source)
    }));
    assert!(low_balance_result.is_err());
    assert!(context.svm.get_account(&battle).is_none());
}

struct SwapFixture {
    battle: solana_pubkey::Pubkey,
    challenger_vault: solana_pubkey::Pubkey,
    asset_vault: solana_pubkey::Pubkey,
    output_mint: solana_pubkey::Pubkey,
    input_policy: solana_pubkey::Pubkey,
    output_policy: solana_pubkey::Pubkey,
    pool_input: solana_pubkey::Pubkey,
    pool_output: solana_pubkey::Pubkey,
    pool_authority: solana_pubkey::Pubkey,
}

fn setup_swap_fixture(context: &mut TestContext, battle_id: [u8; 32]) -> SwapFixture {
    let output_mint = solana_pubkey::Pubkey::new_unique();
    set_mint(&mut context.svm, output_mint, 6);
    let settlement_mint = context.settlement_mint;
    let input_policy = create_token_policy(context, settlement_mint, true, true);
    let output_policy = create_token_policy(context, output_mint, true, true);
    let challenger_source = context.challenger_source;
    let (battle, challenger_vault) = create_battle(context, battle_id, challenger_source);
    join_and_start(context, battle);

    let (asset_vault, _) = solana_pubkey::Pubkey::find_program_address(
        &[
            b"asset_vault",
            battle.as_ref(),
            context.challenger.pubkey().as_ref(),
            output_mint.as_ref(),
        ],
        &pvp_trade::ID,
    );
    let create_vault = program_instruction(
        pvp_trade::accounts::CreateAssetVault {
            battle,
            token_policy: output_policy,
            mint: output_mint,
            asset_vault,
            actor: context.challenger.pubkey(),
            token_program: spl_token_interface::ID,
            system_program: anchor_lang::system_program::ID,
        }
        .to_account_metas(None),
        pvp_trade::instruction::CreateAssetVault {}.data(),
    );
    let transaction = Transaction::new_signed_with_payer(
        &[create_vault],
        Some(&context.challenger.pubkey()),
        &[&context.challenger],
        context.svm.latest_blockhash(),
    );
    context.svm.send_transaction(transaction).unwrap();

    let (pool_authority, _) =
        solana_pubkey::Pubkey::find_program_address(&[b"pool"], &mock_swap::ID);
    let pool_input = solana_pubkey::Pubkey::new_unique();
    let pool_output = solana_pubkey::Pubkey::new_unique();
    set_token_account(
        &mut context.svm,
        pool_input,
        context.settlement_mint,
        pool_authority,
        0,
    );
    set_token_account(
        &mut context.svm,
        pool_output,
        output_mint,
        pool_authority,
        STARTING_BALANCE,
    );

    SwapFixture {
        battle,
        challenger_vault,
        asset_vault,
        output_mint,
        input_policy,
        output_policy,
        pool_input,
        pool_output,
        pool_authority,
    }
}

fn execute_swap_instruction(
    context: &TestContext,
    fixture: &SwapFixture,
    route_destination: solana_pubkey::Pubkey,
) -> Instruction {
    execute_swap_instruction_with_limits(
        context,
        fixture,
        route_destination,
        39_000_000,
        40_000_000,
        250,
    )
}

fn execute_swap_instruction_with_limits(
    context: &TestContext,
    fixture: &SwapFixture,
    route_destination: solana_pubkey::Pubkey,
    minimum_amount_out: u64,
    quoted_amount_out: u64,
    slippage_bps: u16,
) -> Instruction {
    const AMOUNT_IN: u64 = 20_000_000;
    const AMOUNT_OUT: u64 = 40_000_000;
    let mut accounts = pvp_trade::accounts::ExecuteSwap {
        protocol_config: context.protocol_config,
        swap_config: context.swap_config,
        battle: fixture.battle,
        input_policy: fixture.input_policy,
        output_policy: fixture.output_policy,
        input_mint: context.settlement_mint,
        output_mint: fixture.output_mint,
        input_vault: fixture.challenger_vault,
        output_vault: fixture.asset_vault,
        actor: context.challenger.pubkey(),
        swap_program: mock_swap::ID,
    }
    .to_account_metas(None);
    accounts.extend([
        AccountMeta::new(fixture.challenger_vault, false),
        AccountMeta::new(route_destination, false),
        AccountMeta::new(fixture.pool_input, false),
        AccountMeta::new(fixture.pool_output, false),
        AccountMeta::new_readonly(context.settlement_mint, false),
        AccountMeta::new_readonly(fixture.output_mint, false),
        AccountMeta::new_readonly(fixture.challenger_vault, false),
        AccountMeta::new_readonly(fixture.pool_authority, false),
        AccountMeta::new_readonly(spl_token_interface::ID, false),
    ]);
    if route_destination != fixture.asset_vault {
        accounts.push(AccountMeta::new_readonly(fixture.asset_vault, false));
    }
    program_instruction(
        accounts,
        pvp_trade::instruction::ExecuteSwap {
            amount_in: AMOUNT_IN,
            minimum_amount_out,
            quoted_amount_out,
            slippage_bps,
            route_data: mock_swap::instruction::Swap {
                amount_in: AMOUNT_IN,
                amount_out: AMOUNT_OUT,
            }
            .data(),
        }
        .data(),
    )
}

#[test]
fn quoted_output_and_slippage_cannot_hide_an_unsafe_minimum() {
    let mut context = setup();
    let fixture = setup_swap_fixture(&mut context, [13; 32]);
    let swap = execute_swap_instruction_with_limits(
        &context,
        &fixture,
        fixture.asset_vault,
        1,
        40_000_000,
        250,
    );
    let transaction = Transaction::new_signed_with_payer(
        &[swap],
        Some(&context.challenger.pubkey()),
        &[&context.challenger],
        context.svm.latest_blockhash(),
    );
    let error = context
        .svm
        .send_transaction(transaction)
        .expect_err("unsafe minimum output must fail");

    assert!(
        error
            .meta
            .logs
            .iter()
            .any(|log| log.contains("InvalidMinimumOutput")),
        "swap must fail before CPI when its minimum output violates the declared slippage"
    );
    assert_eq!(token_balance(&context.svm, fixture.challenger_vault), STAKE);
    assert_eq!(token_balance(&context.svm, fixture.asset_vault), 0);
    assert_eq!(token_balance(&context.svm, fixture.pool_input), 0);
    assert_eq!(
        token_balance(&context.svm, fixture.pool_output),
        STARTING_BALANCE
    );
}

#[test]
fn approved_swap_moves_exact_input_into_the_players_asset_vault() {
    let mut context = setup();
    let fixture = setup_swap_fixture(&mut context, [11; 32]);
    let swap = execute_swap_instruction(&context, &fixture, fixture.asset_vault);
    let transaction = Transaction::new_signed_with_payer(
        &[swap],
        Some(&context.challenger.pubkey()),
        &[&context.challenger],
        context.svm.latest_blockhash(),
    );
    context.svm.send_transaction(transaction).unwrap();

    assert_eq!(
        token_balance(&context.svm, fixture.challenger_vault),
        STAKE - 20_000_000
    );
    assert_eq!(token_balance(&context.svm, fixture.asset_vault), 40_000_000);
    assert_eq!(token_balance(&context.svm, fixture.pool_input), 20_000_000);
    assert_eq!(
        token_balance(&context.svm, fixture.pool_output),
        STARTING_BALANCE - 40_000_000
    );
}

#[test]
fn route_cannot_redirect_swap_output_to_an_attacker() {
    let mut context = setup();
    let fixture = setup_swap_fixture(&mut context, [12; 32]);
    let attacker = Keypair::new();
    let attacker_destination = solana_pubkey::Pubkey::new_unique();
    set_token_account(
        &mut context.svm,
        attacker_destination,
        fixture.output_mint,
        attacker.pubkey(),
        0,
    );
    let swap = execute_swap_instruction(&context, &fixture, attacker_destination);
    let transaction = Transaction::new_signed_with_payer(
        &[swap],
        Some(&context.challenger.pubkey()),
        &[&context.challenger],
        context.svm.latest_blockhash(),
    );
    let error = context
        .svm
        .send_transaction(transaction)
        .expect_err("redirected output must fail");

    assert!(
        error
            .meta
            .logs
            .iter()
            .any(|log| log.contains("MinimumOutputNotMet")),
        "swap must fail specifically because the player's output vault received too little"
    );
    assert_eq!(token_balance(&context.svm, fixture.challenger_vault), STAKE);
    assert_eq!(token_balance(&context.svm, fixture.asset_vault), 0);
    assert_eq!(token_balance(&context.svm, attacker_destination), 0);
    assert_eq!(token_balance(&context.svm, fixture.pool_input), 0);
    assert_eq!(
        token_balance(&context.svm, fixture.pool_output),
        STARTING_BALANCE
    );
}
