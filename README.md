# CompactVeteran

no context left behind.

```text
SOL
  │
  ├─ checkpoint Git + map
  ├─ dodge compaction
  └─ continue from zero
```

## install

```bash
curl -fsSL https://raw.githubusercontent.com/lmtlssss/CompactVeteran/main/install.sh | sh
```

## usage

use Codex normally. Sol’s automatic compaction and `/compact` checkpoint the
current Git pass, update one project map, and restart clean. Terra and Luna
remain stock. No context left behind.

## uninstall

```bash
./uninstall.sh
```

## build

```bash
cargo check --manifest-path plugins/compactveteran/runtime/Cargo.toml
scripts/prove-system.sh
```
