use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer, SetAuthority};
use spl_token::instruction::AuthorityType;

declare_id!("75ci5gzBz9Rn5LT2vHvSzowkBqrDjXvKpTgCaG4HHP7g");

pub mod constants {
    pub const AGTE_TOKEN_PUBKEY: &str = "4QV4wzDdy7S1EV6y2r9DkmaDsHeoKz6HUvFLVtAsu6dV";
    pub const PROGRAM_OWNER: &str = "956Zsf8FQigRQ55q3siLzSyNSNFWsqeB4k5RaUbwzEgZ";
}

pub fn calculate_redeem(apy: u16, amount: u64, last_redeem_date: i64) -> u64{
    let current_time = Clock::get().unwrap().unix_timestamp;

    let stake_hours = ((current_time - last_redeem_date) as f64 / 3600.).round();
    let redeem_value = (apy as f64 / 8760. / 100. * amount as f64 * stake_hours as f64).round() as u64;
    msg!("Redeem value: {}", redeem_value);
    return redeem_value
}


#[program]
pub mod agrostaking {
    use anchor_lang::__private::bytemuck::Contiguous;
    use super::*;

    const PDA_SEED_NAME: &[u8] = b"agrostaking";

    pub fn initialize(ctx: Context<Initialize>, agte_bump: u8, apy: u16) -> ProgramResult {
        ctx.accounts.settings_account.apy = apy;
        ctx.accounts.settings_account.agte_bump = agte_bump;
        ctx.accounts.settings_account.staked_amount = 0;

        let (settings_authority, _settings_authority_bump) =
            Pubkey::find_program_address(&[b"settings"], ctx.program_id);

        let cpi_accounts = SetAuthority {
            current_authority: ctx.accounts.agte_user.to_account_info().clone(),
            account_or_mint: ctx.accounts.agte_account.to_account_info().clone(),
        };
        let cpi_context = CpiContext::new(ctx.accounts.token_program.to_account_info().clone(), cpi_accounts);

        token::set_authority(
            cpi_context,
            AuthorityType::AccountOwner,
            Some(settings_authority),
        )?;

        Ok(())
    }

    pub fn stake_init(ctx: Context<InitStake>, staker_bump: u8) -> ProgramResult {
        ctx.accounts.staking_info.last_redeem_date = Clock::get().unwrap().unix_timestamp;
        ctx.accounts.staking_info.staker_bump = staker_bump;
        ctx.accounts.staking_info.apy = ctx.accounts.settings_account.apy.clone();

        let (staking_authority, _staking_authority_bump) =
            Pubkey::find_program_address(&[ctx.accounts.user.key.as_ref(), PDA_SEED_NAME], ctx.program_id);

        let cpi_accounts = SetAuthority {
            current_authority: ctx.accounts.staked_user.to_account_info().clone(),
            account_or_mint: ctx.accounts.staking_account.to_account_info().clone(),
        };
        let cpi_context = CpiContext::new(ctx.accounts.token_program.to_account_info().clone(), cpi_accounts);

        token::set_authority(
            cpi_context,
            AuthorityType::AccountOwner,
            Some(staking_authority),
        )?;

        Ok(())
    }

    pub fn stake(ctx: Context<Stake>, amount: u64) -> ProgramResult {
        if ctx.accounts.token_from.mint.to_string() != constants::AGTE_TOKEN_PUBKEY { //TODO: Move this to account constraints
            msg!("You try to stake not agte token");
            return Err(ErrorCode::BadMint.into())
        }

        let apy = ctx.accounts.settings_account.apy;
        let exist_amount = ctx.accounts.staking_account.amount;
        let last_redeem = ctx.accounts.staking_info.last_redeem_date;

        let redeem_value = calculate_redeem(apy, exist_amount, last_redeem);

        ctx.accounts.staking_info.last_redeem_date = Clock::get().unwrap().unix_timestamp;
        ctx.accounts.staking_info.pending_redeem += redeem_value;

        let transfer_cpi_accounts = Transfer {
            from: ctx.accounts.token_from.to_account_info().clone(),
            to: ctx.accounts.staking_account.to_account_info().clone(),
            authority: ctx.accounts.user.to_account_info().clone()
        };

        token::transfer(
            CpiContext::new(ctx.accounts.token_program.to_account_info().clone(), transfer_cpi_accounts),
            amount
        );

        ctx.accounts.settings_account.staked_amount += amount;

        Ok(())
    }

