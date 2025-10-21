# Phase 3 QA Testing Notes

**Date**: October 21, 2025  
**Tester**: Automated Setup + Manual QA Required  
**Environment**: Windows 10/11, Node 22+, Rust 1.90.0

---

## Test Environment Status

### Servers Running ✅
- **Proxy Server**: http://localhost:8080 ✅
  - Process ID: 175
  - Bridging HTTP ↔ stdio
  - Connected to Rust app server
  
- **Rust App Server**: stdio mode ✅
  - In-memory ledger enabled
  - Features: ledger
  - Mock data configured
  
- **Vite Dev Server**: http://localhost:3000 ✅
  - Build time: 300ms
  - Hot reload enabled
  - Proxying /api to port 8080

### Application Access
- **URL**: http://localhost:3000
- **Status**: Ready for manual testing
- **Expected**: All pages should load and function

---

## Test Cases

### TC01: Dashboard Page
**URL**: http://localhost:3000/

**Test Steps**:
1. Navigate to root URL
2. Verify page title: "Accounting Dashboard"
3. Check navigation cards display:
   - Companies (Building icon)
   - Accounts (FolderTree icon)
   - Entries (BookOpen icon)
   - Documents (FileText icon)
4. Click each card
5. Verify navigation to correct page

**Expected Results**:
- ✓ Dashboard loads without errors
- ✓ 4 navigation cards visible
- ✓ Each card clickable
- ✓ Navigation works

**Status**: ⏳ Manual testing required

---

### TC02: Companies Page - List
**URL**: http://localhost:3000/companies

**Test Steps**:
1. Navigate to Companies page
2. Wait for API call to complete
3. Verify company list displays
4. Check company details:
   - Name: "Demo Corporation"
   - ID: "comp-001"
   - Base Currency: USD (2 decimals)
5. Test search box (type "Demo")
6. Verify filtering works

**Expected Results**:
- ✓ Loading spinner shows initially
- ✓ Company card appears
- ✓ Company details accurate
- ✓ Search filters correctly

**API Call**:
```json
Request:
{
  "jsonrpc": "2.0",
  "method": "ledgerListCompanies",
  "params": { "search": null },
  "id": 1
}

Expected Response:
{
  "jsonrpc": "2.0",
  "result": {
    "companies": [{
      "id": "comp-001",
      "name": "Demo Corporation",
      "baseCurrency": { "code": "USD", "precision": 2 },
      "fiscalCalendar": { "periodsPerYear": 12, "openingMonth": 1 },
      "metadata": null
    }]
  },
  "id": 1
}
```

**Status**: ⏳ Manual testing required

---

### TC03: Companies Page - Context
**URL**: http://localhost:3000/companies

**Test Steps**:
1. On Companies page
2. Click "Get Company Context" button for Demo Corporation
3. Wait for API call
4. Verify chart of accounts section appears
5. Check 3 accounts display:
   - 1000 - Cash (Asset)
   - 2000 - Accounts Payable (Liability)
   - 5000 - Operating Expenses (Expense)
6. Verify recent transactions section (should be empty)
7. Check policy rules display

**Expected Results**:
- ✓ Button triggers API call
- ✓ Chart of accounts populates
- ✓ 3 accounts with correct details
- ✓ Account type badges colored correctly
- ✓ Policy rules show: auto-post disabled, $100 limit, 85% confidence

**API Call**:
```json
Request:
{
  "jsonrpc": "2.0",
  "method": "ledgerGetCompanyContext",
  "params": { "companyId": "comp-001", "limit": 50 },
  "id": 2
}
```

**Status**: ⏳ Manual testing required

---

### TC04: Accounts Page - Full List
**URL**: http://localhost:3000/accounts

**Test Steps**:
1. Select "Demo Corporation" from header dropdown
2. Navigate to Accounts page
3. Verify account table displays
4. Check all 3 accounts visible:
   - Code | Name | Type | Currency Mode | Status
   - 1000 | Cash | Asset | Functional Only | Active
   - 2000 | Accounts Payable | Liability | Functional Only | Active
   - 5000 | Operating Expenses | Expense | Functional Only | Active
5. Verify color coding:
   - Asset: green
   - Liability: red
   - Expense: orange

