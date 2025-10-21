# Codex Accounting - Development Documentation Index

## 🎯 Project Status

**Current State**: Backend 80% complete, AI integration 0%, UI 0%  
**Goal**: Transform Codex CLI into autonomous accounting software with ChatGPT  
**Timeline**: 8-12 weeks to MVP, 4-6 months to production

---

## 📚 Documentation Guide

### For Decision Makers & Project Managers

**Start here**: [`EXECUTIVE_SUMMARY.md`](./EXECUTIVE_SUMMARY.md)
- High-level status and vision
- Investment requirements and ROI
- Risk assessment
- Success metrics
- Timeline and milestones

### For Architects & Technical Leads

**Start here**: [`DEVELOPMENT_ROADMAP.md`](./DEVELOPMENT_ROADMAP.md)
- Complete architecture overview
- Data flow diagrams
- Technical specifications for 6 phases
- Integration points and dependencies
- Stack decisions and rationale

### For Developers (Human)

**Start here**: [`QUICK_START_GUIDE.md`](./QUICK_START_GUIDE.md)
- Week-by-week implementation guide
- Code examples and templates
- Quick wins for getting a demo
- Testing strategies
- Debugging tips

### For AI Coding Agents

**Start here**: [`DEVELOPMENT_PLAN.md`](./DEVELOPMENT_PLAN.md)
- 250 granular tasks across 6 phases
- Each task: file path, line count, priority, dependencies
- Clear acceptance criteria
- Organized by blocking relationships

**Then**: [`WEEK_1_TASKS.md`](./WEEK_1_TASKS.md)
- Day-by-day breakdown of first week
- Detailed code templates
- Copy-paste examples
- Expected outputs
- Troubleshooting guide

### For Product Requirements

**See**: [`codex-rs/specs/autonomous-accounting-spec.md`](./codex-rs/specs/autonomous-accounting-spec.md)
- Complete product specification
- User stories and workflows
- Functional requirements
- UX highlights

**And**: [`codex-rs/specs/tasks.md`](./codex-rs/specs/tasks.md)
- Original task breakdown
- Phase-by-phase milestones

---

## 🚀 Quick Navigation by Role

### "I want to understand what we're building"
→ Read: EXECUTIVE_SUMMARY.md → specs/autonomous-accounting-spec.md

### "I need to present this to stakeholders"
→ Use: EXECUTIVE_SUMMARY.md (business case, metrics, timeline)

### "I'm designing the architecture"
→ Study: DEVELOPMENT_ROADMAP.md (data flows, tech stack, integration points)

### "I'm starting development this week"
→ Follow: WEEK_1_TASKS.md → QUICK_START_GUIDE.md → DEVELOPMENT_PLAN.md

### "I'm an AI agent ready to code"
→ Execute: DEVELOPMENT_PLAN.md tasks in order, referencing WEEK_1_TASKS.md for detail

### "I need to understand the existing codebase"
→ Review: Backend code tour below

---

## 🏗️ Existing Codebase Tour

### Core Accounting Engine (Rust)

```
codex-rs/
├── codex-ledger/               ✅ Complete - Double-entry bookkeeping
│   └── src/lib.rs             (723 lines: Company, Account, Journal, Entry)
├── codex-accounting-api/       ✅ Complete - Facade layer
│   ├── src/facade.rs          (574 lines: LedgerFacade, operations)
│   └── src/demo.rs            (Demo data generator)
├── codex-bank-ingest/          ✅ Complete - Bank statement parsing
│   └── src/lib.rs             (732 lines: CSV/OFX parsers, deduplication)
├── codex-reconcile/            ✅ Complete - Match engine
│   └── src/lib.rs             (940 lines: Scoring, sessions, workflows)
├── codex-approvals/            ✅ Complete - Approval workflows
│   └── src/lib.rs             (704 lines: Multi-stage, SLA tracking)
├── codex-policy/               ✅ Complete - Policy evaluation
│   └── src/lib.rs             (879 lines: Rules, thresholds, routing)
├── codex-ocr/                  ✅ Complete - Document classification
├── codex-doc-ingest/           ✅ Complete - Upload pipeline
├── codex-tenancy/              ✅ Complete - Multi-tenant support
├── codex-audit-log/            ✅ Complete - Immutable logging
└── codex-doc-store/            ✅ Complete - Storage abstraction
```

### What's Missing (20% of work)

```
codex-rs/
└── core/
    ├── src/tools/
    │   └── accounting.rs       ❌ TO CREATE - 7 tool handlers
    └── src/accounting/
        ├── document_agent.rs   ❌ TO CREATE - AI extraction
        ├── posting_agent.rs    ❌ TO CREATE - Autonomous posting
        └── context.rs          ❌ TO CREATE - ChatGPT context

app-server/
└── src/
    └── accounting_handlers.rs  ❌ TO CREATE - JSON-RPC methods

apps/
└── codex-gui/                  ❌ TO CREATE - React web app
    ├── src/
    │   ├── lib/rpc-client.ts
    │   ├── pages/
    │   └── components/
    └── package.json
```

---

## 📊 Progress Tracking

### Phase 1: AI Foundation (Weeks 1-3)
- [ ] Core accounting tools (7 handlers)
- [ ] Document extraction agent
- [ ] Posting agent with policy integration
- [ ] CLI commands