    pub fn unstake(ctx: Context<Unstake>) -> ProgramResult {
        let amount = ctx.accounts.token_from.amount;

        // check that this PDA is owner for token_from
        let transfer_cpi_accounts = Transfer {
            from: ctx.accounts.token_from.to_account_info().clone(),
            to: ctx.accounts.token_to.to_account_info().clone(),
            authority: ctx.accounts.staking_info.to_account_info().clone()
        };

        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info().clone(),
                transfer_cpi_accounts,
                &[&[
                    ctx.accounts.user.key.as_ref(), PDA_SEED_NAME,
                    &[ctx.accounts.staking_info.staker_bump]
                ]],
            ), amount
        );

        ctx.accounts.settings_account.staked_amount -= amount;

        Ok(())
    }

    pub fn redeem(ctx: Context<Redeem>) -> ProgramResult {
        let apy = ctx.accounts.staking_info.apy;
        let amount = ctx.accounts.staking_account.amount;
        let last_redeem = ctx.accounts.staking_info.last_redeem_date;
        let pending_redeem = ctx.accounts.staking_info.pending_redeem;

        let redeem_value = calculate_redeem(apy, amount, last_redeem) + pending_redeem;

        let transfer_cpi_accounts = Transfer {
            from: ctx.accounts.agte_account.to_account_info().clone(),
            to: ctx.accounts.token_to.to_account_info().clone(),
            authority: ctx.accounts.settings_account.to_account_info().clone()
        };

        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info().clone(),
                transfer_cpi_accounts,
                &[&[
                    b"settings",
                    &[ctx.accounts.settings_account.agte_bump]
                ]],
            ), redeem_value
        );

        ctx.accounts.staking_info.last_redeem_date = Clock::get().unwrap().unix_timestamp;;
        ctx.accounts.staking_info.pending_redeem = 0;

        Ok(())
    }

    pub fn stake_nft(ctx: Context<StakeNFT>) -> ProgramResult {
        let (staking_authority, _staking_authority_bump) =
            Pubkey::find_program_address(&[ctx.accounts.user.key.as_ref(), PDA_SEED_NAME], ctx.program_id);

        let cpi_accounts = SetAuthority {
            current_authority: ctx.accounts.staked_user.to_account_info().clone(),
            account_or_mint: ctx.accounts.staking_account.to_account_info().clone(),
        };
        let cpi_context = CpiContext::new(ctx.accounts.token_program.to_account_info().clone(), cpi_accounts);

        token::set_authority(
            cpi_context,
            AuthorityType::AccountOwner,
            Some(staking_authority),
        )?;

        let apy = ctx.accounts.staking_info.apy;
        let exist_amount = ctx.accounts.agte_account.amount;
        let last_redeem = ctx.accounts.staking_info.last_redeem_date;

        let redeem_value = calculate_redeem(apy, exist_amount, last_redeem);

        ctx.accounts.staking_info.apy = apy + 10;
        ctx.accounts.staking_info.last_redeem_date = Clock::get().unwrap().unix_timestamp;
        ctx.accounts.staking_info.pending_redeem += redeem_value;

        let transfer_cpi_accounts = Transfer {
            from: ctx.accounts.token_from.to_account_info().clone(),
            to: ctx.accounts.staking_account.to_account_info().clone(),
            authority: ctx.accounts.user.to_account_info().clone()
        };

        token::transfer(
            CpiContext::new(ctx.accounts.token_program.to_account_info().clone(), transfer_cpi_accounts),
            1
        );

        Ok(())
    }

    pub fn unstake_nft(ctx: Context<UnStakeNFT>) -> ProgramResult {
        let apy = ctx.accounts.staking_info.apy;
        let exist_amount = ctx.accounts.agte_account.amount;
        let last_redeem = ctx.accounts.staking_info.last_redeem_date;

        ctx.accounts.staking_info.apy = apy - 10;

        let redeem_value = calculate_redeem(apy, exist_amount, last_redeem);

        ctx.accounts.staking_info.last_redeem_date = Clock::get().unwrap().unix_timestamp;
        ctx.accounts.staking_info.pending_redeem += redeem_value;

        let transfer_cpi_accounts = Transfer {
            from: ctx.accounts.token_from.to_account_info().clone(),
            to: ctx.accounts.token_to.to_account_info().clone(),
            authority: ctx.accounts.staking_info.to_account_info().clone()
        };

        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info().clone(),
                transfer_cpi_accounts,
                &[&[
                    ctx.accounts.user.key.as_ref(), PDA_SEED_NAME,
                    &[ctx.accounts.staking_info.staker_bump]
                ]],
            ), 1
        );

        Ok(())
    }
}


