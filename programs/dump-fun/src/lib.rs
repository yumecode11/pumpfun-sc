use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{mint_to, Mint, MintTo, Token, TokenAccount, Transfer as SplTransfer},
    metadata::{
        create_metadata_accounts_v3,
        mpl_token_metadata::types::DataV2,
        CreateMetadataAccountsV3, 
        Metadata as Metaplex,
    },
};
use anchor_lang::solana_program::{clock::Clock, system_instruction};
use program::DumpFun;
use anchor_spl::token;

declare_id!("3UQnAcGLoi8cpi9LkiJdjDnmkiW2n3LxgSosD9Cecs5P");

#[program]
pub mod dump_fun {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let global = &mut ctx.accounts.global;

        global.initailized = true;

        Ok(())
    }

    pub fn set(ctx: Context<Set>, params: GlobalParams) -> Result<()> {
        let global = &mut ctx.accounts.global;
        require!(global.initailized, Errors::Initialization);

        global.authority = params.authority;
        global.fee_recipient = params.fee_recipient;
        global.initial_virtual_token_reserves = params.initial_virtual_token_reserves;
        global.initial_virtual_sol_reserves = params.initial_virtual_sol_reserves;
        global.initial_real_token_reserves = params.initial_real_token_reserves;
        global.total_token_supply = params.total_token_supply;
        global.fee_basis_points = params.fee_basis_points;

        emit!(InitEvent{
            fee_recipient: params.fee_recipient,
            initial_virtual_token_reserves: params.initial_virtual_token_reserves,
            initial_virtual_sol_reserves: params.initial_virtual_sol_reserves,
            initial_real_token_reserves: params.initial_real_token_reserves,
            total_token_supply: params.total_token_supply,
            fee_basis_points: params.fee_basis_points
        });

        Ok(())
    }

    pub fn create(ctx: Context<Create>, params: CreateParams) -> Result<()> {
        let seeds = &["mint".as_bytes(), &[ctx.bumps.mint]];
        let signer = [&seeds[..]];

        let global = &mut ctx.accounts.global;
        let bonding_curve = &mut ctx.accounts.bonding_curve;

        // let from_account = &ctx.accounts.payer;
        // let to_account = global.fee_recipient;

        // let transfer_instruction = system_instruction::transfer(from_account.key, &to_account.key(), global.fee_basis_points);

        // anchor_lang::solana_program::program::invoke_signed(
        //     &transfer_instruction,
        //     &[
        //         from_account.to_account_info(),
        //         to_account.,
        //         ctx.accounts.system_program.to_account_info(),
        //     ],
        //     &[],
        // )?;

        // msg!("Fees transferred successfully.");

        let token_data: DataV2 = DataV2 {
            name: params.name.clone(),
            symbol: params.symbol.clone(),
            uri: params.uri.clone(),
            seller_fee_basis_points: 0,
            creators: None,
            collection: None,
            uses: None,
        };

        let metadata_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_metadata_program.to_account_info(),
            CreateMetadataAccountsV3 {
                payer: ctx.accounts.payer.to_account_info(),
                update_authority: ctx.accounts.mint.to_account_info(),
                mint: ctx.accounts.mint.to_account_info(),
                metadata: ctx.accounts.metadata.to_account_info(),
                mint_authority: ctx.accounts.mint.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
                rent: ctx.accounts.rent.to_account_info(),
            },
            &signer
        );

        create_metadata_accounts_v3(
            metadata_ctx,
            token_data,
            false,
            true,
            None,
        )?;

        msg!("Token mint created successfully.");

        mint_to(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                MintTo {
                    authority: ctx.accounts.mint.to_account_info(),
                    to: ctx.accounts.associated_bonding_curve.to_account_info(),
                    mint: ctx.accounts.mint.to_account_info(),
                },
                &signer,
            ),
            global.initial_real_token_reserves,
        )?;

        msg!("Token reserve minted successfully.");

        bonding_curve.virtual_token_reserves = global.initial_virtual_token_reserves;
        bonding_curve.virtual_sol_reserves = global.initial_virtual_sol_reserves;
        bonding_curve.real_token_reserves = global.initial_real_token_reserves;
        bonding_curve.real_sol_reserves = 0;
        bonding_curve.total_token_supply = global.total_token_supply;
        bonding_curve.target = params.target;
        bonding_curve.complete = false;

        msg!("Bonding curve initialized.");

        emit!(CreateEvent{
            name: params.name,
            symbol: params.symbol,
            uri: params.uri,
            mint: ctx.accounts.mint.key(),
            bonding_curve: ctx.accounts.bonding_curve.key(),
            user: ctx.accounts.payer.key()
        });

        Ok(())
    }

    pub fn buy(ctx: Context<Buy>, params: BuyParams) -> Result<()> {
        let bonding_curve = &mut ctx.accounts.bonding_curve;

        let from_account = &ctx.accounts.payer;
        let to_account = &ctx.accounts.associated_bonding_curve;

        let transfer_instruction = system_instruction::transfer(from_account.key, &to_account.key(), params.sol_in);

        anchor_lang::solana_program::program::invoke_signed(
            &transfer_instruction,
            &[
                from_account.to_account_info(),
                to_account.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
            &[],
        )?;

        msg!("SOL transferred successfully.");

        let from_ata = &ctx.accounts.associated_bonding_curve;
        let to_ata = &ctx.accounts.associated_token_account;
        let token_program = &ctx.accounts.token_program;
        let authority = &ctx.accounts.payer;

        let cpi_accounts = SplTransfer {
            from: from_ata.to_account_info().clone(),
            to: to_ata.to_account_info().clone(),
            authority: authority.to_account_info().clone(),
        };
        let cpi_program = token_program.to_account_info();

        token::transfer(
            CpiContext::new(cpi_program, cpi_accounts),
            params.token_out
        )?;

        msg!("Token transferred successfully.");

        bonding_curve.virtual_token_reserves -= params.token_out;
        bonding_curve.virtual_sol_reserves += params.sol_in;
        bonding_curve.real_token_reserves -= params.token_out;
        bonding_curve.real_sol_reserves += params.sol_in;

        msg!("Bonding curve updated.");

        let clock = Clock::get().unwrap();
        let timestamp = clock.unix_timestamp;

        emit!(TradeEvent{
            mint: ctx.accounts.mint.key(),
            sol_amount: params.sol_in,
            token_amount: params.token_out,
            is_buy: true,
            user: ctx.accounts.payer.key(),
            timestamp: timestamp,
            virtual_token_reserves: bonding_curve.virtual_token_reserves,
            virtual_sol_reserves: bonding_curve.virtual_sol_reserves
        });

        if bonding_curve.real_sol_reserves >= bonding_curve.target {
            bonding_curve.complete = true;

            emit!(CompleteEvent{
                mint: ctx.accounts.mint.key(),
                user: ctx.accounts.payer.key(),
                bonding_curve: ctx.accounts.bonding_curve.key(),
                timestamp: timestamp
            })
        }

        Ok(())
    }

    pub fn sell(ctx: Context<Sell>, params: SellParams) -> Result<()> {
        let bonding_curve = &mut ctx.accounts.bonding_curve;
        require_gt!(bonding_curve.real_sol_reserves, 70);

        let from_account = &ctx.accounts.associated_bonding_curve;
        let to_account = &ctx.accounts.payer;

        let transfer_instruction = system_instruction::transfer(&from_account.key(), to_account.key, params.sol_out);

        anchor_lang::solana_program::program::invoke_signed(
            &transfer_instruction,
            &[
                from_account.to_account_info(),
                to_account.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
            &[],
        )?;

        msg!("SOL transferred successfully.");

        let from_ata = &ctx.accounts.associated_token_account;
        let to_ata = &ctx.accounts.associated_bonding_curve;
        let token_program = &ctx.accounts.token_program;
        let authority = &ctx.accounts.mint;

        let cpi_accounts = SplTransfer {
            from: from_ata.to_account_info().clone(),
            to: to_ata.to_account_info().clone(),
            authority: authority.to_account_info().clone(),
        };
        let cpi_program = token_program.to_account_info();

        token::transfer(
            CpiContext::new(cpi_program, cpi_accounts),
            params.token_in
        )?;

        msg!("Token transferred successfully.");

        bonding_curve.virtual_token_reserves += params.token_in;
        bonding_curve.virtual_sol_reserves -= params.sol_out;
        bonding_curve.real_token_reserves += params.token_in;
        bonding_curve.real_sol_reserves -= params.sol_out;

        msg!("Bonding curve updated.");

        let clock = Clock::get().unwrap();
        let timestamp = clock.unix_timestamp;

        emit!(TradeEvent{
            mint: ctx.accounts.mint.key(),
            sol_amount: params.sol_out,
            token_amount: params.token_in,
            is_buy: false,
            user: ctx.accounts.payer.key(),
            timestamp: timestamp,
            virtual_token_reserves: bonding_curve.virtual_token_reserves,
            virtual_sol_reserves: bonding_curve.virtual_sol_reserves
        });

        if bonding_curve.real_sol_reserves >= bonding_curve.target {
            bonding_curve.complete = true;

            emit!(CompleteEvent{
                mint: ctx.accounts.mint.key(),
                user: ctx.accounts.payer.key(),
                bonding_curve: ctx.accounts.bonding_curve.key(),
                timestamp: timestamp
            })
        }

        Ok(())
    }

    pub fn withdraw(ctx: Context<Withdraw>) -> Result<()> {
        let bonding_curve = &mut ctx.accounts.bonding_curve;
        require!(bonding_curve.complete, Errors::IncompleteBondingCurve);

        let from_account = &ctx.accounts.associated_bonding_curve;
        let to_account = &ctx.accounts.payer;

        let transfer_instruction = system_instruction::transfer(&from_account.key(), to_account.key, bonding_curve.real_sol_reserves);

        anchor_lang::solana_program::program::invoke_signed(
            &transfer_instruction,
            &[
                from_account.to_account_info(),
                to_account.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
            &[],
        )?;

        msg!("SOL withdrawn successfully.");

        let from_ata = &ctx.accounts.associated_bonding_curve;
        let to_ata = &ctx.accounts.associated_token_account;
        let token_program = &ctx.accounts.token_program;
        let authority = &ctx.accounts.mint;

        let cpi_accounts = SplTransfer {
            from: from_ata.to_account_info().clone(),
            to: to_ata.to_account_info().clone(),
            authority: authority.to_account_info().clone(),
        };
        let cpi_program = token_program.to_account_info();

        token::transfer(
            CpiContext::new(cpi_program, cpi_accounts),
            bonding_curve.real_token_reserves
        )?;

        msg!("Token withdrawn successfully.");

        bonding_curve.virtual_token_reserves = 0;
        bonding_curve.virtual_sol_reserves = 0;
        bonding_curve.real_token_reserves = 0;
        bonding_curve.real_sol_reserves = 0;

        msg!("Bonding curve updated.");

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        seeds = [b"global"],
        bump,
        payer = payer,
        space = Global::INIT_SPACE
    )]
    pub global: Account<'info, Global>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Set<'info> {
    #[account(
        mut,
        seeds = [b"global"],
        bump
    )]
    pub global: Account<'info, Global>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
    pub program: Program<'info, DumpFun>
}

