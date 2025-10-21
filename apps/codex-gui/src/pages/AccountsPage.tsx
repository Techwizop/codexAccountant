import { useState } from 'react'
import { useAccounts } from '@/api/hooks'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Loader2, AlertCircle, FolderTree } from 'lucide-react'
import { formatAccountType, getAccountTypeColor } from '@/lib/format'
import type { LedgerAccountType } from '@/types/protocol'

interface AccountsPageProps {
  companyId: string | null
}

const accountTypes: LedgerAccountType[] = ['asset', 'liability', 'equity', 'revenue', 'expense', 'offBalance']

export function AccountsPage({ companyId }: AccountsPageProps) {
  const [selectedType, setSelectedType] = useState<LedgerAccountType | null>(null)

  const { data, isLoading, error } = useAccounts(
    { companyId: companyId || '', accountType: selectedType },
    { enabled: !!companyId }
  )

  if (!companyId) {
    return (
      <div className="flex items-center justify-center h-96">
        <div className="text-center space-y-4">
          <FolderTree className="h-12 w-12 text-gray-400 mx-auto" />
          <p className="text-lg font-medium">No Company Selected</p>
          <p className="text-sm text-gray-600">Please select a company to view accounts</p>
        </div>
      </div>
    )
  }

  if (error) {
    return (
      <div className="flex items-center justify-center h-96">
        <div className="text-center space-y-4">
          <AlertCircle className="h-12 w-12 text-red-500 mx-auto" />
          <p className="text-lg font-medium">Failed to load accounts</p>
          <p className="text-sm text-gray-600">{error.message}</p>
        </div>
      </div>
    )
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold">Chart of Accounts</h1>
          <p className="mt-2 text-gray-600">Browse accounts for the selected company</p>
        </div>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Filter by Account Type</CardTitle>
          <CardDescription>Select a type to filter accounts</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="flex flex-wrap gap-2">
            <Button
              variant={selectedType === null ? 'default' : 'outline'}
              size="sm"
              onClick={() => setSelectedType(null)}
            >
              All Types
            </Button>
            {accountTypes.map((type) => (
              <Button
                key={type}
                variant={selectedType === type ? 'default' : 'outline'}
                size="sm"
                onClick={() => setSelectedType(type)}
              >
                {formatAccountType(type)}
              </Button>
            ))}
          </div>
        </CardContent>
      </Card>

      {isLoading ? (
        <div className="flex items-center justify-center h-64">
          <Loader2 className="h-8 w-8 animate-spin text-primary" />
        </div>
      ) : (
        <>
          <div className="rounded-lg border bg-white">
            <div className="overflow-x-auto">
              <table className="w-full">
                <thead>
                  <tr className="border-b bg-gray-50">
                    <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                      Code
                    </th>
                    <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                      Name
                    </th>
                    <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                      Type
                    </th>
                    <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                      Currency Mode
                    </th>
                    <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                      Status
                    </th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-gray-200">
                  {data?.accounts.map((account) => (
                    <tr key={account.id} className="hover:bg-gray-50">
                      <td className="px-6 py-4 whitespace-nowrap">
                        <span className="font-mono text-sm font-medium">{account.code}</span>
                      </td>
                      <td className="px-6 py-4">
                        <div>
                          <div className="text-sm font-medium text-gray-900">{account.name}</div>
                          {account.isSummary && (
                            <div className="text-xs text-gray-500 mt-1">Summary Account</div>
                          )}
                        </div>
                      </td>
                      <td className="px-6 py-4 whitespace-nowrap">
                        <Badge className={getAccountTypeColor(account.accountType)}>
                          {formatAccountType(account.accountType)}
                        </Badge>
                      </td>
                      <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-600">
                        {formatAccountType(account.currencyMode)}
                      </td>
                      <td className="px-6 py-4 whitespace-nowrap">
                        <Badge variant={account.isActive ? 'default' : 'secondary'}>
                          {account.isActive ? 'Active' : 'Inactive'}
                        </Badge>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>

          {data?.accounts.length === 0 && (
            <div className="text-center py-12">
              <FolderTree className="h-12 w-12 text-gray-400 mx-auto mb-4" />
              <p className="text-lg font-medium text-gray-900">No accounts found</p>
              <p className="text-sm text-gray-600 mt-1">
                {selectedType
                  ? `No ${formatAccountType(selectedType)} accounts in this company`
                  : 'This company has no accounts configured'}
              </p>
            </div>
          )}
        </>
      )}
    </div>
  )
}
