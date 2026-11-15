# anchor-lint v2: specification

Status: written before the code. The program in `src/main.rs` must be built from this document.

## 1. Purpose

`anchor-lint` reads Rust source files of Anchor (Solana) programs and reports three kinds of risky code, so a human reviewer knows where to look first. It is a review helper, not a verifier: it reports suspects, it does not prove anything.

| Rule | Reports | Why it matters |
|---|---|---|
| R1 | a call to `.unwrap()` or `.expect(...)` | it panics, and a panic aborts the whole transaction |
| R2 | `+`, `-`, `*`, `+=`, `-=`, `*=` on values that are not both constants | integer overflow and underflow silently produce wrong balances in release builds |
| R3 | a field of type `AccountInfo` or `UncheckedAccount` in an `#[derive(Accounts)]` struct without a `/// CHECK:` doc comment | Anchor does not validate such accounts, the author must explain why it is safe |

The rule set is the same as in version 1 on purpose, so the two versions can be compared fairly. What changes is how the rules are implemented.

## 2. Why a second version

Version 1 matched text line by line. That gave six known problems, and this specification removes each of them:

1. `"a + b"` inside a string literal, or a `// x.unwrap()` comment after code, was reported (false positive).
2. `x+y` without spaces was missed, because the pattern was `" + "` (false negative).
3. An expression split over several lines was missed or reported on the wrong line.
4. `BASE + 5`, two constants, was reported although the compiler checks it at compile time.
5. Code under `#[cfg(test)]` and `#[test]` was reported, although a panic in a test is normal.
6. A whole line was skipped if it contained the text `wrapping_` or `checked_` anywhere, even in an unrelated place.

The cause of all six is the same: text has no structure. Version 2 parses the file into a syntax tree and looks at the tree.

## 3. Technologies and why

| Choice | Why |
|---|---|
| Rust, edition 2021, one binary crate, one file `src/main.rs` | a hard limit of 200 lines applies, and one file is easy to read in a review |
| `syn` 2 with features `full` and `visit` | the standard parser for Rust source. `full` is needed for statements and expressions, `visit` gives a ready tree walker, so the code only says what to do at each kind of node |
| `proc-macro2` 1 with feature `span-locations` | `syn` gives line numbers only when this feature is on. We list it directly so the feature is switched on |
| `std` only for everything else: arguments, files, directory walk, sorted set, exit codes | no `clap`, `walkdir`, `regex`, `anyhow`: each would add more build time and more code to explain than it saves in a program this small |

Do not add other dependencies. No `unsafe`.

## 4. Architecture

Data flows in one direction: **path → list of files → source text → syntax tree → set of findings → printed report → exit code.**

```
main()                 reads the argument, drives the steps, owns printing and exit codes
 ├─ collect_files()    a path becomes a sorted list of .rs files
 ├─ lint_source()      pure function: &str -> Result<BTreeSet<(usize, Rule)>, syn::Error>
 │    └─ Linter        a struct that implements syn::visit::Visit; the only place with rules
 └─ report()           prints findings of one file
```

Parts and responsibilities:

- **`enum Rule { R1, R2, R3 }`**, derives `Clone, Copy, PartialEq, Eq, PartialOrd, Ord`. Methods `id()` (returns `"R1"`) and `message()` (returns the text from section 5.3). One place for rule names, so the printer and the tests do not repeat strings.
- **`Linter`**: holds `found: BTreeSet<(usize, Rule)>`. A `BTreeSet` of `(line, rule)` gives sorting by line and removal of duplicates in one structure. `a + b + c` on one line produces one R2 finding, not two.
- **`lint_source(src)`**: parses with `syn::parse_str::<syn::File>`, runs the `Linter`, returns the set. It does no I/O, so every test calls it with a string literal.
- **`collect_files(path)`**: if `path` is a file, returns it. If it is a directory, walks it recursively, keeps files ending in `.rs`, skips directories named `target` and directories whose name starts with `.`, and sorts the result so the output is the same on every run.
- **`main`**: the only place that reads files, prints, and calls `process::exit`.

Efficiency: each file is read once, parsed once, and walked once for all three rules. Version 1 scanned every line three times with substring searches.

## 5. Behaviour

### 5.1 Command line

`anchor-lint <path>` where `<path>` is a file or a directory. Any other number of arguments is a usage error.

Exit codes: `0` nothing found, `1` at least one finding, `2` the run was incomplete: wrong arguments, unreadable path, or a file that does not parse. Code `2` wins over `1`. A file that does not parse is reported on stderr as `path: parse error: <message>`, and the remaining files are still checked.

### 5.2 Output (stdout)

For each finding, in file order then line order:

```
<path>:<line>: [<id>] <message>
  <the source line, trimmed>
```

and at the end one line `<N> finding(s)`. Path is printed as given by `collect_files`.

### 5.3 Rules

Messages: R1 `may panic`, R2 `unchecked arithmetic`, R3 `account without CHECK comment`.

**R1.** At every method call whose method name is exactly `unwrap` or `expect`, add `(line of the method name, R1)`. `unwrap_or`, `unwrap_or_default`, `ok_or` and friends are different names and do not match. `checked_mul(2).unwrap()` is reported (the `unwrap` is the risk).

