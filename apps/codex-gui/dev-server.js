import http from 'http';

const PORT = 8081;

// Mock data
const mockCompanies = [
  {
    id: 'comp-001',
    name: 'Acme Corporation',
    baseCurrency: { code: 'USD', precision: 2 },
    fiscalCalendar: { periodsPerYear: 12, openingMonth: 1 },
  },
  {
    id: 'comp-002',
    name: 'TechStart Inc',
    baseCurrency: { code: 'USD', precision: 2 },
    fiscalCalendar: { periodsPerYear: 12, openingMonth: 1 },
  },
  {
    id: 'comp-003',
    name: 'Global Ventures Ltd',
    baseCurrency: { code: 'EUR', precision: 2 },
    fiscalCalendar: { periodsPerYear: 12, openingMonth: 4 },
  },
];

const mockAccounts = [
  { id: 'acc-001', code: '1000', name: 'Cash', accountType: 'Asset', balanceMinor: 1000000 },
  { id: 'acc-002', code: '1100', name: 'Accounts Receivable', accountType: 'Asset', balanceMinor: 500000 },
  { id: 'acc-003', code: '2000', name: 'Accounts Payable', accountType: 'Liability', balanceMinor: 300000 },
  { id: 'acc-004', code: '3000', name: 'Equity', accountType: 'Equity', balanceMinor: 1200000 },
  { id: 'acc-005', code: '4000', name: 'Revenue', accountType: 'Revenue', balanceMinor: 2000000 },
  { id: 'acc-006', code: '5000', name: 'Expenses', accountType: 'Expense', balanceMinor: 800000 },
];

const mockEntries = [
  {
    id: 'entry-001',
    entryNumber: 'JE-2025-001',
    date: '2025-01-15',
    memo: 'Initial capital contribution',
    lines: [
      { accountId: 'acc-001', accountCode: '1000', accountName: 'Cash', debitMinor: 1000000, creditMinor: 0 },
      { accountId: 'acc-004', accountCode: '3000', accountName: 'Equity', debitMinor: 0, creditMinor: 1000000 },
    ],
  },
  {
    id: 'entry-002',
    entryNumber: 'JE-2025-002',
    date: '2025-01-20',
    memo: 'Sales invoice #1001',
    lines: [
      { accountId: 'acc-002', accountCode: '1100', accountName: 'Accounts Receivable', debitMinor: 500000, creditMinor: 0 },
      { accountId: 'acc-005', accountCode: '4000', accountName: 'Revenue', debitMinor: 0, creditMinor: 500000 },
    ],
  },
];

const mockContext = {
  companyId: 'comp-001',
  chartOfAccounts: mockAccounts,
  policyRules: {
    autoPostEnabled: true,
    autoPostLimitMinor: 100000,
    confidenceFloor: 0.85,
  },
  recentTransactions: mockEntries,
  vendorMappings: {
    'Acme Supplies': 'acc-006',
    'Office Depot': 'acc-006',
  },
};

// Create HTTP server
const server = http.createServer((req, res) => {
  // Handle CORS
  res.setHeader('Access-Control-Allow-Origin', '*');
  res.setHeader('Access-Control-Allow-Methods', 'POST, OPTIONS');
  res.setHeader('Access-Control-Allow-Headers', 'Content-Type');
  
  if (req.method === 'OPTIONS') {
    res.writeHead(200);
    res.end();
    return;
  }

  if (req.method !== 'POST' || req.url !== '/api') {
    res.writeHead(404, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({ error: 'Not found' }));
    return;
  }

  let body = '';
  req.on('data', chunk => {
    body += chunk.toString();
  });

  req.on('end', () => {
    try {
      const { method, params, id } = JSON.parse(body);
      console.log(`[Mock Server] Received request: ${method}`, params);

      let result;

      switch (method) {
        case 'initialize':
          // Handle initialization request
          result = { userAgent: 'Codex Mock Server/1.0.0' };
          break;

        case 'ledgerListCompanies':
          const search = params?.search || '';
          const filteredCompanies = search
            ? mockCompanies.filter((c) => c.name.toLowerCase().includes(search.toLowerCase()))
            : mockCompanies;
          result = { companies: filteredCompanies };
          break;

        case 'ledgerListAccounts':
          const accountType = params?.accountType;
          const filteredAccounts = accountType
            ? mockAccounts.filter((a) => a.accountType === accountType)
            : mockAccounts;
          result = { accounts: filteredAccounts };
          break;

        case 'ledgerListEntries':
          result = { entries: mockEntries, total: mockEntries.length };
          break;

        case 'ledgerGetCompanyContext':
          result = mockContext;
          break;

        case 'ledgerProcessDocument':
          result = {
            suggestion: {
              extractedData: {
                vendor: 'Acme Supplies',
                invoiceNumber: 'INV-2025-001',
                date: '2025-01-25',
                amountMinor: 15000,
                confidence: 0.92,
              },
              proposedEntry: {
                memo: 'Office supplies purchase',
                lines: [
                  { accountCode: '5000', accountName: 'Expenses', debitMinor: 15000, creditMinor: 0 },
                  { accountCode: '1000', accountName: 'Cash', debitMinor: 0, creditMinor: 15000 },
                ],
                confidence: 0.88,
              },
            },
          };
          break;

        default:
          res.writeHead(200, { 'Content-Type': 'application/json' });
          res.end(JSON.stringify({
            jsonrpc: '2.0',
            id,
            error: {
              code: -32601,
              message: `Method not found: ${method}`,
            },
          }));
          return;
      }

      res.writeHead(200, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify({
        jsonrpc: '2.0',
        id,
        result,
      }));
    } catch (error) {
      res.writeHead(200, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify({
        jsonrpc: '2.0',
        id: null,
        error: {
          code: -32603,
          message: error.message,
        },
      }));
    }
  });
});

server.listen(PORT, () => {
  console.log(`🚀 Mock API server running on http://localhost:${PORT}`);
  console.log(`📊 Endpoints available: ledgerListCompanies, ledgerListAccounts, ledgerListEntries, ledgerGetCompanyContext, ledgerProcessDocument`);
});
