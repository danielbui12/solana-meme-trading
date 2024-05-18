import * as anchor from "@coral-xyz/anchor";
import { Program, BN } from "@coral-xyz/anchor";
import { BoosterSwap } from "../target/types/booster_swap";

import { getAccount } from "@solana/spl-token";
import { setupInitializeTest, initialize, TOKEN_TOTAL_SUPPLY, NATIVE_MINT, getMintAuthAddress } from "./utils";
import { expect } from "chai";
import { LAMPORTS_PER_SOL } from "@solana/web3.js";

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

  it("create pool", async () => {
    const [mintAuth] = getMintAuthAddress(program.programId);

    const { configAddress, token0 } =
      await setupInitializeTest(
        program,
        owner,
        mintAuth,
        metadata,
        {
          config_index: 0,
          tradeFromTokenZeroToTokenOneFeeRate: new BN(10),
          tradeFromTokenOneToTokenZeroFeeRate: new BN(10),
          protocolFeeRate: new BN(0),
          fundFeeRate: new BN(0),
          create_fee: new BN(LAMPORTS_PER_SOL * 0.001),
        },
        confirmOptions
      );

    const { poolAddress, poolState } = await initialize(
      program,
      owner,
      configAddress,
      token0,
      confirmOptions,
    );

    const vault0 = await getAccount(
      program.provider.connection,
      poolState.token0Vault,
    );
    expect(vault0.amount.toString()).to.be.eq(TOKEN_TOTAL_SUPPLY.toString());
    const vault1Balance = await program.provider.connection.getBalance(poolState.token1Vault);
    expect(vault1Balance).to.be.gte(0);
    console.log('vault1Balance', vault1Balance / LAMPORTS_PER_SOL);

  });
});
