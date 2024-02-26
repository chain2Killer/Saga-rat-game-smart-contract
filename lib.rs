use anchor_lang::{prelude::*, error};
use anchor_spl::token::{transfer, Token, Transfer, TokenAccount, Mint};
use solana_program::entrypoint::ProgramResult;
use mpl_token_metadata::accounts::Metadata;

declare_id!("Bt8BanPyG5focmdErNVAYqLoVatNGQh75NKkMYqCfX2r");

#[program]
pub mod rat_project {
    use super::*;

    pub fn init_gamehouse(ctx: Context<InitGamehouse>, bumps: u8, rand: Pubkey, collection: Pubkey, max_bet_amount: u64, burn_amount: u64) -> ProgramResult {
        let gamehouse = &mut ctx.accounts.gamehouse;
        gamehouse.owner = ctx.accounts.creator.key();
        gamehouse.rand = rand;
        gamehouse.sol_account = ctx.accounts.sol_account.key();
        gamehouse.utility_token = ctx.accounts.utility_token.key();
        gamehouse.burn_token_account = ctx.accounts.burn_token_account.key();
        gamehouse.collection = collection;
        gamehouse.max_bet_amount = max_bet_amount;
        gamehouse.burn_amount = burn_amount;
        gamehouse.bumps = bumps;
        Ok(())
    }

    pub fn withdraw_token(ctx: Context<WithdrawToken>, amount: u64) -> ProgramResult {
        let gamehouse = &ctx.accounts.gamehouse;
        let gamehouse_seeds = &[gamehouse.rand.as_ref(),&[gamehouse.bumps]];
        let signer = &[&gamehouse_seeds[..]];
        let cpi_ctx= CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info().clone(),
            Transfer{
                from: ctx.accounts.from_account.to_account_info().clone(),
                to: ctx.accounts.to_account.to_account_info().clone(),
                authority: gamehouse.to_account_info().clone()
            },
            signer
        );
        transfer(cpi_ctx, amount)?;
        Ok(())
    }

    pub fn init_gamer_account(ctx: Context<InitGamerAccount>) -> ProgramResult {
        let gamer_data = &mut ctx.accounts.gamer_data;
        gamer_data.gamer = ctx.accounts.gamer.key();
        gamer_data.gamehouse = ctx.accounts.gamehouse.key();
        gamer_data.bet_amount = 0;
        gamer_data.win_state = 0;
        Ok(())
    }

    pub fn start_game(ctx: Context<StartGame>, amount: u64) -> ProgramResult {
        let gamer_data = &mut ctx.accounts.gamer_data;
        let gamehouse = &ctx.accounts.gamehouse;
        if amount > gamehouse.max_bet_amount || amount==0 {
            return Err(error!(GamehouseError::InvalidBettingAmount).into());
        }
        let metadata = Metadata::try_from(&ctx.accounts.metadata)?;
        if metadata.mint != ctx.accounts.nft_mint.key(){
            return Err(error!(GamehouseError::InvalidMetadata).into());
        }
        let mut verified = false;
        if metadata.creators.is_some(){
            if let Some(creators) = &metadata.creators{
                if creators.is_empty(){
                    return Err(error!(GamehouseError::InvalidMetadata).into());
                }
                for creator in creators.iter(){
                    if creator.address==gamehouse.collection && creator.verified==true{
                        verified = true;
                        break;
                    }
                }
            }
        }
        if !verified{
            return Err(error!(GamehouseError::InvalidMetadata).into());
        }
        let cpi_ctx_burn = CpiContext::new(
            ctx.accounts.token_program.to_account_info().clone(),
            Transfer{
                from: ctx.accounts.from_utility_account.to_account_info().clone(),
                to: ctx.accounts.to_utility_account.to_account_info().clone(),
                authority: ctx.accounts.gamer.to_account_info().clone()
            }
        );
        transfer(cpi_ctx_burn, gamehouse.burn_amount)?;
        let cpi_ctx_sol = CpiContext::new(
            ctx.accounts.token_program.to_account_info().clone(),
            Transfer{
                from: ctx.accounts.from_account.to_account_info().clone(),
                to: ctx.accounts.to_account.to_account_info().clone(),
                authority: ctx.accounts.gamer.to_account_info().clone()
            }
        );
        transfer(cpi_ctx_sol, amount)?;
        gamer_data.bet_amount = amount;
        let clock = &ctx.accounts.clock;
        if clock.unix_timestamp % 4 == 0{
            gamer_data.win_state = 1;
        }
        Ok(())
    }

    pub fn claim(ctx: Context<Claim>) -> ProgramResult {
        let gamer_data = &mut ctx.accounts.gamer_data;
        let gamehouse = &ctx.accounts.gamehouse;
        let gamehouse_seeds = &[gamehouse.rand.as_ref(),&[gamehouse.bumps]];
        let signer = &[&gamehouse_seeds[..]];
        let cpi_ctx= CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info().clone(),
            Transfer{
                from: ctx.accounts.from_account.to_account_info().clone(),
                to: ctx.accounts.to_account.to_account_info().clone(),
                authority: gamehouse.to_account_info().clone()
            },
            signer
        );
        transfer(cpi_ctx, gamer_data.bet_amount * 2)?;
        gamer_data.win_state = 0;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Claim<'info>{
    #[account(mut)]
    gamer: Signer<'info>,

    gamehouse: Account<'info, Gamehouse>,

    #[account(mut, has_one=gamer, has_one=gamehouse, constraint=gamer_data.win_state==1)]
    gamer_data: Account<'info, GamerData>,

    #[account(mut, address=gamehouse.sol_account)]
    from_account: Account<'info, TokenAccount>,

    #[account(mut)]
    to_account: Account<'info, TokenAccount>,

    token_program: Program<'info, Token>
}