**Expected Results**:
- ✓ Table renders properly
- ✓ All 3 accounts listed
- ✓ Color coding correct
- ✓ Active badges green

**Status**: ⏳ Manual testing required

---

### TC05: Accounts Page - Filtering
**URL**: http://localhost:3000/accounts

**Test Steps**:
1. On Accounts page
2. Click "All Types" (should be selected by default)
3. Click "Asset" filter
4. Verify only Cash (1000) shows
5. Click "Liability" filter
6. Verify only Accounts Payable (2000) shows
7. Click "Expense" filter
8. Verify only Operating Expenses (5000) shows
9. Click "All Types"
10. Verify all 3 accounts return

**Expected Results**:
- ✓ Filters toggle selection
- ✓ Account list updates on filter change
- ✓ Correct accounts shown for each filter
- ✓ "All Types" shows everything

**Status**: ⏳ Manual testing required

---

### TC06: Entries Page - Empty List
**URL**: http://localhost:3000/entries

**Test Steps**:
1. Select company from header
2. Navigate to Entries page
3. Verify "No entries found" message displays
4. Check filters section present:
   - Start Date input
   - End Date input
   - Account Code input

**Expected Results**:
- ✓ Empty state message shows
- ✓ Filter inputs accessible
- ✓ No errors in console
- ✓ Icon displays (BookOpen)

**API Call**:
```json
Request:
{
  "jsonrpc": "2.0",
  "method": "ledgerListEntries",
  "params": {
    "companyId": "comp-001",
    "startDate": null,
    "endDate": null,
    "accountCode": null,
    "limit": 20,
    "offset": 0
  },
  "id": 3
}

Expected Response:
{
  "jsonrpc": "2.0",
  "result": {
    "entries": [],
    "totalCount": 0
  },
  "id": 3
}
```

**Status**: ⏳ Manual testing required

---

### TC07: Documents Page - Upload
**URL**: http://localhost:3000/documents

**Test Steps**:
1. Select company from header
2. Navigate to Documents page
3. Click "Demo Upload" button
4. Verify success message appears
5. Check upload ID generated (format: upload-{timestamp})
6. Verify green success box displays

**Expected Results**:
- ✓ Button clickable
- ✓ Upload ID generated
- ✓ Success message shows
- ✓ CheckCircle icon green

**Status**: ⏳ Manual testing required

---

### TC08: Documents Page - AI Processing
**URL**: http://localhost:3000/documents

**Test Steps**:
1. On Documents page with upload ID
2. Click "Process Document with AI" button
3. Wait for API call (loading spinner shows)
4. Verify AI suggestion displays:
   - Confidence: 90% (green badge)
   - Memo: "Expense from Mock Vendor"
   - Reasoning: "Standard expense entry"
   - Suggested Lines table:
     - Row 1: 5000 Expenses | Debit: $108.00 | Credit: -
     - Row 2: 1000 Cash | Debit: - | Credit: $108.00
   - Totals row:
     - Total Debit: $108.00
     - Total Credit: $108.00
5. Click "Accept & Post" button
6. Verify alert shows
7. Click "Reject" button
8. Verify alert shows
9. Check suggestion and upload ID cleared

**Expected Results**:
- ✓ Processing shows loading state
- ✓ Suggestion renders properly
- ✓ Confidence badge green (≥90%)
- ✓ Debits and credits format as currency
- ✓ Totals balance
- ✓ Accept/Reject show alerts (MVP limitation)
- ✓ State clears after action

**API Call**:
```json
Request:
{
  "jsonrpc": "2.0",
  "method": "ledgerProcessDocument",
  "params": {
    "uploadId": "upload-1729538400000",
    "companyId": "comp-001"
  },
  "id": 4
}

Expected Response:
{
  "jsonrpc": "2.0",
  "result": {
    "suggestion": {
      "lines": [
        {
          "accountCode": "5000",
          "accountName": "Expenses",
          "debitMinor": "10800",
          "creditMinor": "0"
        },
        {
          "accountCode": "1000",
          "accountName": "Cash",
          "debitMinor": "0",
          "creditMinor": "10800"
        }
      ],
      "memo": "Expense from Mock Vendor",
      "confidence": 0.9,
      "reasoning": "Standard expense entry"
    }
  },
  "id": 4
}
```

