import { Program, BN } from "@coral-xyz/anchor";
import { BoosterSwap } from "../../target/types/booster_swap";
import {
  Connection,
  ConfirmOptions,
  PublicKey,
  Keypair,
  Signer,
  SystemProgram,
  SYSVAR_RENT_PUBKEY,
  Transaction,
} from "@solana/web3.js";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  TOKEN_PROGRAM_ID, getAccount, getAssociatedTokenAddressSync,
} from "@solana/spl-token";
import {
  accountExist,
  getAuthAddress,
  getPoolAddress,
  getPoolVaultAddress,
  createTokenMintAndAssociatedTokenAccount,
  getOracleAccountAddress,
  getAmmConfigAddress,
  TOKEN_METADATA_PROGRAM_ID,
  getMintMetadataAddress,
  NATIVE_MINT,
} from "./index";
import { sendAndConfirmIx } from "./tx";
import { ASSOCIATED_PROGRAM_ID } from "@coral-xyz/anchor/dist/cjs/utils/token";

export async function setupInitializeTest(
  program: Program<BoosterSwap>,
  owner: Signer,
  mintAuth: PublicKey,
  mintMetadata: MintMetadata,
  config: {
    config_index: number;
    tradeFromTokenZeroToTokenOneFeeRate: BN,
    tradeFromTokenOneToTokenZeroFeeRate: BN,
    protocolFeeRate: BN;
    fundFeeRate: BN;
    create_fee: BN;
  },
  confirmOptions?: ConfirmOptions
) {
  await create_mint(
    program,
    owner,
    mintAuth,
    mintMetadata,
    confirmOptions,
  );
  const configAddress = await createAmmConfig(
    program,
    owner,
    config.config_index,
    config.tradeFromTokenZeroToTokenOneFeeRate,
    config.tradeFromTokenOneToTokenZeroFeeRate,
    config.protocolFeeRate,
    config.fundFeeRate,
    config.create_fee,
    confirmOptions
  );
  // const ammConfig = await program.account.ammConfig.fetch(configAddress);
  // console.log('ammConfig', ammConfig);
  // console.log('tradeZeroToOneFeeRate', ammConfig.tradeZeroToOneFeeRate.toString());
  // console.log('tradeOneToZeroFeeRate', ammConfig.tradeOneToZeroFeeRate.toString());

  return { configAddress, token0: mintAuth };
}

export async function setupSwapTest(
  program: Program<BoosterSwap>,
  owner: Signer,
  mintAuth: PublicKey,
  mintMetadata: MintMetadata,
  config: {
    config_index: number;
    tradeFromTokenZeroToTokenOneFeeRate: BN,
    tradeFromTokenOneToTokenZeroFeeRate: BN,
    protocolFeeRate: BN;
    fundFeeRate: BN;
    create_fee: BN;
  },
  confirmOptions?: ConfirmOptions
) {
  const { configAddress, token0 } = await setupInitializeTest(program, owner, mintAuth, mintMetadata, config, confirmOptions);

  const { poolAddress, poolState } = await initialize(
    program,
    owner,
    configAddress,
    token0,
    confirmOptions,
  );

  return { configAddress, poolAddress, poolState };
}

export async function createAmmConfig(
  program: Program<BoosterSwap>,
  owner: Signer,
  config_index: number,
  tradeFromTokenZeroToTokenOneFeeRate: BN,
  tradeFromTokenOneToTokenZeroFeeRate: BN,
  protocolFeeRate: BN,
  fundFeeRate: BN,
  create_fee: BN,
  confirmOptions?: ConfirmOptions
): Promise<PublicKey> {
  const [address, _] = getAmmConfigAddress(
    config_index,
    program.programId
  );
  if (await accountExist(program.provider.connection, address)) {
    return address;
  }

  const ix = await program.methods
    .createAmmConfig(
      config_index,
      tradeFromTokenZeroToTokenOneFeeRate,
      tradeFromTokenOneToTokenZeroFeeRate,
      protocolFeeRate,
      fundFeeRate,
      create_fee
    )
    .accounts({
      owner: owner.publicKey,
      ammConfig: address,
      systemProgram: SystemProgram.programId,
    })
    .instruction();

  const txHash = await sendAndConfirmIx(program.provider.connection, [ix], [owner], undefined, confirmOptions);
  console.log("init amm config tx: ", txHash);
  return address;
}

