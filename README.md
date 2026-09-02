# CompactVeteran

no context left behind.

pending prompts are answered once; completed prompts are never repeated.

for Sols with post-traumatic summary disorder.

stock compaction asks a model to recap the conversation, then later recaps the
recap. exact decisions become paraphrases, the cursor disappears, and the agent
wakes up earlier in the project than it went to sleep.

CompactVeteran treats compaction as a handoff, not a writing assignment.

```text
STOCK
──────────────────────────────────────────────────────────────
context  ──►  recap  ──►  recap of the recap
                                  │
                                  └─ run that same test again

COMPACTVETERAN
──────────────────────────────────────────────────────────────
working tree        ──►  local Git checkpoint
objective + cursor  ──►  exact capsule  ──►  fresh Sol continues
```

Git is the territory. the capsule is the map. the recap is neither.

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

use Codex normally.

every completed Sol pass is checkpointed locally. when automatic compaction or
`/compact` arrives:

```text
01  finish the current pass
02  commit local Git
03  write Objective + Cursor + Next action
04  stop stock compaction
05  launch a fresh Sol at the Cursor
```

```text
Context compaction dodged.
```

Sol gets the 1,050,000-token ceiling. Terra and Luna stay stock.

## uninstall

```bash
curl -fsSL https://raw.githubusercontent.com/lmtlssss/CompactVeteran/main/uninstall.sh | sh
```

## build

```bash
scripts/prove-system.sh
```