#[derive(Accounts)]
#[instruction(params: CreateParams)]
pub struct Create<'info> {
    /// CHECK: New Metaplex Account being created
    #[account(mut)]
    pub metadata: UncheckedAccount<'info>,
    #[account(
        init,
        seeds = [b"mint", mint.key().as_ref()],
        bump,
        payer = payer,
        mint::decimals = 6,
        mint::authority = mint
    )]
    pub mint: Account<'info, Mint>,
    #[account(
        init,
        seeds = [b"bonding-curve", mint.key().as_ref()],
        bump,
        payer = payer,
        space = BondingCurve::INIT_SPACE,
    )]
    pub bonding_curve: Account<'info, BondingCurve>,
    #[account(
        init_if_needed,
        payer = payer,
        associated_token::mint = mint,
        associated_token::authority = mint
    )]
    pub associated_bonding_curve: Account<'info, TokenAccount>,
    #[account(mut, seeds = [b"global"], bump)]
    pub global: Account<'info, Global>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub rent: Sysvar<'info, Rent>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_metadata_program: Program<'info, Metaplex>,
    pub program: Program<'info, DumpFun>
}

#[derive(Accounts)]
#[instruction(params: BuyParams)]
pub struct Buy<'info> {
    #[account(mut, seeds = [b"mint", mint.key().as_ref()], bump)]
    pub mint: Account<'info, Mint>,
    #[account(mut, seeds = [b"bonding-curve", mint.key().as_ref()], bump)]
    pub bonding_curve: Account<'info, BondingCurve>,
    #[account(mut)]
    pub associated_bonding_curve: Account<'info, TokenAccount>,
    #[account(mut, seeds = [b"global"], bump)]
    pub global: Account<'info, Global>,
    #[account(
        init_if_needed,
        payer = payer,
        associated_token::mint = mint,
        associated_token::authority = payer
    )]
    pub associated_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub rent: Sysvar<'info, Rent>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub program: Program<'info, DumpFun>
}