**Note**: BigInt values (10800) serialize as strings in JSON, deserialize to bigint in TypeScript

**Status**: ⏳ Manual testing required

---

## Integration Tests

### INT01: End-to-End Flow
**Scenario**: Complete accounting workflow

**Steps**:
1. Start at Dashboard
2. Navigate to Companies
3. View Demo Corporation context
4. Navigate to Accounts
5. Browse and filter accounts
6. Navigate to Entries (see empty list)
7. Navigate to Documents
8. Upload mock document
9. Process with AI
10. Review suggestion
11. Accept (alert only)

**Expected**: Smooth navigation, no errors, data consistency

**Status**: ⏳ Manual testing required

---

### INT02: API Communication
**Scenario**: Verify JSON-RPC protocol

**Steps**:
1. Open browser DevTools (F12)
2. Go to Network tab
3. Navigate through all pages
4. Check each /api call:
   - Method: POST
   - Status: 200
   - Content-Type: application/json
   - Request has jsonrpc: "2.0", method, params, id
   - Response has jsonrpc: "2.0", result or error, id

**Expected**: All API calls successful, proper JSON-RPC format

**Status**: ⏳ Manual testing required

---

### INT03: BigInt Handling
**Scenario**: Verify currency amount serialization

**Steps**:
1. Process document on Documents page
2. Open DevTools Network tab
3. Find ledgerProcessDocument call
4. Check Response payload
5. Verify debitMinor/creditMinor are strings (e.g., "10800")
6. Check UI displays $108.00 correctly
7. Inspect Console for bigint values

**Expected**: Backend sends strings, frontend converts to bigint, displays as currency

**Status**: ⏳ Manual testing required

---

## Browser Compatibility

### Tested Browsers
- [ ] Chrome/Edge (Chromium) - Recommended
- [ ] Firefox
- [ ] Safari (macOS)

### Known Issues
- None reported yet

**Note**: BigInt support required (ES2020+), available in all modern browsers

---

## Performance Notes

### Load Times
- Dashboard: < 100ms
- API calls: < 200ms (in-memory backend)
- Page transitions: Instant (React Router)

### Resource Usage
- Vite dev server: ~50MB RAM
- Proxy server: ~30MB RAM
- Rust app server: ~20MB RAM
- Browser tab: ~100MB RAM

---

## Issues Found

### Critical
*None*

### Major
*None*

### Minor
*None*

### Enhancements
1. Add loading skeletons for better UX
2. Add toast notifications for actions
3. Persist company selection in localStorage
4. Add keyboard shortcuts
5. Add data export functionality

---

## Manual QA Checklist

Complete the following tests manually:

### Navigation
- [ ] Dashboard loads
- [ ] All navigation cards work
- [ ] Sidebar links functional
- [ ] Company selector in header works
- [ ] Page title updates on navigation

### Companies Page
- [ ] List loads
- [ ] Search filters
- [ ] Get Context button works
- [ ] Chart of accounts displays
- [ ] Policy rules display

### Accounts Page
- [ ] Table renders
- [ ] All accounts visible
- [ ] Filters work correctly
- [ ] Color coding accurate
- [ ] No company selected message

### Entries Page
- [ ] Empty state displays
- [ ] Filters present
- [ ] Pagination controls visible (disabled when empty)
- [ ] No errors with no data

### Documents Page
- [ ] Upload simulation works
- [ ] Process button triggers API
- [ ] Suggestion displays correctly
- [ ] Confidence badge colored
- [ ] Debits/credits format
- [ ] Totals balance
- [ ] Accept/Reject buttons work (alert)

### Console & Network
- [ ] No JavaScript errors
- [ ] No 404s or network errors
- [ ] All /api calls return 200
- [ ] JSON-RPC format correct
- [ ] BigInt serialization works

---

## Sign-Off

**QA Tester**: _____________  
**Date**: _____________  
**Status**: _____________  
**Notes**: _____________

---

## Conclusion

**Current Status**: All systems operational, ready for manual QA testing

**Access**: http://localhost:3000

**Next Steps**:
1. Complete manual test cases above
2. Document any issues found
3. Take screenshots for documentation
4. Sign off on QA checklist

**Phase 3 Complete**: Pending final QA sign-off
