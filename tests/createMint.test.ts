import * as anchor from "@coral-xyz/anchor";
import { Program, BN } from "@coral-xyz/anchor";
import { BoosterSwap } from "../target/types/booster_swap";
import { getMintAuthAddress, create_mint, mint_tokens } from "./utils";
import { expect } from "chai";
import { TOKEN_PROGRAM_ID, getAccount } from "@solana/spl-token";

describe("initialize test", () => {
  anchor.setProvider(anchor.AnchorProvider.env());
  const owner = anchor.Wallet.local().payer;
  console.log("owner: ", owner.publicKey.toString());

  const program = anchor.workspace.BoosterSwap as Program<BoosterSwap>;

  const confirmOptions = {
    skipPreflight: true,
  };
  const metadata = {
    name: "Just a Test Token",
    symbol: "TEST",
    uri: "https://5vfxc4tr6xoy23qefqbj4qx2adzkzapneebanhcalf7myvn5gzja.arweave.net/7UtxcnH13Y1uBCwCnkL6APKsge0hAgacQFl-zFW9NlI",
    decimals: 9,
  };
  const [mintAuth] = getMintAuthAddress(
    program.programId,
  );

  it("create mint", async () => {
    const result = await create_mint(
      program,
      owner,
      mintAuth,
      metadata,
      confirmOptions,
    );
    expect(result).to.be.not.null;
  });

  it("mint tokens", async () => {
    const [mintAuth] = getMintAuthAddress(
      program.programId,
    );

    const quantity = new BN(1000000000);
    const { txHash, destination } = await mint_tokens(
      program,
      owner,
      mintAuth,
      quantity,
      confirmOptions,
    );
    expect(txHash).to.be.not.null;
    const vault = await getAccount(
      program.provider.connection,
      destination,
      "processed",
      TOKEN_PROGRAM_ID
    );
    console.log(vault.amount.toString());
    expect(vault.amount.toString()).to.be.eq(quantity.toString());
  });
});
