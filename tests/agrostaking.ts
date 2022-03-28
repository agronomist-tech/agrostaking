import * as anchor from '@project-serum/anchor';
import {Program, web3 } from '@project-serum/anchor';
import {Agrostaking} from '../target/types/agrostaking';

const {TOKEN_PROGRAM_ID, Token} = require("@solana/spl-token");
import {createRandomMint, mintToAccount, createMint} from './utils';
import * as assert from "assert";
import fs from "fs";


const provider = anchor.Provider.env()
anchor.setProvider(provider);
// @ts-ignore
const program = anchor.workspace.Agrostaking as Program<Agrostaking>;
let userSecret = fs.readFileSync('tests/keys/owner.json');
let keyData = JSON.parse(userSecret.toString());
const userObject = anchor.web3.Keypair.fromSecretKey(new Uint8Array(keyData));

let tokenData = fs.readFileSync('tests/keys/token.json');
let tokenKey = JSON.parse(tokenData.toString());
let mintKey = anchor.web3.Keypair.fromSecretKey(new Uint8Array(tokenKey));

let mintObject;
let mintPubkey;

let nftMintObject;
let nftMintPubkey;
let nftMintKey = anchor.web3.Keypair.fromSecretKey(new Uint8Array(
    JSON.parse(fs.readFileSync('tests/keys/nft.json').toString())
));

// @ts-ignore
const sleep = (seconds) => new Promise((resolve => setTimeout(() => resolve(), seconds * 1000)))


