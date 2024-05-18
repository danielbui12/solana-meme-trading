import * as anchor from "@coral-xyz/anchor";
import { Program, BN } from "@coral-xyz/anchor";
import { BoosterSwap } from "../target/types/booster_swap";
import { LAMPORTS_PER_SOL_DECIMAL, TradeDirection, getMintAuthAddress, logPairBalance, setupSwapTest, swap_base_input, swap_base_output, toBigIntQuantity } from "./utils";
import { assert, expect } from "chai";
import { getAccount, getOrCreateAssociatedTokenAccount } from "@solana/spl-token";

describe("swap test", () => {
  anchor.setProvider(anchor.AnchorProvider.env());
  const owner = anchor.Wallet.local().payer;

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
  const [mintAuth] = getMintAuthAddress(program.programId);
  let configAddress, poolAddress, poolState;

  before(async () => {
    const resp = await setupSwapTest(
      program,
      owner,
      mintAuth,
      metadata,
      {
        config_index: 0,
        tradeFromTokenZeroToTokenOneFeeRate: new BN(1_000_00),
        tradeFromTokenOneToTokenZeroFeeRate: new BN(1_000_00),
        protocolFeeRate: new BN(0),
        fundFeeRate: new BN(0),
        create_fee: new BN(0),
      },
      confirmOptions
    );
    configAddress = resp.configAddress;
    poolAddress = resp.poolAddress;
    poolState = resp.poolState;
  })

  it("swap One for Zero input-based", async () => {
    const token0 = poolState.token0Mint;
    const token0AccountBefore = await getOrCreateAssociatedTokenAccount(
      program.provider.connection,
      owner,
      token0,
      owner.publicKey,
    );
    const token1AccountBalanceBefore = await program.provider.connection.getBalance(
      owner.publicKey,
    );
    const token0VaultBefore = await getAccount(
      program.provider.connection,
      poolState.token0Vault,
    );
    const token1VaultBalanceBefore = await program.provider.connection.getBalance(
      poolState.token1Vault,
    );

    const amount_in = new BN(100000000);
    const txHash = await swap_base_input(
      program,
      owner,
      configAddress,
      TradeDirection.OneForZero,
      token0,
      amount_in,
      new BN(0),
      confirmOptions,
    );
    expect(txHash).to.be.not.null;
    const token0AccountAfter = await getAccount(
      program.provider.connection,
      token0AccountBefore.address,
    );
    const token1BalanceAfter = await program.provider.connection.getBalance(
      owner.publicKey,
    );
    const token0VaultAfter = await getAccount(
      program.provider.connection,
      poolState.token0Vault,
    );
    const token1VaultBalanceAfter = await program.provider.connection.getBalance(
      poolState.token1Vault,
    );

    logPairBalance({
      label: "One For Zero Input-Based account balance change",
      token0BalanceBefore: token0AccountBefore.amount.toString(),
      token1BalanceBefore: token1AccountBalanceBefore,
      token0BalanceAfter: token0AccountAfter.amount.toString(),
      token1BalanceAfter: token1BalanceAfter,
      token0Decimals: metadata.decimals,
      token1Decimals: LAMPORTS_PER_SOL_DECIMAL,
    });

    logPairBalance({
      label: "One For Zero Input-Based vault balance change",
      token0BalanceBefore: token0VaultBefore.amount.toString(),
      token1BalanceBefore: token1VaultBalanceBefore,
      token0BalanceAfter: token0VaultAfter.amount.toString(),
      token1BalanceAfter: token1VaultBalanceAfter,
      token0Decimals: metadata.decimals,
      token1Decimals: LAMPORTS_PER_SOL_DECIMAL,
    });

    const poolStateAfter = await program.account.poolState.fetch(poolAddress);
    console.log('protocolFeesToken0', poolStateAfter.protocolFeesToken0.toString());
    console.log('protocolFeesToken1', poolStateAfter.protocolFeesToken1.toString());
    console.log('fundFeesToken0', poolStateAfter.fundFeesToken0.toString());
    console.log('fundFeesToken1', poolStateAfter.fundFeesToken1.toString());

    assert(token0AccountAfter.amount > token0AccountBefore.amount, "Invalid token_0 balance after swap");
    expect(
      (token1AccountBalanceBefore - token1BalanceAfter)
    ).to.be.gte(amount_in.toNumber());
  });

  it("swap Zero For One input-based", async () => {
    const token0 = poolState.token0Mint;
    const token0AccountBefore = await getOrCreateAssociatedTokenAccount(
      program.provider.connection,
      owner,
      token0,
      owner.publicKey,
    );
    const token1AccountBalanceBefore = await program.provider.connection.getBalance(
      owner.publicKey,
    );
    const token0VaultBefore = await getAccount(
      program.provider.connection,
      poolState.token0Vault,
    );
    const token1VaultBalanceBefore = await program.provider.connection.getBalance(
      poolState.token1Vault,
    );

    const amount_in = new BN(toBigIntQuantity(3319502, metadata.decimals).toString());
    const txHash = await swap_base_input(
      program,
      owner,
      configAddress,
      TradeDirection.ZeroForOne,
      token0,
      amount_in,
      new BN(0),
      confirmOptions,
    );
    expect(txHash).to.be.not.null;
    const token0AccountAfter = await getAccount(
      program.provider.connection,
      token0AccountBefore.address,
    );
    const token1BalanceAfter = await program.provider.connection.getBalance(
      owner.publicKey,
    );
    const token0VaultAfter = await getAccount(
      program.provider.connection,
      poolState.token0Vault,
    );
    const token1VaultBalanceAfter = await program.provider.connection.getBalance(
      poolState.token1Vault,
    );

    logPairBalance({
      label: "Zero For One Input-Based balance change",
      token0BalanceBefore: token0AccountBefore.amount.toString(),
      token1BalanceBefore: token1AccountBalanceBefore,
      token0BalanceAfter: token0AccountAfter.amount.toString(),
      token1BalanceAfter: token1BalanceAfter,
      token0Decimals: metadata.decimals,
      token1Decimals: LAMPORTS_PER_SOL_DECIMAL,
    });

    logPairBalance({
      label: "One For Zero Input-Based vault balance change",
      token0BalanceBefore: token0VaultBefore.amount.toString(),
      token1BalanceBefore: token1VaultBalanceBefore,
      token0BalanceAfter: token0VaultAfter.amount.toString(),
      token1BalanceAfter: token1VaultBalanceAfter,
      token0Decimals: metadata.decimals,
      token1Decimals: LAMPORTS_PER_SOL_DECIMAL,
    });

    const poolStateAfter = await program.account.poolState.fetch(poolAddress);
    console.log('protocolFeesToken0', poolStateAfter.protocolFeesToken0.toString());
    console.log('protocolFeesToken1', poolStateAfter.protocolFeesToken1.toString());
    console.log('fundFeesToken0', poolStateAfter.fundFeesToken0.toString());
    console.log('fundFeesToken1', poolStateAfter.fundFeesToken1.toString());

    expect(
      (token0AccountBefore.amount - token0AccountAfter.amount).toString()
    ).to.be.eq(amount_in.toString());
    assert(token1BalanceAfter > token1AccountBalanceBefore, "Invalid token_1 balance after swap");
  });

  it("swap One For Zero output-based", async () => {
    const token0 = poolState.token0Mint;
    const token0AccountBefore = await getOrCreateAssociatedTokenAccount(
      program.provider.connection,
      owner,
      token0,
      owner.publicKey,
    );
    const token1AccountBalanceBefore = await program.provider.connection.getBalance(
      owner.publicKey,
    );
    const token0VaultBefore = await getAccount(
      program.provider.connection,
      poolState.token0Vault,
    );
    const token1VaultBalanceBefore = await program.provider.connection.getBalance(
      poolState.token1Vault,
    );

    const amount_out = new BN(toBigIntQuantity(3319502, metadata.decimals).toString());
    const maximum_amount_in = new BN(1000000000);
    const txHash = await swap_base_output(
      program,
      owner,
      configAddress,
      TradeDirection.OneForZero,
      token0,
      amount_out,
      maximum_amount_in,
      confirmOptions,
    );
    expect(txHash).not.to.be.null;

    const token0AccountAfter = await getAccount(
      program.provider.connection,
      token0AccountBefore.address,
    );
    const token1BalanceAfter = await program.provider.connection.getBalance(
      owner.publicKey,
    );
    const token0VaultAfter = await getAccount(
      program.provider.connection,
      poolState.token0Vault,
    );
    const token1VaultBalanceAfter = await program.provider.connection.getBalance(
      poolState.token1Vault,
    );

    logPairBalance({
      label: "Zero For One Output-Based balance change",
      token0BalanceBefore: token0AccountBefore.amount.toString(),
      token1BalanceBefore: token1AccountBalanceBefore,
      token0BalanceAfter: token0AccountAfter.amount.toString(),
      token1BalanceAfter: token1BalanceAfter,
      token0Decimals: metadata.decimals,
      token1Decimals: LAMPORTS_PER_SOL_DECIMAL,
    });

    logPairBalance({
      label: "One For Zero Input-Based vault balance change",
      token0BalanceBefore: token0VaultBefore.amount.toString(),
      token1BalanceBefore: token1VaultBalanceBefore,
      token0BalanceAfter: token0VaultAfter.amount.toString(),
      token1BalanceAfter: token1VaultBalanceAfter,
      token0Decimals: metadata.decimals,
      token1Decimals: LAMPORTS_PER_SOL_DECIMAL,
    });

    const poolStateAfter = await program.account.poolState.fetch(poolAddress);
    console.log('protocolFeesToken0', poolStateAfter.protocolFeesToken0.toString());
    console.log('protocolFeesToken1', poolStateAfter.protocolFeesToken1.toString());
    console.log('fundFeesToken0', poolStateAfter.fundFeesToken0.toString());
    console.log('fundFeesToken1', poolStateAfter.fundFeesToken1.toString());

    expect(
      (token0AccountAfter.amount - token0AccountBefore.amount).toString()
    ).to.be.eq(amount_out.toString());
    assert(token1BalanceAfter <= token1AccountBalanceBefore, "Invalid token_1 balance after swap");
  });

  it("swap Zero For one output-based", async () => {
    const token0 = poolState.token0Mint;
    const token0AccountBefore = await getOrCreateAssociatedTokenAccount(
      program.provider.connection,
      owner,
      token0,
      owner.publicKey,
    );
    const token1AccountBalanceBefore = await program.provider.connection.getBalance(
      owner.publicKey,
    );
    const token0VaultBefore = await getAccount(
      program.provider.connection,
      poolState.token0Vault,
    );
    const token1VaultBalanceBefore = await program.provider.connection.getBalance(
      poolState.token1Vault,
    );

    const maximum_amount_in = new BN(toBigIntQuantity(3319502, metadata.decimals).toString());
    const amount_out = new BN(70000000);
    const txHash = await swap_base_output(
      program,
      owner,
      configAddress,
      TradeDirection.ZeroForOne,
      token0,
      amount_out,
      maximum_amount_in,
      confirmOptions
    );
    expect(txHash).not.to.be.null;

    const token0AccountAfter = await getAccount(
      program.provider.connection,
      token0AccountBefore.address,
    );
    const token1BalanceAfter = await program.provider.connection.getBalance(
      owner.publicKey,
    );
    const token0VaultAfter = await getAccount(
      program.provider.connection,
      poolState.token0Vault,
    );
    const token1VaultBalanceAfter = await program.provider.connection.getBalance(
      poolState.token1Vault,
    );

    logPairBalance({
      label: "Zero For One  balance change",
      token0BalanceBefore: token0AccountBefore.amount.toString(),
      token1BalanceBefore: token1AccountBalanceBefore,
      token0BalanceAfter: token0AccountAfter.amount.toString(),
      token1BalanceAfter: token1BalanceAfter,
      token0Decimals: metadata.decimals,
      token1Decimals: LAMPORTS_PER_SOL_DECIMAL,
    });

    logPairBalance({
      label: "One For Zero Input-Based vault balance change",
      token0BalanceBefore: token0VaultBefore.amount.toString(),
      token1BalanceBefore: token1VaultBalanceBefore,
      token0BalanceAfter: token0VaultAfter.amount.toString(),
      token1BalanceAfter: token1VaultBalanceAfter,
      token0Decimals: metadata.decimals,
      token1Decimals: LAMPORTS_PER_SOL_DECIMAL,
    });

    const poolStateAfter = await program.account.poolState.fetch(poolAddress);
    console.log('protocolFeesToken0', poolStateAfter.protocolFeesToken0.toString());
    console.log('protocolFeesToken1', poolStateAfter.protocolFeesToken1.toString());
    console.log('fundFeesToken0', poolStateAfter.fundFeesToken0.toString());
    console.log('fundFeesToken1', poolStateAfter.fundFeesToken1.toString());

    expect(
      Number(token0AccountBefore.amount - token0AccountAfter.amount)
    ).to.be.lt(maximum_amount_in.toNumber());
    assert(token1BalanceAfter >= token1AccountBalanceBefore, "Invalid token_1 balance after swap");
  });
});
