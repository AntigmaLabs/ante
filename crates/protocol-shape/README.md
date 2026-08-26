# ante-protocol-shape

The public wire messages and shared protocol types used by Ante clients and
the Ante daemon.

The crate contains serializable operations, events, identifiers, model
descriptors, tool-call records, and usage accounting. These types define an
external compatibility boundary; consumers should follow the crate's semantic
versions when updating clients.

## Example

```rust
use ante_protocol_shape::{Id, Op, OpMsg};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let message = OpMsg { op: Op::Interrupt, id: Id::op() };
    println!("{}", serde_json::to_string(&message)?);
    Ok(())
}
```