describe('agrostaking', () => {
    before(async () => {
        mintObject = await createMint(mintKey, provider, provider.wallet.publicKey, null, 9, TOKEN_PROGRAM_ID);
        mintPubkey = mintObject.publicKey;

        nftMintObject = await createMint(nftMintKey, provider, provider.wallet.publicKey, null, 0, TOKEN_PROGRAM_ID)
        nftMintPubkey = nftMintObject.publicKey;
    });

    it('Initialize', async () => {
        const agteProd = new web3.PublicKey("4QV4wzDdy7S1EV6y2r9DkmaDsHeoKz6HUvFLVtAsu6dV")
        const agteAcc = await web3.Keypair.generate();
        const token = new Token(
            provider.connection,
            agteProd,
            TOKEN_PROGRAM_ID,
            userObject,
        );
        const agteTokenAccount = await token.createAssociatedTokenAccount(agteAcc.publicKey);
        console.log(agteTokenAccount)

        const [settingsPDA, settingsPDABump] = await web3.PublicKey.findProgramAddress([Buffer.from("settings")], program.programId)

        const tx = await program.rpc.initialize(
            settingsPDABump,
            100, {
                accounts: {
                    settingsAccount: settingsPDA,
                    agteAccount: agteTokenAccount,
                    agteUser: agteAcc.publicKey,
                    owner: userObject.publicKey,
                    tokenProgram: TOKEN_PROGRAM_ID,
                    systemProgram: web3.SystemProgram.programId,
                },
                signers: [userObject, agteAcc]
            }
        )
        console.log('TOKEN ACC ', agteTokenAccount)
        await mintToAccount(provider, mintObject.publicKey, agteTokenAccount, 100_000_000_000_000);

        const d = await program.account.stakingSettings.fetch(settingsPDA);
        assert.equal(d.apy.toString(), 100);

        const postBalance = await provider.connection.getTokenAccountBalance(agteTokenAccount)
        assert.strictEqual(parseInt(postBalance.value.amount), 100_000_000_000);
    });

    it('Initialize stake', async () => {
        const walletTokenAccount = await mintObject.getOrCreateAssociatedAccountInfo(userObject.publicKey);

        const stakingAccount = await web3.Keypair.generate();
        const stakingTokenAccount = await mintObject.createAssociatedTokenAccount(stakingAccount.publicKey);
        const [stakingInfoPDA, stakingInfoPDABump] = await web3.PublicKey.findProgramAddress([userObject.publicKey.toBuffer(), Buffer.from("agrostaking")], program.programId)
        const [settingsPDA, settingsPDABump] = await web3.PublicKey.findProgramAddress([Buffer.from("settings")], program.programId)

        await provider.connection.requestAirdrop(provider.wallet.publicKey, web3.LAMPORTS_PER_SOL * 100);
        await provider.connection.requestAirdrop(userObject.publicKey, web3.LAMPORTS_PER_SOL * 100);

        const preBalance = await provider.connection.getTokenAccountBalance(walletTokenAccount.address);

        if (parseInt(preBalance.value.amount) === 0){
            await mintToAccount(provider, mintObject.publicKey, walletTokenAccount.address, 1_000_000_000_000);
        }

        const tx = await program.rpc.stakeInit(
            stakingInfoPDABump,
            {
                accounts: {
                    stakingInfo: stakingInfoPDA,
                    stakingAccount: stakingTokenAccount,

                    user: userObject.publicKey,
                    stakedUser: stakingAccount.publicKey,
                    settingsAccount: settingsPDA,

                    tokenProgram: TOKEN_PROGRAM_ID,
                    systemProgram: web3.SystemProgram.programId,
                },
                signers: [userObject, stakingAccount]
            });

        console.log(`Initialize stake TX: ${tx}`)

        const d = await program.account.stakeInfo.fetch(stakingInfoPDA)
        assert.ok(d.lastRedeemDate.toString());
        assert.ok(d.stakerBump.toString() === stakingInfoPDABump.toString());
        assert.equal(d.apy, 100);
    });

    it('Stake', async () => {
        const walletTokenAccount = await mintObject.getOrCreateAssociatedAccountInfo(userObject.publicKey);

        const [stakingInfoPDA, stakingInfoPDABump] = await web3.PublicKey.findProgramAddress([userObject.publicKey.toBuffer(), Buffer.from("agrostaking")], program.programId)
        const [settingsPDA, settingsPDABump] = await web3.PublicKey.findProgramAddress([Buffer.from("settings")], program.programId)

        const info = await provider.connection.getTokenAccountsByOwner(stakingInfoPDA, {mint: mintObject.publicKey})

        console.log(`Staking token accs: ${info}`)
        let stakingTokenAccount = info["value"][0].pubkey;

        for (const acc of info["value"]){
            const balance = await provider.connection.getTokenAccountBalance(acc.pubkey);
            if (parseInt(balance.value.amount) > 0) {
                stakingTokenAccount = acc.pubkey
            }
        }

        await provider.connection.requestAirdrop(provider.wallet.publicKey, web3.LAMPORTS_PER_SOL * 100);
        await provider.connection.requestAirdrop(userObject.publicKey, web3.LAMPORTS_PER_SOL * 100);

        const preBalance = await provider.connection.getTokenAccountBalance(walletTokenAccount.address);

        if (parseInt(preBalance.value.amount) === 0){
            await mintToAccount(provider, mintObject.publicKey, walletTokenAccount.address, 1_000_000_000_000);
        }
        console.log(`Staking account ${stakingTokenAccount}`)
        const tx = await program.rpc.stake(
            new anchor.BN(1_000_000_000_000), {
                accounts: {
                    settingsAccount: settingsPDA,
                    tokenFrom: walletTokenAccount.address,

                    stakingInfo: stakingInfoPDA,
                    stakingAccount: stakingTokenAccount,

                    user: userObject.publicKey,

                    tokenProgram: TOKEN_PROGRAM_ID,
                    systemProgram: web3.SystemProgram.programId,
                },
                signers: [userObject]
            });

        console.log(`Stake TX: ${tx}`)

        const d = await program.account.stakeInfo.fetch(stakingInfoPDA)
        assert.ok(d.lastRedeemDate.toString());

        const postBalance = await provider.connection.getTokenAccountBalance(stakingTokenAccount);
        assert.strictEqual(parseInt(postBalance.value.amount), 1_000_000_000_000);

        const userBalance = await provider.connection.getTokenAccountBalance(walletTokenAccount.address);
        assert.strictEqual(parseInt(userBalance.value.amount), 0);
    });

    it("Redeem", async ()=>{
        const [settingsPDA, settingsPDABump] = await web3.PublicKey.findProgramAddress([Buffer.from("settings")], program.programId)
        const [stakingInfoPDA, stakingInfoPDABump] = await web3.PublicKey.findProgramAddress([userObject.publicKey.toBuffer(), Buffer.from("agrostaking")], program.programId)
        const sendTo = await mintObject.getOrCreateAssociatedAccountInfo(userObject.publicKey);
        const info = await provider.connection.getTokenAccountsByOwner(settingsPDA, {mint: mintObject.publicKey})

        await sleep(30);

        const tx = await program.rpc.redeem(
            {
                accounts: {
                    settingsAccount: settingsPDA,
                    agteAccount: info["value"][0].pubkey,

                    tokenTo: sendTo.address,
                    stakingInfo: stakingInfoPDA,
                    user: userObject.publicKey,

                    tokenProgram: TOKEN_PROGRAM_ID,
                    systemProgram: web3.SystemProgram.programId,
                },
                signers: [userObject]
            });
        console.log(`Redeem: ${tx}`);

        const postBalance = await provider.connection.getTokenAccountBalance(sendTo.address)
        assert.ok(parseInt(postBalance.value.amount) > 0);
    })

    it('Unstake', async () => {
        const sendTo = await mintObject.getOrCreateAssociatedAccountInfo(userObject.publicKey);
        const [settingsPDA, settingsPDABump] = await web3.PublicKey.findProgramAddress([Buffer.from("settings")], program.programId)
        const [stakingInfoPDA, stakingInfoPDABump] = await web3.PublicKey.findProgramAddress([userObject.publicKey.toBuffer(), Buffer.from("agrostaking")], program.programId)
        const info = await provider.connection.getTokenAccountsByOwner(stakingInfoPDA, {mint: mintObject.publicKey})

        let sendFrom;

        for (const acc of info["value"]){
            const balance = await provider.connection.getTokenAccountBalance(acc.pubkey);
            if (parseInt(balance.value.amount) > 0) {
                sendFrom = acc.pubkey
            }
        }

        const preBalance = await provider.connection.getTokenAccountBalance(sendTo.address)
        assert.ok(parseInt(preBalance.value.amount) > 0);
        console.log(`Unstake tokenFrom: ${sendFrom}`)
        const tx = await program.rpc.unstake(
            {
                accounts: {
                    settingsAccount: settingsPDA,
                    tokenFrom: sendFrom,
                    tokenTo: sendTo.address,
                    stakingInfo: stakingInfoPDA,

                    user: userObject.publicKey,

                    tokenProgram: TOKEN_PROGRAM_ID,
                    systemProgram: web3.SystemProgram.programId,
                },
                signers: [userObject]
            });

        console.log(`Unstake tx: ${tx}`)

        const postBalance = await provider.connection.getTokenAccountBalance(sendTo.address)
        assert.ok(parseInt(postBalance.value.amount) > 1_000_000_000_000);
    });

    it("Stake NFT", async () => {
        const nftTokenAccount = await nftMintObject.getOrCreateAssociatedAccountInfo(userObject.publicKey);
        await mintToAccount(provider, nftMintObject.publicKey, nftTokenAccount.address, 1);

        const stakingAccount = await web3.Keypair.generate();
        const stakingTokenAccount = await nftMintObject.createAssociatedTokenAccount(stakingAccount.publicKey);

        const [settingsPDA, settingsPDABump] = await web3.PublicKey.findProgramAddress([Buffer.from("settings")], program.programId)
        const [stakingInfoPDA, stakingInfoPDABump] = await web3.PublicKey.findProgramAddress([userObject.publicKey.toBuffer(), Buffer.from("agrostaking")], program.programId)

        const info = await provider.connection.getTokenAccountsByOwner(settingsPDA, {mint: mintObject.publicKey})

        const tx = await program.rpc.stakeNft(
            {
                accounts: {
                    settingsAccount: settingsPDA,
                    tokenFrom: nftTokenAccount.address,

                    stakingInfo: stakingInfoPDA,
                    agteAccount: info["value"][0].pubkey,

                    stakingAccount: stakingTokenAccount,

                    user: userObject.publicKey,
                    stakedUser: stakingAccount.publicKey,

                    tokenProgram: TOKEN_PROGRAM_ID,
                    systemProgram: web3.SystemProgram.programId,
                },
                signers: [userObject, stakingAccount]
            });

        console.log(`Stake NFT TX: ${tx}`)

        const d = await program.account.stakeInfo.fetch(stakingInfoPDA)
        assert.equal(d.apy, 110);
    });

    it("Unstake NFT", async () => {
        const [settingsPDA, settingsPDABump] = await web3.PublicKey.findProgramAddress([Buffer.from("settings")], program.programId)
        const [stakingInfoPDA, stakingInfoPDABump] = await web3.PublicKey.findProgramAddress([userObject.publicKey.toBuffer(), Buffer.from("agrostaking")], program.programId)

        const sendTo = await nftMintObject.getOrCreateAssociatedAccountInfo(userObject.publicKey);
        const info = await provider.connection.getTokenAccountsByOwner(stakingInfoPDA, {mint: nftMintObject.publicKey})

        let sendFrom;

        for (const acc of info["value"]){
            const balance = await provider.connection.getTokenAccountBalance(acc.pubkey);
            if (parseInt(balance.value.amount) > 0) {
                sendFrom = acc.pubkey
                break
            }
        }

        const tx = await program.rpc.unstakeNft(
            {
                accounts: {
                    settingsAccount: settingsPDA,
                    tokenFrom: sendFrom,
                    tokenTo: sendTo.address,

                    stakingInfo: stakingInfoPDA,

                    user: userObject.publicKey,

                    tokenProgram: TOKEN_PROGRAM_ID,
                    systemProgram: web3.SystemProgram.programId,
                },
                signers: [userObject]
            });

        console.log(`Unstake NFT TX: ${tx}`)

        const d = await program.account.stakeInfo.fetch(stakingInfoPDA)
        assert.equal(d.apy, 100);
    })
});