#[derive(Accounts)]
#[instruction(params: SellParams)]
pub struct Sell<'info> {
    #[account(mut, seeds = [b"mint", mint.key().as_ref()], bump)]
    pub mint: Account<'info, Mint>,
    #[account(mut, seeds = [b"bonding-curve", mint.key().as_ref()], bump)]
    pub bonding_curve: Account<'info, BondingCurve>,
    #[account(mut)]
    pub associated_bonding_curve: Account<'info, TokenAccount>,
    #[account(mut, seeds = [b"global"], bump)]
    pub global: Account<'info, Global>,
    #[account(mut)]
    pub associated_token_account: Account<'info, TokenAccount>,
    pub payer: Signer<'info>,
    pub rent: Sysvar<'info, Rent>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub program: Program<'info, DumpFun>
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut, seeds = [b"mint", mint.key().as_ref()], bump)]
    pub mint: Account<'info, Mint>,
    #[account(mut, seeds = [b"bonding-curve", mint.key().as_ref()], bump)]
    pub bonding_curve: Account<'info, BondingCurve>,
    #[account(mut)]
    pub associated_bonding_curve: Account<'info, TokenAccount>,
    #[account(mut, seeds = [b"global"], bump)]
    pub global: Account<'info, Global>,
    #[account(
        init_if_needed,
        payer = payer,
        associated_token::mint = mint,
        associated_token::authority = payer
    )]
    pub associated_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub rent: Sysvar<'info, Rent>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub program: Program<'info, DumpFun>
}