export type MintMetadata = {
  name: string,
  symbol: string,
  uri: string,
  decimals: number,
}
export async function create_mint(
  program: Program<BoosterSwap>,
  creator: Signer,
  mintAuth: PublicKey,
  mintMetadata: MintMetadata,
  confirmOptions?: ConfirmOptions,
) {
  const [metadataAddr] = getMintMetadataAddress(mintAuth);

  const ix = await program.methods
    .createMint(mintMetadata)
    .accounts({
      metadata: metadataAddr,
      mint: mintAuth,
      creator: creator.publicKey,
      tokenProgram: TOKEN_PROGRAM_ID,
      tokenMetadataProgram: TOKEN_METADATA_PROGRAM_ID,
      systemProgram: SystemProgram.programId,
      rent: SYSVAR_RENT_PUBKEY,
    })
    .instruction();
  const txHash = await sendAndConfirmIx(program.provider.connection, [ix], [creator], undefined, confirmOptions);
  console.log("create mint tx: ", txHash);
  return txHash;
}

export async function mint_tokens(
  program: Program<BoosterSwap>,
  creator: Signer,
  mint: PublicKey,
  quantity: BN,
  confirmOptions?: ConfirmOptions,
) {

  const destination = getAssociatedTokenAddressSync(
    mint,
    creator.publicKey,
  );

  const ix = await program.methods
    .mintTokens(quantity)
    .accounts({
      mint,
      destination: destination,
      payer: creator.publicKey,
      rent: SYSVAR_RENT_PUBKEY,
      systemProgram: SystemProgram.programId,
      tokenProgram: TOKEN_PROGRAM_ID,
      associatedTokenProgram: ASSOCIATED_PROGRAM_ID,
    })
    .instruction();
  const txHash = await sendAndConfirmIx(program.provider.connection, [ix], [creator], undefined, confirmOptions);
  console.log("mint tokens tx: ", txHash);
  return { txHash, destination };
}

export async function initialize(
  program: Program<BoosterSwap>,
  creator: Signer,
  configAddress: PublicKey,
  token0: PublicKey,
  confirmOptions?: ConfirmOptions,
  createPoolFee = new PublicKey("Kd8e8t428wuB68bpksHTqu4VbM97cqYa3AKP3osYsKH")
) {
  const [poolAddress] = getPoolAddress(
    configAddress,
    token0,
    program.programId
  );

  const [authority] = getAuthAddress(
    program.programId
  );
  const [vault0] = getPoolVaultAddress(
    poolAddress,
    token0,
    program.programId
  );
  const [vault1] = getPoolVaultAddress(
    poolAddress,
    SystemProgram.programId,
    program.programId,
  );

  const [observationAddress] = getOracleAccountAddress(
    poolAddress,
    program.programId
  );

  const ix = await program.methods
    .initialize(new BN(0))
    .accounts({
      creator: creator.publicKey,
      ammConfig: configAddress,
      authority: authority,
      poolState: poolAddress,
      token0Mint: token0,
      // token1Mint: token1,
      token0Vault: vault0,
      token1Vault: vault1,
      createPoolFee,
      observationState: observationAddress,
      tokenProgram: TOKEN_PROGRAM_ID,
      associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      systemProgram: SystemProgram.programId,
      rent: SYSVAR_RENT_PUBKEY,
    })
    .instruction();
  const txHash = await sendAndConfirmIx(program.provider.connection, [ix], [creator], undefined, confirmOptions);
  console.log("initialize tx: ", txHash);
  const poolState = await program.account.poolState.fetch(poolAddress);
  const createPoolBalance = await program.provider.connection.getBalance(createPoolFee);
  console.log("createPoolBalance after init:", createPoolBalance.toString());
  return { poolAddress, poolState };
}

