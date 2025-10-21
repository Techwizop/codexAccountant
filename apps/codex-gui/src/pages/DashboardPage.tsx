import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Building2, FolderTree, BookOpen, FileText, ArrowRight } from 'lucide-react'

interface DashboardPageProps {
  onNavigate: (page: string) => void
}

export function DashboardPage({ onNavigate }: DashboardPageProps) {
  const cards = [
    {
      id: 'companies',
      title: 'Companies',
      description: 'Manage your companies and view their context',
      icon: Building2,
      color: 'text-blue-600',
      bgColor: 'bg-blue-50',
    },
    {
      id: 'accounts',
      title: 'Chart of Accounts',
      description: 'Browse and filter accounts by type',
      icon: FolderTree,
      color: 'text-green-600',
      bgColor: 'bg-green-50',
    },
    {
      id: 'entries',
      title: 'Journal Entries',
      description: 'View and search journal entries with pagination',
      icon: BookOpen,
      color: 'text-purple-600',
      bgColor: 'bg-purple-50',
    },
    {
      id: 'documents',
      title: 'Document Processing',
      description: 'Upload documents and review AI suggestions',
      icon: FileText,
      color: 'text-orange-600',
      bgColor: 'bg-orange-50',
    },
  ]

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-3xl font-bold">Dashboard</h1>
        <p className="mt-2 text-gray-600">
          Welcome to Codex Accounting. Select a workflow to get started.
        </p>
      </div>

      <div className="grid gap-6 md:grid-cols-2 lg:grid-cols-2">
        {cards.map((card) => {
          const Icon = card.icon
          return (
            <Card key={card.id} className="hover:shadow-lg transition-shadow cursor-pointer">
              <CardHeader>
                <div className="flex items-center gap-4">
                  <div className={`${card.bgColor} p-3 rounded-lg`}>
                    <Icon className={`h-6 w-6 ${card.color}`} />
                  </div>
                  <div>
                    <CardTitle>{card.title}</CardTitle>
                    <CardDescription className="mt-1">{card.description}</CardDescription>
                  </div>
                </div>
              </CardHeader>
              <CardContent>
                <Button variant="outline" className="w-full gap-2" onClick={() => onNavigate(card.id)}>
                  Go to {card.title}
                  <ArrowRight className="h-4 w-4" />
                </Button>
              </CardContent>
            </Card>
          )
        })}
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Getting Started</CardTitle>
          <CardDescription>Quick guide to using Codex Accounting</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="space-y-2">
            <h3 className="font-semibold">1. Select a Company</h3>
            <p className="text-sm text-gray-600">
              Use the company selector in the header to choose which company you want to work with.
            </p>
          </div>
          <div className="space-y-2">
            <h3 className="font-semibold">2. Browse Accounting Data</h3>
            <p className="text-sm text-gray-600">
              View your chart of accounts, journal entries, and company context through the navigation menu.
            </p>
          </div>
          <div className="space-y-2">
            <h3 className="font-semibold">3. Process Documents</h3>
            <p className="text-sm text-gray-600">
              Upload invoices and receipts to get AI-powered journal entry suggestions.
            </p>
          </div>
        </CardContent>
      </Card>
    </div>
  )
}
