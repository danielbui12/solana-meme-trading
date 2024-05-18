import * as anchor from "@coral-xyz/anchor";
import { web3 } from "@coral-xyz/anchor";
import {
  Connection,
  PublicKey,
  Keypair,
  Signer,
  TransactionInstruction,
  SystemProgram,
  Transaction,
  sendAndConfirmTransaction,
} from "@solana/web3.js";
import {
  createMint,
  TOKEN_PROGRAM_ID,
  getOrCreateAssociatedTokenAccount,
  mintTo,
  TOKEN_2022_PROGRAM_ID,
  getAssociatedTokenAddressSync,
  ExtensionType,
  getMintLen,
  createInitializeTransferFeeConfigInstruction,
  createInitializeMintInstruction,
  getAccount,
} from "@solana/spl-token";
import { BN } from "bn.js";
// import { sendTransaction } from "./index";

// create a token mint and a token2022 mint with transferFeeConfig
export async function createTokenMintAndAssociatedTokenAccount(
  connection: Connection,
  payer: Signer,
  mintAuthority: PublicKey,
  transferFeeConfig: { transferFeeBasisPoints: number; MaxFee: number }
) {
  // let ixs: TransactionInstruction[] = [];
  // ixs.push(
  //   web3.SystemProgram.transfer({
  //     fromPubkey: payer.publicKey,
  //     toPubkey: mintAuthority.publicKey,
  //     lamports: web3.LAMPORTS_PER_SOL,
  //   })
  // );
  // await sendTransaction(connection, ixs, [payer]);

  interface Token {
    address: PublicKey;
    program: PublicKey;
  }

  let tokenArray: Token[] = [];
  let token0 = await createMint(
    connection,
    payer,
    mintAuthority,
    null,
    9
  );

  tokenArray.push({ address: token0, program: TOKEN_PROGRAM_ID });

  // let token1 = await createMintWithTransferFee(
  //   connection,
  //   payer,
  //   mintAuthority,
  //   Keypair.generate(),
  //   transferFeeConfig
  // );

  // tokenArray.push({ address: token1, program: TOKEN_2022_PROGRAM_ID });

  // tokenArray.sort(function (x, y) {
  //   if (x.address < y.address) {
  //     return -1;
  //   }
  //   if (x.address > y.address) {
  //     return 1;
  //   }
  //   return 0;
  // });

  token0 = tokenArray[0].address;
  // token1 = tokenArray[1].address;
  //   console.log("Token 0", token0.toString());
  //   console.log("Token 1", token1.toString());
  const token0Program = tokenArray[0].program;
  // const token1Program = tokenArray[1].program;

  // const ownerToken0Account = await getOrCreateAssociatedTokenAccount(
  //   connection,
  //   payer,
  //   token0,
  //   payer.publicKey,
  //   false,
  //   "processed",
  //   { skipPreflight: true },
  //   token0Program
  // );

  // await mintTo(
  //   connection,
  //   payer,
  //   token0,
  //   ownerToken0Account.address,
  //   mintAuthority,
  //   100_000_000_000_000,
  //   [],
  //   { skipPreflight: true },
  //   token0Program
  // );

  // console.log(
  //   "ownerToken0Account key: ",
  //   ownerToken0Account.address.toString()
  // );

  // const ownerToken1Account = await getOrCreateAssociatedTokenAccount(
  //   connection,
  //   payer,
  //   token1,
  //   payer.publicKey,
  //   false,
  //   "processed",
  //   { skipPreflight: true },
  //   token1Program
  // );
  // console.log(
  //   "ownerToken1Account key: ",
  //   ownerToken1Account.address.toString()
  // );
  // await mintTo(
  //   connection,
  //   payer,
  //   token1,
  //   ownerToken1Account.address,
  //   mintAuthority,
  //   100_000_000_000_000,
  //   [],
  //   { skipPreflight: true },
  //   token1Program
  // );

  return [
    { token0, token0Program },
    // { token1, token1Program },
  ];
}

