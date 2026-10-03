# `cgt_ai_train`

`cgt-ai-train` trains AlphaZero-style players for the games in [`cgt_ai_core`](../cgt_ai_core) by self-play, and pits, analyses and exports the resulting networks.

```console
$ cargo run --release --package cgt_ai_train -- train --game quelhas
```

## Training

Both presets below were tuned on a Quadro T2000 Max-Q (Turing, 4 GB), where self-play is GPU-bound at about 30k (32x3) and 8.4k (64x6) network evaluations per second once 4 workers each keep 64 games in flight.

The quick preset trains a 32x3 network in about 5 minutes. In timed head-to-head runs a 32x3 network trained on every move of 64-simulation games beat larger networks, cheaper searches and fewer games per iteration.

```console
$ cargo run --release --package cgt_ai_train -- train --game quelhas --checkpoint-dir checkpoints/quick \
    --channels 32 --num-blocks 3 --games-per-iteration 256 --simulations 64 --fast-simulations 0 \
    --workers 4 --parallel-games 64 --batch-size 512 --replay-buffer-size 50000 --min-buffer-size 1500 \
    --iterations 16 --time-limit 5
```

The long preset trains a 64x6 network in about 6 hours, at roughly 100 s per iteration. 64x6 is the smallest network that the compute-optimal size scaling for AlphaZero-style training ([Neumann and Gros 2023](https://arxiv.org/abs/2210.00849)) supports at 72x the compute of the quick preset.

```console
$ cargo run --release --package cgt_ai_train -- train --game quelhas --checkpoint-dir checkpoints \
    --channels 64 --num-blocks 6 --games-per-iteration 256 --simulations 128 --fast-simulations 0 \
    --workers 4 --parallel-games 64 --batch-size 512 --replay-buffer-size 250000 --min-buffer-size 20000 \
    --iterations 220 --time-limit 360 --checkpoint-every 10 --buffer-save-every 50
```

Both write `quelhas_iter<N>.mpk` and `latest.mpk` to the checkpoint directory. Ctrl-C stops at the next safe point and saves `interrupted.mpk` and `latest.mpk` together with the replay buffer, and a second Ctrl-C exits at once. Running the same command with `--resume-from checkpoints/latest.mpk` added continues from there, reloading the replay buffer, with `--iterations` and `--time-limit` counting from the restart.
