## TMCP: TSP + MCP (rust)

Progress:

- [X] Read local wallet
- [ ] Create new wallet with private vid if wallet does not exists
- [ ] Check if generated my_did is published 
- [ ] Publish my_did
- [X] Seal TSP Message for MCP using correct wallet
- [X] Open TSP Message for MCP using correct wallet

## How to use?

```
cargo run --example client ${server address} ${server's did}  
```

RUST_LOG=INFO cargo run --example client http://127.0.0.1:8001/mcp did:webvh:QmcXTzxvpamcqMBvPsfDaHQcwFYqjubPccvLa9kPfmiydL:did.teaspoon.world:endpoint:tmcp-d9344e41-dd27-4fae-8d80-c0eb3032520a


## Old Code
Old Code is rust equivalent of https://github.com/openwallet-foundation-labs/tmcp-python/blob/main/src/tmcp/tmcp.py

## New Code
New code is based on re-using as much code from https://github.com/openwallet-foundation-labs/tsp/blob/main/examples/src/cli.rs. 