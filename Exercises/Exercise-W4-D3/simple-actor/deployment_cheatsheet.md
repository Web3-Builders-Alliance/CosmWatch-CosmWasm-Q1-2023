# Contract Deployment Cheatsheet

## Set environment variables

```bash
# CONTRACT ADDRESSES (FOR USE LATER)
export CW20_CONTRACT_ADDRESS=''
export LP_CONTRACT_ADDRESS=''

# CHAIN INFO
export CHAIN_ID='uni-6'
export RPC_URL='https://juno-rpc.reece.sh:443'
export LOCAL_URL='tcp://localhost:26657'
export TX_FLAG="--gas-prices 0.025ujunox --gas auto --gas-adjustment 1.3 --broadcast-mode block"
export NET_FLAG="--chain-id ${CHAIN_ID} --node ${RPC_URL}"
```

## Build contract and run it through the optimizer
> ❕ **NOTE:** You will need to use `workspace-optimizer` instead of `rust-optimizer` if you want to optimize multiple contracts in one go. To use one or the other, simply replace `rust-optimizer` with `workspace-optimizer` in the `docker run` command below or vice-versa.
Check [here](https://github.com/cosmwasm/rust-optimizer) for the latest optimizer version.

```bash
# Optional: Purge backup artifacts folder and create a backup copy of existing optimized contracts
# sudo rm -rf artifacts.old/
# cp -r artifacts/ artifacts.old/

# Optimize contract(s)
docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/rust-optimizer:0.12.11
```

## Store the optimized contract(s) on-chain
> ❕ **NOTE:** Remember to rename the `artifacts/contract.wasm` file below to match both of your contract names AND the name of your key after the `--from` flag
```bash
# CW20 CONTRACT
# LAST CODE ID:
export RES=$(junod tx wasm store artifacts/contract.wasm --from admin $TX_FLAG $NET_FLAG -y --output json)
export CODE_ID_CW20=$(echo $RES | jq -r '.logs[0].events[-1].attributes[1].value')
echo "CW20 CODE ID:" $CODE_ID_CW20

# LP CONTRACT
# LAST CODE ID:
export RES=$(junod tx wasm store artifacts/contract.wasm --from admin $TX_FLAG $NET_FLAG -y --output json)
export CODE_ID_LP=$(echo $RES | jq -r '.logs[0].events[-1].attributes[1].value')
echo "LP CODE ID:" $CODE_ID_LP
```
