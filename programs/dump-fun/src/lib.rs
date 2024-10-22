use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{mint_to, Mint, MintTo, Token, TokenAccount},
    metadata::{
        create_metadata_accounts_v3,
        mpl_token_metadata::types::DataV2,
        CreateMetadataAccountsV3, 
        Metadata as Metaplex,
    },
};
use anchor_lang::solana_program::system_instruction;
use program::DumpFun;

declare_id!("3UQnAcGLoi8cpi9LkiJdjDnmkiW2n3LxgSosD9Cecs5P");

#[program]
pub mod dump_fun {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, params: InitParams) -> Result<()> {
        let global = &mut ctx.accounts.global;

        global.fee_recipient = params.fee_recipient;
        global.initial_virtual_token_reserves = params.initial_virtual_token_reserves;
        global.initial_virtual_sol_reserves = params.initial_virtual_sol_reserves;
        global.initial_real_token_reserves = params.initial_real_token_reserves;
        global.initial_real_sol_reserves = params.initial_real_sol_reserves;
        global.total_token_supply = params.total_token_supply;
        global.fee_basis_points = params.fee_basis_points;

        emit!(InitGlobalParamsEvent{
            fee_recipient: params.fee_recipient,
            initial_virtual_token_reserves: params.initial_virtual_token_reserves,
            initial_virtual_sol_reserves: params.initial_virtual_sol_reserves,
            initial_real_token_reserves: params.initial_real_token_reserves,
            initial_real_sol_reserves: params.initial_real_sol_reserves,
            total_token_supply: params.total_token_supply,
            fee_basis_points: params.fee_basis_points
        });

        Ok(())
    }

    pub fn create(ctx: Context<Create>, params: CreateParams) -> Result<()> {
        let seeds = &["mint".as_bytes(), &[ctx.bumps.mint]];
        let signer = [&seeds[..]];

        let global = &mut ctx.accounts.global;

        let token_data: DataV2 = DataV2 {
            name: params.name,
            symbol: params.symbol,
            uri: params.uri,
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

        let from_account = &ctx.accounts.payer;
        let to_account = &ctx.accounts.associated_bonding_curve;

        let transfer_instruction = system_instruction::transfer(from_account.key, &to_account.key(), global.initial_real_sol_reserves);

        anchor_lang::solana_program::program::invoke_signed(
            &transfer_instruction,
            &[
                from_account.to_account_info(),
                to_account.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
            &[],
        )?;

        msg!("SOL reserve transferred successfully.");

        let bonding_curve = &mut ctx.accounts.bonding_curve;

        bonding_curve.virtual_token_reserves = global.initial_virtual_token_reserves;
        bonding_curve.virtual_sol_reserves = global.initial_virtual_sol_reserves;
        bonding_curve.real_token_reserves = global.initial_real_token_reserves;
        bonding_curve.real_sol_reserves = global.initial_real_sol_reserves;
        bonding_curve.total_token_supply = global.total_token_supply;
        bonding_curve.target = params.target;
        bonding_curve.complete = false;

        Ok(())
    }

    // pub fn buy(ctx: Context<Create>, params: CreateParams) -> Result<()> {
    //     Ok(())
    // }
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

#[account]
pub struct Global {
    fee_recipient: Pubkey,
    initial_virtual_token_reserves: u64,
    initial_virtual_sol_reserves: u64,
    initial_real_token_reserves: u64,
    initial_real_sol_reserves: u64,
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
pub struct InitParams {
    fee_recipient: Pubkey,
    initial_virtual_token_reserves: u64,
    initial_virtual_sol_reserves: u64,
    initial_real_token_reserves: u64,
    initial_real_sol_reserves: u64,
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

#[event]
pub struct InitGlobalParamsEvent {
    fee_recipient: Pubkey,
    initial_virtual_token_reserves: u64,
    initial_virtual_sol_reserves: u64,
    initial_real_token_reserves: u64,
    initial_real_sol_reserves: u64,
    total_token_supply: u64,
    fee_basis_points: u64
}

impl Space for Global {
    const INIT_SPACE: usize = 32 + 8 + 8 + 8 + 8 + 8 + 8;
}

impl Space for BondingCurve {
    const INIT_SPACE: usize = 8 + 8 + 8 + 8 + 8 + 8 + 1;
}