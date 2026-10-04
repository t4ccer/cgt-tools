# `cgt_ai_train`

`cgt-ai-train` trains AlphaZero-style players for the games in [`cgt_ai_core`](../cgt_ai_core) by self-play.
It also pits networks against each other with `arena`, exports them for the website with `openings` and `export`, and turns the PyTorch networks into checkpoints with `import`.

```console
$ cargo run --release --package cgt_ai_train -- train --game quelhas
```

It runs the networks with libtorch on the first CUDA device, so it needs an NVIDIA GPU.
`nix develop .#ai` provides libtorch with CUDA.
Elsewhere, the build downloads a libtorch without CUDA, with which the build succeeds but training does not run.

## Training

Both presets below were tuned on a Quadro T2000 Max-Q (Turing, 4 GB), where self-play is GPU-bound at about 30k (32x3) and 8.4k (64x6) network evaluations per second once 4 workers each keep 64 games in flight.

The quick preset trains a 32x3 network in about 5 minutes.
In timed head-to-head runs a 32x3 network trained on every move of 64-simulation games beat larger networks, cheaper searches and fewer games per iteration.

```console
$ cargo run --release --package cgt_ai_train -- train --game quelhas --checkpoint-dir checkpoints/quick \
    --channels 32 --num-blocks 3 --games-per-iteration 256 --simulations 64 --fast-simulations 0 \
    --workers 4 --parallel-games 64 --batch-size 512 --replay-buffer-size 50000 --min-buffer-size 1500 \
    --iterations 16 --time-limit 5
```

The long preset trains a 64x6 network in about 6 hours, at roughly 100 s per iteration.
64x6 is the smallest network that the compute-optimal size scaling for AlphaZero-style training ([Neumann and Gros 2023](https://arxiv.org/abs/2210.00849)) supports at 72x the compute of the quick preset.

```console
$ cargo run --release --package cgt_ai_train -- train --game quelhas --checkpoint-dir checkpoints \
    --channels 64 --num-blocks 6 --games-per-iteration 256 --simulations 128 --fast-simulations 0 \
    --workers 4 --parallel-games 64 --batch-size 512 --replay-buffer-size 250000 --min-buffer-size 20000 \
    --iterations 220 --time-limit 360 --checkpoint-every 10 --buffer-save-every 50
```

Both write `quelhas_iter<N>.mpk` and `quelhas_latest.mpk` to the checkpoint directory, every file named after the game.
Ctrl-C stops at the next safe point and saves `quelhas_interrupted.mpk` and `quelhas_latest.mpk` together with the replay buffer, `quelhas_replay_buffer.bin`, and a second Ctrl-C exits at once.
Running the same command with `--resume-from checkpoints/quelhas_latest.mpk` added continues from there, reloading the replay buffer, with `--iterations` and `--time-limit` counting from the restart.

The quick preset of Fjords trains a 64x3 graph network in about 5 minutes, at roughly 7 s per iteration.
A graph network over the 64 vertices is cheaper to evaluate than the grid network of Quelhas, so it gets through about 42 iterations.
In timed head-to-head runs it beat a 128x4 network and a 64x3 network searching 128 simulations 35 to 25 each, won all 60 games against a random player, and 16 of 40 against the 512x7 network.
Its files are named after Fjords, so it can share `checkpoints/quick` with the Quelhas preset.

```console
$ cargo run --release --package cgt_ai_train -- train --game fjords --checkpoint-dir checkpoints/quick \
    --channels 64 --num-blocks 3 --games-per-iteration 256 --simulations 64 --fast-simulations 0 \
    --workers 4 --parallel-games 64 --batch-size 512 --replay-buffer-size 50000 --min-buffer-size 1500 \
    --iterations 50 --time-limit 5
```

## Playing on the website

The website plays with a model file, which holds the network, its shape and the opening table that a game with the pie rule starts from.
`openings` searches every first move deeply, and `export` puts its table into the model file, checking that the exported network gives the same outputs as the checkpoint:

```console
$ cargo run --release --package cgt_ai_train -- openings --game quelhas \
    --checkpoint checkpoints/quelhas_latest.mpk --out checkpoints/quelhas_openings.json
$ cargo run --release --package cgt_ai_train -- export --game quelhas \
    --checkpoint checkpoints/quelhas_latest.mpk --openings checkpoints/quelhas_openings.json --out checkpoints/quelhas.bin
```

Fjords deals a new board for every game and has no pie rule, so its model file goes without an opening table:

```console
$ cargo run --release --package cgt_ai_train -- export --game fjords \
    --checkpoint checkpoints/quick/fjords_latest.mpk --out checkpoints/fjords_quick.bin
```

[`cgt_website`](../cgt_website) describes how the site picks up model files.