#[account]
pub struct StakeInfo {
    pub staker_bump: u8,  // save bump
    pub last_redeem_date: i64,
    pub pending_redeem: u64,
    pub apy: u16
}

#[account]
pub struct StakingSettings {
    pub agte_bump: u8,
    pub apy: u16,
    pub staked_amount: u64
}


#[derive(Accounts)]
#[instruction(agte_bump: u8)]
pub struct Initialize<'info> {
    #[account(
        init_if_needed,
        payer = owner,
        space = 24,
        seeds = [b"settings"],
        bump = agte_bump)]
    pub settings_account: Account<'info, StakingSettings>,
    #[account(mut)]
    pub agte_account: Account<'info, TokenAccount>, // token account for settings_account for agte tokens
    #[account(mut)]
    pub agte_user: Signer<'info>,  // user who will stake

    #[account(mut, address = constants::PROGRAM_OWNER.parse::<Pubkey>().unwrap())]
    pub owner: Signer<'info>,

    pub token_program: Program<'info, Token>,  //  address for token program for transfer
    pub system_program: Program<'info, System>,
}


#[derive(Accounts)]
#[instruction(staker_bump: u8)]
pub struct InitStake<'info> {
    #[account(
        mut,
        seeds = [b"settings"],
        bump)]
    pub settings_account: Account<'info, StakingSettings>,
    #[account(
        init_if_needed,
        payer = user,
        space = 64,
        seeds = [user.key().as_ref(), b"agrostaking"],
        bump = staker_bump)]
    pub staking_info: Account<'info, StakeInfo>,
    #[account(
        mut,
        constraint = staking_account.owner != settings_account.key()
    )]
    pub staking_account: Account<'info, TokenAccount>, // when we send money
    #[account(mut)]
    pub user: Signer<'info>,  // user who will stake
    #[account(mut)]
    pub staked_user: Signer<'info>,  // owner for staking_account token account

    pub token_program: Program<'info, Token>,  //  address for token program for transfer
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(staker_bump: u8)]
pub struct Stake<'info> {
    #[account(
        mut,
        seeds = [b"settings"],
        bump)]
    pub settings_account: Account<'info, StakingSettings>,
    #[account(mut)]
    pub token_from: Account<'info, TokenAccount>, // from which user associated account we can transfer
    //the authority allowed to transfer from token_from
    #[account(
        mut,
        seeds = [user.key().as_ref(), b"agrostaking"],
        bump)]
    pub staking_info: Account<'info, StakeInfo>,
    #[account(
        mut,
        constraint = staking_account.owner == staking_info.key()
    )]
    pub staking_account: Account<'info, TokenAccount>, // when we send money
    #[account(mut)]
    pub user: Signer<'info>,  // user who will stake

    pub token_program: Program<'info, Token>,  //  address for token program for transfer
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Unstake<'info> {
    #[account(
        mut,
        seeds = [b"settings"],
        bump)]
    pub settings_account: Account<'info, StakingSettings>,
    #[account(
        mut,
        constraint = token_from.owner == staking_info.key()
    )]
    pub token_from: Account<'info, TokenAccount>, // agte token account for this user
    #[account(mut)]
    pub token_to: Account<'info, TokenAccount>, // user agte token account
    #[account(mut, seeds = [user.key().as_ref(), b"agrostaking"], bump)]
    pub staking_info: Account<'info, StakeInfo>,
    #[account(mut)]
    pub user: Signer<'info>,  // user who will unstake

    pub token_program: Program<'info, Token>,  //  address for token program for transfer
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Redeem<'info> {
     #[account(
        mut,
        seeds = [b"settings"],
        bump
     )]
    pub settings_account: Account<'info, StakingSettings>,
    #[account(
        mut,
        constraint = agte_account.owner == settings_account.key()
    )]
    pub agte_account: Account<'info, TokenAccount>, // token account for settings_account for agte tokens
    #[account(
        mut,
        constraint = staking_account.owner == staking_info.key()
    )]
    pub staking_account: Account<'info, TokenAccount>, // account to get user staked amount
    #[account(mut)]
    pub token_to: Account<'info, TokenAccount>, // user agte token account
    #[account(mut, seeds = [user.key().as_ref(), b"agrostaking"], bump)]
    pub staking_info: Account<'info, StakeInfo>,
    #[account(mut)]
    pub user: Signer<'info>,  // user who will unstake

    pub token_program: Program<'info, Token>,  //  address for token program for transfer
    pub system_program: Program<'info, System>,
}


