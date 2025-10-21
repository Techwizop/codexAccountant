# Codex Accounting Web UI

A modern React web application for Codex Accounting that provides intuitive workflows for accounting operations, document processing, and AI-powered journal entry suggestions.

## 🚀 Features

- **Company Management**: Browse companies, view context, and switch between tenants
- **Chart of Accounts**: Filter and view accounts by type with detailed information
- **Journal Entries**: Browse entries with pagination, filters, and drill-down details
- **Document Processing**: Upload documents and review AI-generated journal entry suggestions
- **Real-time Updates**: React Query integration for efficient server state management
- **Type Safety**: Full TypeScript support with auto-generated types from Rust protocol

## 📋 Prerequisites

- Node.js >= 22
- pnpm >= 9.0.0
- Codex app server running with ledger feature enabled

## 🛠️ Installation

1. **Generate TypeScript Bindings** (from Rust protocol):
   ```bash
   cd ../../codex-rs/app-server-protocol
   cargo test --features ledger
   ```

2. **Install Dependencies**:
   ```bash
   cd apps/codex-gui
   pnpm install
   ```

## 🏃 Running the Application

### Development Mode

1. **Start the Codex App Server** (in one terminal):
   ```bash
   cd codex-rs
   CODEX_LEDGER_IN_MEMORY=1 cargo run --features ledger --bin codex-app-server
   ```
   The app server will run on `http://localhost:8080`

2. **Start the Vite Dev Server** (in another terminal):
   ```bash
   cd apps/codex-gui
   pnpm dev
   ```
   The web UI will run on `http://localhost:3000`

3. **Open your browser** to `http://localhost:3000`

### Production Build

```bash
pnpm build
pnpm preview
```

## 📁 Project Structure

```
src/
├── api/
│   ├── client.ts           # JSON-RPC client
│   └── hooks.ts            # React Query hooks
├── components/
│   ├── ui/                 # UI primitives (Button, Card, Badge, Input)
│   └── layout/             # Layout components (AppLayout, Sidebar, Header)
├── pages/
│   ├── DashboardPage.tsx   # Dashboard with navigation
│   ├── CompaniesPage.tsx   # Company list and context
│   ├── AccountsPage.tsx    # Chart of accounts
│   ├── EntriesPage.tsx     # Journal entries
│   └── DocumentsPage.tsx   # Document processing
├── lib/
│   ├── utils.ts            # Utility functions
│   └── format.ts           # Formatting helpers
├── types/
│   └── protocol.ts         # TypeScript types (placeholder)
├── App.tsx                 # Main app component
├── main.tsx                # React entry point
└── index.css               # Global styles
```

## 🎨 Tech Stack

### Core
- **React 19** - UI framework
- **TypeScript** - Type safety
- **Vite** - Build tool and dev server

### Styling
- **TailwindCSS** - Utility-first CSS
- **shadcn/ui** - Component primitives
- **Lucide React** - Icons

### State Management
- **TanStack Query (React Query)** - Server state management
- **Zustand** (ready for use) - Client state management

### API Integration
- Custom JSON-RPC 2.0 client
- Type-safe hooks with React Query
- Auto-generated TypeScript types from Rust

## 🔌 API Integration

The UI communicates with the Codex app server via JSON-RPC 2.0 over HTTP. The following endpoints are available:

### Implemented Endpoints

| Method | Description |
|--------|-------------|
| `ledgerListCompanies` | List all companies with optional search |
| `ledgerListAccounts` | Get chart of accounts for a company |
| `ledgerListEntries` | Browse journal entries with pagination |
| `ledgerGetCompanyContext` | Get aggregated context for AI operations |
| `ledgerProcessDocument` | Process uploaded document with AI |

### Usage Example

```typescript
import { useCompanies } from '@/api/hooks'

function CompanyList() {
  const { data, isLoading, error } = useCompanies({ search: 'Acme' })
  
  if (isLoading) return <div>Loading...</div>
  if (error) return <div>Error: {error.message}</div>
  
  return (
    <ul>
      {data?.companies.map(company => (
        <li key={company.id}>{company.name}</li>
      ))}
    </ul>
  )
}
```

