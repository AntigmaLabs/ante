# ante-gateway

Slack and Discord gateway for [Ante](https://github.com/AntigmaLabs/ante). It connects channels to isolated Ante agent sessions so users can interact with Ante directly from chat.

## Build from source

Build and install `ante-gateway`:

```bash
cargo install --path examples/ante-gateway
```

Once installed, it can be invoked via `ante` or directly:

```bash
ante gateway
# or
ante-gateway
```

## Quick Start

1. Start an Ante host:
   ```bash
   ante serve --sock
   ```

2. Configure your channels in `~/.ante/channels.json` (see [Gateway documentation](https://docs.antigma.ai/usage/gateway)).

3. Run the gateway:
   ```bash
   ante gateway
   ```
