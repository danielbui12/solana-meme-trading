import { ComputeBudgetProgram, ConfirmOptions, Connection, Signer, Transaction, TransactionInstruction, sendAndConfirmTransaction } from "@solana/web3.js";

class SendIxError extends Error {
  logs: string;

  constructor(originalError: Error & { logs?: string[] }) {
    //The newlines don't actually show up correctly in chai's assertion error, but at least
    // we have all the information and can just replace '\n' with a newline manually to see
    // what's happening without having to change the code.
    const logs = originalError.logs?.join('\n') || "error had no logs";
    super(originalError.message + "\nlogs:\n" + logs);
    this.stack = originalError.stack;
    this.logs = logs;
  }
}

export const sendAndConfirmIx = async (
  connection: Connection,
  ix: TransactionInstruction[] | Promise<TransactionInstruction[]>,
  signer: Signer | Signer[],
  computeUnits?: number,
  confirmOptions?: ConfirmOptions
) => {
  let [signers, units] = (() => {
    return [
      Array.isArray(signer)
        ? signer
        : [signer],
      computeUnits
    ];
  })();
  let ixs: TransactionInstruction[] = []
  if (Array.isArray(ix)) {
    ixs.push(...ix)
  } else {
    ixs.push(...(await ix))
  }
  const tx = new Transaction().add(...ixs);
  if (units)
    tx.add(ComputeBudgetProgram.setComputeUnitLimit({ units }));
  try {
    return await sendAndConfirmTransaction(connection, tx, signers, confirmOptions);
  }
  catch (error: any) {
    throw new SendIxError(error);
  }
}