async function createMintWithTransferFee(
  connection: Connection,
  payer: Signer,
  mintAuthority: PublicKey,
  mintKeypair = Keypair.generate(),
  transferFeeConfig: { transferFeeBasisPoints: number; MaxFee: number }
) {
  const transferFeeConfigAuthority = Keypair.generate();
  const withdrawWithheldAuthority = Keypair.generate();

  const extensions = [ExtensionType.TransferFeeConfig];

  const mintLen = getMintLen(extensions);
  const decimals = 9;

  const mintLamports = await connection.getMinimumBalanceForRentExemption(
    mintLen
  );
  const mintTransaction = new Transaction().add(
    SystemProgram.createAccount({
      fromPubkey: payer.publicKey,
      newAccountPubkey: mintKeypair.publicKey,
      space: mintLen,
      lamports: mintLamports,
      programId: TOKEN_2022_PROGRAM_ID,
    }),
    createInitializeTransferFeeConfigInstruction(
      mintKeypair.publicKey,
      transferFeeConfigAuthority.publicKey,
      withdrawWithheldAuthority.publicKey,
      transferFeeConfig.transferFeeBasisPoints,
      BigInt(transferFeeConfig.MaxFee),
      TOKEN_2022_PROGRAM_ID
    ),
    createInitializeMintInstruction(
      mintKeypair.publicKey,
      decimals,
      mintAuthority,
      null,
      TOKEN_2022_PROGRAM_ID
    )
  );
  await sendAndConfirmTransaction(
    connection,
    mintTransaction,
    [payer, mintKeypair],
    undefined
  );

  return mintKeypair.publicKey;
}

export async function getUserAndPoolVaultAmount(
  owner: PublicKey,
  token0Mint: PublicKey,
  token0Program: PublicKey,
  token1Mint: PublicKey,
  token1Program: PublicKey,
  poolToken0Vault: PublicKey,
  poolToken1Vault: PublicKey
) {
  const ownerToken0AccountAddr = getAssociatedTokenAddressSync(
    token0Mint,
    owner,
    false,
    token0Program
  );

  const ownerToken1AccountAddr = getAssociatedTokenAddressSync(
    token1Mint,
    owner,
    false,
    token1Program
  );

  const ownerToken0Account = await getAccount(
    anchor.getProvider().connection,
    ownerToken0AccountAddr,
    "processed",
    token0Program
  );

  const ownerToken1Account = await getAccount(
    anchor.getProvider().connection,
    ownerToken1AccountAddr,
    "processed",
    token1Program
  );

  const poolVault0TokenAccount = await getAccount(
    anchor.getProvider().connection,
    poolToken0Vault,
    "processed",
    token0Program
  );

  const poolVault1TokenAccount = await getAccount(
    anchor.getProvider().connection,
    poolToken1Vault,
    "processed",
    token1Program
  );
  return {
    ownerToken0Account,
    ownerToken1Account,
    poolVault0TokenAccount,
    poolVault1TokenAccount,
  };
}

export function isEqual(amount1: bigint, amount2: bigint) {
  if (
    BigInt(amount1) === BigInt(amount2) ||
    BigInt(amount1) - BigInt(amount2) === BigInt(1) ||
    BigInt(amount1) - BigInt(amount2) === BigInt(-1)
  ) {
    return true;
  }
  return false;
}

export function toBigIntQuantity(quantity: number, decimals: number): bigint {
  return BigInt(quantity) * BigInt(10) ** BigInt(decimals)
}

export function fromBigIntQuantity(quantity: bigint, decimals: number): string {
  return (Number(quantity) / 10 ** decimals).toFixed(6)
}

export const logPairBalance = ({
  label,
  token0BalanceBefore,
  token1BalanceBefore,
  token0BalanceAfter,
  token1BalanceAfter,
  token0Decimals,
  token1Decimals,
}) => {
  console.log('===========', label, '===========');
  console.table([
    {
      balanceBefore: fromBigIntQuantity(token0BalanceBefore, token0Decimals),
      balanceAfter: fromBigIntQuantity(token0BalanceAfter, token0Decimals),
    },
    {
      balanceBefore: fromBigIntQuantity(token1BalanceBefore, token1Decimals),
      balanceAfter: fromBigIntQuantity(token1BalanceAfter, token1Decimals),
    },
  ]);
  console.log('===========', label, '===========');
}