**R2.** At every binary expression whose operator is `+`, `-`, `*`, `+=`, `-=` or `*=`, add `(line of the operator, R2)`, unless **both** operands are constant-like. In `syn` 2 the compound assignments are also binary expressions with the operators `AddAssign`, `SubAssign`, `MulAssign`. An operand is constant-like if it is a literal, a parenthesised constant-like expression, or a path whose last segment contains an uppercase letter and no lowercase letter (the Rust naming convention for constants, e.g. `FEE` or `u64::MAX`). Everything else (variables, field accesses, calls) is not constant-like. So `x += 1` is reported (left side is a variable), `FEE + 5` is not.
`checked_add`, `saturating_*` and `wrapping_*` are method calls, not operators, so they are never reported by R2. This is deliberate and stays documented: `wrapping_*` hides overflow on purpose, a reviewer must read the line.

**R2 and macros.** The tree walker does not look inside macro calls, because a macro body is just tokens. Handle macros explicitly. For a macro whose last path segment is in this list, do nothing: `msg, println, print, eprintln, eprint, format, write, writeln, dbg, info, debug, warn, error, trace` (logging and formatting; arithmetic in a log line is harmless). For every other macro (this is how `require!`, `require_eq!`, `assert!`, `emit!` appear in Anchor code) try to parse the macro body as comma-separated expressions (`Punctuated<Expr, Token![,]>::parse_terminated` through `Macro::parse_body_with`) and visit each expression, so `require!(a + b > 10, Err::Big)` is checked. If the body does not parse that way, skip the macro silently.

**R3.** In every struct that has `#[derive(Accounts)]` (detect with `Attribute::parse_nested_meta` looking for the identifier `Accounts`), look at each field. If the last segment of the field type path is `AccountInfo` or `UncheckedAccount`, and the field has no doc attribute whose text (trimmed) starts with `CHECK:`, add `(line of the field type, R3)`. A `/// CHECK: text` comment is stored by the parser as `#[doc = " CHECK: text"]`, which is why trimming is needed. Structs without `#[derive(Accounts)]` are ignored.

**Skipped code.** Do not descend into a module or a function that has the attribute `#[test]` or `#[cfg(test)]`. (`cfg(test)` is recognised when the attribute's token list is exactly `test`.)

**Lines.** The line is where the interesting token is, not where the statement starts. For a multi-line `x\n + y` the R2 finding is on the line with `+`.

### 5.4 Known limits (kept on purpose, to be described in the README)

- The tool has no type information. `a + b` on `f64` or `String` is reported the same as on `u64`.
- `a.wrapping_add(b)` is not reported (section 5.3).
- Arithmetic produced by a macro that is not parsed as expressions, or hidden in a macro-generated item, is not seen.
- A constant written in lower case, or a non-constant written in upper case, is classified wrongly.
- Only `.rs` files are read. No configuration file, no ignore list, no JSON output.

## 6. Expected results

Run each file separately. `(line, rule)`:

| File | Findings | Exit |
|---|---|---|
| `samples/bad.rs` | (9, R2) (11, R1) (12, R2) (21, R3) | 1 |
| `samples/good.rs` | none | 0 |
| `samples/limits.rs` | (9, R2). Line 7 `BASE + 5` is no longer reported; line 9 `vault.total+amount` is now found. | 1 |
| `samples/tricky.rs` | (13, R2) (21, R2) (26, R2) (31, R3) | 1 |
| no argument, missing path | message on stderr | 2 |
| a file with a syntax error | `parse error` on stderr | 2 |

`tricky.rs` shows the six old problems: line 7 (string) and line 8 (comment) are ignored, line 13 is the multi-line `+ y`, line 17 `FEE + 5` is ignored, line 21 `x+y` is found, line 25 `msg!` is ignored while line 26 `require!` is reported, line 31 is the unchecked account, line 33 is a field with `CHECK:` (ignored), line 37 is a field in a struct without `#[derive(Accounts)]` (ignored), the `#[cfg(test)]` module at the end is ignored.

## 7. Tests (inside `src/main.rs`, `#[cfg(test)]`)

All call `lint_source` with a string. Cover at least these cases. One `#[test]` may hold several asserts; group them so the whole file stays within 200 lines:

1. `unwrap` and `expect` are found; `unwrap_or(0)` is not.
2. A string literal and a trailing comment with `unwrap` or `+` are not found.
3. `x+y` without spaces is found.
4. A multi-line `x\n + y` is found on the operator's line.
5. `x += 1` is found; `FEE + 5` and `1 + 2` are not.
6. `checked_add` and `wrapping_add` are not reported by R2.
7. `msg!("{}", a + b)` is not reported; `require!(a + b > 1, E)` is reported.
8. R3: field without `CHECK:` is found; with `/// CHECK:` is not; the same field in a struct without `derive(Accounts)` is not.
9. A `#[cfg(test)]` module and a `#[test]` function are skipped.
10. `a + b + c` on one line gives exactly one R2 finding.
11. `lint_source("fn (")` returns an error.

## 8. Constraints

- `src/main.rs`: at most **200 lines in total, tests included**.
- `cargo build` with no warnings; `cargo clippy` clean; `cargo fmt --check` clean; `cargo test` green.
- No `.unwrap()` or `.expect(` outside the test module (the tool should pass its own rule R1).
- Stable Rust, edition 2021.
- Public README must say that the code was written by agents from this specification.

## 9. Non-goals

Type checking, data-flow analysis, fixing the code automatically, reading Anchor's `#[account(...)]` constraints, configuration, colours, IDE integration. They would be the next versions.
