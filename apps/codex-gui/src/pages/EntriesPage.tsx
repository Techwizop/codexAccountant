import { useState } from 'react'
import { useEntries } from '@/api/hooks'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Loader2, AlertCircle, BookOpen, ChevronLeft, ChevronRight } from 'lucide-react'
import { formatCurrency, formatAccountType } from '@/lib/format'
import type { LedgerJournalEntry } from '@/types/protocol'

interface EntriesPageProps {
  companyId: string | null
}

export function EntriesPage({ companyId }: EntriesPageProps) {
  const [limit] = useState(20)
  const [offset, setOffset] = useState(0)
  const [startDate, setStartDate] = useState('')
  const [endDate, setEndDate] = useState('')
  const [accountCode, setAccountCode] = useState('')
  const [selectedEntry, setSelectedEntry] = useState<LedgerJournalEntry | null>(null)

  const { data, isLoading, error } = useEntries(
    {
      companyId: companyId || '',
      startDate: startDate || null,
      endDate: endDate || null,
      accountCode: accountCode || null,
      limit,
      offset,
    },
    { enabled: !!companyId }
  )

  const handlePrevPage = () => {
    setOffset(Math.max(0, offset - limit))
  }

  const handleNextPage = () => {
    if (data && offset + limit < data.totalCount) {
      setOffset(offset + limit)
    }
  }

  if (!companyId) {
    return (
      <div className="flex items-center justify-center h-96">
        <div className="text-center space-y-4">
          <BookOpen className="h-12 w-12 text-gray-400 mx-auto" />
          <p className="text-lg font-medium">No Company Selected</p>
          <p className="text-sm text-gray-600">Please select a company to view entries</p>
        </div>
      </div>
    )
  }

  if (error) {
    return (
      <div className="flex items-center justify-center h-96">
        <div className="text-center space-y-4">
          <AlertCircle className="h-12 w-12 text-red-500 mx-auto" />
          <p className="text-lg font-medium">Failed to load entries</p>
          <p className="text-sm text-gray-600">{error.message}</p>
        </div>
      </div>
    )
  }

  const getStatusColor = (status: string) => {
    const colors: Record<string, string> = {
      draft: 'bg-gray-100 text-gray-800',
      proposed: 'bg-yellow-100 text-yellow-800',
      posted: 'bg-green-100 text-green-800',
      reversed: 'bg-red-100 text-red-800',
    }
    return colors[status] || 'bg-gray-100 text-gray-800'
  }

  const getOriginColor = (origin: string) => {
    const colors: Record<string, string> = {
      manual: 'bg-blue-100 text-blue-800',
      ingestion: 'bg-purple-100 text-purple-800',
      aiSuggested: 'bg-indigo-100 text-indigo-800',
      adjustment: 'bg-orange-100 text-orange-800',
    }
    return colors[origin] || 'bg-gray-100 text-gray-800'
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold">Journal Entries</h1>
          <p className="mt-2 text-gray-600">Browse and search journal entries</p>
        </div>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Filters</CardTitle>
          <CardDescription>Filter entries by date range and account</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">Start Date</label>
              <Input
                type="date"
                value={startDate}
                onChange={(e) => setStartDate(e.target.value)}
              />
            </div>
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">End Date</label>
              <Input
                type="date"
                value={endDate}
                onChange={(e) => setEndDate(e.target.value)}
              />
            </div>
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">Account Code</label>
              <Input
                placeholder="e.g., 1000"
                value={accountCode}
                onChange={(e) => setAccountCode(e.target.value)}
              />
            </div>
          </div>
        </CardContent>
      </Card>

      {isLoading ? (
        <div className="flex items-center justify-center h-64">
          <Loader2 className="h-8 w-8 animate-spin text-primary" />
        </div>
      ) : (
        <>
          <div className="space-y-4">
            {data?.entries.map((entry) => (
              <Card
                key={entry.id}
                className={`cursor-pointer transition-all ${
                  selectedEntry?.id === entry.id ? 'ring-2 ring-primary' : 'hover:shadow-md'
                }`}
                onClick={() => setSelectedEntry(entry)}
              >
                <CardHeader>
                  <div className="flex items-center justify-between">
                    <div>
                      <CardTitle className="text-lg">Entry {entry.id}</CardTitle>
                      <CardDescription className="mt-1">
                        {entry.memo || 'No memo'}
                      </CardDescription>
                    </div>
                    <div className="flex gap-2">
                      <Badge className={getStatusColor(entry.status)}>
                        {formatAccountType(entry.status)}
                      </Badge>
                      <Badge className={getOriginColor(entry.origin)}>
                        {formatAccountType(entry.origin)}
                      </Badge>
                    </div>
                  </div>
                </CardHeader>
                <CardContent>
                  <div className="text-sm text-gray-600">
                    {entry.lines.length} line{entry.lines.length !== 1 ? 's' : ''} | Journal:{' '}
                    {entry.journalId}
                  </div>
                </CardContent>
              </Card>
            ))}
          </div>

          {selectedEntry && (
            <Card>
              <CardHeader>
                <CardTitle>Entry Details</CardTitle>
                <CardDescription>Debit and credit lines for this entry</CardDescription>
              </CardHeader>
              <CardContent>
                <div className="rounded-lg border">
                  <table className="w-full">
                    <thead>
                      <tr className="border-b bg-gray-50">
                        <th className="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase">
                          Account
                        </th>
                        <th className="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase">
                          Side
                        </th>
                        <th className="px-4 py-3 text-right text-xs font-medium text-gray-500 uppercase">
                          Amount
                        </th>
                        <th className="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase">
                          Memo
                        </th>
                      </tr>
                    </thead>
                    <tbody className="divide-y divide-gray-200">
                      {selectedEntry.lines.map((line) => (
                        <tr key={line.id}>
                          <td className="px-4 py-3 text-sm">{line.accountId}</td>
                          <td className="px-4 py-3">
                            <Badge variant={line.side === 'debit' ? 'default' : 'secondary'}>
                              {line.side}
                            </Badge>
                          </td>
                          <td className="px-4 py-3 text-sm text-right font-mono">
                            {formatCurrency(line.amountMinor, line.currency)}
                          </td>
                          <td className="px-4 py-3 text-sm text-gray-600">
                            {line.memo || '-'}
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </CardContent>
            </Card>
          )}

          {data && data.totalCount > 0 && (
            <div className="flex items-center justify-between">
              <p className="text-sm text-gray-600">
                Showing {offset + 1} to {Math.min(offset + limit, data.totalCount)} of{' '}
                {data.totalCount} entries
              </p>
              <div className="flex gap-2">
                <Button
                  variant="outline"
                  size="sm"
                  onClick={handlePrevPage}
                  disabled={offset === 0}
                >
                  <ChevronLeft className="h-4 w-4 mr-1" />
                  Previous
                </Button>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={handleNextPage}
                  disabled={offset + limit >= data.totalCount}
                >
                  Next
                  <ChevronRight className="h-4 w-4 ml-1" />
                </Button>
              </div>
            </div>
          )}

          {data?.entries.length === 0 && (
            <div className="text-center py-12">
              <BookOpen className="h-12 w-12 text-gray-400 mx-auto mb-4" />
              <p className="text-lg font-medium text-gray-900">No entries found</p>
              <p className="text-sm text-gray-600 mt-1">
                Try adjusting your filter criteria
              </p>
            </div>
          )}
        </>
      )}
    </div>
  )
}
