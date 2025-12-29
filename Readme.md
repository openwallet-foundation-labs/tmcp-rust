## TMCP: TSP + MCP (rust)

Progress:

- [X] Read local wallet
- [X] Create new wallet with private vid if wallet does not exists
- [X] Check if generated my_did is published 
- [X] Publish my_did
- [X] Seal TSP Message for MCP using correct wallet
- [X] Open TSP Message for MCP using correct wallet
- [X] TMCP Client testing
- [] TMCP Server testing

Connecting Tmcp Python server
Steps: 
- git clone https://github.com/openwallet-foundation-labs/tmcp-python 
- cd demo/server && uv run server.py
- Copy the existing did from the terminal.
- in another folder git clone https://github.com/openwallet-foundation-labs/tmcp-rust
- Create a Anthropic account and get ANTHROPIC_API_KEY
- run `RUST_LOG=INFO ANTHROPIC_API_KEY=${anthropic_api_key} cargo run http://127.0.0.1:8001/mcp ${copied server_did}`

```
RUST_LOG=INFO ANTHROPIC_API_KEY=${anthropic_api_key} cargo run --example client ${server address} ${server's did}  
```

You may need to try a few times.

You should get a chat terminal:
```
Connected to server: Some("Demo")
intended_receiver: did:webvh:QmUzwbsRtCcXaNCpcrHDH49R3vV8H4Qra16xprFM7jSseV:did.teaspoon.world:endpoint:tmcp-2a831a23-f06c-4027-9691-6f5d4580fc74
Available tools: ["add", "favorite_animal_guesser", "show_roots"]

TMCP Client Started!
Type your queries or 'quit' to exit
```

``` Type 2 + 3```

You should get:

I'll help you calculate 2 + 3 using the `add` function.
intended_receiver: did:webvh:QmUzwbsRtCcXaNCpcrHDH49R3vV8H4Qra16xprFM7jSseV:did.teaspoon.world:endpoint:tmcp-2a831a23-f06c-4027-9691-6f5d4580fc74
[2025-12-29T09:28:23Z INFO  client] Calling tool add with args {"a":2,"b":3}
`5`
Processing query: 2 + 3
