## Context

The `ModelProvider` trait in `crates/model/src/provider.rs` defines a synchronous interface for model providers. Current implementations use blocking I/O (`reqwest::blocking::Client` for OpenAI), which blocks async runtimes and reduces concurrency. The orchestrator and other consumers call `generate()` synchronously, preventing efficient async usage.

## Goals / Non-Goals

**Goals:**
- Make `ModelProvider` trait async to enable non-blocking I/O
- Maintain existing error types and semantics
- Keep API surface minimal (only `generate` and `capabilities`)
- Ensure all trait implementors compile with async trait

**Non-Goals:**
- Adding new methods to the trait
- Changing error type signatures
- Introducing streaming/chunked responses
- Modifying the `GenerationRequest` or `GenerationResponse` types

## Decisions

### Decision 1: Use `async_trait` or native async traits

**Chosen:** Native async traits with generics (no `dyn`)

**Rationale:**
- The project uses Rust 2024 edition (per AGENTS.md)
- Native async traits are stable and avoid the boxing overhead of `async_trait`
- Using generics instead of `dyn ModelProvider` avoids the dyn-compatibility issue
- Generic parameters provide better performance (monomorphization)

**Alternatives considered:**
- `#[async_trait]` macro - adds complexity and boxing
- `dyn ModelProvider` with trait objects - not dyn-compatible with `impl Future` returns

### Decision 2: Async `capabilities()` method

**Chosen:** Make `capabilities()` async

**Rationale:**
- Consistency with `generate()`
- Future-proofing: some providers may need async discovery (e.g., remote capability lookup)
- Minimal cost for implementors who can return immediately with `std::future::ready`

### Decision 3: `Send` bound on futures

**Chosen:** Require futures to be `Send`

**Rationale:**
- Futures returned by trait methods need to cross thread boundaries in async runtimes
- `ModelProvider: Send + Sync` already implies thread-safe implementations
- Matches common async usage patterns

### Decision 4: Generics vs trait objects

**Chosen:** Use generics instead of `dyn ModelProvider`

**Rationale:**
- `impl Future<Output = ...>` return types are not dyn-compatible
- Generic parameters enable monomorphization for better performance
- All usages (`ModelClient`, `WorkerExecutor`, `RemediationEngine`) will become generic over `P: ModelProvider`
- Call sites will use concrete types (e.g., `Arc<OpenAiProvider>`)

**Trade-offs:**
- Slightly more verbose type signatures
- No runtime flexibility (can't swap implementations at runtime)
- Better compile-time optimization

## Risks / Trade-offs

**Breaking change for downstream crates** → All consumers must update call sites to use `.await`. This is acceptable given the project's controlled dependency graph.

**Test complexity** → Async tests require `#[tokio::test]` instead of `#[test]`. Extension for integration tests with mock servers.

**Synchronous provider implementations** → Providers that don't need async (e.g., in-memory mocks) can use `std::future::ready` or implement async fns with `.await` on immediate values.

## Open Questions

(None - design is straightforward async migration)