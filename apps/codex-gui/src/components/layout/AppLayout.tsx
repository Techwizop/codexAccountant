import { ReactNode } from 'react'
import { Sidebar } from './Sidebar'
import { Header } from './Header'

interface AppLayoutProps {
  children: ReactNode
  currentPage: string
  onNavigate: (page: string) => void
  selectedCompanyId: string | null
  onSelectCompany: (companyId: string | null) => void
}

export function AppLayout({
  children,
  currentPage,
  onNavigate,
  selectedCompanyId,
  onSelectCompany,
}: AppLayoutProps) {
  return (
    <div className="flex h-screen bg-gray-50">
      <Sidebar currentPage={currentPage} onNavigate={onNavigate} />
      <div className="flex flex-1 flex-col overflow-hidden">
        <Header selectedCompanyId={selectedCompanyId} onSelectCompany={onSelectCompany} />
        <main className="flex-1 overflow-auto p-6">{children}</main>
      </div>
    </div>
  )
}
