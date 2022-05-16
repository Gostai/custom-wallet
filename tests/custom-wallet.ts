import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { CustomWallet } from "../target/types/custom_wallet";
import { TOKEN_PROGRAM_ID, Token } from "@solana/spl-token";
import { assert } from "chai";

describe("custom-wallet", () => {
  // Use a local provider.
  const provider = anchor.Provider.local();

  // Configure the client to use the local cluster.
  anchor.setProvider(provider);
  
  const program = anchor.workspace.CustomWallet as Program<CustomWallet>;
  
  //Initialize variables
  let mintToken = null as Token;
  let userTokenAccount = null;
  let recepientTokenAccount = null;
  let vault_sol_account_pda = null;
  let vault_sol_account_bump = null;
  let vault_account_pda = null;  
  let vault_account_bump = null;
  let vault_authority_pda = null;
  
  const solAmount =1000000000;
  const userAmount = 1000;
  const transferAmount = 750;
  const transferBackAmount = 250;
  const allowAmount = 100;
  
  const mintAuthority = anchor.web3.Keypair.generate();
  const payer = anchor.web3.Keypair.generate();
  const walletAccount = anchor.web3.Keypair.generate();
  const recepientAccount = anchor.web3.Keypair.generate();
  
  let feeAccount = null;
  const feeAddress = "4LnHwNdQCBEV9YHQtjz5oPYjZiJu7WYsFx9RGvTZmxYT";  
  const seed = "uncover find gloom alley carpet shy ride attend reunion aerobic acoustic lady";  
  
  it("Initialize program state", async () => {
      
    // Airdropping tokens to a payer.
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(payer.publicKey, solAmount),
      "processed"
    );
    // Airdropping tokens to a recepientAccount
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(recepientAccount.publicKey, solAmount),
      "processed"
    );    
    
    //Create token mint
    mintToken = await Token.createMint(
      provider.connection,
      payer,
      mintAuthority.publicKey,
      null,
      0,
      TOKEN_PROGRAM_ID
    );
    
    //Create token accounts
    userTokenAccount = 
        await mintToken.createAccount(payer.publicKey);
     
    recepientTokenAccount = 
        await mintToken.createAccount(recepientAccount.publicKey);
    
    //Mint tokens to first account
    await mintToken.mintTo(
      userTokenAccount,
      mintAuthority.publicKey,
      [mintAuthority],
      userAmount
    );
     
    let _userTokenAccount  = await mintToken.getAccountInfo(userTokenAccount);
     
    let _recepientTokenAccount  = await mintToken.getAccountInfo(recepientTokenAccount);
     
    assert.ok(_userTokenAccount.amount.toNumber() == userAmount);
    assert.ok(_recepientTokenAccount.amount.toNumber() == 0);
    
    //Create fee account
    let hex = Uint8Array.from(Buffer.from(seed));
    feeAccount = anchor.web3.Keypair.fromSeed(hex.slice(0, 32));
    assert.ok(feeAccount.publicKey.toString() == feeAddress);
    
    // Airdropping tokens to a feeAccount 
    await provider.connection.confirmTransaction(
    await provider.connection.requestAirdrop(feeAccount.publicKey, solAmount),
      "processed"
    );
     
  });     
     
  it("Initialize wallet", async () => {
    
    //Find PDA for vaultAccount with SOL
    const [_vault_sol_account_pda, _vault_sol_account_bump] = await anchor.web3.PublicKey.findProgramAddress(
      [Buffer.from(anchor.utils.bytes.utf8.encode("sol-seed"))],
      program.programId
    );
    vault_sol_account_pda = _vault_sol_account_pda;
    vault_sol_account_bump = _vault_sol_account_bump;
    
    //Find PDA for vaultAccount  
    const [_vault_account_pda, _vault_account_bump] = await anchor.web3.PublicKey.findProgramAddress(
      [Buffer.from(anchor.utils.bytes.utf8.encode("token-seed"))],
      program.programId
    );
    vault_account_pda = _vault_account_pda;
    vault_account_bump = _vault_account_bump;
    
    //Find PDA for vault authority
    const [_vault_authority_pda, _vault_authority_bump] = await anchor.web3.PublicKey.findProgramAddress(
      [Buffer.from(anchor.utils.bytes.utf8.encode("wallet"))],
      program.programId
    );
    vault_authority_pda = _vault_authority_pda;
    
    //Initialize wallet account with state
    let tx = await program.rpc.initialize(          
          payer.publicKey,      
      {
        accounts: {
          initializer: payer.publicKey,
          walletAccount: walletAccount.publicKey,
          vaultSolAccount: vault_sol_account_pda,          
          vaultAccount: vault_account_pda,
          mint: mintToken.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,  
          rent: anchor.web3.SYSVAR_RENT_PUBKEY,
          tokenProgram: TOKEN_PROGRAM_ID,
        },
        instructions: [
          await program.account.walletAccount.createInstruction(walletAccount),
        ],
        signers: [walletAccount, payer],
      }
    );     
  
    let _vault = await mintToken.getAccountInfo(vault_account_pda);
     
    let _walletAccount = await program.account.walletAccount.fetch(
         walletAccount.publicKey
    );     
    
    assert.ok(_vault.owner.equals(vault_authority_pda));
    assert.ok(_vault.amount.toNumber()==0);    
    assert.ok(_walletAccount.allowance==false);
    assert.ok(_walletAccount.feeValue.toNumber()==10);          
   
  });
  
   it("Set wallet fee", async () => {      
    let newFee = 15;
    
    //Setting new fee in walletAccount in percents
    let tx = await program.rpc.setFee(          
           new anchor.BN(newFee),      
      {
        accounts: {
          authority: payer.publicKey,
          walletAccount: walletAccount.publicKey,          
        },       
        signers: [payer],
      }
    );
     
    let _walletAccount = await program.account.walletAccount.fetch(
         walletAccount.publicKey
    );
    
    assert.ok(_walletAccount.feeValue.toNumber()==15);
    
  });
  
   
   it("Transfer SOL to wallet", async () => {
      
    let transferAmount = 2000;
    
    let _vault_sol_before = await provider.connection.getBalance(vault_sol_account_pda);
    
    let _user_before = await provider.connection.getBalance(payer.publicKey);
    
    let _fee_before = await provider.connection.getBalance(feeAccount.publicKey);
    
    //Make a transfer to wallets vault for SOL
    await program.rpc.transferSolFrom(
          new anchor.BN(transferAmount),      
      {
        accounts: {          
          walletAccount: walletAccount.publicKey,
          user: payer.publicKey,
          vaultSolAccount: vault_sol_account_pda,
          feeAccount: feeAccount.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,  
        },
        signers: [payer],
      }
    );
     
    let _user_after = await provider.connection.getBalance(payer.publicKey);
  
    let _vault_sol = await provider.connection.getBalance(vault_sol_account_pda);
    
    let _fee_after = await provider.connection.getBalance(feeAccount.publicKey);    
    
    assert.ok(_vault_sol_before + transferAmount- 300==_vault_sol); 
    assert.ok(_fee_after - _fee_before==300);      
    
  });
   
   
  it("Transfer SOL from wallet to recepient", async () => {
      
    let transferAmount = 2000;
    
    let _vault_sol_before = await provider.connection.getBalance(vault_sol_account_pda);
    
    let _recepient_before = await provider.connection.getBalance(recepientAccount.publicKey);
    
    let _fee_before = await provider.connection.getBalance(feeAccount.publicKey);    
     
    //Make a transfer from wallets vault 
    await program.rpc.transferSolTo(
          new anchor.BN(transferAmount),      
      {
        accounts: {          
          walletAccount: walletAccount.publicKey,
          authority: payer.publicKey,
          recepient: recepientAccount.publicKey,
          vaultSolAccount: vault_sol_account_pda,
          feeAccount: feeAccount.publicKey,          
          systemProgram: anchor.web3.SystemProgram.programId,   
        },        
        signers: [payer],
      }
    );
     
    let _recepient_after = await provider.connection.getBalance(recepientAccount.publicKey);
  
    let _vault_sol = await provider.connection.getBalance(vault_sol_account_pda);
    
    let _fee_after = await provider.connection.getBalance(feeAccount.publicKey);
          
    assert.ok(_recepient_before + transferAmount - 300 ==_recepient_after); 
    assert.ok(_fee_after - _fee_before==300); 
  });  
  
  it("Transfer tokens to wallet", async () => {
      
    let transferAmount = 750;
    
    //Make a transfer to wallets vault for tokens
    await program.rpc.transferFrom(
          new anchor.BN(transferAmount),      
      {
        accounts: {          
          walletAccount: walletAccount.publicKey,
          user: payer.publicKey,
          vaultAccount: vault_account_pda,
          userDepositTokenAccount: userTokenAccount,
          systemProgram: anchor.web3.SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
        },        
        signers: [payer],
      }
    );     
  
    let _vault = await mintToken.getAccountInfo(vault_account_pda);
     
    // Check new vault amount
    assert.ok(_vault.amount.toNumber()==transferAmount);
     
    let _userTokenAccount  = await mintToken.getAccountInfo(userTokenAccount);
     
    assert.ok(_userTokenAccount.amount.toNumber() == userAmount-transferAmount);
    
  });
  
  it("Transfer tokens from wallet", async () => {
     
    //Make a transfer from wallets vault for tokens
    await program.rpc.transferTo(
          new anchor.BN(transferBackAmount),      
      {
        accounts: {          
          walletAccount: walletAccount.publicKey,
          authority: payer.publicKey,
          vaultAuthority: vault_authority_pda,
          vaultAccount: vault_account_pda,
          userDepositTokenAccount: userTokenAccount,
          systemProgram: anchor.web3.SystemProgram.programId,            
          tokenProgram: TOKEN_PROGRAM_ID,
        },        
        signers: [payer],
      }
    );     
  
    let _vault = await mintToken.getAccountInfo(vault_account_pda);
     
    // Check new vault amount
    assert.ok(_vault.amount.toNumber()==transferAmount-transferBackAmount);
     
    let _userTokenAccount  = await mintToken.getAccountInfo(userTokenAccount);
     
    assert.ok(_userTokenAccount.amount.toNumber() == userAmount-transferAmount+transferBackAmount);
    
  });
  
  
  it("Set transfer allowance to recepient", async () => {
    
    //Setiing vault amount and recepient authority in walletAccount
    await program.rpc.allowTo(
          new anchor.BN(allowAmount),      
      {
        accounts: {          
          walletAccount: walletAccount.publicKey,
          authority: payer.publicKey,          
          vaultAccount: vault_account_pda,
          recepient: recepientAccount.publicKey,          
        },        
        signers: [payer],
      }
    );
    
    let _walletAccount = await program.account.walletAccount.fetch(
         walletAccount.publicKey
    );
    
    assert.ok(_walletAccount.allowance == true);    
    assert.ok(_walletAccount.recepient.equals(recepientAccount.publicKey));
    assert.ok(_walletAccount.allowanceValue == allowAmount);
    
  });
  
  
  it("Take allowance by recepient", async () => {
    
    //Take the allowance by recepient and zeroing aloowance state
    await program.rpc.takeAllowance(         
      {
        accounts: {
            
          walletAccount: walletAccount.publicKey,
          recepient: recepientAccount.publicKey,
          recepientAccount:recepientTokenAccount,
          vaultAuthority: vault_authority_pda,
          vaultAccount: vault_account_pda,
          systemProgram: anchor.web3.SystemProgram.programId,            
          tokenProgram: TOKEN_PROGRAM_ID,
          
        },        
        signers: [recepientAccount],
      }
    );
    
    let _walletAccount = await program.account.walletAccount.fetch(
         walletAccount.publicKey
    );
     
    let _recepientTokenAccount  = await mintToken.getAccountInfo(recepientTokenAccount);     
     
    assert.ok(_recepientTokenAccount.amount.toNumber() == allowAmount);
    assert.ok(_walletAccount.allowance == false);
    assert.ok(_walletAccount.allowanceValue == 0);
    
  });
  
});
