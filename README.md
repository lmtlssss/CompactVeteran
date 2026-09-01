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

## uninstall

```bash
curl -fsSL https://raw.githubusercontent.com/lmtlssss/CompactVeteran/main/uninstall.sh | sh
```

## build

```bash
scripts/prove-system.sh
```
