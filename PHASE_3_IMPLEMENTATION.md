# Phase 3 Implementation Plan: Web UI Development

**Start Date**: October 21, 2025  
**Status**: In Progress  
**Dependencies**: Phase 1 ✅ Complete | Phase 2 ✅ Complete

---

## 🎯 Overview

Build a modern React web application for Codex Accounting that provides:
- Company management and context viewing
- Chart of accounts browser
- Journal entries ledger
- Document upload and AI suggestion review
- JSON-RPC integration with the app server

---

## 📋 Prerequisites Checklist

✅ Phase 1 Complete - AI Agent Integration  
✅ Phase 2 Complete - App Server API Layer  
✅ TypeScript bindings ready to generate from protocol types  
✅ App server endpoints functional:
- `ledgerListCompanies`
- `ledgerListAccounts`
- `ledgerListEntries`
- `ledgerGetCompanyContext`
- `ledgerProcessDocument`

---

## 🏗️ Architecture

### Tech Stack

**Core Framework**:
- React 19 with TypeScript (strict mode)
- Vite for build tooling and dev server
- TypeScript 5.x with strict type checking

**UI & Styling**:
- TailwindCSS for utility-first styling
- shadcn/ui for high-quality component primitives
- Lucide React for icons
- Radix UI primitives (via shadcn/ui)

**State Management**:
- TanStack Query (React Query) for server state
- TanStack Router for type-safe routing
- Zustand for lightweight UI state (modals, selections)

**API Integration**:
- Custom JSON-RPC client for app server communication
- WebSocket support for real-time updates (future)
- Auto-generated TypeScript types from Rust protocol

**Development Tools**:
- ESLint for code linting
- Prettier for code formatting
- Vitest for unit testing
- Playwright for E2E testing (future)

---

## 📁 Project Structure

```
apps/codex-gui/
├── src/
│   ├── api/
│   │   ├── client.ts           # JSON-RPC client
│   │   ├── types.ts            # Re-export generated types
│   │   └── hooks/              # React Query hooks
│   │       ├── useCompanies.ts
│   │       ├── useAccounts.ts
│   │       ├── useEntries.ts
│   │       └── useDocuments.ts
│   ├── components/
│   │   ├── ui/                 # shadcn/ui components
│   │   ├── layout/
│   │   │   ├── AppLayout.tsx
│   │   │   ├── Sidebar.tsx
│   │   │   └── Header.tsx
│   │   ├── companies/
│   │   │   ├── CompanyList.tsx
│   │   │   └── CompanyCard.tsx
│   │   ├── accounts/
│   │   │   ├── AccountList.tsx
│   │   │   └── AccountTree.tsx
│   │   ├── entries/
│   │   │   ├── EntryList.tsx
│   │   │   └── EntryDetail.tsx
│   │   └── documents/
│   │       ├── DocumentUpload.tsx
│   │       └── SuggestionReview.tsx
│   ├── pages/
│   │   ├── Dashboard.tsx
│   │   ├── Companies.tsx
│   │   ├── Accounts.tsx
│   │   ├── Entries.tsx
│   │   └── Documents.tsx
│   ├── lib/
│   │   ├── utils.ts            # Utility functions
│   │   └── format.ts           # Currency/date formatting
│   ├── stores/
│   │   └── uiStore.ts          # Zustand stores
│   ├── routes/
│   │   └── __root.tsx          # TanStack Router root
│   ├── App.tsx
│   ├── main.tsx
│   └── index.css
├── bindings/                   # Generated TS types from Rust
├── public/
├── index.html
├── package.json
├── tsconfig.json
├── vite.config.ts
├── tailwind.config.js
├── components.json             # shadcn/ui config
└── README.md
```

---

## 🚀 Implementation Tasks

### Task 1: Generate TypeScript Bindings ⏳

**Goal**: Export TypeScript types from Rust protocol definitions

**Steps**:
1. Run `cargo test` in `codex-rs/app-server-protocol` with ledger feature
2. Verify bindings are generated in `bindings/` directory
3. Copy or symlink bindings to `apps/codex-gui/bindings/`

**Commands**:
```bash
cd codex-rs/app-server-protocol
cargo test --features ledger
ls bindings/  # Verify *.ts files exist
```

**Expected Output**:
- `LedgerListCompaniesParams.ts`
- `LedgerListCompaniesResponse.ts`
- `LedgerListAccountsParams.ts`
- `LedgerListAccountsResponse.ts`
- `LedgerListEntriesParams.ts`
- `LedgerListEntriesResponse.ts`
- `LedgerGetCompanyContextParams.ts`
- `LedgerGetCompanyContextResponse.ts`
- `LedgerProcessDocumentParams.ts`
- `LedgerProcessDocumentResponse.ts`
- `LedgerCompany.ts`
- `LedgerAccount.ts`
- `LedgerJournalEntry.ts`
- `LedgerJournalEntrySuggestion.ts`
- Supporting types...

**Duration**: 5 minutes

---

### Task 2: Bootstrap Vite Project ⏳

**Goal**: Create React + TypeScript + Vite project structure

