## TMCP: TSP + MCP (rust)

Progress:

- [X] Read local wallet
- [ ] Create new wallet with private vid if wallet does not exists
- [ ] Check if generated my_did is published 
- [ ] Publish my_did
- [X] Seal TSP Message for MCP using correct wallet
- [X] Open TSP Message for MCP using correct wallet


```
cargo run --example client ${server address} ${server's did}  
```


## How to connect to tmcp python server with existing wallets?

- copy wallets/server.sqlite to tmcp-python/demo/server/wallet.sqlite
- in tmcp-python/demo/server run "uv run server.py"
- run the following code:
RUST_LOG=INFO cargo run --example client http://127.0.0.1:8001/mcp did:webvh:QmcZxFGLsxDB5aoFpnhpgnMVdZUhmTN5i6vJAZsho2WxzE:did.teaspoon.world:endpoint:tmcp-ac4baf41-3a93-41dd-b49a-388356ee4826


## Old Code
Old Code is rust equivalent of https://github.com/openwallet-foundation-labs/tmcp-python/blob/main/src/tmcp/tmcp.py

## New Code
New code is based on re-using as much code from https://github.com/openwallet-foundation-labs/tsp/blob/main/examples/src/cli.rs. 