export const TradeDirection = {
  ZeroForOne: 0,
  OneForZero: 1,
};
export async function swap_base_input(
  program: Program<BoosterSwap>,
  owner: Signer,
  configAddress: PublicKey,
  tradeDirection: number,
  token0: PublicKey,
  amountIn: BN,
  minimumAmountOut: BN,
  confirmOptions?: ConfirmOptions,
  createPoolFee = new PublicKey("Kd8e8t428wuB68bpksHTqu4VbM97cqYa3AKP3osYsKH"),
) {
  const [auth] = getAuthAddress(program.programId);
  const [poolAddress] = getPoolAddress(
    configAddress,
    token0,
    program.programId
  );

  const [vault0] = getPoolVaultAddress(
    poolAddress,
    token0,
    program.programId
  );
  const [vault1] = getPoolVaultAddress(
    poolAddress,
    SystemProgram.programId,
    program.programId,
  );;

  const token0Account = getAssociatedTokenAddressSync(
    token0,
    owner.publicKey,
  );
  const token1Account = owner.publicKey;
  const [observationAddress] = getOracleAccountAddress(
    poolAddress,
    program.programId
  );

  const ix = await program.methods
    .swapBaseInput(
      tradeDirection,
      amountIn,
      minimumAmountOut
    )
    .accounts({
      payer: owner.publicKey,
      authority: auth,
      createPoolFee: createPoolFee,
      ammConfig: configAddress,
      poolState: poolAddress,
      token0Account: token0Account,
      token1Account: token1Account,
      token0Vault: vault0,
      token1Vault: vault1,
      token0Mint: token0,
      observationState: observationAddress,
      tokenProgram: TOKEN_PROGRAM_ID,
    })
    .instruction();
  const txHash = await sendAndConfirmIx(program.provider.connection, [ix], [owner], undefined, confirmOptions);
  const createPoolBalance = await program.provider.connection.getBalance(createPoolFee);
  console.log("swapBaseInput after:", createPoolBalance.toString());
  return txHash;
}

export async function swap_base_output(
  program: Program<BoosterSwap>,
  owner: Signer,
  configAddress: PublicKey,
  tradeDirection: number,
  token0: PublicKey,
  amountOut: BN,
  maximumAmountIn: BN,
  confirmOptions?: ConfirmOptions,
  createPoolFee = new PublicKey("Kd8e8t428wuB68bpksHTqu4VbM97cqYa3AKP3osYsKH"),
) {
  const [auth] = getAuthAddress(program.programId);
  const [poolAddress] = getPoolAddress(
    configAddress,
    token0,
    program.programId
  );

  const [vault0] = getPoolVaultAddress(
    poolAddress,
    token0,
    program.programId
  );
  const [vault1] = getPoolVaultAddress(
    poolAddress,
    SystemProgram.programId,
    program.programId,
  );

  const token0Account = getAssociatedTokenAddressSync(
    token0,
    owner.publicKey,
  );
  const token1Account = owner.publicKey;
  const [observationAddress] = getOracleAccountAddress(
    poolAddress,
    program.programId
  );

  const ix = await program.methods
    .swapBaseOutput(
      tradeDirection,
      maximumAmountIn,
      amountOut,
    )
    .accounts({
      payer: owner.publicKey,
      authority: auth,
      createPoolFee: createPoolFee,
      ammConfig: configAddress,
      poolState: poolAddress,
      token0Account: token0Account,
      token1Account: token1Account,
      token0Vault: vault0,
      token1Vault: vault1,
      token0Mint: token0,
      observationState: observationAddress,
      tokenProgram: TOKEN_PROGRAM_ID,
    })
    .instruction();
  const txHash = await sendAndConfirmIx(program.provider.connection, [ix], [owner], undefined, confirmOptions);
  const createPoolBalance = await program.provider.connection.getBalance(createPoolFee);
  console.log("swapBaseOutput after:", createPoolBalance.toString());
  return txHash;
}

