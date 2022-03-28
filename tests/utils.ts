import * as anchor from '@project-serum/anchor';
import { TOKEN_PROGRAM_ID, Token, MintLayout } from "@solana/spl-token";

async function createRandomMint(provider, decimals) {
    const mint = await Token.createMint(
        provider.connection,
        provider.wallet.payer,
        provider.wallet.publicKey,
        null,
        decimals,
        TOKEN_PROGRAM_ID
    );
    return mint;
}

async function mintToAccount(
    provider,
    mint,
    destination,
    amount
) {
    const tx = new anchor.web3.Transaction();
    tx.add(
      Token.createMintToInstruction(
        TOKEN_PROGRAM_ID,
        mint,
        destination,
        provider.wallet.publicKey,
        [],
        amount
      )
    );
    await provider.send(tx);
}

async function sendLamports(
    provider,
    destination,
    amount
) {
    const tx = new anchor.web3.Transaction();
    tx.add(
        anchor.web3.SystemProgram.transfer(
            {
                fromPubkey: provider.wallet.publicKey,
                lamports: amount,
                toPubkey: destination
            }
        )
    );
    await provider.send(tx);
}

async function createMint(
    mintAccount,
    provider,
    mintAuthority,
    freezeAuthority,
    decimals,
    programId,
) {
    const token = new Token(
        provider.connection,
        mintAccount.publicKey,
        programId,
        provider.wallet.payer,
      );

    try {
        const info = await token.getMintInfo();
        if (info.isInitialized){
            return token
        }
    } catch (err) {
        console.log(err);
    }

    // Allocate memory for the account
    const balanceNeeded = await Token.getMinBalanceRentForExemptMint(
        provider.connection,
    );

    const transaction = new anchor.web3.Transaction();
    transaction.add(
        anchor.web3.SystemProgram.createAccount({
            fromPubkey: provider.wallet.payer.publicKey,
            newAccountPubkey: mintAccount.publicKey,
            lamports: balanceNeeded,
            space: MintLayout.span,
            programId,
        }),
    );

    transaction.add(
        Token.createInitMintInstruction(
            programId,
            mintAccount.publicKey,
            decimals,
            mintAuthority,
            freezeAuthority,
        ),
    );

    await provider.send(transaction, [mintAccount]);
    return token;
}


export {
    createRandomMint,
    createMint,
    mintToAccount,
    sendLamports,
};
