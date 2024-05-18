import {
  MAX_FEE_BASIS_POINTS,
  ONE_IN_BASIS_POINTS,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import { PublicKey } from "@solana/web3.js";

export const FREEZED_AMOUNT = BigInt("200000000000000000");
export const AVAILABLE_AMOUNT = BigInt("800000000000000000");
export const BASE_INIT_TOKEN_1_AMOUNT = BigInt("24000000000");
export const TOKEN_TOTAL_SUPPLY = FREEZED_AMOUNT + AVAILABLE_AMOUNT;
export const NATIVE_MINT = new PublicKey('So11111111111111111111111111111111111111111');
export const LAMPORTS_PER_SOL_DECIMAL = 9;

export function calculateFee(
  transferFeeConfig: { transferFeeBasisPoints: number; MaxFee: number },
  preFeeAmount: bigint,
  tokenProgram: PublicKey
): bigint {
  if (tokenProgram.equals(TOKEN_PROGRAM_ID)) {
    return BigInt(0);
  }
  if (preFeeAmount === BigInt(0)) {
    return BigInt(0);
  } else {
    const numerator =
      preFeeAmount * BigInt(transferFeeConfig.transferFeeBasisPoints);
    const rawFee =
      (numerator + ONE_IN_BASIS_POINTS - BigInt(1)) / ONE_IN_BASIS_POINTS;
    const fee =
      rawFee > transferFeeConfig.MaxFee ? transferFeeConfig.MaxFee : rawFee;
    return BigInt(fee);
  }
}

export function calculatePreFeeAmount(
  transferFeeConfig: { transferFeeBasisPoints: number; MaxFee: number },
  postFeeAmount: bigint,
  tokenProgram: PublicKey
) {
  if (
    transferFeeConfig.transferFeeBasisPoints == 0 ||
    tokenProgram.equals(TOKEN_PROGRAM_ID)
  ) {
    return postFeeAmount;
  } else {
    let numerator = postFeeAmount * BigInt(MAX_FEE_BASIS_POINTS);
    let denominator =
      MAX_FEE_BASIS_POINTS - transferFeeConfig.transferFeeBasisPoints;

    return (numerator + BigInt(denominator) - BigInt(1)) / BigInt(denominator);
  }
}
