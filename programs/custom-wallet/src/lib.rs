use anchor_lang::prelude::*;
use anchor_lang::solana_program::system_program;
use anchor_spl::token::{
    self,     
    Mint, 
    SetAuthority,
    TokenAccount,
    Transfer
    };
use spl_token::instruction::AuthorityType;

use std::str::FromStr;



declare_id!("6D3CSu8PQd7R9k7SD9rMNLCCVFzsgJ8CVGhckpANTqKi");

   
#[account]
pub struct WalletAccount {    
    pub allowance: bool,
    pub recepient: Pubkey,
    pub allowance_value: u64,
    pub fee_value: u64,
    pub authority: Pubkey,
}

fn fee_calculation ( &fee_value: &u64, &amount: &u64)-> u64 {
    
    let mut fee_amount = (fee_value * amount)/100;
    let reminder = (fee_value * amount)%100;
                
        if reminder!=0{
            if ((amount * 10 )/reminder) > 4 {
                fee_amount+=1;
            } 
        }
    return fee_amount;        
}

#[program]
pub mod custom_wallet {
    use super::*;
    
    const WALLET_PDA_SEED: &[u8] = b"wallet";
    const FEE_ACCOUNT: &str = "4LnHwNdQCBEV9YHQtjz5oPYjZiJu7WYsFx9RGvTZmxYT";
    
    //initialize state
    pub fn initialize(
        ctx: Context<Initialize>,  
        authority: Pubkey,
    ) -> Result<()> {
        
        ctx.accounts.wallet_account.allowance = false;       
        ctx.accounts.wallet_account.fee_value = 10;
        ctx.accounts.wallet_account.authority = authority;
        
        let (vault_authority, _vault_authority_bump) =
            Pubkey::find_program_address(
                &[WALLET_PDA_SEED],
                ctx.program_id
            );            
        
        token::set_authority(
            ctx.accounts.into_set_authority_context(),
            AuthorityType::AccountOwner,
            Some(vault_authority),
        )?;
        
        Ok(())
    }
    
    //set fee value in percents
    pub fn set_fee( 
        ctx: Context<SetFee>,          
        value: u64,        
        ) -> Result<()>   {
        
        //Check that fee percent amount betwen  0 and 100  tokens
        require!(value <= 100, MyError::FeeTooLarge);
        require!(value >= 0, MyError::FeeTooSmall);
        
        ctx.accounts.wallet_account.fee_value = value;
        
        Ok(())
    }
    
    //Transfer SOL to smart contract
    pub fn transfer_sol_from( 
        ctx: Context<TransferSOLFrom>,        
        amount: u64,        
        ) -> Result<()>   {
        
        //Check that transfer amount is more then 0 tokens        
        require!(amount > 0, MyError::AmountTooSmall);        
        
        let fee_amount=
            fee_calculation(
                &ctx.accounts.wallet_account.fee_value,
                &amount
            );   
            
        //Check that user have enouph lamports for transfer  
        if **ctx.accounts.user.try_borrow_lamports()? < amount  {
            return Err(error!(MyError::InsufficientFundsForTransaction));
        }
        
        let fee_account_saved = Pubkey::from_str(FEE_ACCOUNT)
                .expect("Unknown Account coded in smartcontract");
                
        require!(ctx.accounts.fee_account.key()==
                fee_account_saved,
                MyError::FeeToUnknown );
        
        //Transfer amount
        let ix = anchor_lang::solana_program::system_instruction::transfer(
            &ctx.accounts.user.key(),
            &ctx.accounts.vault_sol_account.key(),
            amount - fee_amount,
        );
        anchor_lang::solana_program::program::invoke(
            &ix,
            &[
                ctx.accounts.user.to_account_info(),
                ctx.accounts.vault_sol_account.to_account_info(),
            ],
        )?;        
        
        //Transfer fee
        let ix_fee = anchor_lang::solana_program::system_instruction::transfer(
            &ctx.accounts.user.key(),
            &ctx.accounts.fee_account.key(),
            fee_amount,
        );
        anchor_lang::solana_program::program::invoke(
            &ix_fee,
            &[
                ctx.accounts.user.to_account_info(),
                ctx.accounts.fee_account.to_account_info(),
            ],
        )?;
        
        Ok(())
    }
    
    //transfer SOL to recepient
    pub fn transfer_sol_to( 
        ctx: Context<TransferSOLTo>,        
        amount: u64,        
        ) -> Result<()>   {
        
        //Check that transfer amount is more then 0 tokens        
        require!(amount > 0, MyError::AmountTooSmall);
        
        let fee_account_saved = Pubkey::from_str(FEE_ACCOUNT)
                .expect("Unknown Account coded in smartcontract");
                
        require!(ctx.accounts.fee_account.key()==
                fee_account_saved,
                MyError::FeeToUnknown );        
        
        let fee_amount=
            fee_calculation(
                &ctx.accounts.wallet_account.fee_value,
                &amount
            );
        
        if **ctx.accounts
            .vault_sol_account
            .try_borrow_lamports()? < amount  {
            return Err(error!(MyError::InsufficientFundsForTransaction));
        }
        // Debit vault_sol_account and credit recepient and fee accounts
        **ctx.accounts.vault_sol_account.try_borrow_mut_lamports()? -= amount ;
        **ctx.accounts.recepient.try_borrow_mut_lamports()? += amount - fee_amount;  
        **ctx.accounts.fee_account.try_borrow_mut_lamports()? += fee_amount;
        
        Ok(())
    }
    
