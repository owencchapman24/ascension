# Pull-request CI

This bounded engineering milestone adds [.github/workflows/ci.yml](../../.github/workflows/ci.yml) on `chore/github-actions-ci`, from the merged Stage 4.5 baseline `4a6c32d0a84e61d5350baccc1e2640239ab409f7`. It runs existing correctness checks without changing application behavior, test assertions, dependency versions, schemas, rewards, or protocols. Stage 4.6A is not included.

**Status: local validation only; GitHub-hosted execution has not been observed.** Nothing has been pushed and no PR has been created by this milestone. The owner must inspect both hosted jobs after creating a PR before merging. Historical Stage 3a–4.5 results, including the Stage 4.5 conditional verdict and unresolved installed restore issue, remain unchanged.

## Events and environments

Ordinary `pull_request` events `opened`, `synchronize`, `reopened`, and `ready_for_review` trigger both jobs for all target branches, including draft PRs. There are no path filters, push/manual/scheduled triggers, or `pull_request_target` runs. Checkout uses the event's default synthetic merge commit to test integration with the target branch. GitHub may require maintainer approval for fork contributions and does not run conflicted PR merge commits; these are platform gates, not passing checks. See GitHub's [PR event reference](https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows#pull_request).

| Job | Standard hosted x64 image | Toolchain | Timeout |
| --- | --- | --- | --- |
| Frontend | `ubuntu-24.04` | Node.js `22.23.3`; bundled npm | 15 minutes |
| Rust (Windows) | `windows-2025` | Rust `1.99.1-x86_64-pc-windows-msvc`, rustfmt, Clippy | 45 minutes |

The explicit labels are listed in the official [runner image table](https://github.com/actions/runner-images/blob/main/README.md). They fix the OS generation, not every installed component: images still receive updates. At review, the [Windows image manifest](https://github.com/actions/runner-images/blob/main/images/windows/Windows2025-VS2026-Readme.md) includes rustup, MSVC C++ tools, the Windows SDK, and PowerShell; [Ubuntu's manifest](https://github.com/actions/runner-images/blob/main/images/ubuntu/Ubuntu2404-Readme.md) supports the selected frontend environment. Node `22.23.3` matches the development baseline and satisfies the lockfile's Node `22.22.2` minimum within the 22.x line.

The Windows job uses preinstalled rustup to install the exact MSVC toolchain with the minimal profile plus rustfmt and Clippy, without self-updating rustup. Job-local `RUSTUP_TOOLCHAIN` selects it without changing the machine's default toolchain. `CARGO_BUILD_JOBS=1` limits compiler concurrency; tests also use one thread. No installer, frontend asset transfer, or Tauri CLI build is configured in that job.

## Checks and dependency contracts

Frontend commands, in order:

```sh
npm ci
npm run typecheck
npm test
npm run build
```

`package-lock.json` fixes the dependency graph and integrity hashes. `npm ci` rejects manifest/lock disagreement. The existing jsdom/Vitest suites cover frontend interaction and typed client behavior with mocked native calls; the build runs TypeScript and Vite. No replacement framework is introduced.

Windows Rust commands, in order:

```powershell
rustup toolchain install 1.99.1-x86_64-pc-windows-msvc --profile minimal --component rustfmt --component clippy --no-self-update
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml --locked -- --test-threads=1
cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings
```

`src-tauri/Cargo.lock` stays committed and unchanged. Hosted Cargo commands allow normal public registry downloads and deliberately omit `--offline`. These are the existing Rust correctness suites against synthetic real temporary SQLite databases, including Windows locks/junctions, migration, persistence, replay/rollback, import rejection, recovery, and authority invariants. Four explicitly ignored release benchmarks remain outside the normal correctness run. Test totals are observations, not platform-independent assertions.

## Security and resource review

- Workflow-wide `permissions: contents: read`; unspecified token scopes are `none`. Neither job overrides permissions. No secrets, PATs, registry authentication, external account credentials, environments, deployments, repository writes, publication, or automatic merges are configured.
- Checkout uses `persist-credentials: false` in both jobs. Untrusted PR code runs only on standard disposable hosted runners with the ordinary PR permission model. No privileged follow-up workflow or self-hosted runner is introduced.
- `setup-node` has `package-manager-cache: false`; no cache action, registry URL, or artifact upload is configured. No databases, profiles, recovery files, exports, screenshots, installers, or debug archives are uploaded. Standard Actions console logs still exist; use synthetic inputs and avoid printing private content.
- Concurrency group `${{ github.workflow }}-pr-${{ github.event.pull_request.number }}` isolates each PR and cancels its superseded pending/running workflow. The two jobs may run concurrently; memory-intensive Cargo commands run sequentially within one job. There is no matrix or cross-run dependency cache.
- The 15/45-minute timeouts bound job execution, not queue delay. Cold dependency downloads and Windows compilation may dominate runtime; measure the first hosted run before tuning. Standard hosted runners are free for public repositories under [GitHub's current billing rules](https://docs.github.com/en/billing/concepts/product-billing/github-actions), but consume runner capacity and remain subject to platform limits. No artifact/cache storage is requested by this workflow.

This follows GitHub's [secure-use guidance](https://docs.github.com/en/actions/reference/security/secure-use) and [permission semantics](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax#permissions). SHA pinning prevents a moved tag from changing action code; it does not prove dependencies benign. PR code and dependency installation scripts execute, can use the network, and can alter their own proposed workflow. A maintainer must review workflow changes and fork-run approvals. No branch-protection rules or repository settings are changed here.

## Verified action identities

Verified on 2026-10-22 using each release's link to its upstream commit and `action.yml` at that exact SHA, rather than copying an example hash:

| Upstream action | Release | Pinned full commit |
| --- | --- | --- |
| `actions/checkout` | [v7.0.1](https://github.com/actions/checkout/releases/tag/v7.0.1) | [`3d3c42e5aac5ba805825da76410c181273ba90b1`](https://github.com/actions/checkout/commit/3d3c42e5aac5ba805825da76410c181273ba90b1) |
| `actions/setup-node` | [v7.1.0](https://github.com/actions/setup-node/releases/tag/v7.1.0) | [`949feb2413d6458794dcd2491c4babbbce0c15c1`](https://github.com/actions/setup-node/commit/949feb2413d6458794dcd2491c4babbbce0c15c1) |

Pinned metadata: [checkout](https://raw.githubusercontent.com/actions/checkout/3d3c42e5aac5ba805825da76410c181273ba90b1/action.yml), [setup-node](https://raw.githubusercontent.com/actions/setup-node/949feb2413d6458794dcd2491c4babbbce0c15c1/action.yml). Both use their internal Node 24 action runtime and require runner version at least `2.327.1`; project checks still use Node `22.23.3`. No third-party Rust setup action is needed; rustup's [version/host syntax](https://rust-lang.github.io/rustup/concepts/toolchains.html) selects the existing MSVC contract.

To update a pin, visit the actual upstream repository, inspect the release and changes, follow its commit link (dereference an annotated tag if applicable), and confirm the full 40-character commit belongs to that upstream. Review its `action.yml`, runtime support, new inputs/defaults, and credential/cache behavior. Update the SHA, version comment, and this table together; rerun local validation and inspect both hosted jobs before merging. Toolchain or runner updates require a separately reviewed compatibility check, not an unreviewed switch to `latest` or floating `stable`.

## Local reproduction and native boundary

From the repository root, run the four frontend commands above; on Windows use `npm.cmd` if PowerShell script policy blocks `npm.ps1`. The local verified Node/npm versions are `22.23.3`/`10.9.9`. A repository-local npm cache can be selected with `$env:npm_config_cache = Join-Path (Get-Location) '.npm'`.

With the pinned Rust MSVC toolchain and its components already installed, set `$env:CARGO_BUILD_JOBS = '1'` and select that toolchain with `$env:RUSTUP_TOOLCHAIN` (or an explicitly versioned Cargo invocation). Run the three Cargo commands above. Check `rustc -Vv` and `cargo --version` first; an installed floating stable alias alone is not a reproducible pin. The hosted installation command is for disposable runners; installing a toolchain on the owner's machine still requires authorization under AGENTS.md.

The separate local native gate remains `scripts/verify-native.ps1`, optionally with `-RestoreStress`, as documented in [README](../../README.md#checks-and-windows-package) and [release/recovery guidance](RELEASE_AND_RECOVERY.md). Its packaging, isolated installation, actual WebView2 lifecycle/offline checks, native dialogs, keyboard/visual review, and cleanup are **not** executed by hosted CI. Stage 4.5 historically reported 111 installed lifecycle checks; that is neither a new result nor a CI test count. This milestone does not run the installed harness or touch the normal app/profile.

## Validation evidence — 2026-10-22

All application commands below ran locally on Windows with Node `22.23.3`, npm `10.9.9`, Rust/Cargo `1.99.1` MSVC, C++ Build Tools `17.14.37806.7`, and existing public dependency caches. No machine-wide installation or security-policy change was needed. The installed stable alias was version-checked; the workflow's fresh rustup installation was not run on the owner's machine.

| Local gate | Observed result |
| --- | --- |
| `actionlint 1.7.12 .github/workflows/ci.yml` | Exit 0, no diagnostics; YAML, workflow schema, events, expressions and job configuration accepted |
| Shell syntax | Git Bash `-n` accepted the four extracted frontend commands; PowerShell's parser accepted all eight `run` commands |
| Security/dependency review | Exact upstream action identities and runner labels verified; triggers, permissions, inputs, toolchain setup, concurrency and timeouts independently reviewed |
| `npm.cmd ci` with repository-local cache | Passed; 107 packages installed, lockfile unchanged |
| `npm.cmd run typecheck` | Passed both TypeScript configurations |
| `npm.cmd test` | **146 passed in eight files**, zero failed; 23.92 s |
| `npm.cmd run build` | Passed; 40 modules, Vite build 327 ms |
| `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | Passed |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked -- --test-threads=1` | **191 passed**, zero failed; four existing release benchmarks ignored; compilation 1m 33s |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings` | Passed with `dist` absent; 23.88 s |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked --no-run` | All test binaries rebuilt successfully with `dist` absent; 59.99 s |
| Scope/diff review | Application source, tests, manifests/lockfiles, specifications and historical verification reports unchanged; `git diff --check` passed |

For the missing-asset check, the ignored `dist` directory was temporarily moved inside the repository and restored afterward. Touching only the timestamp of `src-tauri/build.rs` forced package recompilation without changing its bytes. This establishes that these debug Cargo checks do not need a frontend build. It is not a cold hosted-runner test: dependency caches and the local Windows toolchain were already available. The final frontend build passed after restoration. Rustup emitted a local home-path canonicalization warning; all recorded Cargo commands exited zero.

Actionlint was downloaded only into ignored `.toolchain/ci-actionlint/` from its [official v1.7.12 release](https://github.com/rhysd/actionlint/releases/tag/v1.7.12); the Windows AMD64 archive's SHA-256 `6e7241b51e6817ea6a047693d8e6fed13b31819c9a0dd6c5a726e1592d22f6e9` matched the official checksum manifest. No global installation or PATH change occurred. ShellCheck was unavailable; Bash syntax parsing and the simple literal npm commands were checked separately. PowerShell parsing used the installed local parser, not a hosted PowerShell execution.

Ignored local logs are `.artifacts/ci-{npm-ci,typecheck,frontend-tests,frontend-build,rust-fmt,rust-tests,clippy-no-dist,test-compile-no-dist}.log`. They are not committed or uploaded. Hosted Ubuntu frontend behavior, fresh registry/toolchain provisioning, hosted Windows timing and actual GitHub execution remain **unobserved**. The installed lifecycle harness was not rerun. Both hosted jobs must be inspected after the owner pushes this branch and opens a PR.

## Inspecting a PR failure

Open the PR's Checks tab, select `Frontend` or `Rust (Windows)` and expand the first failed step. Check the tested merge SHA, runner image/version in `Set up job`, toolchain installation output, exact command, and first substantive error. Distinguish checkout/registry/toolchain availability, timeout/resource failure, and an assertion/compiler failure. For a cold-runner failure, investigate the actual missing prerequisite or platform assumption before changing application code or tests. Preserve the intended test contract and never turn a failed assertion into a skip merely to pass CI.

Reproduce using the pinned toolchains and committed lockfiles. If the PR has merge conflicts or awaits fork-run approval, resolve that platform gate before expecting a run. After a fix or justified rerun, inspect both current checks rather than relying on an older green commit or a canceled run. Local lint/tests are not evidence that GitHub executed the workflow, and green CI alone does not replace installed Windows acceptance.
