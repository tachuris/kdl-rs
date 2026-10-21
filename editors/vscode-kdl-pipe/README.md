# KDL node pipes for Visual Studio Code

This extension highlights **experimental** KDL node pipes. The expression `a #> b` is equivalent to `b { a }`:

```kdl
route "/users" { handler list-users #> auth role=admin #> cache ttl=60 #> log }
```

The extension augments the [KDL extension](https://marketplace.visualstudio.com/items?itemName=kdl-org.kdl)'s grammar with rules for node pipes:

- `#>` as an operator (`keyword.operator.pipe.kdl`).
- The node name after `#>` as a node name (`entity.name.tag`), including names preceded by a type annotation.
- Slashdashed nodes and stages as comments (`comment.block.slashdash.kdl`), including pipe chains that span multiple lines.

Without these rules, the KDL grammar highlights the node name after `#>` as an argument unless the name starts a new line.
Its slashdash rules can also leave parts of a slashdashed node or stage outside the comment.

## Install this extension

1. Install the KDL extension (`kdl-org.kdl`).
2. Run **Developer: Install Extension from Location…** in VS Code.
3. Select this folder. No packaging is required.

If you edit the grammar, run **Developer: Reload Window**.
To check a token's scopes, use **Developer: Inspect Editor Tokens and Scopes**.

## Configure the language server

To enable diagnostics for node pipes, build `kdl-lsp` with node pipes support from the repository root:

```sh
cargo build --release -p kdl-lsp --features kdl/experimental-node-pipes
```

Set `kdl.command` in your VS Code user settings to the built executable's absolute path:

```json
"kdl.command": "/absolute/path/to/kdl-rs/target/release/kdl-lsp"
```

To restore the bundled language server, remove the `kdl.command` setting.
