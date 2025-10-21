import { useState } from 'react'
import { useProcessDocument } from '@/api/hooks'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Loader2, FileText, Upload, CheckCircle2, AlertCircle } from 'lucide-react'
import { formatCurrency, getConfidenceColor, formatConfidence } from '@/lib/format'
import type { LedgerJournalEntrySuggestion } from '@/types/protocol'

interface DocumentsPageProps {
  companyId: string | null
}

export function DocumentsPage({ companyId }: DocumentsPageProps) {
  const [uploadId, setUploadId] = useState('')
  const [suggestion, setSuggestion] = useState<LedgerJournalEntrySuggestion | null>(null)
  const processDocument = useProcessDocument()

  const handleUploadDemo = () => {
    // Mock upload - generate a random upload ID
    const mockUploadId = `upload-${Date.now()}`
    setUploadId(mockUploadId)
  }

  const handleProcess = async () => {
    if (!companyId || !uploadId) return

    try {
      const result = await processDocument.mutateAsync({
        uploadId,
        companyId,
      })
      setSuggestion(result.suggestion)
    } catch (error) {
      console.error('Failed to process document:', error)
    }
  }

  const handleAccept = () => {
    alert('Accept action would post this entry to the ledger (not implemented in MVP)')
    setSuggestion(null)
    setUploadId('')
  }

  const handleReject = () => {
    alert('Reject action would discard this suggestion (not implemented in MVP)')
    setSuggestion(null)
    setUploadId('')
  }

  if (!companyId) {
    return (
      <div className="flex items-center justify-center h-96">
        <div className="text-center space-y-4">
          <FileText className="h-12 w-12 text-gray-400 mx-auto" />
          <p className="text-lg font-medium">No Company Selected</p>
          <p className="text-sm text-gray-600">Please select a company to process documents</p>
        </div>
      </div>
    )
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold">Document Processing</h1>
          <p className="mt-2 text-gray-600">Upload documents and review AI suggestions</p>
        </div>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Upload Document</CardTitle>
          <CardDescription>
            Upload invoices, receipts, or other accounting documents
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="border-2 border-dashed border-gray-300 rounded-lg p-12 text-center hover:border-primary transition-colors cursor-pointer">
            <Upload className="h-12 w-12 text-gray-400 mx-auto mb-4" />
            <p className="text-sm font-medium text-gray-900">
              Drag and drop or click to upload
            </p>
            <p className="text-xs text-gray-500 mt-1">
              Supported formats: PDF, PNG, JPG (MVP: Mock upload only)
            </p>
          </div>
          <Button onClick={handleUploadDemo} className="w-full" variant="outline">
            <Upload className="h-4 w-4 mr-2" />
            Demo Upload (Generate Mock Upload ID)
          </Button>
          {uploadId && (
            <div className="p-4 bg-green-50 border border-green-200 rounded-lg">
              <div className="flex items-center gap-2">
                <CheckCircle2 className="h-5 w-5 text-green-600" />
                <div>
                  <p className="text-sm font-medium text-green-900">Upload successful!</p>
                  <p className="text-xs text-green-700 mt-1">Upload ID: {uploadId}</p>
                </div>
              </div>
            </div>
          )}
        </CardContent>
      </Card>

      {uploadId && !suggestion && (
        <Card>
          <CardHeader>
            <CardTitle>Process Document</CardTitle>
            <CardDescription>
              Use AI to extract data and generate journal entry suggestion
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <Button
              onClick={handleProcess}
              className="w-full"
              disabled={processDocument.isPending}
            >
              {processDocument.isPending ? (
                <>
                  <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                  Processing...
                </>
              ) : (
                <>
                  <FileText className="h-4 w-4 mr-2" />
                  Process Document with AI
                </>
              )}
            </Button>
            {processDocument.isError && (
              <div className="p-4 bg-red-50 border border-red-200 rounded-lg">
                <div className="flex items-center gap-2">
                  <AlertCircle className="h-5 w-5 text-red-600" />
                  <div>
                    <p className="text-sm font-medium text-red-900">Processing failed</p>
                    <p className="text-xs text-red-700 mt-1">
                      {processDocument.error?.message || 'Unknown error'}
                    </p>
                  </div>
                </div>
              </div>
            )}
          </CardContent>
        </Card>
      )}

      {suggestion && (
        <>
          <Card>
            <CardHeader>
              <div className="flex items-center justify-between">
                <div>
                  <CardTitle>AI Suggestion</CardTitle>
                  <CardDescription>Review the proposed journal entry</CardDescription>
                </div>
                <Badge className={getConfidenceColor(suggestion.confidence)}>
                  Confidence: {formatConfidence(suggestion.confidence)}
                </Badge>
              </div>
            </CardHeader>
            <CardContent className="space-y-6">
              <div>
                <h3 className="font-semibold mb-2">Memo</h3>
                <p className="text-sm text-gray-700">{suggestion.memo}</p>
              </div>

              <div>
                <h3 className="font-semibold mb-2">AI Reasoning</h3>
                <div className="p-4 bg-blue-50 border border-blue-200 rounded-lg">
                  <p className="text-sm text-gray-700">{suggestion.reasoning}</p>
                </div>
              </div>

              <div>
                <h3 className="font-semibold mb-3">Suggested Lines</h3>
                <div className="rounded-lg border">
                  <table className="w-full">
                    <thead>
                      <tr className="border-b bg-gray-50">
                        <th className="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase">
                          Account
                        </th>
                        <th className="px-4 py-3 text-right text-xs font-medium text-gray-500 uppercase">
                          Debit
                        </th>
                        <th className="px-4 py-3 text-right text-xs font-medium text-gray-500 uppercase">
                          Credit
                        </th>
                      </tr>
                    </thead>
                    <tbody className="divide-y divide-gray-200">
                      {suggestion.lines.map((line, index) => (
                        <tr key={index}>
                          <td className="px-4 py-3">
                            <div>
                              <p className="text-sm font-medium">{line.accountName}</p>
                              <p className="text-xs text-gray-500">{line.accountCode}</p>
                            </div>
                          </td>
                          <td className="px-4 py-3 text-right">
                            {line.debitMinor > 0 ? (
                              <span className="font-mono text-sm debit">
                                {formatCurrency(line.debitMinor, { code: 'USD', precision: 2 })}
                              </span>
                            ) : (
                              <span className="text-gray-400">-</span>
                            )}
                          </td>
                          <td className="px-4 py-3 text-right">
                            {line.creditMinor > 0 ? (
                              <span className="font-mono text-sm credit">
                                {formatCurrency(line.creditMinor, { code: 'USD', precision: 2 })}
                              </span>
                            ) : (
                              <span className="text-gray-400">-</span>
                            )}
                          </td>
                        </tr>
                      ))}
                      <tr className="bg-gray-50 font-semibold">
                        <td className="px-4 py-3 text-sm">Total</td>
                        <td className="px-4 py-3 text-right font-mono text-sm">
                          {formatCurrency(
                            suggestion.lines.reduce((sum, l) => sum + l.debitMinor, 0n),
                            { code: 'USD', precision: 2 }
                          )}
                        </td>
                        <td className="px-4 py-3 text-right font-mono text-sm">
                          {formatCurrency(
                            suggestion.lines.reduce((sum, l) => sum + l.creditMinor, 0n),
                            { code: 'USD', precision: 2 }
                          )}
                        </td>
                      </tr>
                    </tbody>
                  </table>
                </div>
              </div>

              <div className="flex gap-3">
                <Button onClick={handleAccept} className="flex-1" size="lg">
                  <CheckCircle2 className="h-4 w-4 mr-2" />
                  Accept & Post
                </Button>
                <Button onClick={handleReject} variant="destructive" className="flex-1" size="lg">
                  <AlertCircle className="h-4 w-4 mr-2" />
                  Reject
                </Button>
              </div>

              <div className="text-xs text-gray-500 text-center">
                Note: Accept and Reject actions are placeholders in this MVP
              </div>
            </CardContent>
          </Card>
        </>
      )}
    </div>
  )
}