**Milestone**: Upload invoice → AI extracts → suggests entry

### Phase 2: App Server API (Weeks 3-4)
- [ ] Protocol definitions (TypeScript bindings)
- [ ] JSON-RPC handlers (15-20 methods)
- [ ] Integration tests

**Milestone**: UI can call all accounting operations

### Phase 3: Web UI (Weeks 4-8)
- [ ] Project setup (React + Vite)
- [ ] Layout and navigation
- [ ] Upload center with progress
- [ ] Approval queue
- [ ] Transaction ledger
- [ ] Reports (trial balance, P&L, balance sheet)
- [ ] Chat assistant

**Milestone**: Full web app with all features

### Phase 4: CLI/TUI (Weeks 5-8)
- [ ] Detailed CLI implementations
- [ ] TUI screens

**Milestone**: Complete command-line experience

### Phase 5: AI Enhancements (Weeks 8-9)
- [ ] Learning from corrections
- [ ] Pattern recognition
- [ ] Smart features (fuzzy matching, tax suggestions, etc.)

**Milestone**: 85%+ accuracy on test documents

### Phase 6: Production Ready (Weeks 9-12)
- [ ] PostgreSQL persistence
- [ ] S3 document storage
- [ ] Security (RBAC, encryption, auth)
- [ ] Performance optimization
- [ ] Compliance (audit trails, retention)

**Milestone**: Production deployment, pilot launch

---

## 🎓 Learning Resources

### Accounting Basics
- Double-entry bookkeeping: debits = credits
- Chart of accounts: assets, liabilities, equity, revenue, expenses
- Journal entries: recording financial transactions

### Tech Stack
- **Backend**: Rust (memory-safe, concurrent, fast)
- **AI**: ChatGPT-4 (document understanding, reasoning)
- **UI**: React 19 + Vite + TailwindCSS
- **API**: JSON-RPC over WebSocket
- **Database**: PostgreSQL (for production)
- **Storage**: S3-compatible (documents)

### Key Patterns
- **Facades**: Clean API layer over domain services
- **Tool Registry**: Extensible function calling for ChatGPT
- **Policy Engine**: Route decisions (auto-post vs. approval)
- **Agent Loop**: Continuous processing of queued documents

---

## 🛠️ Development Commands

### Build & Test
```bash
# Build with accounting features
cd codex-rs
cargo build --features ledger

# Run tests
cargo test --features ledger

# Run specific test
cargo test --features ledger accounting

# Format code (required before commit)
just fmt

# Fix linter issues
just fix -p codex-core
```

### Run Services
```bash
# App server
cargo run --bin codex-app-server --features ledger

# CLI
cargo run --bin codex -- accounting company list

# Web UI (in separate terminal)
cd apps/codex-gui
pnpm dev
```

---

## 📞 Getting Help

### Issues During Development

1. **Compilation errors**: Check feature flags, verify dependencies
2. **Test failures**: Review acceptance criteria, check mocks
3. **ChatGPT integration**: Verify function schemas, test prompts
4. **UI connection**: Inspect WebSocket in DevTools
5. **Performance**: Profile with `cargo flamegraph`

### Architecture Questions

- Review DEVELOPMENT_ROADMAP.md section on that component
- Check existing code in similar crate (e.g., approvals → reconcile)
- See specs/ for product requirements

### Task Clarifications

- Each task in DEVELOPMENT_PLAN.md has acceptance criteria
- WEEK_1_TASKS.md has detailed examples
- QUICK_START_GUIDE.md has troubleshooting section

---

## ✅ Definition of Done

### For Each Task
- [ ] Code compiles without warnings
- [ ] Tests pass (>80% coverage for P0/P1 tasks)
- [ ] Follows Rust style guide (format!, inline args, etc.)
- [ ] Acceptance criteria met
- [ ] No clippy warnings

### For Each Phase
- [ ] All phase tasks complete
- [ ] Integration tests pass
- [ ] Milestone demo works
- [ ] Documentation updated

### For MVP (Week 8-12)
- [ ] Upload document → AI posts entry (80%+ accuracy)
- [ ] Approval workflow functional
- [ ] Bank reconciliation works (70%+ auto-match)
- [ ] Basic reports generate
- [ ] Web UI fully functional
- [ ] 1 pilot CPA firm onboarded

### For Production (Month 6)
- [ ] Database persistence
- [ ] Security hardened (RBAC, encryption, auth)
- [ ] Performance tested (100 users, <2s response)
- [ ] 90%+ AI accuracy
- [ ] 5+ CPA firms, 50+ companies
- [ ] SOC2 audit in progress

---

## 🎯 Start Here

1. **Understand the vision**: Read EXECUTIVE_SUMMARY.md
2. **Review architecture**: Skim DEVELOPMENT_ROADMAP.md
3. **Start coding**: Follow WEEK_1_TASKS.md day-by-day
4. **Track progress**: Check off tasks in DEVELOPMENT_PLAN.md
5. **Get support**: Reference this README for navigation

**First task**: Task 1 in WEEK_1_TASKS.md (create accounting tools file)

Good luck! You have everything you need. 🚀