    //Transfer tokens to smart contract
    pub fn transfer_from( 
        ctx: Context<TransferFrom>,        
        amount: u64,        
        ) -> Result<()>   {
        
        //Check that transfer amount is more then 0 tokens 
        require!(amount > 0, MyError::AmountTooSmall);
        
        msg!("Amount of tokens user have on account {}",
            ctx.accounts.user_deposit_token_account.amount);
            
        //Check that user have enouph tokens for transfer
        require!(ctx.accounts.user_deposit_token_account.amount >= amount, MyError::InsuficientUserFunds);
        
        
        //Transfer user's tokens to treasury acoount
        token::transfer(
            ctx.accounts.into_transfer_to_pda_context(),
            amount,
        )?;
        
        Ok(())
    }
    
    //Transfer tokens to recepient
    pub fn transfer_to( 
        ctx: Context<TransferTo>,        
        amount: u64,        
        ) -> Result<()>   {
        
        //Check that transfer amount is more then 0 tokens 
        require!(amount > 0, MyError::AmountTooSmall);
                            
        //Check that wallet have enouph tokens for transfer
        require!(ctx.accounts.vault_account.amount >= amount, MyError::InsuficientWallet);
        
        //Calculate wallet authority
        let (_vault_authority, vault_authority_bump) =
            Pubkey::find_program_address(&[WALLET_PDA_SEED],
                    ctx.program_id);
        let authority_seeds = &[&WALLET_PDA_SEED[..],
            &[vault_authority_bump]];
        
        //Transfer wallets tokens to user acoount       
       token::transfer(
                ctx.accounts.into_transfer_to_user_context()
                .with_signer(&[&authority_seeds[..]]),
                amount,
        )?;        
        
        Ok(())
    }
    
    //allow to recepient
    pub fn allow_to( 
        ctx: Context<AllowTo>,          
        amount: u64,        
        ) -> Result<()>   {
                
        //Check that transfer amount is more then 0 tokens 
        require!(amount > 0, MyError::AmountTooSmall);
            
        //Check that user have enouph tokens for the bet
        require!(ctx.accounts.vault_account.amount >= amount, MyError::InsuficientWallet);
        
         ctx.accounts.wallet_account.recepient = 
            *ctx.accounts.recepient.to_account_info().key;
         ctx.accounts.wallet_account.allowance_value = 
            amount;
         ctx.accounts.wallet_account.allowance = true;
                
        Ok(())
    }    
    
    //Take allowance by the recepient
    pub fn take_allowance( 
        ctx: Context<TakeAllowance>,
        ) -> Result<()>   {
        
        require!(ctx.accounts.wallet_account.allowance ==
                true,
                MyError::ForbidedAllowanceTaking );
                
         //Calculate wallet authority
        let (_vault_authority, vault_authority_bump) =
            Pubkey::find_program_address(&[WALLET_PDA_SEED],
                    ctx.program_id);
        let authority_seeds = &[&WALLET_PDA_SEED[..],
            &[vault_authority_bump]];
        
        //Transfer wallets tokens to user acoount       
       token::transfer(
                ctx.accounts.into_transfer_allowance_context()
                .with_signer(&[&authority_seeds[..]]),
                ctx.accounts.wallet_account.allowance_value,
        )?;        
       
         ctx.accounts.wallet_account.allowance_value = 0;
         ctx.accounts.wallet_account.allowance = false;        
        
        Ok(())
    }
       
}
 

#[derive(Accounts)]
pub struct Initialize<'info> {
    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account(mut, signer)]
    pub initializer: AccountInfo<'info>,
    #[account(zero)]
    pub wallet_account: Box<Account<'info, WalletAccount>>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account(
        init,
        seeds = [b"sol-seed".as_ref()],        
        payer = initializer,
        bump,
        space = 8 + 8,
    )]
    pub vault_sol_account: AccountInfo<'info>,
    /// CHECK: This is not dangerous because we don't read or write from this account        
    pub mint: Account<'info, Mint>,
    #[account(
        init,
        seeds = [b"token-seed".as_ref()],
        bump,
        payer = initializer,
        token::mint = mint,
        token::authority = initializer,
    )]
    pub vault_account: Account<'info, TokenAccount>,    
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub system_program: AccountInfo<'info>,  
    pub rent: Sysvar<'info, Rent>,    
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub token_program: AccountInfo<'info>,
    
}


impl<'info> Initialize<'info> {
    
    fn into_set_authority_context(&self) -> CpiContext<'_, '_, '_, 'info, SetAuthority<'info>> {
        let cpi_accounts = SetAuthority {
            account_or_mint: self.vault_account.to_account_info().clone(),
            current_authority: self.initializer.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }
}