#[derive(Accounts)]
pub struct StartGame<'info>{
    #[account(mut)]
    gamer: Signer<'info>,

    gamehouse: Account<'info, Gamehouse>,

    #[account(mut, has_one=gamer, has_one=gamehouse, constraint=gamer_data.win_state==0)]
    gamer_data: Account<'info, GamerData>,

    #[account(mut)]
    from_account: Account<'info, TokenAccount>,

    #[account(mut, address=gamehouse.sol_account)]
    to_account: Account<'info, TokenAccount>,

    #[account(mut)]
    from_utility_account: Account<'info, TokenAccount>,

    #[account(mut, address=gamehouse.burn_token_account)]
    to_utility_account: Account<'info, TokenAccount>,

    nft_mint: Account<'info, Mint>,

    #[account(constraint=nft_account.mint==nft_mint.key() && nft_account.amount==1 && nft_account.owner==gamer.key())]
    nft_account: Account<'info, TokenAccount>,

    /// CHECK: Metadata Account
    metadata : AccountInfo<'info>,

    token_program: Program<'info, Token>,

    clock : Sysvar<'info, Clock>,
}

#[derive(Accounts)]
pub struct InitGamerAccount<'info> {
    #[account(mut)]
    gamer: Signer<'info>,

    gamehouse: Account<'info, Gamehouse>,

    #[account(init, seeds=[gamer.key().as_ref(), gamehouse.key().as_ref()], bump, payer=gamer, space=8+GAMER_DATA_SIZE)]
    gamer_data: Account<'info, GamerData>,

    system_program: Program<'info, System>
}

#[derive(Accounts)]
pub struct WithdrawToken<'info> {
    #[account(mut)]
    owner: Signer<'info>,

    #[account(has_one=owner)]
    gamehouse: Account<'info, Gamehouse>,

    #[account(mut)]
    from_account: Account<'info, TokenAccount>,

    #[account(mut)]
    to_account: Account<'info, TokenAccount>,

    token_program: Program<'info, Token>
}

#[derive(Accounts)]
#[instruction(bumps:u8, rand: Pubkey)]
pub struct InitGamehouse<'info> {
    #[account(mut)]
    creator: Signer<'info>,

    #[account(init, seeds=[rand.as_ref()], bump, payer=creator, space=8+GAMEHOUSE_SIZE)]
    gamehouse: Account<'info, Gamehouse>,

    #[account(constraint=sol_account.owner==gamehouse.key())]
    sol_account: Account<'info, TokenAccount>,

    utility_token: Account<'info, Mint>,

    #[account(constraint=burn_token_account.mint==utility_token.key())]
    burn_token_account: Account<'info, TokenAccount>,

    system_program: Program<'info, System>
}

pub const GAMEHOUSE_SIZE: usize = 32 + 32 + 32 + 32 + 32 + 32 + 8 + 8 + 1;
pub const GAMER_DATA_SIZE: usize = 32 + 32 + 8 + 1;

#[account]
pub struct Gamehouse {
    pub owner: Pubkey,
    pub rand: Pubkey,
    pub sol_account: Pubkey,
    pub utility_token: Pubkey,
    pub burn_token_account: Pubkey,
    pub collection: Pubkey,
    pub max_bet_amount: u64,
    pub burn_amount: u64,
    pub bumps: u8
}

#[account]
pub struct GamerData {
    pub gamer: Pubkey,
    pub gamehouse: Pubkey,
    pub bet_amount: u64,
    pub win_state: u8,
}

#[error_code]
pub enum GamehouseError {
    #[msg("Invalid betting amount")]
    InvalidBettingAmount,

    #[msg("Invalid metadata")]
    InvalidMetadata
}
