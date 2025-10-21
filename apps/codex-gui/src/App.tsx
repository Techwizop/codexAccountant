import { useState } from 'react'
import { AppLayout } from './components/layout/AppLayout'
import { CompaniesPage } from './pages/CompaniesPage'
import { AccountsPage } from './pages/AccountsPage'
import { EntriesPage } from './pages/EntriesPage'
import { DocumentsPage } from './pages/DocumentsPage'
import { DashboardPage } from './pages/DashboardPage'

type Page = 'dashboard' | 'companies' | 'accounts' | 'entries' | 'documents'

function App() {
  const [currentPage, setCurrentPage] = useState<Page>('dashboard')
  const [selectedCompanyId, setSelectedCompanyId] = useState<string | null>(null)

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