#[derive(Accounts)]
pub struct SetFee<'info> {        

    #[account(mut, has_one = authority)]
    pub wallet_account: Box<Account<'info, WalletAccount>>, 
    pub authority: Signer<'info>,
}


#[derive(Accounts)]
pub struct TransferSOLFrom<'info> {   

    #[account()]
    pub wallet_account: Box<Account<'info, WalletAccount>>,     
    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account(mut, signer)]
    pub user: AccountInfo<'info>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account(mut)]
    pub vault_sol_account: AccountInfo<'info>,   
    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account(mut)]
    pub fee_account: AccountInfo<'info>,  
    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account(address = system_program::ID)]
    pub system_program: AccountInfo<'info>,       
    
}


#[derive(Accounts)]
pub struct TransferSOLTo<'info> {
        
    #[account(mut, has_one = authority)]
    pub wallet_account: Box<Account<'info, WalletAccount>>, 
    pub authority: Signer<'info>,         
    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account(mut)]
    pub recepient: AccountInfo<'info>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account(mut)]
    pub vault_sol_account: AccountInfo<'info>, 
    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account(mut)]
    pub fee_account: AccountInfo<'info>,  
    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account(address = system_program::ID)]
    pub system_program: AccountInfo<'info>,       
    
}

#[derive(Accounts)]
pub struct TransferFrom<'info> {
        
    #[account(mut)]
    pub wallet_account: Box<Account<'info, WalletAccount>>,      
    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account(mut, signer)]
    pub user: AccountInfo<'info>,
    #[account(mut)]
    pub vault_account: Account<'info, TokenAccount>,    
    #[account(mut)]
    pub user_deposit_token_account: Account<'info, TokenAccount>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account(address = system_program::ID)]
    pub system_program: AccountInfo<'info>,       
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub token_program: AccountInfo<'info>,
}


impl<'info> TransferFrom<'info> {
    fn into_transfer_to_pda_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self
                .user_deposit_token_account
                .to_account_info()
                .clone(),
            to: self.vault_account.to_account_info().clone(),
            authority: self.user.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }
    
}


#[derive(Accounts)]
pub struct TransferTo<'info> {
        
    #[account(mut, has_one = authority )]
    pub wallet_account: Box<Account<'info, WalletAccount>>,      
    /// CHECK: This is not dangerous because we don't read or write from this account    
    pub authority: Signer<'info>,    
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub vault_authority: AccountInfo<'info>,    
    #[account(mut)]
    pub vault_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub user_deposit_token_account: Account<'info, TokenAccount>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account(address = system_program::ID)]
    pub system_program: AccountInfo<'info>,       
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub token_program: AccountInfo<'info>,   
    
}

impl<'info> TransferTo<'info> {
    fn into_transfer_to_user_context(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.vault_account.to_account_info().clone(),
            to: self
                .user_deposit_token_account
                .to_account_info()
                .clone(),
            authority: self.vault_authority.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }
        
}

#[derive(Accounts)]
pub struct AllowTo<'info> {
        
    #[account(mut, has_one = authority)]
    pub wallet_account: Box<Account<'info, WalletAccount>>,
    pub authority: Signer<'info>,
    #[account(mut)]
    pub vault_account: Account<'info, TokenAccount>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub recepient: AccountInfo<'info>,  
    
}


#[derive(Accounts)]
pub struct TakeAllowance<'info> {
        
    #[account(mut, has_one = recepient)]
    pub wallet_account: Box<Account<'info, WalletAccount>>,      
    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account(mut)]
    pub recepient_account: Account<'info, TokenAccount>,
    pub recepient: Signer<'info>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub vault_authority: AccountInfo<'info>,
    #[account(mut)]
    pub vault_account: Account<'info, TokenAccount>,    
     /// CHECK: This is not dangerous because we don't read or write from this account
    #[account(address = system_program::ID)]
    pub system_program: AccountInfo<'info>,       
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub token_program: AccountInfo<'info>,
    
}


impl<'info> TakeAllowance<'info> {
    fn into_transfer_allowance_context(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.vault_account.to_account_info().clone(),
            to: self
                .recepient_account
                .to_account_info()
                .clone(),
            authority: self.vault_authority.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }
        
}

#[error_code]
pub enum MyError {    
    #[msg("Program may only  transfer more then 0")]
    AmountTooSmall,
    #[msg("Program can set Fee less then 100%")]
    FeeTooLarge,
    #[msg("Program can set Fee more then 0%")]
    FeeTooSmall,
    #[msg("User dont have enouph funds for the trandfer")]
    InsuficientUserFunds,
    #[msg("The wallet does not respond amount for transfer")]
    InsuficientWallet,    
    #[msg("The provided fee account is unknown")]
    FeeToUnknown,    
    #[msg("Transfer is not authorized by wallet owner")]
    UnauthorizedTransfer,
    #[msg("Transfer is not allowed or already taken")]
    ForbidedAllowanceTaking,
    #[msg("There is not enougph SOL to make a transfer")]
    InsufficientFundsForTransaction   
   
}

