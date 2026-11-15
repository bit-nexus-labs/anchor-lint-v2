# anchor-lint v2

A small linter for Anchor (Solana) programs, rebuilt from a written specification. It reads `.rs` files and points a human reviewer at three kinds of risky code:

| Rule | Reports | Why |
|---|---|---|
| R1 | `.unwrap()` / `.expect(...)` | a panic aborts the whole transaction |
| R2 | `+ - * += -= *=` unless both sides are constants | integer overflow silently gives wrong balances in release builds |
| R3 | `AccountInfo` / `UncheckedAccount` field in a `#[derive(Accounts)]` struct without `/// CHECK:` | Anchor does not validate these accounts, the author must explain why it is safe |

This is a review helper. It reports suspects; it does not prove anything.

This is the second version. The first one is here: <https://github.com/bit-nexus-labs/anchor-lint> (commit `d39f0c1`).

## Usage

```
cargo build --release
./target/release/anchor-lint <file-or-directory>
```

Output, one block per finding, then a total:

```
samples/bad.rs:9: [R2] unchecked arithmetic
  vault.total = vault.total + amount;
...
4 finding(s)
```

Exit codes: `0` nothing found, `1` findings, `2` the run was incomplete (wrong arguments, unreadable path, a file that does not parse). A directory is searched recursively for `.rs` files; `target` and hidden directories are skipped.

## What this repository contains

| File | What |
|---|---|
| `SPEC.md` | the specification (written first) |
| `PROMPT.md` | the exact prompt given to the agent, and a log of the run |
| `src/main.rs` | the program, 199 lines including tests, written by an agent from `SPEC.md` |
| `samples/` | `bad.rs`, `good.rs`, `limits.rs` (from version 1) and `tricky.rs` (new) |
| `README.md` | this file |

## What changed compared with version 1, and why

Version 1 looked at each line of text. That caused six concrete problems; `samples/tricky.rs` and `samples/limits.rs` show them. Version 2 parses the file into a syntax tree with the `syn` crate and checks the tree.

| Problem in version 1 | Version 2 |
|---|---|
| `"a + b"` in a string, or `// x.unwrap()` after code, was reported | strings and comments are not code in the tree, so they are never seen |
| `x+y` without spaces was missed (the pattern was `" + "`) | found, because the operator is a node in the tree |
| an expression on two lines (`x` / `+ y`) was found only by luck | found on the line of the operator |
| `BASE + 5` (two constants) was reported | not reported: both operands look like constants |
| code under `#[cfg(test)]` and `#[test]` was reported | skipped |
| the whole line was skipped if it contained `wrapping_`/`checked_` anywhere | `checked_`, `wrapping_` are method calls, not operators, so R2 never looks at them; nothing else on the line is skipped |
| `require!(a + b > 10, ...)` | still reported: macro bodies are parsed as expressions. Log macros (`msg!`, `println!`, ...) are skipped on purpose |
| R3 matched any line with `AccountInfo<'info>` | only fields of structs with `#[derive(Accounts)]`, and the type is matched by name, so `anchor_lang::prelude::AccountInfo` also works |

Measured on the same files (both versions were run on every file):

| | version 1 | version 2 |
|---|---|---|
| `samples/bad.rs` | 4 findings (9, 11, 12, 21) | the same 4 |
| `samples/good.rs` | 0 | 0 |
| `samples/limits.rs` | line 7 (false positive); line 9 missed | line 9 (correct) |
| `samples/tricky.rs` | 10 findings: 3 correct, **7 false**, 1 correct case missed | **4 findings, all correct** |
| lines of Rust | 175 | 199 |
| dependencies | none (`std`) | `syn`, `proc-macro2` (4 external crates in `Cargo.lock`) |
| work per file | 3 substring searches on every line | one parse, one walk for all rules |
| tests | 6 | 1 test with 11 cases (see below) |

The rules, messages, output format and exit codes are the same on purpose, so the two versions can be compared fairly. Version 2 is not shorter: it trades 24 more lines and two dependencies for correctness.

## How it was built

1. I wrote `SPEC.md` first: purpose, architecture, crates and why, exact behaviour of each rule, expected results for every sample file, tests, and constraints.
2. I gave the specification to an agent with a prompt (`PROMPT.md`) and asked it to build the program in at most 200 lines. The agent was told not to read version 1's code.
3. The agent: Gemini 3.8 Flash (High), run through Antigravity (the executor called Quant). After the independent check below no change to the code was requested.
4. The result was checked independently: clean build with no warnings, `cargo test`, `cargo clippy -- -D warnings`, `cargo fmt --check`, the six sample cases, error cases, directory mode, and a separate reference implementation compared on extra edge cases. All matched.

**Who did what, honestly.** The specification, the prompt and this README were drafted with Claude (an AI assistant) at my request, because I could not write them alone; I read and approved them. The Rust code was written by the agent named above. Claude did not change the code; it built and tested it separately and compared the behaviour with its own reference implementation of the specification.

## Design in short

- `lint_source(&str) -> Result<BTreeSet<(line, Rule)>, syn::Error>` is a pure function with no I/O. Every test calls it with a string.
- `Linter` implements `syn::visit::Visit`; it is the only place with rules. The result is a `BTreeSet`, so findings come out sorted by line and `a + b + c` on one line gives one finding, not two.
- `collect_files` turns a path into a sorted list of `.rs` files; `main` is the only place that prints and exits.
- Only `std` is used besides `syn` and `proc-macro2` (the latter is listed so that its `span-locations` feature is switched on; without it `syn` gives no line numbers).

## Testing

`cargo test` runs one test (`test_rules`). It parses one source text that contains all eleven cases from the specification (unwrap/expect, strings and comments, `x+y`, a two-line expression, `+=`, constants, `checked_`/`wrapping_`, log and non-log macros, R3 with and without `CHECK:`, `#[cfg(test)]` and `#[test]`, three operators in one line, a syntax error) and compares the exact set of `(line, rule)` pairs. The cases are grouped into one test because of the 200-line limit; the price is that a failure points to the whole set, not to one case.

On top of that, the four sample files give the expected results from section 6 of the specification, and a missing path, no argument and a file with a syntax error all exit with `2`.

## Known limits

- No type information: `a + b` on floats or strings is reported like on `u64`.
- `a.wrapping_add(b)` is not reported. Version 1 skipped it too; it hides overflow on purpose, so a reviewer must read the line.
- A constant written in lower case, or a non-constant in upper case, is classified wrongly.
- A wrapper such as `Option<AccountInfo>` is not detected by R3 (only the outermost type name is checked).
- Macros that do not parse as comma-separated expressions are skipped.
- Run on its own source, the tool reports 2 findings (`total += 1`, `l - 1`). Both are safe in context (a counter, and a line number that starts at 1) but the tool cannot know that.
- Only `.rs` files; no configuration, no ignore list, no JSON output.