#[account]
pub struct Global {
    initailized: bool,
    authority: Pubkey,
    fee_recipient: Pubkey,
    initial_virtual_token_reserves: u64,
    initial_virtual_sol_reserves: u64,
    initial_real_token_reserves: u64,
    total_token_supply: u64,
    fee_basis_points: u64
}

#[account]
pub struct BondingCurve {
    virtual_token_reserves: u64,
    virtual_sol_reserves: u64,
    real_token_reserves: u64,
    real_sol_reserves: u64,
    total_token_supply: u64,
    target: u64,
    complete: bool
}

#[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone)]
pub struct GlobalParams {
    authority: Pubkey,
    fee_recipient: Pubkey,
    initial_virtual_token_reserves: u64,
    initial_virtual_sol_reserves: u64,
    initial_real_token_reserves: u64,
    total_token_supply: u64,
    fee_basis_points: u64
}

#[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone)]
pub struct CreateParams {
    name: String,
    symbol: String,
    uri: String,
    target: u64,
}

#[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone)]
pub struct BuyParams {
    sol_in: u64,
    token_out: u64
}

#[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone)]
pub struct SellParams {
    sol_out: u64,
    token_in: u64
}

#[event]
pub struct InitEvent {
    fee_recipient: Pubkey,
    initial_virtual_token_reserves: u64,
    initial_virtual_sol_reserves: u64,
    initial_real_token_reserves: u64,
    total_token_supply: u64,
    fee_basis_points: u64
}

#[event]
pub struct CreateEvent {
    name: String,
    symbol: String,
    uri: String,
    mint: Pubkey,
    bonding_curve: Pubkey,
    user: Pubkey
}

#[event]
pub struct TradeEvent {
    mint: Pubkey,
    sol_amount: u64,
    token_amount: u64,
    is_buy: bool,
    user: Pubkey,
    timestamp: i64,
    virtual_token_reserves: u64,
    virtual_sol_reserves: u64
}

#[event]
pub struct CompleteEvent {
    mint: Pubkey,
    user: Pubkey,
    bonding_curve: Pubkey,
    timestamp: i64
}

impl Space for Global {
    const INIT_SPACE: usize = 32 + 8 + 8 + 8 + 8 + 8 + 8;
}

impl Space for BondingCurve {
    const INIT_SPACE: usize = 8 + 8 + 8 + 8 + 8 + 8 + 1;
}

#[error_code]
pub enum Errors {
    #[msg("The global state is not yet initialized.")]
    Initialization,

    #[msg("The bonding curve is not yet complete.")]
    IncompleteBondingCurve,
}