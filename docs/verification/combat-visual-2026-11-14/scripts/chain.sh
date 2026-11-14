#!/usr/bin/env bash
# Sequential verification chain at the current HEAD. Logs + exit codes in $V.
V=target/verification/combat-visual-2026-11-13
git rev-parse HEAD > $V/chain-head.txt
step(){ n=$1; shift; "$@" > $V/$n.log 2>&1; echo "$n exit $?" >> $V/chain.out; }
: > $V/chain.out
step build-head nix develop --command cargo build --locked --bin capture -j12 --target-dir target/market-visual --config 'profile.dev.opt-level=0' --config 'profile.dev.package."*".opt-level=0' --config 'profile.dev.debug=0'
cp target/market-visual/debug/capture $V/capture-head
step lib-tests nix develop --command cargo test --locked --release --lib -j12
step sim-build nix develop --command cargo build --locked --release --bin balance_sim -j12
cp target/release/balance_sim $V/balance-sim-head
step clippy nix develop --command cargo clippy --locked --release --all-targets -j12 -- -D warnings
step wasm-check nix develop --command cargo check --locked --target wasm32-unknown-unknown --lib -j12
step fmt-check nix develop --command cargo fmt -- --check
echo CHAIN DONE >> $V/chain.out
