# CompactVeteran

no context left behind.

```text
SOL
  │
  ├─ checkpoint Git + map
  └─ dodge compaction
       ├─ Context compaction dodged.
       └─ continue from zero
```

## install

```bash
curl -fsSL https://raw.githubusercontent.com/lmtlssss/CompactVeteran/main/install.sh | sh
```

inspect first:

```bash
curl -fsSLO https://raw.githubusercontent.com/lmtlssss/CompactVeteran/main/install.sh
less install.sh
sh install.sh
```

## usage

use Codex normally. Sol’s automatic compaction and `/compact` checkpoint the
current Git pass, update one project map, and restart clean. Terra and Luna
remain stock.
The zero-model capsule is passed directly to the fresh Sol; its verified transcript prefix remains the fallback.
Commits stay local; runtime checkpointing requires no remote or network. GitHub
is used only for installation and releases.

## uninstall

```bash
curl -fsSL https://raw.githubusercontent.com/lmtlssss/CompactVeteran/main/uninstall.sh | sh
```

## build

```bash
scripts/prove-system.sh
```
