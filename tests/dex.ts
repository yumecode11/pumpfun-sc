// import * as anchor from "@coral-xyz/anchor";
// import { Program } from "@coral-xyz/anchor";
// import { ASSOCIATED_PROGRAM_ID, TOKEN_PROGRAM_ID } from "@coral-xyz/anchor/dist/cjs/utils/token";
// import { SYSVAR_RENT_PUBKEY, LAMPORTS_PER_SOL, SystemProgram, PublicKey, Keypair } from "@solana/web3.js"

// import { BondingCurve } from "../target/types/bonding_curve";
// import { expect } from "chai";

// describe("bonding_curve", () => {
//   anchor.setProvider(anchor.AnchorProvider.env());

//   const program = anchor.workspace.BondingCurve as Program<BondingCurve>;

//   it("Liquidity pool created", async () => {
    
//     const [poolPda] = anchor.web3.PublicKey.findProgramAddressSync(
//       [Buffer.from("liquidity_pool")],
//       program.programId
//     )

//     await program.methods.createPool().accounts({
//       pool: poolPda,
//       tokenMint: mint1,
//       poolTokenAccount: poolToken,
//       payer: user.publicKey,
//       tokenProgram: TOKEN_PROGRAM_ID,
//       rent: SYSVAR_RENT_PUBKEY,
//       associatedTokenProgram: ASSOCIATED_PROGRAM_ID,
//       systemProgram: SystemProgram.programId
//     })
//     .rpc();

//     let pool = await program.account.liquidityPool.fetch(poolPda);

//     expect(pool.reserveSol).to.equal(0);
//     expect(pool.bump).not.null;
//   });
// });
