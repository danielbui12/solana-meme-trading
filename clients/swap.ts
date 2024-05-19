import * as anchor from "@coral-xyz/anchor";
import { Program, BN } from "@coral-xyz/anchor";
import { BoosterSwap } from "../target/types/booster_swap";
import {
  LAMPORTS_PER_SOL_DECIMAL,
  TradeDirection,
  getMintAuthAddress,
  getObservation,
  logPairBalance,
  setupSwapTest,
  sleep,
  swap_base_input,
  toBigIntQuantity
} from "../tests/utils";
import { getAccount, getOrCreateAssociatedTokenAccount } from "@solana/spl-token";

async function main() {
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

  async function setup() {
    console.info('[Setup] executing ...');
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
    console.info('[Setup] done');
  }

  async function swapOneForZero() {
    console.log("[Swap One For Zero] executing ...");
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
    console.info('swap tx hash', txHash);

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
    await sleep(1000);
    console.info('[Swap One For Zero] done');
  }

  const getOracle = async () => {
    const oracle = await getObservation(program, poolState.observationKey);
    oracle.filter((o) => !o.blockTimestamp.isZero()).forEach((o) => {
      console.log('oracle blockTimestamp', o.blockTimestamp.toString());
      console.log('oracle cumulativeToken0PriceX32', o.cumulativeToken0PriceX32.toString());
      console.log('oracle cumulativeToken1PriceX32', o.cumulativeToken1PriceX32.toString());
    })
    await sleep(1000);
  };

  const swapZeroForOne = async () => {
    console.info('[Swap Zero For One] executing ...');
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

    const amount_in = new BN(toBigIntQuantity(2319502, metadata.decimals).toString());
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

    await sleep(1000);
    console.info('[Swap Zero For One] done');
  };

  await setup();
  await swapOneForZero();
  await swapZeroForOne();
  await getOracle();
}

main();