# solana-meme-trading

solana-meme-trading

## Environment Setup

1. Install Rust.
2. Install Solana and then run solana-keygen new to create a keypair at the default location.
3. Install Anchor.

## Quickstart

Clone the repository and test the program.

```shell

git clone https://github.com/danielbui12/solana-meme-trading
cd solana-meme-trading && anchor test
```

## Flow to deploy to Raydium
1. Create open-book
2. Initialize
3. Deposit
  - Approve
  - Wrap Native SOL
  - Invoke Deposit instruction 

```
anchor test --skip-build --skip-deploy
```