## 🎯 Available Pages

### Dashboard
Landing page with navigation cards and getting started guide.

### Companies
- List all companies
- Search companies by name
- View company context (chart of accounts, policy rules, recent transactions)
- Select company for other operations

### Accounts (Chart of Accounts)
- Browse all accounts for selected company
- Filter by account type (Asset, Liability, Equity, Revenue, Expense)
- View account details (code, name, currency mode, status)

### Entries (Journal Entries)
- Browse journal entries with pagination
- Filter by date range and account code
- View entry details (debit/credit lines, status, origin)
- See entry metadata (status, origin, reconciliation status)

### Documents (Document Processing)
- Upload documents (mock upload in MVP)
- Process documents with AI
- Review AI-generated journal entry suggestions
- View confidence scores and AI reasoning
- Accept/Reject suggestions (placeholder in MVP)

## 🧪 Development Scripts

```bash
# Start development server
pnpm dev

# Build for production
pnpm build

# Preview production build
pnpm preview

# Run linter
pnpm lint

# Format code
pnpm format

# Check formatting
pnpm format:check

# Type check
pnpm typecheck
```

## 🎨 Styling Conventions

### Tailwind Utilities
The project uses TailwindCSS with custom utilities for accounting-specific styling:

```css
.debit   /* Green text for debit amounts */
.credit  /* Red text for credit amounts */
.currency /* Monospace font with tabular numbers */
```

### Color Coding
- **Account Types**: Color-coded badges (Asset=green, Liability=red, etc.)
- **Confidence Scores**: Traffic light colors (>90%=green, 70-90%=yellow, <70%=red)
- **Entry Status**: Status badges (Posted=green, Proposed=yellow, Draft=gray)

## 🔧 Configuration

### Vite Configuration
API requests to `/api` are proxied to `http://localhost:8080` in development mode. This is configured in `vite.config.ts`:

```typescript
server: {
  proxy: {
    '/api': 'http://localhost:8080',
  },
}
```

### Environment Variables
Create a `.env` file for custom configuration (optional):

```bash
VITE_API_URL=http://localhost:8080/api
```

## 📦 Building for Production

```bash
pnpm build
```

Output will be in the `dist/` directory. Serve with any static file server:

```bash
pnpm preview  # Preview with Vite
# or
npx serve dist
```

## 🚧 Known Limitations (MVP)

1. **Mock Document Upload**: File upload creates mock upload IDs
2. **No Authentication**: Login flow to be added later
3. **No WebSocket**: Real-time updates deferred
4. **Placeholder Actions**: Accept/Reject buttons show alerts
5. **Basic Routing**: Simple state-based navigation (TanStack Router ready)

## 🔮 Planned Enhancements

- [ ] Authentication and session management
- [ ] WebSocket for real-time updates
- [ ] Actual document upload to S3
- [ ] Approval workflow implementation
- [ ] Dark mode toggle
- [ ] Financial reports (P&L, Balance Sheet)
- [ ] Export capabilities (CSV, PDF)
- [ ] Advanced search and filtering
- [ ] Keyboard shortcuts
- [ ] Comprehensive test coverage

## 🤝 Contributing

Follow the coding conventions:
- Use TypeScript strict mode
- Follow Prettier formatting (run `pnpm format`)
- Lint code before commit (run `pnpm lint`)
- Keep components small and focused
- Use React Query for server state
- Use Zustand for complex client state

## 📄 License

See the root LICENSE file for details.

## 🔗 Related Documentation

- [Phase 3 Implementation Plan](../../PHASE_3_IMPLEMENTATION.md)
- [Phase 3 Progress](../../PHASE_3_PROGRESS.md)
- [Development Roadmap](../../DEVELOPMENT_ROADMAP.md)
- [Rust Protocol Types](../../codex-rs/app-server-protocol/src/protocol.rs)

---

**Status**: ✅ Phase 3 Complete - Ready for testing  
**Version**: 0.1.0  
**Last Updated**: October 21, 2025
