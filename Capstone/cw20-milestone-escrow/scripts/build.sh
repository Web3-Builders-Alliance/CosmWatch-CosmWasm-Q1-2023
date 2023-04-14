# Optional: Purge backup artifacts folder and create a backup copy of existing optimized contracts
# sudo rm -rf artifacts.old/
# cp -r artifacts/ artifacts.old/

# Optimize contract(s)
docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/rust-optimizer:0.12.13