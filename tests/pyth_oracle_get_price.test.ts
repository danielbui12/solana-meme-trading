import * as anchor from "@coral-xyz/anchor";
import { assert } from "chai";
import { PublicKey } from "@solana/web3.js";
import { parsePriceData } from "@pythnetwork/client";

describe("initialize test", () => {
  anchor.setProvider(anchor.AnchorProvider.env());
  const owner = anchor.Wallet.local().payer;
  console.log("owner: ", owner.publicKey.toString());
  const connection = anchor.getProvider().connection;

  it("create mint", async () => {
    const SOL_PRICE_FEED_ID = new PublicKey("H6ARHf6YXhGYeQfUzQNGk6rDNnLBQKrenN712K4AQJEG");
    const accountData = await connection.getAccountInfo(SOL_PRICE_FEED_ID);
    const priceData = parsePriceData(accountData.data);
    console.log('SOL/USD', priceData.priceComponents[0].aggregate.price);
    assert(true);
  });
});
