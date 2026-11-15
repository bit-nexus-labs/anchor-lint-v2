# Prompt and run log

## Run log

| | |
|---|---|
| Date | 15 November 2026, evening |
| Agent / harness | Gemini 3.8 Flash (High), through Antigravity (the executor called Quant) |
| Input | `SPEC.md` and the four files in `samples/` |
| Runs | 1 |
| Result | `src/main.rs`, 199 lines; build, tests, clippy and fmt clean (details in README) |

How the agent was started: one sentence in the agent's chat, pointing to the task file below.

```
Read tasks/SVM-04_build_v2.md and do it. Do not commit or push. When done, write only: Build ready SVM-04
```

The task file is the actual prompt. It tells the agent what to read, what not to read, what to create, what to verify, and where to write its notes. The specification itself is `SPEC.md` in this repository. The task file is reproduced as it was given, except that the author's local folder paths (including the full path in the sentence above) were replaced with short relative ones.

## The prompt (task file)

# SVM-04: build anchor-lint v2 from a specification

Hi! This is the second assignment of the course: a specification first, then a better version of the program built from it. The specification is already written. Your job is to build the program from it.

Working folder: the repository root (`anchor-lint-v2/`)

## Read first (only these)
1. `SPEC.md` in the working folder. It is the single source of truth. Follow it exactly, including the 200-line limit (tests included) and the rule details.
2. `samples\bad.rs`, `samples\good.rs`, `samples\limits.rs`, `samples\tricky.rs` in the working folder. Section 6 of the spec lists the expected findings for each.

## Do NOT read
- Anything else on the disk. In particular do not open `Homework_week1`, the old `anchor-lint`, or other files in `review` and `tasks`. The new version must come from the specification, not from the old code.
- `.env` files, keys, tokens. Do not ask for secrets.

## Preflight (stop and report if any of this fails)
- `rustc --version`, `cargo --version` work.
- `cargo` can download crates (crates.io is reachable). If it cannot, stop and say so.

## What to create (only these files, inside the working folder)
- `Cargo.toml` (package `anchor-lint`, version `0.2.0`, edition 2021, dependencies exactly as in SPEC section 3).
- `src/main.rs` (at most 200 lines in total, tests included).
- `.gitignore` containing `/target`.
- `Cargo.lock` is created by cargo; keep it.

## Do NOT
- Do not run `git init`, commit or push. Another step will do that after review.
- Do not add dependencies beyond SPEC section 3. No `unsafe`.
- Do not change `SPEC.md` or the samples. If the specification looks wrong or impossible, stop and describe the problem in the notes file instead of working around it silently.
- Do not change more than the files listed.

## Verification (run all of it and save the results)
1. `cargo build` with no warnings. Run `cargo clean` first so no cached build hides warnings.
2. `cargo test`.
3. `cargo clippy -- -D warnings`.
4. `cargo fmt --check`. If it fails, run `cargo fmt` and re-check the 200-line limit.
5. `wc -l src/main.rs` (must be 200 or less).
6. Run the program on each of the four sample files and on a missing file and on no argument. Save stdout, stderr and the exit code of each.
7. Run the program on `src` (the directory) to check directory mode.

## Where to write results (outside the working folder, so they are not part of the repository)
Folder: `review/`, next to the repository (outside it, so these notes are not part of it)
- `SVM-04_output.txt`: the output of steps 1 to 7, with the exact commands.
- `SVM-04_notes.md`: what you built, any place where you deviated from the spec or found it unclear (be honest, even if small), and the line count.
- `SVM-04_main.rs`: a copy of the final `src/main.rs`.
- Do not paste the code into the chat.

When finished, write only: `Build ready SVM-04`.

