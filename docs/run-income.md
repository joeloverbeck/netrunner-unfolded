# Run income and Tread Lightly

[Issue #18](https://github.com/joeloverbeck/netrunner-unfolded/issues/18) adds
Tread Lightly, Red Team and Pennyshaver through the existing match, controller,
observations and saved continuation. Complete introductory setup and the
remaining card families are still unavailable. These engineering inputs retain
the exact introductory inventory; they do not create an ordinary-play mode.

Prepare the [revision-7 artwork](turns-assets.md) in a fresh directory. This
revision uses `ordinary-turns-7`; earlier turn saves are refused and preserved,
without migration. Keep the frozen #17 executable, revision-6 artwork and its
accepted saves together. The original Brân continuation remains supported.

## Runner choices and return

Prepare the action after a failed Red Team run on HQ:

```sh
cargo run --locked --bin netrunner-driver -- \
  --engineering-turns income18 --view runner --seed 18 --stop-after 7 \
  --save /absolute/fresh/income18.json
cargo run --locked --features tabletop --bin netrunner-tabletop -- \
  --turns --seat runner --assets /absolute/path/to/revision-7-assets \
  --save /absolute/fresh/income18.json
```

The Runner has **8 pool credits, 3 clicks**, Red Team has **12 credits**, and
Pennyshaver has **0**. The central history says **HQ**. Red Team offers R&D and
Archives; its HQ action is absent even though the earlier run failed. A basic
HQ run remains legal.

1. Choose **Red Team · rig 1: run R&D**. At movement choose **Continue run**.
2. At **Breach R&D**, the pool already reads **11**, Red Team **9**, and
   Pennyshaver **1**. Success rewards occur before access. Choose **Access the
   next R&D card**, then **Leave card · finish access** for Hedge Fund.
3. At the action phase, quit and reopen with the same native command. Choose
   **Pennyshaver · 1 click: place 1, then take 2 credits**. The pool becomes
   **13** and Pennyshaver becomes **0**. Those hosted credits were never a
   payment source before collection.

For a short later-turn inspection, prepare a separate fresh save using the same
driver command with `--stop-after 31`. It starts the next Runner turn with
**14 pool credits, 4 clicks**, Red Team **9**, Pennyshaver **0**, and no central
runs yet. All three Red Team targets are offered again. Choose Archives,
continue at movement, then collect Pennyshaver: the pool reaches **19**, Red
Team **6**, Pennyshaver **0**. The intervening Corp actions in this prepared
witness are click-credit choices; the ordinary native Corp policy can choose a
different economic line if played through instead.

## Corp's free installation and paid rez

```sh
cargo run --locked --bin netrunner-driver -- \
  --engineering-turns tread18 --view corp --seed 18 --stop-after 7 \
  --save /absolute/fresh/tread18.json
cargo run --locked --features tabletop --bin netrunner-tabletop -- \
  --turns --seat corp --assets /absolute/path/to/revision-7-assets \
  --save /absolute/fresh/tread18.json
```

This begins at Brân's actual optional installation, after Tread Lightly was
played. Corp has **6 credits**; Runner has **7**. Choose **Install Palisade
directly inward · free**, **Trash Brân 1.0**, then **Finish inward installation
· free**. Corp still has **6**. The ordinary Runner policy continues inward.
The next Corp rez choice reads **Rez Palisade · 6 credits**: its ordinary three
plus Tread Lightly's three. Pay it and the Corp pool becomes **0**. Declining
that rez remains legal. The surcharge affects ICE only and ends with this run.

The direct rez checkpoint is also reproducible with `--stop-after 14` into a
separate fresh save. It exposes that same Corp choice without repeating the
installation.

These routes reuse the current table, textured counters, action buttons and
history panel. Stop on a different phase/count, missing art, persistence error
or the agreed inspection limit, preserving the save. Automated tests establish
the stated transitions and native button dispatch, not physical input comfort,
readability, enjoyment or owner acceptance. New human inspection requires its
own remaining allowance; the preparation commands consume none.
