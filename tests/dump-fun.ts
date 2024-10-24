import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { DumpFun } from "../target/types/dump_fun";
import { assert } from "chai";

describe("dump-fun", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.DumpFun as Program<DumpFun>;

  const [global] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("global")],
    program.programId
  )
  console.log(global)

  it("Is initialized!", async () => {
    const tx = await program.methods.initialize().rpc();
    console.log("Your transaction signature", tx);

    // const info = await anchor

    // assert(program.account.global)
  });
});