**Steps**:
```bash
cd apps/codex-gui
pnpm create vite@latest . --template react-ts
pnpm install
```

**Files to Create/Modify**:
- `package.json` - Add dependencies
- `tsconfig.json` - Configure TypeScript strict mode
- `vite.config.ts` - Configure Vite
- `.eslintrc.cjs` - ESLint configuration
- `.prettierrc` - Prettier configuration
- `.gitignore` - Git ignore patterns

**Duration**: 15 minutes

---

### Task 3: Install Dependencies ⏳

**Goal**: Add all required npm packages

**Commands**:
```bash
cd apps/codex-gui

# Core dependencies
pnpm add react@^19 react-dom@^19
pnpm add @tanstack/react-query @tanstack/react-router
pnpm add zustand

# UI dependencies
pnpm add tailwindcss postcss autoprefixer
pnpm add lucide-react
pnpm add class-variance-authority clsx tailwind-merge

# Dev dependencies
pnpm add -D @types/react @types/react-dom
pnpm add -D @typescript-eslint/eslint-plugin @typescript-eslint/parser
pnpm add -D eslint eslint-plugin-react-hooks eslint-plugin-react-refresh
pnpm add -D prettier prettier-plugin-tailwindcss
pnpm add -D vitest @testing-library/react @testing-library/jest-dom

# Initialize Tailwind
pnpm dlx tailwindcss init -p
```

**Duration**: 10 minutes

---

### Task 4: Configure shadcn/ui ⏳

**Goal**: Set up shadcn/ui component library

**Steps**:
```bash
cd apps/codex-gui
pnpm dlx shadcn-ui@latest init

# Add base components
pnpm dlx shadcn-ui@latest add button
pnpm dlx shadcn-ui@latest add card
pnpm dlx shadcn-ui@latest add table
pnpm dlx shadcn-ui@latest add dialog
pnpm dlx shadcn-ui@latest add select
pnpm dlx shadcn-ui@latest add input
pnpm dlx shadcn-ui@latest add badge
pnpm dlx shadcn-ui@latest add tabs
pnpm dlx shadcn-ui@latest add toast
```

**Duration**: 10 minutes

---

### Task 5: Create JSON-RPC Client ⏳

**Goal**: Build type-safe API client for app server

**File**: `src/api/client.ts`

**Features**:
- JSON-RPC 2.0 request/response handling
- Request ID generation
- Error handling and parsing
- TypeScript generics for type safety
- Configurable base URL

**Sample Code**:
```typescript
export class JsonRpcClient {
  constructor(private baseUrl: string) {}
  
  async call<TParams, TResponse>(
    method: string,
    params: TParams
  ): Promise<TResponse> {
    const id = crypto.randomUUID();
    const response = await fetch(this.baseUrl, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        jsonrpc: '2.0',
        method,
        params,
        id,
      }),
    });
    
    const json = await response.json();
    if (json.error) {
      throw new Error(json.error.message);
    }
    return json.result;
  }
}
```

**Duration**: 30 minutes

---

### Task 6: Create React Query Hooks ⏳

**Goal**: Wrap JSON-RPC calls in React Query hooks

**Files**:
- `src/api/hooks/useCompanies.ts`
- `src/api/hooks/useAccounts.ts`
- `src/api/hooks/useEntries.ts`
- `src/api/hooks/useCompanyContext.ts`
- `src/api/hooks/useProcessDocument.ts`

**Sample Hook**:
```typescript
export function useCompanies(search?: string) {
  return useQuery({
    queryKey: ['companies', search],
    queryFn: () => client.call('ledgerListCompanies', { search }),
  });
}
```

**Duration**: 45 minutes

---

### Task 7: Layout & Navigation ⏳

**Goal**: Create app shell with sidebar navigation

**Files**:
- `src/components/layout/AppLayout.tsx`
- `src/components/layout/Sidebar.tsx`
- `src/components/layout/Header.tsx`

**Features**:
- Responsive sidebar
- Navigation menu (Dashboard, Companies, Accounts, Entries, Documents)
- Header with app title
- Mobile menu toggle

**Duration**: 1 hour

---

### Task 8: Companies Page ⏳

**Goal**: List companies and view company context

**Files**:
- `src/pages/Companies.tsx`
- `src/components/companies/CompanyList.tsx`
- `src/components/companies/CompanyCard.tsx`
- `src/components/companies/CompanyContext.tsx`

**Features**:
- List all companies
- Search/filter companies
- Click to view company details
- Display company context (chart of accounts, recent transactions, policy)
- Show loading and error states

**Duration**: 1.5 hours

---

### Task 9: Accounts Page ⏳

**Goal**: Browse chart of accounts with filtering

**Files**:
- `src/pages/Accounts.tsx`
- `src/components/accounts/AccountList.tsx`
- `src/components/accounts/AccountTree.tsx`

**Features**:
- List accounts for selected company
- Filter by account type (Asset, Liability, etc.)
- Display account details (code, name, type, balance)
- Hierarchical tree view (if parent accounts exist)
- Currency formatting

