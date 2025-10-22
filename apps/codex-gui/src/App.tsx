import { useState, useEffect } from 'react'
import { AppLayout } from './components/layout/AppLayout'
import { CompaniesPage } from './pages/CompaniesPage'
import { AccountsPage } from './pages/AccountsPage'
import { EntriesPage } from './pages/EntriesPage'
import { DocumentsPage } from './pages/DocumentsPage'
import { DashboardPage } from './pages/DashboardPage'
import { apiClient } from './api/client'

type Page = 'dashboard' | 'companies' | 'accounts' | 'entries' | 'documents'

function App() {
  const [currentPage, setCurrentPage] = useState<Page>('dashboard')
  const [selectedCompanyId, setSelectedCompanyId] = useState<string | null>(null)
  const [isInitializing, setIsInitializing] = useState(true)
  const [initError, setInitError] = useState<string | null>(null)

  // Initialize the API client when the app loads
  useEffect(() => {
    async function initializeApiClient() {
      try {
        await apiClient.initialize({
          name: 'Codex Accounting Web UI',
          title: null,
          version: '0.1.0',
        })
        console.log('API client initialized successfully')
        setIsInitializing(false)
      } catch (error) {
        console.error('Failed to initialize API client:', error)
        setInitError(error instanceof Error ? error.message : 'Unknown error')
        setIsInitializing(false)
      }
    }

    initializeApiClient()
  }, [])

  const handleNavigate = (page: string) => {
    setCurrentPage(page as Page)
  }

  const renderPage = () => {
    switch (currentPage) {
      case 'dashboard':
        return <DashboardPage onNavigate={handleNavigate} />
      case 'companies':
        return <CompaniesPage onSelectCompany={setSelectedCompanyId} />
      case 'accounts':
        return <AccountsPage companyId={selectedCompanyId} />
      case 'entries':
        return <EntriesPage companyId={selectedCompanyId} />
      case 'documents':
        return <DocumentsPage companyId={selectedCompanyId} />
      default:
        return <DashboardPage onNavigate={handleNavigate} />
    }
  }

  // Show loading state while initializing
  if (isInitializing) {
    return (
      <div className="flex items-center justify-center min-h-screen">
        <div className="text-center">
          <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-primary mx-auto mb-4"></div>
          <p className="text-lg font-medium">Initializing...</p>
        </div>
      </div>
    )
  }

  // Show error state if initialization failed
  if (initError) {
    return (
      <div className="flex items-center justify-center min-h-screen">
        <div className="text-center max-w-md">
          <div className="text-red-500 mb-4">
            <svg className="h-12 w-12 mx-auto" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
            </svg>
          </div>
          <h2 className="text-xl font-bold mb-2">Initialization Failed</h2>
          <p className="text-gray-600 mb-4">{initError}</p>
          <button
            onClick={() => window.location.reload()}
            className="px-4 py-2 bg-primary text-white rounded-lg hover:bg-primary/90"
          >
            Retry
          </button>
        </div>
      </div>
    )
  }

  return (
    <AppLayout
      currentPage={currentPage}
      onNavigate={handleNavigate}
      selectedCompanyId={selectedCompanyId}
      onSelectCompany={setSelectedCompanyId}
    >
      {renderPage()}
    </AppLayout>
  )
}

export default App
