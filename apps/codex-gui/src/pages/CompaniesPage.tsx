import { useState } from 'react'
import { useCompanies, useCompanyContext } from '@/api/hooks'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { Input } from '@/components/ui/input'
import { Building2, Search, Loader2, AlertCircle } from 'lucide-react'
import { formatCurrency, getAccountTypeColor, formatAccountType } from '@/lib/format'

interface CompaniesPageProps {
  onSelectCompany: (companyId: string) => void
}

export function CompaniesPage({ onSelectCompany }: CompaniesPageProps) {
  const [search, setSearch] = useState('')
  const [selectedId, setSelectedId] = useState<string | null>(null)

  const { data, isLoading, error } = useCompanies({ search })
  const { data: contextData, isLoading: isLoadingContext } = useCompanyContext(
    { companyId: selectedId || '', limit: 50 },
    { enabled: !!selectedId }
  )

  const handleSelectCompany = (id: string) => {
    setSelectedId(id)
    onSelectCompany(id)
  }

  if (error) {
    return (
      <div className="flex items-center justify-center h-96">
        <div className="text-center space-y-4">
          <AlertCircle className="h-12 w-12 text-red-500 mx-auto" />
          <p className="text-lg font-medium">Failed to load companies</p>
          <p className="text-sm text-gray-600">{error.message}</p>
        </div>
      </div>
    )
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold">Companies</h1>
          <p className="mt-2 text-gray-600">View and manage your companies</p>
        </div>
      </div>

      <div className="relative">
        <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-gray-400" />
        <Input
          placeholder="Search companies..."
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          className="pl-10"
        />
      </div>

      {isLoading ? (
        <div className="flex items-center justify-center h-64">
          <Loader2 className="h-8 w-8 animate-spin text-primary" />
        </div>
      ) : (
        <div className="grid gap-6 md:grid-cols-2">
          {data?.companies.map((company) => (
            <Card
              key={company.id}
              className={`cursor-pointer transition-all ${
                selectedId === company.id
                  ? 'ring-2 ring-primary shadow-lg'
                  : 'hover:shadow-md'
              }`}
              onClick={() => handleSelectCompany(company.id)}
            >
              <CardHeader>
                <div className="flex items-start justify-between">
                  <div className="flex items-center gap-3">
                    <div className="p-2 bg-blue-50 rounded-lg">
                      <Building2 className="h-6 w-6 text-blue-600" />
                    </div>
                    <div>
                      <CardTitle>{company.name}</CardTitle>
                      <CardDescription className="mt-1">ID: {company.id}</CardDescription>
                    </div>
                  </div>
                </div>
              </CardHeader>
              <CardContent>
                <div className="space-y-3">
                  <div>
                    <p className="text-sm font-medium text-gray-700">Base Currency</p>
                    <p className="text-sm text-gray-600">
                      {company.baseCurrency.code} (Precision: {company.baseCurrency.precision})
                    </p>
                  </div>
                  <div>
                    <p className="text-sm font-medium text-gray-700">Fiscal Calendar</p>
                    <p className="text-sm text-gray-600">
                      {company.fiscalCalendar.periodsPerYear} periods/year, Opens month{' '}
                      {company.fiscalCalendar.openingMonth}
                    </p>
                  </div>
                </div>
              </CardContent>
            </Card>
          ))}
        </div>
      )}

      {selectedId && contextData && (
        <Card>
          <CardHeader>
            <CardTitle>Company Context</CardTitle>
            <CardDescription>
              Context information for AI-powered accounting operations
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-6">
            {isLoadingContext ? (
              <div className="flex items-center justify-center py-8">
                <Loader2 className="h-6 w-6 animate-spin text-primary" />
              </div>
            ) : (
              <>
                <div>
                  <h3 className="font-semibold mb-3">Chart of Accounts</h3>
                  <div className="space-y-2">
                    {contextData.chartOfAccounts.slice(0, 5).map((account) => (
                      <div
                        key={account.id}
                        className="flex items-center justify-between p-3 bg-gray-50 rounded-lg"
                      >
                        <div className="flex items-center gap-3">
                          <span className="font-mono text-sm font-medium">{account.code}</span>
                          <span className="text-sm">{account.name}</span>
                        </div>
                        <Badge className={getAccountTypeColor(account.accountType)}>
                          {formatAccountType(account.accountType)}
                        </Badge>
                      </div>
                    ))}
                    {contextData.chartOfAccounts.length > 5 && (
                      <p className="text-sm text-gray-600 text-center py-2">
                        + {contextData.chartOfAccounts.length - 5} more accounts
                      </p>
                    )}
                  </div>
                </div>

                <div>
                  <h3 className="font-semibold mb-3">Policy Rules</h3>
                  <div className="grid grid-cols-3 gap-4">
                    <div className="p-3 bg-gray-50 rounded-lg">
                      <p className="text-sm text-gray-600">Auto Post Enabled</p>
                      <p className="text-lg font-semibold">
                        {contextData.policyRules.autoPostEnabled ? 'Yes' : 'No'}
                      </p>
                    </div>
                    <div className="p-3 bg-gray-50 rounded-lg">
                      <p className="text-sm text-gray-600">Auto Post Limit</p>
                      <p className="text-lg font-semibold">
                        {formatCurrency(
                          contextData.policyRules.autoPostLimitMinor,
                          { code: 'USD', precision: 2 }
                        )}
                      </p>
                    </div>
                    <div className="p-3 bg-gray-50 rounded-lg">
                      <p className="text-sm text-gray-600">Confidence Floor</p>
                      <p className="text-lg font-semibold">
                        {(contextData.policyRules.confidenceFloor * 100).toFixed(0)}%
                      </p>
                    </div>
                  </div>
                </div>

                <div>
                  <h3 className="font-semibold mb-3">Recent Transactions</h3>
                  <p className="text-sm text-gray-600">
                    {contextData.recentTransactions.length} recent transactions
                  </p>
                </div>

                <div>
                  <h3 className="font-semibold mb-3">Vendor Mappings</h3>
                  <p className="text-sm text-gray-600">
                    {Object.keys(contextData.vendorMappings).length} vendor mappings configured
                  </p>
                </div>
              </>
            )}
          </CardContent>
        </Card>
      )}

      {data?.companies.length === 0 && (
        <div className="text-center py-12">
          <Building2 className="h-12 w-12 text-gray-400 mx-auto mb-4" />
          <p className="text-lg font-medium text-gray-900">No companies found</p>
          <p className="text-sm text-gray-600 mt-1">Try adjusting your search criteria</p>
        </div>
      )}
    </div>
  )
}