**Duration**: 1.5 hours

---

### Task 10: Entries Page ⏳

**Goal**: Browse journal entries with pagination

**Files**:
- `src/pages/Entries.tsx`
- `src/components/entries/EntryList.tsx`
- `src/components/entries/EntryDetail.tsx`

**Features**:
- List journal entries for selected company
- Pagination controls
- Date range filtering
- Account code filtering
- Click to view entry details (debit/credit lines)
- Show entry status, origin, memo

**Duration**: 1.5 hours

---

### Task 11: Document Processing Page ⏳

**Goal**: Upload documents and review AI suggestions

**Files**:
- `src/pages/Documents.tsx`
- `src/components/documents/DocumentUpload.tsx`
- `src/components/documents/SuggestionReview.tsx`

**Features**:
- File upload with drag & drop (mock for now)
- Process document button
- Display AI suggestion:
  - Suggested journal entry lines
  - Memo/description
  - Confidence score
  - AI reasoning
- Accept/Reject actions (mock for now)
- Visual confidence indicator

**Duration**: 2 hours

---

### Task 12: Utility Functions ⏳

**Goal**: Create helper functions for formatting and utilities

**Files**:
- `src/lib/utils.ts` - Class name utilities
- `src/lib/format.ts` - Currency and date formatting

**Features**:
```typescript
// Format currency from minor units
export function formatCurrency(minor: number, precision: number): string {
  const major = minor / Math.pow(10, precision);
  return new Intl.NumberFormat('en-US', {
    style: 'currency',
    currency: 'USD',
  }).format(major);
}

// Format dates
export function formatDate(dateStr: string): string {
  return new Date(dateStr).toLocaleDateString();
}
```

**Duration**: 30 minutes

---

### Task 13: Router Configuration ⏳

**Goal**: Set up TanStack Router for navigation

**Files**:
- `src/routes/__root.tsx`
- `src/routes/index.tsx`
- `src/routes/companies.tsx`
- `src/routes/accounts.tsx`
- `src/routes/entries.tsx`
- `src/routes/documents.tsx`

**Duration**: 1 hour

---

### Task 14: Documentation ⏳

**Goal**: Update README with setup and run instructions

**File**: `apps/codex-gui/README.md`

**Content**:
- Project overview
- Prerequisites
- Installation steps
- Development server commands
- Build commands
- Environment variables
- Architecture overview
- API integration details

**Duration**: 30 minutes

---

## 🧪 Testing Strategy

### Manual Testing Checklist

- [ ] Companies page loads and displays mock data
- [ ] Accounts page shows filtered accounts
- [ ] Entries page displays paginated entries
- [ ] Document upload shows AI suggestion
- [ ] Navigation works between all pages
- [ ] Loading states display correctly
- [ ] Error states display correctly
- [ ] Responsive design works on mobile

### Unit Tests (Future)

- Component rendering tests
- API client tests with mocked responses
- Utility function tests

### E2E Tests (Future)

- Full workflow: Upload → Process → Review → Post
- Navigation flows
- Error handling

---

## 📊 Success Criteria

✅ Vite project runs without errors  
✅ TypeScript bindings imported successfully  
✅ JSON-RPC client communicates with app server  
✅ All pages render with mock data  
✅ Navigation works smoothly  
✅ UI is responsive and accessible  
✅ Code follows repo conventions (linting, formatting)  
✅ README documents setup and usage  

---

## 🚧 Known Limitations (MVP)

1. **Mock Data**: Using mock responses where backend isn't wired yet
2. **No Authentication**: Login flow to be added later
3. **No WebSocket**: Real-time updates deferred to future
4. **No Document Storage**: File upload creates mock upload_id
5. **No Approval Actions**: Accept/Reject buttons are placeholders
6. **Basic Styling**: Polish and dark mode deferred

---

## 🔮 Future Enhancements

- Authentication & session management
- WebSocket for real-time updates
- Document preview (PDF/image viewer)
- Approval workflow actions
- Dark mode toggle
- Export features (CSV, PDF)
- Advanced filtering and search
- Financial reports (P&L, Balance Sheet)
- Bulk operations
- Keyboard shortcuts
- Mobile app (React Native)

---

## 📅 Timeline Estimate

**Total Duration**: 12-15 hours

- Task 1-4: Setup & Config (1 hour)
- Task 5-6: API Client (1.5 hours)
- Task 7: Layout (1 hour)
- Task 8-11: Pages & Components (6.5 hours)
- Task 12-13: Utils & Router (1.5 hours)
- Task 14: Documentation (0.5 hours)
- Testing & Polish (1 hour)

**Timeline**: 2-3 days for 1 developer

---

## 🎯 Next Session Continuation

If interrupted, resume with:
1. Check `PHASE_3_PROGRESS.md` for completed tasks
2. Review generated TypeScript bindings in `apps/codex-gui/bindings/`
3. Continue with next pending task in the plan
4. Run `pnpm dev` to test progress

---

**Status**: 🚀 Ready to Start  
**Dependencies**: ✅ All prerequisites met  
**Blockers**: None