#[derive(Accounts)]
pub struct StakeNFT<'info> {
    #[account(
        mut,
        seeds = [b"settings"],
        bump)]
    pub settings_account: Account<'info, StakingSettings>,
    #[account(
        mut
    )]
    pub token_from: Account<'info, TokenAccount>, // from which user associated account we can transfer
    //the authority allowed to transfer from token_from
    #[account(
        mut,
        seeds = [user.key().as_ref(), b"agrostaking"],
        bump)]
    pub staking_info: Account<'info, StakeInfo>,
    #[account(
        mut,
        constraint = agte_account.owner == staking_info.key()
    )]
    pub agte_account: Account<'info, TokenAccount>, // to get current agte amount and calculate reward
    #[account(
        mut,
        constraint = staking_account.owner != settings_account.key()
    )]
    pub staking_account: Account<'info, TokenAccount>, // when we send money
    #[account(mut)]
    pub user: Signer<'info>,  // user who will stake
    #[account(mut)]
    pub staked_user: Signer<'info>,  // owner for staking_account token account

    pub token_program: Program<'info, Token>,  //  address for token program for transfer
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UnStakeNFT<'info> {
    #[account(
        mut,
        seeds = [b"settings"],
        bump)]
    pub settings_account: Account<'info, StakingSettings>,
    #[account(
        mut,
        constraint = agte_account.owner == staking_info.key()
    )]
    pub agte_account: Account<'info, TokenAccount>, // to get current agte amount and calculate reward
    #[account(
        mut,
        constraint = token_from.owner == staking_info.key()
    )]
    pub token_from: Account<'info, TokenAccount>, // PDA agte token account for this user
    #[account(mut)]
    pub token_to: Account<'info, TokenAccount>, // user agte token account
    #[account(mut, seeds = [user.key().as_ref(), b"agrostaking"], bump)]
    pub staking_info: Account<'info, StakeInfo>,
    #[account(mut)]
    pub user: Signer<'info>,  // user who will unstake

    pub token_program: Program<'info, Token>,  //  address for token program for transfer
    pub system_program: Program<'info, System>,
}


#[error]
pub enum ErrorCode {
    #[msg("This mint address is invalid")]
    BadMint,
}