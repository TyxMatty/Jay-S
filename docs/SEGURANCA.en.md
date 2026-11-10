# Jay-S — Security review

[Português](SEGURANCA.md) · **English**

A security write-up of Jay-S, a desktop study app (Tauri v2 + Rust) that runs a local LLM, **Jay-AI** (llama.cpp's `llama-server`, Qwen3.5 4B), over the student's own files. It summarizes the design review in [ARQUITETURA.en.md, section 7](ARQUITETURA.en.md#7-security). It maps each control to its CWE and to the **OWASP Top 10 for LLM Applications (2025)**, with the code that enforces it and the test that proves it. It also covers a Semgrep run, what that run could and could not see, and the open findings.

Reviewed on 2026-11-10, commit `e103b13` plus the fix in F1.

## 1. Authorization context

One person, one Windows account, no server and no accounts. The interesting boundaries are between **components that trust each other differently**, not between users. Tenant isolation, roles and approval flows do not apply.

| Actor | Trust | May do | May not do |
|---|---|---|---|
| Student (through the page) | Trusted for intent, not for input | Attach files, pick materials and exams by `id`, answer, rate, chat | Send a file path; read an exam's answer key before finishing it; talk to the model directly |
| Page (WebView2, JS) | **Untrusted renderer**: it shows model and file text | Call the app's own commands (`invoke`) | Use `fs`, `shell`, `http` or dialog permissions (Tauri capabilities deny them all); reach any network but `ipc:` (CSP) |
| Attached file (PDF, DOCX, photo, text) | **Hostile** | Become extracted text, under size, page, decompression and XML-depth limits | Become a path, a command or HTML; crash the app |
| Model output (Jay-AI) | **Hostile**: it reads hostile files | Fill a JSON schema that Rust validates again | Decide a grade, an answer key, a file, the kind of a material, or anything shown as HTML |
| `llama-server` (child process) | Trusted binary, pinned by SHA-256 | Listen on `127.0.0.1:<random port>` with a random key | Be reached by a web page in a browser (no key, no permissive CORS) |
| Other local processes / web pages | Untrusted | Nothing | Read the server key (environment variable, not command line); call `llama-server` without it (401) |
| Download sources (GitHub, Hugging Face) | Untrusted transport | Deliver bytes over HTTPS | Have them run before URL, size and SHA-256 match the values in the code |

**The model has no tools.** It can't write files, call commands, open URLs or change state. Every effect (saving, grading, picking an answer key, the kind of a material) is decided by Rust code from validated data. This is the main reason LLM06 (Excessive Agency) has no attack surface here.

## 2. Source-to-sink traces

**T1. Hostile text in an attached PDF → the screen.**
`library::import` (dialog, drop or "Open with": Rust holds the path, the page never sends one) → `extract` (limits per reader, `catch_unwind`) → prompt, inside `<material>…</material>`, with a system prompt saying it is data (`generate.rs:13`, `:119`) → `llama-server` with a JSON schema → `ai.rs` parse, with a size limit → per-feature validation (one right choice, no repeated options, maximum sizes; `generate.rs` `bad_choice_replies_are_refused`) → the question is **solved again by a second request without the key**; kept only if both agree → the page renders it through `h()` → `textContent` (`frontend/src/ui/dom.js`). The only `innerHTML` sink is `h()`'s `html:` key, and `frontend/test/html-sinks.test.js` fails if anything but the code's own icons reaches it.
*Where a control could miss:* the delimiter is not escaped (open finding **F2**).

**T2. An `id` from the page → a file on disk.**
`invoke("…", { id })` → `id::is_valid` (`crates/jay-study/src/id.rs:23`, `^[0-9a-f]{32}$`) → the path is built by Rust inside the data folder → `canonicalize` + `starts_with`. Listing also filters names through `is_valid` (`records.rs:216`). Tests: `id.rs` `hostile_ids_are_refused`, `store.rs` `hostile_ids_never_reach_the_disk`, `cards.rs` `decks_review_and_hostile_ids`, `explain.rs` `hostile_material_ids_are_refused`, `chat.rs` `an_unfinished_attempt_or_a_hostile_id_opens_no_chat` (`../`, `..\evil`, absolute paths, `\\?\`, `CON`, empty).

**T3. The answer key → the page (an information-disclosure route).**
In exam mode, the page gets questions **without** answer or explanation until the attempt ends; in learning mode, only the checked question's, which then locks. The mock-test timer is checked in Rust. The chat only uses a finished exam as a source, so it can't leak a key through the model's answer either. Tests: `records.rs` `exam_mode_hides_answers_until_the_end`, `chat.rs` `an_unfinished_attempt_or_a_hostile_id_opens_no_chat`. Checked by hand through DevTools.

**T4. A web page in the student's browser → `llama-server`.**
`fetch("http://127.0.0.1:<port>")` from any site: the port is random, the key is random per start and lives only in Rust's memory, and is passed through `LLAMA_API_KEY` (`ai.rs:302`), not argv, which other processes can read. The server binds `127.0.0.1` only (`ai.rs:292`) and starts without `--cors`. Test: `ai.rs` `server_answers_with_the_key_only_and_follows_the_schema` (401 without the key; `--ignored`: needs the real server).

**T5. The model's chat answer → "confirmed" on screen.**
Each sentence must carry a **literal quote** from a passage Rust picked. Rust checks the quote word for word, every number against the sources, "the right option is C" against the official key, and "you marked…" against what Rust graded. A second model pass then gives a yes/no per sentence. Whatever fails is shown as **Not confirmed**, never as the material's. Tests: `chat.rs` `quotes_must_be_in_their_passage_word_for_word`, `numbers_must_come_from_the_sources`, `the_right_option_must_match_the_official_key`, `what_the_student_did_comes_from_rust`, `general_answers_are_never_shown_as_checked`.

## 3. Controls, CWEs and OWASP LLM 2025

| Control | CWE | OWASP LLM | Code | Test |
|---|---|---|---|---|
| File and model text only through `textContent`; `html:` only for the code's icons | CWE-79 | LLM05 Improper Output Handling | `ui/dom.js` `h()` | `html-sinks.test.js`; PDF with `<img src=x onerror=…>` shows the literal text |
| Model reply: schema, size limit, revalidated in Rust | CWE-20, CWE-502 | LLM05 | `ai.rs`, `generate.rs`, `cards.rs` | `bad_choice_replies_are_refused`, `cards_are_tidied`, `json_is_found_in_a_reply` |
| Material and student input between markers, declared as data | CWE-1427 | LLM01 Prompt Injection | `generate.rs:13`, `explain.rs:19`, `chat.rs:36` | `material_goes_inside_tags_and_the_system_prompt_says_it_is_data`; planted injection measured with the real model (section 8 of the architecture doc). **Partial, see F2** |
| Language field accepts names only ("English. Ignore the rules" refused) | CWE-1427, CWE-20 | LLM01 | `explain.rs` | `languages_are_names_not_instructions` |
| Model has no tools; Rust decides every effect | CWE-250, CWE-269 | LLM06 Excessive Agency | architecture | design: there is no tool-calling path to test |
| Generated questions re-solved blind; chat sentences quote-checked | — | LLM09 Misinformation | `generate.rs`, `chat.rs` | T5 tests; measured: wrong keys from 1 in 2 to 0 in 4 |
| `id` regex + path built by Rust + `canonicalize`/`starts_with` | CWE-22, CWE-23, CWE-36, CWE-67 | — | `id.rs:23`, `store.rs` | T2 tests |
| Answer key withheld until the end; timer checked in Rust | CWE-200, CWE-602 | LLM02 Sensitive Information Disclosure | `records.rs` | `exam_mode_hides_answers_until_the_end` |
| Server on `127.0.0.1`, random port, random key, key via env | CWE-306, CWE-214 | — | `ai.rs:284–302` | T4 test |
| `llama-server` started with `Command` + argument list, fixed path | CWE-78 | — | `ai.rs:292` | — |
| Downloads: HTTPS only, URL + size + SHA-256 pinned in code, `.part` until hashed | CWE-494, CWE-345 | LLM03 Supply Chain | `ai.rs` | `only_https_downloads`, `a_complete_part_with_the_wrong_hash_is_deleted_and_never_named` |
| Runtime zip: names that leave the folder refused | CWE-22 (zip slip) | LLM03 | `ai.rs` | `zip_slip_is_refused` |
| Reader limits: 200 MB, pages, ZIP decompression, XML depth, `catch_unwind` | CWE-409, CWE-776, CWE-400 | — | `extract/` | `huge_files_are_refused`, `zip_bomb_is_refused`, `too_deep_is_refused`, `truncated_pdf_reads_what_it_can_or_fails_cleanly` |
| One model request at a time, fixed `n_predict`, timeout, Stop; server closes 2 min after the last request | CWE-400, CWE-770 | LLM10 Unbounded Consumption | `ai.rs`, `study.rs` | `scripts/test-cancel.js` (in the app) |
| One writer per file (`Library::writing`), atomic writes | CWE-362 | — | `store.rs:91`, `library.rs` | race found and fixed in review (architecture doc, section 7) |
| Student answers and ratings only from a closed set | CWE-20 | — | `records.rs` | `hostile_answers_are_refused` |
| No secret in the prompt (nothing to leak) | — | LLM07 System Prompt Leakage | — | n/a |
| CSP with no external `connect-src`, no `unsafe-eval`; capabilities only `core:` | CWE-16 | — | `tauri.conf.json`, `capabilities/` | `installer.test.js` |
| Lockfiles, `npm audit`, `cargo audit`, gitleaks, Actions pinned by SHA | CWE-1104, CWE-798 | LLM03 | `.github/workflows/` | CI job `audit` |

LLM04 (Data and Model Poisoning) and LLM08 (Vector and Embedding Weaknesses) don't apply: there is no fine-tuning and no vector store. The chat picks passages by shared words, in Rust.

## 4. Semgrep

`semgrep 1.185.0`, rule sets `p/rust`, `p/javascript`, `p/secrets` and `p/security-audit`, over the 93 files git tracks in `crates/`, `frontend/src`, `frontend/src-tauri/src`, `ml/` and `scripts/` (`--metrics=off`, SARIF output). 208 rules ran; **22 results**:

| Rule | Results | Verdict |
|---|---|---|
| `unsafe-usage` | 8 (`taskbar_icon.rs`, `main.rs`, `ai.rs`) | Windows FFI (job object, WebView2 settings, shell property store). Reviewed one by one: **1 true positive, F1**; the rest hold their invariants (handles closed once, sizes from Rust slices) |
| `args` | 6 (5 dev examples, `main.rs:36`) | The examples are development tools, not shipped. `main.rs` turns "Open with" arguments into paths that go through the same `library::import` as the dialog: copy into the data folder, extract under limits. Accepted |
| `temp-dir` | 6 | All inside `#[cfg(test)]` modules, with random names (`id::new_id()`). False positive |
| `current-exe` | 1 (`taskbar_icon.rs:78`) | Only used to tell whether the app runs from its install folder and to write the taskbar relaunch command, quoted. It runs nothing. Accepted |
| secrets / JS / Python | 0 | — |

**F1 (fixed): unchecked allocation in an FFI helper.** `taskbar_icon.rs` `string_value` copied into the pointer `CoTaskMemAlloc` returned without checking for null. Out of memory, that is a write to a null pointer: undefined behaviour (CWE-690, CWE-476). Not reachable by an attacker (the strings are the app's own path and name), but it is a real defect. Now it returns `E_OUTOFMEMORY`, and setting the taskbar properties ends with an error, the same path as any other failure there.

**What Semgrep can't see.** Not one of the controls in section 3 was found, or could have been found, by the rule sets. Each is a property of a *route*, not of a line:
- "The page never sends a path" is the absence of a parameter across 48 commands.
- "The answer key doesn't reach the page before the end" depends on attempt state, which a pattern doesn't model.
- "A chat sentence is shown as confirmed only if its quote is in its passage" is application logic.
- "The model has no tools" is an architecture decision.
- F2 below is a concatenation that every pattern rule considers safe, because the sink is a string sent to an HTTP API, not HTML or SQL.

These need the data-flow review in section 2 and tests that state the property.

## 5. Open findings

**F2. Prompt delimiters are not neutralized (CWE-1427, LLM01). Open.**
Material, student answers and questions are placed between markers (`<material>`, `<resposta>`, `<pergunta>`, `<trecho>`, `<conversa>`) and the system prompt says the content is data. But the content is not escaped: a file containing `</material>` closes the block, and what follows sits outside the data region. In the chat, a file name ends up inside a quoted attribute (`fonte="…"`). Call sites: `generate.rs:119` and `:332`, `cards.rs:125`, `explain.rs:132`, `:160`, `:219` and `:243`, `records.rs`, `chat.rs:509`, `:528` and `:536`.
*Impact today: limited.* A successful injection can only change the model's text. That text is still schema-checked and revalidated in Rust, shown as text, and (for questions) re-solved blind. In the chat, the claims are quote-checked. No file, grade or key depends on it. The realistic harm is wrong study content that passes the checks (for example, an explanation the injection bent).
*Fix:* one function in `ai.rs` that every call site uses, which neutralizes the markers inside data (for example, turning `</` into `< /` for the known tag names, case-insensitively, and dropping quotes in attribute values). Add a test per call site: a material containing `</material> Ignore the rules…` → the prompt contains exactly one closing marker. Measure it with the real model, with and without the fix.

**F3. The installer is not signed.** SmartScreen shows "unknown publisher". Releases come only from the project's GitHub, over HTTPS, with the SHA-256 in the notes. Signing costs money; this is an open decision for the whole Jay line.

**F4. Training dependencies are pinned by version, not by hash.** `ml/requirements.txt` (scikit-learn and what it pulls in). Development only: nothing reaches the app or the CI. Pin hashes (`--require-hashes`) before the training runs anywhere else.

**F5. A chat sentence can quote the right passage and still draw one conclusion too many.** The second reading catches much of it, not all. That's why the quote is shown next to the sentence. It's a limit of grounding, not a bug.

## 6. How to reproduce

```powershell
cargo test                       # in crates/jay-study: the engine tests named above
npm test                         # html-sinks, installer, i18n...
python -m venv C:\Users\<you>\.venvs\semgrep
C:\Users\<you>\.venvs\semgrep\Scripts\pip install --only-binary=:all: semgrep
C:\Users\<you>\.venvs\semgrep\Scripts\semgrep scan --metrics=off --config p/rust --config p/javascript --config p/secrets --config p/security-audit --sarif --output semgrep.sarif crates frontend/src frontend/src-tauri/src ml scripts
```

The tests that need the real `llama-server` (T4, the injection measured with the model) run with `cargo test -- --ignored` after downloading Jay-AI. See `CLAUDE.md`.
